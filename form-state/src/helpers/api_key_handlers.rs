use std::sync::Arc;
use tokio::sync::Mutex;
use axum::{
    extract::{State, Path},
    response::IntoResponse,
    Json,
    http::StatusCode,
};
use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc, Duration};

use crate::datastore::DataStore;
use crate::auth::JwtClaims;
use crate::api_keys::{ApiKeyScope, ApiKeyMetadata, create_api_key};

/// Request to create a new API key
#[derive(Debug, Serialize, Deserialize)]
pub struct CreateApiKeyRequest {
    /// Name for the new API key
    pub name: String,
    
    /// Scope for the new API key
    pub scope: ApiKeyScope,
    
    /// Optional description
    pub description: Option<String>,
    
    /// Optional expiration date
    pub expires_at: Option<DateTime<Utc>>,
}

/// Response for API key creation
#[derive(Debug, Serialize, Deserialize)]
pub struct ApiKeyResponse {
    /// Success flag
    pub success: bool,
    
    /// API key metadata
    pub api_key: Option<ApiKeyMetadata>,
    
    /// Secret to display to the user (only included once during creation)
    pub secret: Option<String>,
    
    /// Error message if operation failed
    pub error: Option<String>,
}

/// Response for listing API keys
#[derive(Debug, Serialize, Deserialize)]
pub struct ListApiKeysResponse {
    /// Success flag
    pub success: bool,
    
    /// List of API key metadata
    pub api_keys: Vec<ApiKeyMetadata>,
    
    /// Total count
    pub total: usize,
    
    /// Maximum allowed keys
    pub max_allowed: u32,
}

/// Create a new API key
pub async fn create_api_key_handler(
    State(state): State<Arc<Mutex<DataStore>>>,
    JwtClaims(claims): JwtClaims,
    Json(request): Json<CreateApiKeyRequest>,
) -> impl IntoResponse {
    // Get the user's account
    let mut datastore = state.lock().await;
    
    // Find the account by the user's ID
    let account = match datastore.account_state.get_account(&claims.sub) {
        Some(account) => account,
        None => {
            return (
                StatusCode::NOT_FOUND,
                Json(ApiKeyResponse {
                    success: false,
                    api_key: None,
                    secret: None,
                    error: Some("Account not found".to_string()),
                }),
            );
        }
    };
    
    // Check if the name is unique
    let key_count = account.api_keys.len();
    let name_exists = account.api_keys.values().any(|key| key.name == request.name);
    if name_exists {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiKeyResponse {
                success: false,
                api_key: None,
                secret: None,
                error: Some("An API key with this name already exists".to_string()),
            }),
        );
    }
    
    // Check if the account has reached its API key limit
    let max_allowed = account.max_allowed_api_keys();
    if key_count >= max_allowed as usize {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiKeyResponse {
                success: false,
                api_key: None,
                secret: None,
                error: Some(format!("API key limit reached ({}/{})", key_count, max_allowed)),
            }),
        );
    }
    
    // Create a new API key
    let mut account_clone = account.clone();
    let result = create_api_key(
        &mut account_clone,
        request.name,
        request.scope,
        request.description,
    );
    
    match result {
        Ok((metadata, secret)) => {
            // Apply expiration if provided
            if let Some(expires_at) = request.expires_at {
                if let Some(key) = account_clone.api_keys.get_mut(&metadata.id) {
                    key.expires_at = Some(expires_at);
                }
            }
            
            // Update the account in the datastore
            let op = datastore.account_state.update_account_local(account_clone);
            if let Err(err) = datastore.handle_account_op(op).await {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR, 
                    Json(ApiKeyResponse {
                        success: false,
                        api_key: None,
                        secret: None,
                        error: Some(format!("Failed to update account: {}", err)),
                    }),
                );
            }
            
            // Return success with the key
            (
                StatusCode::CREATED,
                Json(ApiKeyResponse {
                    success: true,
                    api_key: Some(metadata),
                    secret: Some(secret),
                    error: None,
                }),
            )
        },
        Err(error) => {
            (
                StatusCode::BAD_REQUEST,
                Json(ApiKeyResponse {
                    success: false,
                    api_key: None,
                    secret: None,
                    error: Some(error),
                }),
            )
        }
    }
}

