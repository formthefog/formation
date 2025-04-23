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
use serde_json::{self, json};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use crate::api::is_localhost_request;

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
        
        // Create a JSON error response with more details
        let error_json = match &self {
            AuthError::MissingToken => json!({
                "error": "authentication_required",
                "message": "Authentication is required to access this resource",
                "details": "No authorization token was provided in the request"
            }),
            AuthError::InvalidTokenFormat => json!({
                "error": "invalid_token_format",
                "message": "The provided authentication token is invalid",
                "details": "Token must be a valid Bearer token"
            }),
            AuthError::TokenValidationFailed(msg) => json!({
                "error": "invalid_token",
                "message": "The provided authentication token is invalid",
                "details": msg
            }),
            AuthError::InsufficientPermissions => json!({
                "error": "insufficient_permissions",
                "message": "You don't have permission to access this resource",
                "details": "The authenticated user lacks the required role or project access"
            }),
            AuthError::Other(msg) => json!({
                "error": "authentication_error",
                "message": "An unexpected authentication error occurred",
                "details": msg
            }),
        };
        
        // Build the response with appropriate headers
        let mut response = Response::builder()
            .status(status);
        
        // Add WWW-Authenticate header for 401 responses
        if status == StatusCode::UNAUTHORIZED {
            response = response.header(
                header::WWW_AUTHENTICATE, 
                format!("Bearer error=\"{}\"", match &self {
                    AuthError::MissingToken => "invalid_request",
                    AuthError::InvalidTokenFormat => "invalid_token",
                    AuthError::TokenValidationFailed(_) => "invalid_token",
                    _ => "invalid_token",
                })
            );
        }
        
        // Build the response with JSON body
        let body = axum::body::Body::from(serde_json::to_string(&error_json).unwrap_or_default());
        response.header(header::CONTENT_TYPE, "application/json")
            .body(body)
            .unwrap_or_else(|_| {
                // Fallback to simple string response if JSON fails
                (status, self.to_string()).into_response()
            })
    }
}

