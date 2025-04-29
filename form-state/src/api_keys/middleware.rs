use axum::{
    async_trait,
    extract::{FromRequestParts, State},
    http::{request::Parts, Request, StatusCode, header, Method},
    middleware::Next,
    response::Response,
    body::Body,
    response::IntoResponse,
};
use std::sync::Arc;
use tokio::sync::Mutex;
use serde_json::json;
use once_cell::sync::Lazy;

use crate::datastore::DataStore;
use crate::api_keys::{ApiKey, ApiKeyError, ApiKeyRateLimiter, RateLimitCheckResult, get_rate_limit_headers};
use crate::api_keys::audit::{ApiKeyEvent, ApiKeyAuditLog, API_KEY_AUDIT_LOG};
use crate::accounts::Account;
use crate::api::{is_localhost_request, is_public_endpoint};

// Global rate limiter instance
static RATE_LIMITER: Lazy<ApiKeyRateLimiter> = Lazy::new(|| {
    // Start a background task to periodically clean up expired entries
    tokio::spawn(async {
        let rate_limiter = RATE_LIMITER.clone();
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(3600)).await; // Every hour
            rate_limiter.cleanup_expired();
        }
    });
    
    ApiKeyRateLimiter::new()
});

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
    log::info!("Function imported: crate::api::is_public_endpoint = {:?}", std::any::type_name::<fn(&str) -> bool>());
    
    // Log request path and method
    let path = request.uri().path().to_string();
    let method = request.method().clone();
    log::info!("Request path: {:?}, method: {:?}", path, method);
    
    // DIRECT PATH CHECK for common endpoints - temporary solution
    if method == Method::GET && (
        path == "/agents" || 
        path == "/instances" || 
        path == "/accounts" || 
        path == "/models"
    ) {
        log::info!("DIRECT MATCH: Specific GET endpoint detected, bypassing auth: {}", path);
        return Ok(next.run(request).await);
    }
    
    // Check if request is from localhost - bypass auth if it is
    let is_localhost = is_localhost_request(&request);
    log::info!("Is localhost request? {}", is_localhost);
    if is_localhost {
        log::info!("Localhost detected, bypassing API key authentication");
        return Ok(next.run(request).await);
    }
    
    // Skip auth for GET requests to public endpoints
    let is_get = method == Method::GET;
    let public_path = match crate::api::is_public_endpoint(&path) {
        true => {
            log::info!("Path {} IS a public endpoint", path);
            true
        },
        false => {
            log::info!("Path {} is NOT a public endpoint", path);
            false
        }
    };
    
    if is_get && public_path {
        log::info!("Public GET endpoint detected, bypassing API key authentication: {}", path);
        return Ok(next.run(request).await);
    } else {
        log::info!("Auth required: is_get={}, public_path={}", is_get, public_path);
    }
    
    // Get client IP address and user agent
    let ip_address = get_client_ip(&request);
    log::info!("Client IP: {:?}", ip_address);
    
    let user_agent = request.headers()
        .get(header::USER_AGENT)
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_string());
    
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
    
    // Check rate limits
    let subscription_tier = auth_data.account.subscription
        .as_ref()
        .map(|sub| sub.tier)
        .unwrap_or_default();
        
    let rate_limit_result = RATE_LIMITER.check_rate_limit(&auth_data.api_key.id, &subscription_tier);
    
    // If rate limit exceeded, return 429 Too Many Requests and log the event
    let is_rate_limited = match &rate_limit_result {
        RateLimitCheckResult::Allowed { .. } => {
            // Rate limit not exceeded, continue processing
            false
        },
        _ => {
            // Rate limit exceeded, return 429 with appropriate headers
            log::warn!("Rate limit exceeded for API key: {}", auth_data.api_key.id);
            
            // Log rate limit event
            let event = ApiKeyEvent::new_usage(
                auth_data.api_key.id.clone(),
                auth_data.account.address.clone(),
                path.clone(),
                method.clone(),
                StatusCode::TOO_MANY_REQUESTS.as_u16(),
                ip_address.clone(),
                user_agent.clone(),
                true, // rate limited
            );
            
            // Record the event
            API_KEY_AUDIT_LOG.record(event.clone()).await;
            
            // Persist the event to permanent storage (in background to not block response)
            let state_clone = state.clone();
            tokio::spawn(async move {
                ApiKeyAuditLog::persist_event(event, state_clone).await;
            });
            
            let headers = get_rate_limit_headers(&rate_limit_result);
            let mut response = api_key_error_response(ApiKeyError::RateLimitExceeded);
            
            // Add rate limit headers to response
            let response_headers = response.headers_mut();
            for (key, value) in headers {
                if let Ok(name) = header::HeaderName::from_bytes(key.as_bytes()) {
                    if let Ok(val) = header::HeaderValue::from_str(&value) {
                        response_headers.insert(name, val);
                    }
                }
            }
            
            return Ok(response);
        }
    };
    
    // Store the validated API key and account in request extensions
    request.extensions_mut().insert(auth_data.clone());
    
    // Continue with the request
    let mut response = next.run(request).await;
    
    // Log successful API key usage event
    let status_code = response.status().as_u16();
    let event = ApiKeyEvent::new_usage(
        auth_data.api_key.id.clone(),
        auth_data.account.address.clone(),
        path,
        method,
        status_code,
        ip_address,
        user_agent,
        is_rate_limited,
    );
    
    // Record the event
    API_KEY_AUDIT_LOG.record(event.clone()).await;
    
    // Persist the event to permanent storage (in background to not block response)
    let state_clone = state.clone();
    tokio::spawn(async move {
        ApiKeyAuditLog::persist_event(event, state_clone).await;
    });
    
    // Add rate limit headers to the response
    let headers = get_rate_limit_headers(&rate_limit_result);
    let response_headers = response.headers_mut();
    for (key, value) in headers {
        if let Ok(name) = header::HeaderName::from_bytes(key.as_bytes()) {
            if let Ok(val) = header::HeaderValue::from_str(&value) {
                response_headers.insert(name, val);
            }
        }
    }
    
    Ok(response)
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