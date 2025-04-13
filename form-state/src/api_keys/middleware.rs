use axum::{
    async_trait,
    extract::{FromRequestParts, State},
    http::{request::Parts, Request, StatusCode, header},
    middleware::Next,
    response::Response,
    body::Body,
    response::IntoResponse,
};
use std::sync::Arc;
use tokio::sync::Mutex;
use serde_json::json;
use std::net::IpAddr;

use crate::datastore::DataStore;
use crate::api_keys::{ApiKey, ApiKeyError};
use crate::accounts::Account;

/// Structure containing validated API key and account
#[derive(Clone)]
pub struct ApiKeyAuth {
    /// The API key that was validated
    pub api_key: ApiKey,
    /// The account associated with this API key
    pub account: Account,
}

/// Extract API key from request
pub async fn api_key_auth_middleware(
    State(state): State<Arc<Mutex<DataStore>>>,
    mut request: Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    log::info!("API key auth middleware called");
    
    // Log request path and method
    log::info!("Request path: {:?}, method: {:?}", request.uri().path(), request.method());
    
    // Extract the API key from either the X-API-Key header or Authorization header
    let api_key_str = extract_api_key_from_request(&request);
    
    // If no API key is found, return 401 Unauthorized
    let api_key_str = match api_key_str {
        Some(key) => key,
        None => {
            log::warn!("No API key found in request");
            return Err(StatusCode::UNAUTHORIZED);
        }
    };
    
    log::info!("API key extracted, length: {}", api_key_str.len());
    
    // Attempt to validate the API key and retrieve the associated account
    let auth_data = {
        let datastore = state.lock().await;
        
        // Iterate through all accounts to find a matching API key
        let mut auth_data = None;
        for account in datastore.account_state.list_accounts() {
            if let Some(api_key) = account.get_api_key_by_secret(api_key_str) {
                // Make sure the key is valid (not revoked or expired)
                if !api_key.is_valid() {
                    log::warn!("API key is not valid (revoked or expired)");
                    continue;
                }
                
                // Check IP restrictions if configured
                if let Some(client_ip) = get_client_ip(&request) {
                    if !api_key.is_allowed_from_ip(&client_ip) {
                        log::warn!("API key is not allowed from IP: {}", client_ip);
                        continue;
                    }
                }
                
                // Found a valid key
                auth_data = Some(ApiKeyAuth {
                    api_key: api_key.clone(),
                    account: account.clone(),
                });
                break;
            }
        }
        
        auth_data
    };
    
    // If no valid API key was found, return 401 Unauthorized
    let auth_data = match auth_data {
        Some(data) => data,
        None => {
            log::warn!("No valid API key found for the provided key");
            return Err(StatusCode::UNAUTHORIZED);
        }
    };
    
    log::info!("API key validated for account: {}", auth_data.account.address);
    
    // Store the validated API key and account in request extensions
    request.extensions_mut().insert(auth_data);
    
    // Continue with the request
    Ok(next.run(request).await)
}

/// Extract API key from either X-API-Key header or Authorization header
fn extract_api_key_from_request(request: &Request<Body>) -> Option<&str> {
    // First, try the X-API-Key header
    if let Some(api_key) = request.headers().get("X-API-Key") {
        if let Ok(key_str) = api_key.to_str() {
            return Some(key_str);
        }
    }
    
    // If not found, try the Authorization header (Bearer format)
    if let Some(auth_header) = request.headers().get(header::AUTHORIZATION) {
        if let Ok(auth_str) = auth_header.to_str() {
            if auth_str.starts_with("Bearer ") {
                return Some(&auth_str[7..]);
            }
        }
    }
    
    None
}

/// Get client IP address from request
fn get_client_ip(request: &Request<Body>) -> Option<String> {
    // First try X-Forwarded-For header (common with proxies)
    if let Some(forwarded_for) = request.headers().get("X-Forwarded-For") {
        if let Ok(forwarded_str) = forwarded_for.to_str() {
            // X-Forwarded-For can contain a comma-separated list, take the first one
            let first_ip = forwarded_str.split(',').next()?.trim();
            return Some(first_ip.to_string());
        }
    }
    
    // If no X-Forwarded-For, try to get it from the connection info
    if let Some(conn_info) = request.extensions().get::<axum::extract::ConnectInfo<std::net::SocketAddr>>() {
        return Some(conn_info.ip().to_string());
    }
    
    None
}

/// Extractor for getting the API key and account from a request
#[async_trait]
impl<S> FromRequestParts<S> for ApiKeyAuth
where
    S: Send + Sync,
{
    type Rejection = StatusCode;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        // Extract ApiKeyAuth from request extensions
        parts.extensions
            .get::<ApiKeyAuth>()
            .cloned()
            .ok_or(StatusCode::UNAUTHORIZED)
    }
}

/// Custom error responses for API key errors
pub fn api_key_error_response(error: ApiKeyError) -> Response {
    let (status, error_json) = match error {
        ApiKeyError::Missing => (
            StatusCode::UNAUTHORIZED,
            json!({
                "error": "missing_api_key",
                "message": "API key is required but was not provided",
                "details": "Include your API key via the X-API-Key header or Bearer token"
            })
        ),
        ApiKeyError::InvalidFormat => (
            StatusCode::UNAUTHORIZED,
            json!({
                "error": "invalid_api_key_format",
                "message": "The provided API key is invalid"
            })
        ),
        ApiKeyError::NotFound => (
            StatusCode::UNAUTHORIZED,
            json!({
                "error": "api_key_not_found",
                "message": "The provided API key is not recognized"
            })
        ),
        ApiKeyError::Revoked => (
            StatusCode::UNAUTHORIZED,
            json!({
                "error": "api_key_revoked",
                "message": "The provided API key has been revoked"
            })
        ),
        ApiKeyError::Expired => (
            StatusCode::UNAUTHORIZED,
            json!({
                "error": "api_key_expired",
                "message": "The provided API key has expired"
            })
        ),
        ApiKeyError::InsufficientPermissions => (
            StatusCode::FORBIDDEN,
            json!({
                "error": "insufficient_permissions",
                "message": "The provided API key does not have permission for this operation"
            })
        ),
        ApiKeyError::IpNotAllowed => (
            StatusCode::FORBIDDEN,
            json!({
                "error": "ip_not_allowed",
                "message": "Your IP address is not allowed to use this API key"
            })
        ),
        ApiKeyError::RateLimitExceeded => (
            StatusCode::TOO_MANY_REQUESTS,
            json!({
                "error": "rate_limit_exceeded",
                "message": "Rate limit exceeded for this API key"
            })
        ),
    };

    (status, axum::Json(error_json)).into_response()
} 