/// Extract and validate the JWT from the Authorization header
pub async fn jwt_auth_middleware(
    State(jwks_manager): State<Arc<JWKSManager>>,
    mut request: Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    log::info!("JWT auth middleware called");
    
    // Log request path and method
    log::info!("Request path: {:?}, method: {:?}", request.uri().path(), request.method());
    
    // Check if request is from localhost - bypass auth if it is
    if is_localhost_request(&request) {
        log::info!("Localhost detected, bypassing JWT authentication");
        return Ok(next.run(request).await);
    }
    
    // Log headers for CORS debugging
    log::info!("Request headers:");
    for (name, value) in request.headers() {
        if let Ok(value_str) = value.to_str() {
            log::info!("  {} = {}", name, value_str);
        }
    }
    
    // Check for CORS preflight request
    if request.method() == axum::http::Method::OPTIONS {
        log::info!("CORS preflight request detected, allowing through");
        return Ok(next.run(request).await);
    }
    
    // Extract the token from the Authorization header
    let auth_header = match request.headers().get(header::AUTHORIZATION) {
        Some(header) => header,
        None => {
            log::warn!("No Authorization header found");
            return Err(StatusCode::UNAUTHORIZED);
        }
    };
    
    log::info!("Auth header: {:?}", auth_header);
    
    let auth_header_str = match auth_header.to_str() {
        Ok(s) => s,
        Err(e) => {
            log::warn!("Failed to convert Authorization header to string: {:?}", e);
            return Err(StatusCode::UNAUTHORIZED);
        }
    };
    
    // Check if it's a Bearer token
    log::info!("Auth header string: {:?}", auth_header_str);
    if !auth_header_str.starts_with("Bearer ") {
        log::warn!("Authorization header is not a Bearer token");
        return Err(StatusCode::UNAUTHORIZED);
    }
    
    // Extract the token without the "Bearer " prefix
    let token = &auth_header_str[7..];
    log::info!("Token extracted: length={}", token.len());
    
    // Log JWKS manager state before validation
    let jwks_url = jwks_manager.get_jwks_url();
    log::info!("JWKS URL: {}", jwks_url);
    
    let key_count = jwks_manager.cached_key_count().await;
    log::info!("Cached keys count before validation: {}", key_count);
    
    let is_refreshing = jwks_manager.is_refreshing();
    log::info!("JWKS is currently refreshing: {}", is_refreshing);
    
    // Log token header for debugging
    match jsonwebtoken::decode_header(token) {
        Ok(header) => {
            log::info!("Token header - alg: {:?}, typ: {:?}, kid: {:?}", 
                header.alg, header.typ, header.kid);
            
            // Check if the key ID exists in our cache
            if let Some(kid) = &header.kid {
                log::info!("Checking if key ID '{}' exists in cache", kid);
                let key_exists = jwks_manager.get_key_by_id(kid).await.is_some();
                log::info!("Key ID '{}' exists in cache: {}", kid, key_exists);
                
                if !key_exists {
                    log::info!("Key not found in cache, will attempt to refresh");
                }
            } else {
                log::warn!("Token has no key ID (kid) in header");
            }
        },
        Err(e) => {
            log::warn!("Failed to decode token header: {:?}", e);
        }
    }
    
    // Log JWT claims for debugging audience issues
    match decode_jwt_claims(token) {
        Ok(claims) => {
            if let Some(aud) = claims.get("aud") {
                log::info!("JWT aud claim: {:?}", aud);
            } else {
                log::warn!("JWT has no audience claim");
            }
            
            if let Some(iss) = claims.get("iss") {
                log::info!("JWT iss claim: {:?}", iss);
            }
        },
        Err(e) => {
            log::warn!("Failed to decode JWT claims: {:?}", e);
        }
    }
    
    // Log the configured audience in our application
    if let Some(audience) = &jwks_manager.get_audience() {
        log::info!("Configured audience: {:?}", audience);
    } else {
        log::info!("No audience configured in application");
    }
    
    // Attempt to validate the token and log the result
    log::info!("Attempting to validate token...");
    let token_data = match jwks_manager.validate_token(token).await {
        Ok(data) => {
            log::info!("Token validated successfully!");
            data
        },
        Err(err) => {
            log::error!("Token validation failed: {}", err);
            // Try forcing a refresh of JWKS keys and validate again
            log::info!("Forcing JWKS refresh and trying again...");
            
            if let Err(refresh_err) = jwks_manager.refresh_keys().await {
                log::error!("JWKS refresh failed: {}", refresh_err);
                return Err(StatusCode::UNAUTHORIZED);
            }
            
            log::info!("JWKS refreshed, cached keys count: {}", jwks_manager.cached_key_count().await);
            
            // Try validating again after refresh
            match jwks_manager.validate_token(token).await {
                Ok(data) => {
                    log::info!("Token validated successfully after JWKS refresh!");
                    data
                },
                Err(retry_err) => {
                    log::error!("Token validation still failed after JWKS refresh: {}", retry_err);
                    return Err(StatusCode::UNAUTHORIZED);
                }
            }
        }
    };
    
    // Log token claims for debugging
    log::info!("Token validated for subject: {}", token_data.claims.sub);
    if let Some(email) = token_data.claims.email() {
        log::info!("User email: {}", email);
    }
    log::info!("User role: {:?}", token_data.claims.user_role());
    
    // Store the validated claims in request extensions for handlers to access
    request.extensions_mut().insert(token_data);
    log::info!("Added token data to request extensions");
    
    // Pass the request with the validated claims to the next middleware/handler
    log::info!("Passing request to next middleware/handler");
    Ok(next.run(request).await)
}

