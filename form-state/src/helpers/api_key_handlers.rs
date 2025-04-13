use std::sync::Arc;
use tokio::sync::Mutex;
use axum::{
    extract::{State, Path, Query},
    response::IntoResponse,
    Json,
    http::StatusCode,
};
use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc, Duration};
use serde_json::json;

use crate::datastore::DataStore;
use crate::auth::JwtClaims;
use crate::api_keys::{ApiKeyScope, ApiKeyMetadata, create_api_key, ApiKeyAuth};
use crate::api_keys::audit::{ApiKeyEvent, API_KEY_AUDIT_LOG};

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

/// Request to revoke an API key
#[derive(Debug, Serialize, Deserialize)]
pub struct RevokeApiKeyRequest {
    /// Reason for revoking the API key
    pub reason: String,
}

/// Handler for creating a new API key
pub async fn create_api_key_handler(
    State(state): State<Arc<Mutex<DataStore>>>,
    claims: JwtClaims,
    Json(request): Json<CreateApiKeyRequest>,
) -> impl IntoResponse {
    log::info!("Creating new API key for user: {}", claims.0.sub);
    
    // Get the account from the database
    let mut datastore = state.lock().await;
    let account_id = claims.0.sub.clone();
    
    let mut account = match datastore.account_state.get_account(&account_id) {
        Some(account) => account,
        None => {
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({
                    "success": false,
                    "error": "Account not found"
                }))
            );
        }
    };
    
    // Validate the request
    if request.name.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "success": false,
                "error": "API key name cannot be empty"
            }))
        );
    }
    
    // Check if the account has reached the API key limit
    let max_allowed = account.max_allowed_api_keys();
    let current_count = account.list_active_api_keys().len() as u32;
    
    if current_count >= max_allowed {
        return (
            StatusCode::FORBIDDEN,
            Json(serde_json::json!({
                "success": false,
                "error": format!("API key limit reached ({}/{})", current_count, max_allowed)
            }))
        );
    }
    
    // Create the API key
    let scope = request.scope;
    
    let (key_metadata, secret) = match create_api_key(&mut account, request.name.clone(), scope, request.description.clone()) {
        Ok((metadata, secret)) => (metadata, secret),
        Err(err) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "success": false,
                    "error": err
                }))
            );
        }
    };
    
    // Update the account in the database
    let op = datastore.account_state.update_account_local(account.clone());
    if let Err(err) = datastore.handle_account_op(op).await {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "success": false,
                "error": format!("Failed to update account: {}", err)
            }))
        );
    }
    
    // Log API key creation event
    let ip_address = None; // In a real implementation, extract from request
    let user_agent = None; // In a real implementation, extract from request
    
    let event = ApiKeyEvent::new_creation(
        key_metadata.id.clone(),
        account_id.clone(),
        ip_address,
        user_agent,
    );
    
    // Record the event
    API_KEY_AUDIT_LOG.record(event.clone()).await;
    
    // Persist the event to permanent storage (in background to not block response)
    let state_clone = state.clone();
    tokio::spawn(async move {
        crate::api_keys::audit::ApiKeyAuditLog::persist_event(event, state_clone).await;
    });
    
    // Return success with the API key details
    (
        StatusCode::CREATED,
        Json(serde_json::json!({
            "success": true,
            "message": "API key created successfully",
            "api_key": {
                "id": key_metadata.id,
                "name": key_metadata.name,
                "scope": format!("{:?}", key_metadata.scope),
                "created_at": key_metadata.created_at,
                "expires_at": key_metadata.expires_at,
                // Only show the secret once, when the key is created
                "secret": secret
            }
        }))
    )
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

