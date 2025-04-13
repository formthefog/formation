pub mod middleware;
pub mod rate_limiter;
pub mod audit;

pub use middleware::{api_key_auth_middleware, ApiKeyAuth, api_key_error_response};
pub use rate_limiter::{ApiKeyRateLimiter, RateLimitCheckResult, get_rate_limit_headers};
pub use audit::{ApiKeyEvent, ApiKeyEventType, ApiKeyAuditLog, API_KEY_AUDIT_LOG};

use crate::accounts::Account;
use chrono::{DateTime, Utc};
use serde::{Serialize, Deserialize};

// Define API key types directly in this module
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ApiKeyStatus {
    Active,
    Revoked,
    Expired,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ApiKeyScope {
    ReadOnly,
    ReadWrite,
    Admin,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ApiKey {
    pub id: String,
    pub name: String,
    pub account_id: String,
    pub hashed_secret: String,
    pub scope: ApiKeyScope,
    pub status: ApiKeyStatus,
    pub created_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ApiKeyMetadata {
    pub id: String,
    pub name: String,
    pub scope: ApiKeyScope,
    pub status: ApiKeyStatus,
    pub created_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub description: Option<String>,
}

pub enum ApiKeyError {
    Missing,
    InvalidFormat,
    NotFound,
    Revoked,
    Expired,
    InsufficientPermissions,
    IpNotAllowed,
    RateLimitExceeded,
}

pub enum ApiKeyOp {
    Create(ApiKey),
    Update(ApiKey),
    Revoke(String),
    Delete(String),
}

impl ApiKey {
    pub fn new(name: String, account_id: String, scope: ApiKeyScope, description: Option<String>) -> (Self, String) {
        // Implementation details
        let id = format!("key_{}", uuid::Uuid::new_v4());
        let secret = format!("sk_{}", uuid::Uuid::new_v4());
        let hashed_secret = format!("hashed_{}", &secret); // In real app, hash this

        (
            Self {
                id,
                name,
                account_id,
                hashed_secret,
                scope,
                status: ApiKeyStatus::Active,
                created_at: Utc::now(),
                expires_at: None,
                description,
            },
            secret
        )
    }

    pub fn is_valid(&self) -> bool {
        match self.status {
            ApiKeyStatus::Active => {
                // Check if expired
                if let Some(expires_at) = self.expires_at {
                    Utc::now() < expires_at
                } else {
                    true
                }
            },
            _ => false,
        }
    }

    pub fn is_allowed_from_ip(&self, _ip: &str) -> bool {
        // Implement IP restrictions if needed
        true
    }

    pub fn can_perform_operation(&self, operation: &str) -> bool {
        match self.scope {
            ApiKeyScope::Admin => true,
            ApiKeyScope::ReadWrite => !operation.starts_with("admin."),
            ApiKeyScope::ReadOnly => operation.starts_with("models.get") || operation.starts_with("models.list"),
        }
    }

    pub fn verify_secret(&self, full_key: &str) -> bool {
        // In a real implementation, we would hash the secret and compare with hashed_secret
        // For simplicity, we're using a placeholder implementation
        let expected_secret = format!("hashed_{}", full_key);
        self.hashed_secret == expected_secret
    }

    pub fn revoke(&mut self) {
        self.status = ApiKeyStatus::Revoked;
    }
}

impl From<&ApiKey> for ApiKeyMetadata {
    fn from(key: &ApiKey) -> Self {
        Self {
            id: key.id.clone(),
            name: key.name.clone(),
            scope: key.scope.clone(),
            status: key.status.clone(),
            created_at: key.created_at,
            expires_at: key.expires_at,
            description: key.description.clone(),
        }
    }
}

/// Create a new API key for an account
pub fn create_api_key(
    account: &mut Account,
    name: String,
    scope: ApiKeyScope,
    description: Option<String>,
) -> Result<(ApiKeyMetadata, String), String> {
    // Generate a new API key
    let (api_key, secret) = ApiKey::new(name, account.address.clone(), scope, description);
    
    // Add the key to the account
    let metadata = ApiKeyMetadata::from(&api_key);
    account.add_api_key(api_key)?;
    
    Ok((metadata, secret))
}