/// Decode JWT claims without verification for debugging purposes
pub fn decode_jwt_claims(token: &str) -> Result<serde_json::Value, String> {
    // JWT format is header.payload.signature
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() != 3 {
        return Err("Invalid JWT format".to_string());
    }
    
    // Decode the payload (middle part)
    let payload = parts[1];
    
    // The base64 in JWT is URL-safe, may need padding
    let payload = match URL_SAFE_NO_PAD.decode(payload) {
        Ok(bytes) => bytes,
        Err(e) => return Err(format!("Failed to decode base64: {}", e)),
    };
    
    // Parse JSON
    match serde_json::from_slice(&payload) {
        Ok(claims) => Ok(claims),
        Err(e) => Err(format!("Failed to parse claims JSON: {}", e)),
    }
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
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        // First extract the JWT claims
        let JwtClaims(claims) = JwtClaims::from_request_parts(parts, state)
            .await
            .map_err(|_| AuthError::MissingToken.into_response())?;
        
        // Check if the user has admin role
        if claims.is_admin() {
            Ok(AdminClaims(claims))
        } else {
            // Return a detailed role rejection response
            Err(create_role_rejection(UserRole::Admin, Some(claims.user_role())))
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
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        // First extract the JWT claims
        let JwtClaims(claims) = JwtClaims::from_request_parts(parts, state)
            .await
            .map_err(|_| AuthError::MissingToken.into_response())?;
        
        // Check if the user has developer or admin role
        if claims.is_developer() {
            Ok(DeveloperOrAdminClaims(claims))
        } else {
            // Return a detailed role rejection response
            Err(create_role_rejection(UserRole::Developer, Some(claims.user_role())))
        }
    }
}

/// Extractor for requests that require specifically a developer role (not admin)
/// This is an example of a specialized role extractor
pub struct DeveloperOnlyClaims(pub DynamicClaims);

#[async_trait]
impl<S> FromRequestParts<S> for DeveloperOnlyClaims
where
    S: Send + Sync,
{
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        // First extract the JWT claims
        let JwtClaims(claims) = JwtClaims::from_request_parts(parts, state)
            .await
            .map_err(|_| AuthError::MissingToken.into_response())?;
        
        // Check if the user has specifically the developer role (not admin)
        if claims.user_role() == UserRole::Developer {
            Ok(DeveloperOnlyClaims(claims))
        } else {
            // Create a custom error message
            let error_json = json!({
                "error": "incorrect_role",
                "message": "This endpoint requires specifically a Developer role",
                "details": match claims.user_role() {
                    UserRole::Admin => "Admin users must use the admin-specific endpoint",
                    UserRole::User => "User role is insufficient for this operation",
                    _ => "Current role does not have access to this endpoint",
                },
                "required_role": "developer",
                "user_role": format!("{:?}", claims.user_role()).to_lowercase(),
            });
            
            // Return a custom response
            let response = Response::builder()
                .status(StatusCode::FORBIDDEN)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(serde_json::to_string(&error_json).unwrap_or_default()))
                .unwrap_or_else(|_| (StatusCode::FORBIDDEN, "Incorrect role").into_response());
            
            Err(response)
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

/// Create a custom rejection response for role-based access denial
pub fn create_role_rejection(required_role: UserRole, user_role: Option<UserRole>) -> Response {
    let error_json = json!({
        "error": "insufficient_permissions",
        "message": format!("This endpoint requires {} role", required_role_display(&required_role)),
        "details": match user_role {
            Some(role) => format!("Current user has {} role which is insufficient", role_display(&role)),
            None => "Current user has no role assigned".to_string()
        },
        "required_role": format!("{:?}", required_role).to_lowercase(),
        "user_role": user_role.map(|r| format!("{:?}", r).to_lowercase()),
    });

    Response::builder()
        .status(StatusCode::FORBIDDEN)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_string(&error_json).unwrap_or_default()))
        .unwrap_or_else(|_| (StatusCode::FORBIDDEN, "Insufficient permissions").into_response())
}

/// Create a custom rejection response for project access denial
pub fn create_project_rejection(project_id: &str, user_project: Option<&str>) -> Response {
    let error_json = json!({
        "error": "project_access_denied",
        "message": format!("You don't have access to project {}", project_id),
        "details": match user_project {
            Some(pid) => format!("Current user belongs to project {} but requested access to {}", pid, project_id),
            None => "Current user has no project assigned".to_string()
        },
        "requested_project": project_id,
        "user_project": user_project,
    });

    Response::builder()
        .status(StatusCode::FORBIDDEN)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_string(&error_json).unwrap_or_default()))
        .unwrap_or_else(|_| (StatusCode::FORBIDDEN, "Project access denied").into_response())
}