/// Handler for revoking an API key
pub async fn revoke_api_key_handler(
    State(state): State<Arc<Mutex<DataStore>>>,
    claims: JwtClaims,
    Path(key_id): Path<String>,
    Json(request): Json<RevokeApiKeyRequest>,
) -> impl IntoResponse {
    log::info!("Revoking API key: {}", key_id);
    
    // Get the account from the database
    let mut datastore = state.lock().await;
    let account_id = claims.0.sub.clone();
    
    let mut account = match datastore.account_state.get_account(&account_id) {
        Some(account) => account,
        None => {
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({
                    "success": false,
                    "error": "Account not found"
                }))
            );
        }
    };
    
    // Check if the API key exists
    if account.get_api_key(&key_id).is_none() {
        return (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "success": false,
                "error": format!("API key with ID {} not found", key_id)
            }))
        );
    }
    
    // Revoke the API key
    if !account.revoke_api_key(&key_id) {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "success": false,
                "error": "Failed to revoke API key"
            }))
        );
    }
    
    // Update the account in the database
    let op = datastore.account_state.update_account_local(account.clone());
    if let Err(err) = datastore.handle_account_op(op).await {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "success": false,
                "error": format!("Failed to update account: {}", err)
            }))
        );
    }
    
    // Log API key revocation event
    let ip_address = None; // In a real implementation, extract from request
    let user_agent = None; // In a real implementation, extract from request
    
    let event = ApiKeyEvent::new_revocation(
        key_id.clone(),
        account_id.clone(),
        Some(request.reason.clone()),
        ip_address,
        user_agent,
    );
    
    // Record the event
    API_KEY_AUDIT_LOG.record(event.clone()).await;
    
    // Persist the event to permanent storage (in background to not block response)
    let state_clone = state.clone();
    tokio::spawn(async move {
        crate::api_keys::audit::ApiKeyAuditLog::persist_event(event, state_clone).await;
    });
    
    // Return success
    (
        StatusCode::OK,
        Json(serde_json::json!({
            "success": true,
            "message": format!("API key {} successfully revoked", key_id)
        }))
    )
}

/// Handler for retrieving API key audit logs
pub async fn get_api_key_audit_logs(
    auth: ApiKeyAuth,
    Path(api_key_id): Path<String>,
) -> impl IntoResponse {
    log::info!("Getting audit logs for API key: {}", api_key_id);
    
    // Check operation permission
    if !auth.api_key.can_perform_operation("api_keys.view") {
        return (
            StatusCode::FORBIDDEN,
            Json(json!({
                "success": false,
                "error": "API key does not have permission to view API key audit logs"
            }))
        );
    }
    
    // Verify ownership - only the account owner can view its API key logs
    if !auth.account.get_api_key(&api_key_id).is_some() {
        return (
            StatusCode::FORBIDDEN,
            Json(json!({
                "success": false,
                "error": "You do not have permission to view audit logs for this API key"
            }))
        );
    }
    
    // Get the audit logs for this API key
    let logs = API_KEY_AUDIT_LOG.get_events_for_key(&api_key_id).await;
    
    // Return the logs
    (
        StatusCode::OK,
        Json(json!({
            "success": true,
            "api_key_id": api_key_id,
            "total_events": logs.len(),
            "events": logs
        }))
    )
}

/// Handler for retrieving all API key audit logs for an account
pub async fn get_account_api_key_audit_logs(
    auth: ApiKeyAuth,
    Query(params): Query<PaginationParams>,
) -> impl IntoResponse {
    log::info!("Getting all API key audit logs for account: {}", auth.account.address);
    
    // Check operation permission
    if !auth.api_key.can_perform_operation("api_keys.view") {
        return (
            StatusCode::FORBIDDEN,
            Json(json!({
                "success": false,
                "error": "API key does not have permission to view API key audit logs"
            }))
        );
    }
    
    // Get the pagination parameters
    let limit = params.limit.unwrap_or(100);
    let offset = params.offset.unwrap_or(0);
    
    // Get all audit logs for this account
    let logs = API_KEY_AUDIT_LOG.get_events_for_account(&auth.account.address).await;
    
    // Apply pagination
    let total = logs.len();
    let logs = if offset < logs.len() {
        let end = std::cmp::min(offset + limit, logs.len());
        logs[offset..end].to_vec()
    } else {
        Vec::new()
    };
    
    // Return the logs
    (
        StatusCode::OK,
        Json(json!({
            "success": true,
            "account_id": auth.account.address,
            "total_events": total,
            "events": logs,
            "pagination": {
                "limit": limit,
                "offset": offset,
                "has_more": offset + logs.len() < total
            }
        }))
    )
}

/// Query parameters for pagination
#[derive(Debug, Deserialize)]
pub struct PaginationParams {
    /// Maximum number of items to return
    pub limit: Option<usize>,
    /// Number of items to skip
    pub offset: Option<usize>,
} 