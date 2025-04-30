use axum::{
    body::Body, extract::State, http::{Request, StatusCode}, middleware::Next, response::{IntoResponse, Response}, Json
};
use std::sync::Arc;
use serde_json::json;
use super::{ApiKeyError, client::ApiKeyClient};

/// API Key authentication data
#[derive(Debug, Clone)]
pub struct ApiKeyAuth {
    pub key_id: String,
    pub account_id: String,
}

/// Create a standardized error response for API key errors
fn create_error_response(error: ApiKeyError) -> (StatusCode, Json<serde_json::Value>) {
    let (status, error_message) = match error {
        ApiKeyError::Missing => (StatusCode::UNAUTHORIZED, "API key is missing"),
        ApiKeyError::InvalidFormat => (StatusCode::UNAUTHORIZED, "Invalid API key format"),
        ApiKeyError::NotFound => (StatusCode::UNAUTHORIZED, "API key not found"),
        ApiKeyError::Revoked => (StatusCode::UNAUTHORIZED, "API key has been revoked"),
        ApiKeyError::Expired => (StatusCode::UNAUTHORIZED, "API key has expired"),
        ApiKeyError::InsufficientPermissions => (StatusCode::FORBIDDEN, "Insufficient permissions"),
        ApiKeyError::RateLimitExceeded => (StatusCode::TOO_MANY_REQUESTS, "Rate limit exceeded"),
        ApiKeyError::ServiceError => (StatusCode::INTERNAL_SERVER_ERROR, "Internal service error"),
    };

    let body = Json(json!({
        "error": error_message,
        "status": status.as_u16()
    }));

    (status, body)
}

/// Create a standardized error response for API key errors as an IntoResponse
pub fn api_key_error_response(error: ApiKeyError) -> impl IntoResponse {
    create_error_response(error)
}

/// Extract API key from the X-API-Key header
fn extract_api_key(req: &Request<Body>) -> Result<String, ApiKeyError> {
    let api_key = req
        .headers()
        .get("X-API-Key")
        .and_then(|value| value.to_str().ok())
        .map(|s| s.to_string())
        .ok_or(ApiKeyError::Missing)?;

    if !api_key.starts_with("sk_") || api_key.len() < 10 {
        return Err(ApiKeyError::InvalidFormat);
    }

    Ok(api_key)
}

/// API key authentication middleware
pub async fn api_key_auth_middleware(
    State(api_key_client): State<Arc<ApiKeyClient>>,
    mut req: Request<Body>,
    next: Next,
) -> Result<Response, (StatusCode, Json<serde_json::Value>)> {
    // Skip auth for specific endpoints if needed
    if req.uri().path() == "/health" || req.uri().path() == "/ping" {
        return Ok(next.run(req).await);
    }

    // Check if JWT auth was successful (extension contains Some(String))
    let jwt_auth_present = req.extensions().get::<Option<String>>()
        .map_or(false, |token| token.is_some());
    
    // If JWT auth was successful, we don't need to validate API key
    if jwt_auth_present {
        // Set API key to None since we're using JWT auth
        req.extensions_mut().insert::<Option<String>>(None);
        return Ok(next.run(req).await);
    }

    // Try to extract API key
    match extract_api_key(&req) {
        Ok(api_key) => {
            // Validate the API key
            match api_key_client.validate_key(&api_key).await {
                Ok(api_key_info) => {
                    if !api_key_info.is_valid() {
                        if api_key_info.status == super::ApiKeyStatus::Revoked {
                            return Err(create_error_response(ApiKeyError::Revoked));
                        } else {
                            return Err(create_error_response(ApiKeyError::Expired));
                        }
                    }
                    
                    // Check if the API key has permission for this operation
                    let operation = req.uri().path().trim_start_matches('/');
                    if !api_key_info.can_perform_operation(operation) {
                        return Err(create_error_response(ApiKeyError::InsufficientPermissions));
                    }
                    
                    // Add API key info to request extensions
                    let auth_info = ApiKeyAuth {
                        key_id: api_key_info.id.clone(),
                        account_id: api_key_info.account_id.clone(),
                    };
                    req.extensions_mut().insert(auth_info);
                    
                    // Add the API key client to the extensions for potential service-to-service calls
                    req.extensions_mut().insert(Arc::clone(&api_key_client));
                    
                    // Add the original API key to extensions for passing to other services
                    req.extensions_mut().insert::<Option<String>>(Some(api_key));
                },
                Err(_) => {
                    return Err(create_error_response(ApiKeyError::NotFound));
                }
            }
        },
        Err(e) => {
            // If no JWT auth and no API key, return unauthorized
            return Err(create_error_response(e));
        }
    }
    
    Ok(next.run(req).await)
} 