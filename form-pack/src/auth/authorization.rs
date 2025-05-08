use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::sync::Arc;
use log;
use super::ecdsa::{RecoveredAddress, create_auth_client};
use thiserror::Error;

/// Authorization errors
#[derive(Debug, Error)]
pub enum AuthorizationError {
    #[error("Resource not found")]
    ResourceNotFound,
    
    #[error("Access denied")]
    AccessDenied,
    
    #[error("Network error: {0}")]
    NetworkError(String),
    
    #[error("Unknown error: {0}")]
    UnknownError(String),
}

/// Authorization client for checking permissions with form-state
pub struct AuthorizationClient {
    /// Base URL for the form-state API
    base_url: String,
    /// HTTP client
    client: Client,
}

impl AuthorizationClient {
    /// Create a new authorization client
    pub fn new(base_url: String) -> Self {
        Self {
            base_url,
            client: Client::new(),
        }
    }
    
    /// Check if a user has access to a resource
    pub async fn check_resource_access(
        &self,
        address: &str,
        resource_id: &str,
        resource_type: &str,
    ) -> Result<bool, AuthorizationError> {
        let url = format!("{}/auth/check_access", self.base_url);
        
        let payload = serde_json::json!({
            "address": address,
            "resource_id": resource_id,
            "resource_type": resource_type,
        });
        
        let response = self.client.post(&url)
            .json(&payload)
            .send()
            .await
            .map_err(|e| AuthorizationError::NetworkError(e.to_string()))?;
        
        match response.status() {
            StatusCode::OK => {
                let body: serde_json::Value = response.json()
                    .await
                    .map_err(|e| AuthorizationError::UnknownError(e.to_string()))?;
                
                // Parse the response to check if access is granted
                if let Some(has_access) = body.get("has_access").and_then(|v| v.as_bool()) {
                    Ok(has_access)
                } else {
                    log::error!("Unexpected response format: {:?}", body);
                    Err(AuthorizationError::UnknownError("Unexpected response format".to_string()))
                }
            },
            StatusCode::NOT_FOUND => Err(AuthorizationError::ResourceNotFound),
            StatusCode::FORBIDDEN => Err(AuthorizationError::AccessDenied),
            _ => {
                let error_msg = format!(
                    "Unexpected status code: {}", 
                    response.status()
                );
                log::error!("{}", error_msg);
                Err(AuthorizationError::UnknownError(error_msg))
            }
        }
    }
    
    /// Forward a request to form-state with the authenticated user's address
    /// but with the admin node's credentials
    pub async fn forward_authenticated_request<T, R>(
        &self,
        endpoint: &str,
        user_address: &str,
        payload: &T,
        admin_signature: &str,
        admin_recovery_id: u8,
        admin_message: &str,
    ) -> Result<R, AuthorizationError>
    where
        T: Serialize,
        R: for<'de> Deserialize<'de>,
    {
        // Create a client with admin credentials
        let client = create_auth_client(admin_signature, admin_recovery_id, admin_message);
        
        // Add the user's address to the payload
        let mut payload_with_user = serde_json::to_value(payload)
            .map_err(|e| AuthorizationError::UnknownError(e.to_string()))?;
        
        if let serde_json::Value::Object(ref mut map) = payload_with_user {
            map.insert(
                "original_user_address".to_string(), 
                serde_json::Value::String(user_address.to_string())
            );
        }
        
        let url = format!("{}{}", self.base_url, endpoint);
        
        let response = client.post(&url)
            .json(&payload_with_user)
            .send()
            .await
            .map_err(|e| AuthorizationError::NetworkError(e.to_string()))?;
        
        match response.status() {
            StatusCode::OK | StatusCode::CREATED => {
                response.json::<R>()
                    .await
                    .map_err(|e| AuthorizationError::UnknownError(e.to_string()))
            },
            StatusCode::NOT_FOUND => Err(AuthorizationError::ResourceNotFound),
            StatusCode::FORBIDDEN => Err(AuthorizationError::AccessDenied),
            _ => {
                let error_msg = format!(
                    "Unexpected status code: {}", 
                    response.status()
                );
                log::error!("{}", error_msg);
                Err(AuthorizationError::UnknownError(error_msg))
            }
        }
    }
}

/// Helper function to extract an address for authorization
pub fn extract_address_for_auth(recovered: &RecoveredAddress) -> String {
    recovered.as_hex()
} 