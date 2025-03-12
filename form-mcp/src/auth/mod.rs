// Authentication module for the MCP server
//
// This module handles authentication and authorization for the MCP server,
// including signature verification and permission management.

pub mod keypair;
pub mod signature;
pub mod permissions;

use actix_web::{
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    Error, HttpMessage,
};
use crate::errors::AuthError;
use futures_util::future::{ok, LocalBoxFuture, Ready};
use std::rc::Rc;
use std::sync::Arc;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use hmac::{Hmac, Mac};
use jwt::{SignWithKey, VerifyWithKey};
use sha2::Sha256;
use std::collections::BTreeMap;
use serde::{Serialize, Deserialize};
use std::time::{SystemTime, UNIX_EPOCH};

/// Token claims for JWT authentication
#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    /// Subject (user ID)
    pub sub: String,
    /// Issued at timestamp
    pub iat: u64,
    /// Expiration timestamp
    pub exp: u64,
    /// Roles assigned to the user
    pub roles: Vec<String>,
}

/// Verifies the authentication of a request
pub async fn verify_authentication(req: &ServiceRequest) -> Result<AuthData, AuthError> {
    // Get the authorization header
    let auth_header = req
        .headers()
        .get("Authorization")
        .ok_or(AuthError::MissingAuth)?;
    
    // Parse the header value
    let auth_str = auth_header.to_str().map_err(|_| AuthError::InvalidToken)?;
    
    // Check if it's a Bearer token
    if !auth_str.starts_with("Bearer ") {
        return Err(AuthError::InvalidToken);
    }
    
    // Get the token
    let token = auth_str.trim_start_matches("Bearer ").trim();
    if token.is_empty() {
        return Err(AuthError::InvalidToken);
    }
    
    // Parse and verify the token
    let auth_data = verify_token(token)?;
    
    Ok(auth_data)
}

/// Checks if a request is authorized to access a particular resource
pub async fn check_authorization(
    auth_data: &AuthData,
    resource: &str,
    action: &str
) -> Result<bool, AuthError> {
    // Check if the user has permission to access the resource
    for permission in &auth_data.permissions {
        // In a real implementation, permissions might have a more structured format
        // For now, we just check if the permission string matches the resource:action pattern
        if permission == &format!("{}:{}", resource, action) || permission == "admin" {
            return Ok(true);
        }
    }
    
    Ok(false)
}

/// Create a new JWT token for a user
pub fn create_token(
    user_id: &str, 
    roles: Vec<String>, 
    secret: &[u8], 
    expires_in_seconds: u64
) -> Result<String, AuthError> {
    // Create a HMAC-SHA256 key from the secret
    let key: Hmac<Sha256> = Hmac::new_from_slice(secret)
        .map_err(|_| AuthError::Internal("Failed to create signing key".to_string()))?;
    
    // Get current timestamp
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| AuthError::Internal("Failed to get current time".to_string()))?
        .as_secs();
    
    // Create claims
    let mut claims = BTreeMap::new();
    claims.insert("sub", user_id);
    
    // Fix temporary lifetime issues by creating owned strings
    let iat_str = now.to_string();
    claims.insert("iat", &iat_str);
    
    let exp_str = (now + expires_in_seconds).to_string();
    claims.insert("exp", &exp_str);
    
    // Add roles as a comma-separated string
    let roles_str = roles.join(",");
    claims.insert("roles", &roles_str);
    
    // Sign the token
    let token = claims.sign_with_key(&key)
        .map_err(|_| AuthError::Internal("Failed to create token".to_string()))?;
    
    Ok(token)
}

/// Verify a JWT token and extract the user information
pub fn verify_token(token: &str) -> Result<AuthData, AuthError> {
    // In a real implementation, the secret would be loaded from configuration
    // For now, we use a hard-coded secret for development purposes
    let secret = b"your-secret-key-which-should-be-very-long-and-complex";
    
    // Create a HMAC-SHA256 key from the secret
    let key: Hmac<Sha256> = Hmac::new_from_slice(secret)
        .map_err(|_| AuthError::Internal("Failed to create verification key".to_string()))?;
    
    // Verify and decode the token
    let claims: BTreeMap<String, String> = token.verify_with_key(&key)
        .map_err(|_| AuthError::InvalidToken)?;
    
    // Extract user ID
    let user_id = claims.get("sub")
        .ok_or(AuthError::InvalidToken)?
        .to_string();
    
    // Check token expiration
    let exp = claims.get("exp")
        .ok_or(AuthError::InvalidToken)?
        .parse::<u64>()
        .map_err(|_| AuthError::InvalidToken)?;
    
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| AuthError::Internal("Failed to get current time".to_string()))?
        .as_secs();
    
    if exp < now {
        return Err(AuthError::TokenExpired);
    }
    
    // Extract roles
    let roles = claims.get("roles")
        .map(|r| r.split(',').map(|s| s.to_string()).collect())
        .unwrap_or_else(|| Vec::new());
    
    // Convert roles to permissions
    // In a real implementation, this would involve looking up the permissions
    // associated with each role from a database or configuration
    let permissions = roles.clone();
    
    Ok(AuthData {
        user_id,
        permissions,
    })
}

/// Middleware for handling authentication
#[derive(Clone)]
pub struct AuthenticationMiddleware {
    pub enable_auth: bool,
}

impl AuthenticationMiddleware {
    pub fn new(enable_auth: bool) -> Self {
        Self { enable_auth }
    }
}

impl<S, B> Transform<S, ServiceRequest> for AuthenticationMiddleware
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Transform = AuthenticationMiddlewareService<S>;
    type InitError = ();
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ok(AuthenticationMiddlewareService {
            service: Rc::new(service),
            enable_auth: self.enable_auth,
        })
    }
}

pub struct AuthenticationMiddlewareService<S> {
    service: Rc<S>,
    enable_auth: bool,
}

impl<S, B> Service<ServiceRequest> for AuthenticationMiddlewareService<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let svc = self.service.clone();
        let enable_auth = self.enable_auth;
        
        Box::pin(async move {
            // Skip authentication for certain paths
            let path = req.path();
            if path == "/health" || path == "/api/v1/health" || path.starts_with("/public") {
                return svc.call(req).await;
            }
            
            // If auth is disabled, skip verification
            if !enable_auth {
                return svc.call(req).await;
            }
            
            // Verify authentication
            match verify_authentication(&req).await {
                Ok(auth_data) => {
                    // Store auth data in request extensions
                    req.extensions_mut().insert(auth_data);
                    svc.call(req).await
                }
                Err(err) => {
                    let error_message = format!("Authentication failed: {}", err);
                    Err(actix_web::error::ErrorUnauthorized(error_message))
                }
            }
        })
    }
}

/// Authentication data that will be attached to requests
#[derive(Debug, Clone, serde::Serialize)]
pub struct AuthData {
    pub user_id: String,
    pub permissions: Vec<String>,
}

/// Extract AuthData from request extensions
pub fn get_auth_data(req: &ServiceRequest) -> Option<AuthData> {
    req.extensions().get::<AuthData>().cloned()
} 