pub mod ecdsa;

pub use ecdsa::{
    RecoveredAddress,
    OptionalRecoveredAddress,
    SignatureError,
    ecdsa_auth_middleware,
    extract_signature_parts,
    recover_address,
};

// Placeholder implementations to make the codebase compile
// These will be replaced with ECDSA-based authentication
use serde::{Serialize, Deserialize};
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};

// JWT Claims placeholder
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DynamicClaims {
    pub sub: String,
    pub dynamic_user_id: Option<String>,
    pub email: Option<String>,
    pub role: Option<String>,
}

impl DynamicClaims {
    pub fn user_role(&self) -> Option<UserRole> {
        self.role.as_ref().and_then(|r| r.parse().ok())
    }
    
    pub fn email(&self) -> Option<&str> {
        self.email.as_deref()
    }
}

// Error type for authentication
#[derive(Debug)]
pub enum AuthError {
    Unauthorized,
    Forbidden,
    InvalidToken,
    MissingClaims,
}

impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        let status = match self {
            Self::Unauthorized => StatusCode::UNAUTHORIZED,
            Self::Forbidden => StatusCode::FORBIDDEN,
            Self::InvalidToken => StatusCode::UNAUTHORIZED, 
            Self::MissingClaims => StatusCode::UNAUTHORIZED,
        };
        
        let body = Json(serde_json::json!({
            "error": format!("{:?}", self)
        }));
        
        (status, body).into_response()
    }
}

// User role enum
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum UserRole {
    Admin,
    Developer,
    User,
}

impl std::str::FromStr for UserRole {
    type Err = ();
    
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "admin" => Ok(Self::Admin),
            "developer" => Ok(Self::Developer),
            "user" => Ok(Self::User),
            _ => Err(()),
        }
    }
}

// Path access verification helper
pub fn verify_project_path_access(claims: &DynamicClaims, project_id: &str) -> Result<(), AuthError> {
    // In a real implementation, this would check project access
    // For now, just allow access
    Ok(())
}

// Resource access verification helper
pub fn has_resource_access(claims: &DynamicClaims, resource_id: &str) -> bool {
    // In a real implementation, this would check resource access
    // For now, just allow access
    true
}

// Role verification helper
pub fn verify_role(claims: &DynamicClaims, required_role: UserRole) -> Result<(), AuthError> {
    match claims.user_role() {
        Some(role) if role == required_role => Ok(()),
        _ => Err(AuthError::Forbidden),
    }
}

// Extract user info helper
pub fn extract_user_info(claims: &DynamicClaims) -> (String, Option<String>) {
    let user_id = claims.sub.clone();
    let email = claims.email.clone();
    (user_id, email)
}

/// Extract the original user address from the request body if present
/// This is used when an admin node is making a request on behalf of a user
pub fn extract_original_user_address(body: &serde_json::Value) -> Option<String> {
    body.get("original_user_address")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}
