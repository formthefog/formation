use axum::{
    body::Body, extract::State, http::{Request, StatusCode}, middleware::Next, response::{IntoResponse, Response}, Json
};
use std::sync::Arc;
use serde_json::json;
use super::claims::{JwtClaims, UserRole};
use super::jwt_client::JwtClient;

#[derive(Debug, Clone)]
pub enum AuthError {
    InvalidToken,
    MissingToken,
    ExpiredToken,
    InsufficientPermissions,
    ProjectAccessDenied,
    UnauthorizedRole,
}

/// Create a standardized error response
fn create_error_response(error: AuthError) -> (StatusCode, Json<serde_json::Value>) {
    let (status, error_message) = match error {
        AuthError::InvalidToken => (StatusCode::UNAUTHORIZED, "Invalid token"),
        AuthError::MissingToken => (StatusCode::UNAUTHORIZED, "Missing token"),
        AuthError::ExpiredToken => (StatusCode::UNAUTHORIZED, "Token expired"),
        AuthError::InsufficientPermissions => (StatusCode::FORBIDDEN, "Insufficient permissions"),
        AuthError::ProjectAccessDenied => (StatusCode::FORBIDDEN, "Project access denied"),
        AuthError::UnauthorizedRole => (StatusCode::FORBIDDEN, "Unauthorized role"),
    };

    let body = Json(json!({
        "error": error_message,
        "status": status.as_u16()
    }));

    (status, body)
}

/// Create a standardized error response as an IntoResponse
pub fn create_auth_error_response(error: AuthError) -> impl IntoResponse {
    create_error_response(error)
}

/// Extract the JWT token from the authorization header
pub fn extract_token_from_header(req: &Request<Body>) -> Result<String, AuthError> {
    let token = req
        .headers()
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|auth_header| auth_header.to_str().ok())
        .and_then(|auth_value| {
            if auth_value.starts_with("Bearer ") {
                Some(auth_value[7..].to_string())
            } else {
                None
            }
        })
        .ok_or(AuthError::MissingToken)?;

    Ok(token)
}

/// Extract authentication information from a request
pub async fn extract_auth_info(req: &Request<Body>) -> Result<JwtClaims, AuthError> {
    let token = extract_token_from_header(req)?;
    
    let jwt_client = req
        .extensions()
        .get::<Arc<JwtClient>>()
        .ok_or(AuthError::InvalidToken)?;
    
    let claims = jwt_client.validate_token(&token).await
        .map_err(|_| AuthError::InvalidToken)?;
    
    Ok(claims)
}

/// JWT authentication middleware
pub async fn jwt_auth_middleware(
    State(jwt_client): State<Arc<JwtClient>>,
    mut req: Request<Body>,
    next: Next,
) -> Result<Response, (StatusCode, Json<serde_json::Value>)> {
    // Skip auth for specific endpoints if needed
    if req.uri().path() == "/health" || req.uri().path() == "/ping" {
        return Ok(next.run(req).await);
    }

    // Try to extract token
    match extract_token_from_header(&req) {
        Ok(token) => {
            // Validate the token
            match jwt_client.validate_token(&token).await {
                Ok(claims) => {
                    // Add the claims to request extensions
                    req.extensions_mut().insert(claims);
                    
                    // Add the JWT client to the extensions for potential service-to-service calls
                    req.extensions_mut().insert(Arc::clone(&jwt_client));
                    
                    // Add the token to extensions for passing to other services
                    req.extensions_mut().insert(Some(token));
                },
                Err(_) => {
                    // Don't return an error immediately, allow the API key middleware to handle it
                    req.extensions_mut().insert::<Option<String>>(None);
                }
            }
        },
        Err(_) => {
            // Don't return an error immediately, allow the API key middleware to handle it
            req.extensions_mut().insert::<Option<String>>(None);
        }
    }
    
    Ok(next.run(req).await)
}

/// Verify if a user has access to a specific project
pub fn verify_project_access(claims: &JwtClaims, project_id: &str) -> Result<(), AuthError> {
    if claims.is_admin() || claims.has_project_access(project_id) {
        Ok(())
    } else {
        Err(AuthError::ProjectAccessDenied)
    }
}

/// Verify if a user has the required role
pub fn verify_role(claims: &JwtClaims, required_role: UserRole) -> Result<(), AuthError> {
    match (&claims.role, &required_role) {
        (UserRole::Admin, _) => Ok(()),
        (UserRole::Developer, UserRole::Developer) => Ok(()),
        (UserRole::Developer, UserRole::Viewer) => Ok(()),
        (UserRole::Viewer, UserRole::Viewer) => Ok(()),
        _ => Err(AuthError::UnauthorizedRole),
    }
}

/// Check if user has resource access
pub fn has_resource_access(claims: &JwtClaims, resource_id: &str) -> bool {
    claims.is_admin() || claims.has_project_access(resource_id)
} 