/// List all API keys for the authenticated user
pub async fn list_api_keys_handler(
    State(state): State<Arc<Mutex<DataStore>>>,
    JwtClaims(claims): JwtClaims,
) -> impl IntoResponse {
    // Get the user's account
    let datastore = state.lock().await;
    
    // Find the account by the user's ID
    let account = match datastore.account_state.get_account(&claims.sub) {
        Some(account) => account,
        None => {
            return (
                StatusCode::NOT_FOUND,
                Json(ListApiKeysResponse {
                    success: false,
                    api_keys: Vec::new(),
                    total: 0,
                    max_allowed: 0,
                }),
            );
        }
    };
    
    // Get all API keys
    let api_keys: Vec<ApiKeyMetadata> = account.api_keys.values()
        .map(ApiKeyMetadata::from)
        .collect();
    
    // Get the maximum allowed keys
    let max_allowed = account.max_allowed_api_keys();
    
    // Return success with the keys
    (
        StatusCode::OK,
        Json(ListApiKeysResponse {
            success: true,
            total: api_keys.len(),
            api_keys,
            max_allowed,
        }),
    )
}

/// Get a specific API key by ID
pub async fn get_api_key_handler(
    State(state): State<Arc<Mutex<DataStore>>>,
    JwtClaims(claims): JwtClaims,
    Path(key_id): Path<String>,
) -> impl IntoResponse {
    // Get the user's account
    let datastore = state.lock().await;
    
    // Find the account by the user's ID
    let account = match datastore.account_state.get_account(&claims.sub) {
        Some(account) => account,
        None => {
            return (
                StatusCode::NOT_FOUND,
                Json(ApiKeyResponse {
                    success: false,
                    api_key: None,
                    secret: None,
                    error: Some("Account not found".to_string()),
                }),
            );
        }
    };
    
    // Find the API key
    let api_key = match account.get_api_key(&key_id) {
        Some(key) => key,
        None => {
            return (
                StatusCode::NOT_FOUND,
                Json(ApiKeyResponse {
                    success: false,
                    api_key: None,
                    secret: None,
                    error: Some(format!("API key with ID {} not found", key_id)),
                }),
            );
        }
    };
    
    // Return success with the key metadata
    (
        StatusCode::OK,
        Json(ApiKeyResponse {
            success: true,
            api_key: Some(ApiKeyMetadata::from(api_key)),
            secret: None, // Never return the secret after creation
            error: None,
        }),
    )
}

/// Revoke an API key
pub async fn revoke_api_key_handler(
    State(state): State<Arc<Mutex<DataStore>>>,
    JwtClaims(claims): JwtClaims,
    Path(key_id): Path<String>,
) -> impl IntoResponse {
    // Get the user's account
    let mut datastore = state.lock().await;
    
    // Find the account by the user's ID
    let account = match datastore.account_state.get_account(&claims.sub) {
        Some(account) => account,
        None => {
            return (
                StatusCode::NOT_FOUND,
                Json(ApiKeyResponse {
                    success: false,
                    api_key: None,
                    secret: None,
                    error: Some("Account not found".to_string()),
                }),
            );
        }
    };
    
    // Check if the API key exists
    if account.get_api_key(&key_id).is_none() {
        return (
            StatusCode::NOT_FOUND,
            Json(ApiKeyResponse {
                success: false,
                api_key: None,
                secret: None,
                error: Some(format!("API key with ID {} not found", key_id)),
            }),
        );
    }
    
    // Clone and update the account
    let mut account_clone = account.clone();
    
    // Revoke the API key
    if !account_clone.revoke_api_key(&key_id) {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiKeyResponse {
                success: false,
                api_key: None,
                secret: None,
                error: Some("Failed to revoke API key".to_string()),
            }),
        );
    }
    
    // Update the account in the datastore
    let op = datastore.account_state.update_account_local(account_clone.clone());
    if let Err(err) = datastore.handle_account_op(op).await {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiKeyResponse {
                success: false,
                api_key: None,
                secret: None,
                error: Some(format!("Failed to update account: {}", err)),
            }),
        );
    }
    
    // Get the updated API key
    let api_key = account_clone.get_api_key(&key_id).unwrap();
    
    // Return success
    (
        StatusCode::OK,
        Json(ApiKeyResponse {
            success: true,
            api_key: Some(ApiKeyMetadata::from(api_key)),
            secret: None,
            error: None,
        }),
    )
} 