/// Create a custom combined rejection for both role and project access denial
pub fn create_access_rejection(
    project_id: &str, 
    required_role: UserRole,
    claims: &DynamicClaims
) -> Response {
    // Determine which aspect failed - project, role, or both
    let project_mismatch = !claims.is_for_project(project_id);
    let role_insufficient = !claims.has_role(&required_role);
    
    let error_json = json!({
        "error": if project_mismatch && role_insufficient {
            "access_denied"
        } else if project_mismatch {
            "project_access_denied"
        } else {
            "insufficient_permissions"
        },
        "message": if project_mismatch && role_insufficient {
            format!("You need both access to project {} and {} role", 
                   project_id, required_role_display(&required_role))
        } else if project_mismatch {
            format!("You don't have access to project {}", project_id)
        } else {
            format!("This endpoint requires {} role", required_role_display(&required_role)) 
        },
        "details": {
            "project_access": {
                "has_access": !project_mismatch,
                "requested_project": project_id,
                "user_project": claims.project_id()
            },
            "role_access": {
                "has_access": !role_insufficient,
                "required_role": format!("{:?}", required_role).to_lowercase(),
                "user_role": format!("{:?}", claims.user_role()).to_lowercase()
            }
        }
    });

    Response::builder()
        .status(StatusCode::FORBIDDEN)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_string(&error_json).unwrap_or_default()))
        .unwrap_or_else(|_| (StatusCode::FORBIDDEN, "Access denied").into_response())
}

// Helper function to display role names in a user-friendly way
fn role_display(role: &UserRole) -> &'static str {
    match role {
        UserRole::Admin => "Admin",
        UserRole::Developer => "Developer",
        UserRole::User => "User",
    }
}

// Helper function to display required role names with articles
fn required_role_display(role: &UserRole) -> &'static str {
    match role {
        UserRole::Admin => "an Admin",
        UserRole::Developer => "a Developer",
        UserRole::User => "a User",
    }
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

/// Extractor that validates both project access and role permissions
pub struct ProjectRoleExtractor {
    pub claims: DynamicClaims,
    pub project_id: String,
}

#[async_trait]
impl<S> FromRequestParts<S> for ProjectRoleExtractor
where
    S: Send + Sync,
{
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        // First extract the JWT claims
        let JwtClaims(claims) = JwtClaims::from_request_parts(parts, state)
            .await
            .map_err(|_| AuthError::MissingToken.into_response())?;
        
        // Extract project ID from the path parameters or other sources
        // This is an example using a path parameter, adjust as needed
        let project_id = match parts.uri.path().split('/').nth(2) {
            Some(id) => id.to_string(),
            None => {
                return Err(Response::builder()
                    .status(StatusCode::BAD_REQUEST)
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(json!({
                        "error": "missing_project_id",
                        "message": "Project ID is required but was not found in the request path",
                        "details": "Expected path pattern: /projects/{project_id}/..."
                    }).to_string()))
                    .unwrap_or_else(|_| (StatusCode::BAD_REQUEST, "Missing project ID").into_response()));
            }
        };
        
        Ok(ProjectRoleExtractor {
            claims,
            project_id,
        })
    }
}

impl ProjectRoleExtractor {
    /// Verify the user has the required role for this project
    pub fn verify_role(&self, required_role: UserRole) -> Result<(), Response> {
        // First check project access
        if !self.claims.is_for_project(&self.project_id) {
            return Err(create_project_rejection(&self.project_id, self.claims.project_id()));
        }
        
        // Then check role permissions
        if !self.claims.has_role(&required_role) {
            return Err(create_role_rejection(required_role, Some(self.claims.user_role())));
        }
        
        Ok(())
    }
    
    /// Check if user is an administrator for this project
    pub fn verify_admin(&self) -> Result<(), Response> {
        self.verify_role(UserRole::Admin)
    }
    
    /// Check if user is a developer (or admin) for this project
    pub fn verify_developer(&self) -> Result<(), Response> {
        self.verify_role(UserRole::Developer)
    }
    
    /// Get user information
    pub fn user_id(&self) -> &str {
        &self.claims.sub
    }
    
    /// Get project information
    pub fn project_id(&self) -> &str {
        &self.project_id
    }
    
    /// Get the user's role
    pub fn role(&self) -> UserRole {
        self.claims.user_role()
    }
} 