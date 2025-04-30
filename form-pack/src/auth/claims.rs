use axum::http::HeaderMap;
use serde::{Serialize, Deserialize};
use std::fmt;
use std::collections::HashMap;

/// User roles in the system
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum UserRole {
    Admin,
    Developer,
    Viewer,
}

impl fmt::Display for UserRole {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UserRole::Admin => write!(f, "admin"),
            UserRole::Developer => write!(f, "developer"),
            UserRole::Viewer => write!(f, "viewer"),
        }
    }
}

/// JWT claims structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JwtClaims {
    pub sub: String,
    pub exp: u64,
    pub iat: u64,
    pub email: Option<String>,
    pub wallet_address: Option<String>,
    pub role: UserRole,
    pub project_access: HashMap<String, UserRole>,
}

impl JwtClaims {
    pub fn has_project_access(&self, project_id: &str) -> bool {
        self.project_access.contains_key(project_id)
    }

    pub fn get_project_role(&self, project_id: &str) -> Option<&UserRole> {
        self.project_access.get(project_id)
    }

    pub fn is_admin(&self) -> bool {
        self.role == UserRole::Admin
    }

    pub fn is_developer(&self) -> bool {
        self.role == UserRole::Developer
    }

    pub fn is_viewer(&self) -> bool {
        self.role == UserRole::Viewer
    }
}

/// Extract token from authorization header
pub fn extract_token(headers: &HeaderMap) -> Option<String> {
    let auth_header = headers.get("authorization")?;
    let auth_header = auth_header.to_str().ok()?;
    
    if !auth_header.starts_with("Bearer ") {
        return None;
    }
    
    Some(auth_header[7..].to_string())
} 