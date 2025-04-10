use crate::auth::claims::DynamicClaims;
use crate::auth::jwks::JWKSManager;
use crate::auth::UserRole;
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
use jsonwebtoken::TokenData;

/// Custom error type for authentication errors
#[derive(Debug)]
pub enum AuthError {
    /// Missing Authorization header
    MissingToken,
    /// Invalid token format (not a Bearer token)
    InvalidTokenFormat,
    /// Token validation failed with a specific error
    TokenValidationFailed(String),
    /// Authorization error (insufficient permissions)
    InsufficientPermissions,
    /// Other unexpected errors
    Other(String),
}

impl std::fmt::Display for AuthError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AuthError::MissingToken => write!(f, "Missing Authorization header"),
            AuthError::InvalidTokenFormat => write!(f, "Invalid token format (not a Bearer token)"),
            AuthError::TokenValidationFailed(msg) => write!(f, "Token validation failed: {}", msg),
            AuthError::InsufficientPermissions => write!(f, "Insufficient permissions for this operation"),
            AuthError::Other(msg) => write!(f, "Authentication error: {}", msg),
        }
    }
}

impl std::error::Error for AuthError {}

impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        let status = match &self {
            AuthError::MissingToken => StatusCode::UNAUTHORIZED,
            AuthError::InvalidTokenFormat => StatusCode::UNAUTHORIZED,
            AuthError::TokenValidationFailed(_) => StatusCode::UNAUTHORIZED,
            AuthError::InsufficientPermissions => StatusCode::FORBIDDEN,
            AuthError::Other(_) => StatusCode::INTERNAL_SERVER_ERROR,
        };
        
        (status, self.to_string()).into_response()
    }
}

/// Extract and validate the JWT from the Authorization header
pub async fn jwt_auth_middleware(
    State(jwks_manager): State<Arc<JWKSManager>>,
    mut request: Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    // Extract the token from the Authorization header
    let auth_header = request
        .headers()
        .get(header::AUTHORIZATION)
        .ok_or(StatusCode::UNAUTHORIZED)?;
    
    let auth_header_str = auth_header
        .to_str()
        .map_err(|_| StatusCode::UNAUTHORIZED)?;
    
    // Check if it's a Bearer token
    if !auth_header_str.starts_with("Bearer ") {
        return Err(StatusCode::UNAUTHORIZED);
    }
    
    // Extract the token without the "Bearer " prefix
    let token = &auth_header_str[7..];
    
    // Validate the token
    let token_data = jwks_manager
        .validate_token(token)
        .await
        .map_err(|_| StatusCode::UNAUTHORIZED)?;
    
    // Store the validated claims in request extensions for handlers to access
    request.extensions_mut().insert(token_data);
    
    // Pass the request with the validated claims to the next middleware/handler
    Ok(next.run(request).await)
}

/// Extractor for getting the JWT claims from a request
pub struct JwtClaims(pub DynamicClaims);

#[async_trait]
impl<S> FromRequestParts<S> for JwtClaims
where
    S: Send + Sync,
{
    type Rejection = StatusCode;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        // Extract TokenData from request extensions
        let token_data = parts
            .extensions
            .get::<TokenData<DynamicClaims>>()
            .ok_or(StatusCode::UNAUTHORIZED)?;
        
        // Return the claims
        Ok(JwtClaims(token_data.claims.clone()))
    }
}

/// Role-based extractors

/// Extractor for requests that require admin role
pub struct AdminClaims(pub DynamicClaims);

#[async_trait]
impl<S> FromRequestParts<S> for AdminClaims
where
    S: Send + Sync,
{
    type Rejection = AuthError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        // First extract the JWT claims
        let JwtClaims(claims) = JwtClaims::from_request_parts(parts, state)
            .await
            .map_err(|_| AuthError::MissingToken)?;
        
        // Check if the user has admin role
        if claims.is_admin() {
            Ok(AdminClaims(claims))
        } else {
            Err(AuthError::InsufficientPermissions)
        }
    }
}

/// Extractor for requests that require developer or admin role
pub struct DeveloperOrAdminClaims(pub DynamicClaims);

#[async_trait]
impl<S> FromRequestParts<S> for DeveloperOrAdminClaims
where
    S: Send + Sync,
{
    type Rejection = AuthError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        // First extract the JWT claims
        let JwtClaims(claims) = JwtClaims::from_request_parts(parts, state)
            .await
            .map_err(|_| AuthError::MissingToken)?;
        
        // Check if the user has developer or admin role
        if claims.is_developer() {
            Ok(DeveloperOrAdminClaims(claims))
        } else {
            Err(AuthError::InsufficientPermissions)
        }
    }
}

/// Helper functions for claims validation

/// Verify that the claims belong to the specified project
pub fn verify_project_access(claims: &DynamicClaims, project_id: &str) -> Result<(), AuthError> {
    if claims.is_for_project(project_id) {
        Ok(())
    } else {
        Err(AuthError::InsufficientPermissions)
    }
}

/// Verify that the claims have the required role
pub fn verify_role(claims: &DynamicClaims, required_role: UserRole) -> Result<(), AuthError> {
    if claims.has_role(&required_role) {
        Ok(())
    } else {
        Err(AuthError::InsufficientPermissions)
    }
}

/// Verify both project access and role
pub fn verify_project_and_role(claims: &DynamicClaims, project_id: &str, required_role: UserRole) -> Result<(), AuthError> {
    verify_project_access(claims, project_id)?;
    verify_role(claims, required_role)
}

/// Get active wallet address from claims
pub fn get_wallet_address(claims: &DynamicClaims) -> Option<&str> {
    claims.wallet_address()
}

/// Get user email from claims
pub fn get_user_email(claims: &DynamicClaims) -> Option<&str> {
    claims.email()
}

/// Check if token has expired
pub fn is_token_valid(claims: &DynamicClaims) -> bool {
    let current_time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    
    claims.is_valid_time(current_time)
}

/// Get project ID from path parameter and verify access
pub fn verify_project_path_access(
    claims: &DynamicClaims, 
    project_id: &str
) -> Result<(), AuthError> {
    if !claims.is_for_project(project_id) {
        return Err(AuthError::InsufficientPermissions);
    }
    Ok(())
}

/// Create response with error details for auth failures
pub fn create_auth_error_response(error: AuthError) -> Response {
    error.into_response()
}

/// Extract token from Authorization header string (Bearer format)
pub fn extract_token_from_header(auth_header: &str) -> Result<&str, AuthError> {
    if !auth_header.starts_with("Bearer ") {
        return Err(AuthError::InvalidTokenFormat);
    }
    
    Ok(&auth_header[7..])
}

/// Check if user has access to a specific resource
pub fn has_resource_access(
    claims: &DynamicClaims, 
    _resource_id: &str, 
    resource_project_id: &str
) -> Result<(), AuthError> {
    // First check if user belongs to the project that owns the resource
    if !claims.is_for_project(resource_project_id) {
        // If user is admin, they can access resources from any project
        if claims.is_admin() {
            return Ok(());
        }
        return Err(AuthError::InsufficientPermissions);
    }
    
    Ok(())
}

/// Extract user information for logging/auditing
pub fn extract_user_info(claims: &DynamicClaims) -> serde_json::Value {
    serde_json::json!({
        "user_id": claims.sub,
        "wallet_address": claims.wallet_address(),
        "email": claims.email(),
        "project": claims.project_id(),
        "role": format!("{:?}", claims.user_role()),
        "dynamic_user_id": claims.dynamic_user_id,
    })
} 