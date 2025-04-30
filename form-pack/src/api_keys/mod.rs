pub mod middleware;
pub mod client;

pub use middleware::{api_key_auth_middleware, ApiKeyAuth, api_key_error_response};
pub use client::ApiKeyClient;

use chrono::{DateTime, Utc};
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ApiKeyStatus {
    Active,
    Revoked,
    Expired,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ApiKeyScope {
    ReadOnly,
    ReadWrite,
    Admin,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ApiKeyInfo {
    pub id: String,
    pub name: String,
    pub account_id: String,
    pub scope: ApiKeyScope,
    pub status: ApiKeyStatus,
    pub created_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone)]
pub enum ApiKeyError {
    Missing,
    InvalidFormat,
    NotFound,
    Revoked,
    Expired,
    InsufficientPermissions,
    RateLimitExceeded,
    ServiceError,
}

impl ApiKeyInfo {
    pub fn is_valid(&self) -> bool {
        if self.status != ApiKeyStatus::Active {
            return false;
        }
        
        if let Some(expires_at) = self.expires_at {
            Utc::now() < expires_at
        } else {
            true
        }
    }
    
    pub fn can_perform_operation(&self, operation: &str) -> bool {
        match self.scope {
            ApiKeyScope::Admin => true,
            ApiKeyScope::ReadWrite => !operation.starts_with("admin."),
            ApiKeyScope::ReadOnly => operation.starts_with("get") || operation.starts_with("list"),
        }
    }
} 