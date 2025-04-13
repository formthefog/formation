use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Represents the possible user roles in the system
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UserRole {
    /// Administrator with full system access
    Admin,
    /// Developer with project management capabilities
    Developer,
    /// Standard user with limited permissions
    User,
}

impl Default for UserRole {
    fn default() -> Self {
        Self::User
    }
}

impl UserRole {
    /// Parse a role string into a UserRole enum
    pub fn from_str(role: &str) -> Option<Self> {
        match role.to_lowercase().as_str() {
            "admin" => Some(Self::Admin),
            "developer" => Some(Self::Developer),
            "user" => Some(Self::User),
            _ => None,
        }
    }
    
    /// Check if this role is at least as powerful as the required role
    pub fn has_permission(&self, required_role: &UserRole) -> bool {
        match (self, required_role) {
            // Admin can do anything
            (UserRole::Admin, _) => true,
            // Developer can do Developer and User things
            (UserRole::Developer, UserRole::Developer | UserRole::User) => true,
            // User can only do User things
            (UserRole::User, UserRole::User) => true,
            // Otherwise, not enough permissions
            _ => false,
        }
    }
}

/// Custom JWT claims structure for Dynamic Auth tokens
/// 
/// This struct maps the JWT claims from Dynamic Auth to Rust types
/// and includes application-specific claims like project and role.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DynamicClaims {
    /// Standard JWT subject claim - usually contains user ID
    pub sub: String,
    
    /// JWT expiration timestamp
    pub exp: u64,
    
    /// JWT issued-at timestamp
    pub iat: u64,
    
    /// JWT issuer - should match the configured issuer
    #[serde(default)]
    pub iss: Option<String>,
    
    /// JWT audience - should match the configured audience
    #[serde(default)]
    pub aud: Option<String>,
    
    /// JWT not-before timestamp - token not valid before this time
    #[serde(default)]
    pub nbf: Option<u64>,
    
    /// JWT ID - unique identifier for this token
    #[serde(default)]
    pub jti: Option<String>,
    
    /// Project ID for project-scoping - this is a custom claim
    /// that identifies which project the token is valid for
    #[serde(default)]
    pub project: Option<String>,
    
    /// User role for role-based access control
    #[serde(default)]
    pub role: Option<String>,
    
    /// User's wallet address - from Dynamic Auth
    #[serde(default)]
    pub wallet_address: Option<String>,
    
    /// User's email address
    #[serde(default)]
    pub email: Option<String>,
    
    /// User's name or display name
    #[serde(default)]
    pub name: Option<String>,
    
    /// User ID in Dynamic Auth system
    #[serde(default)]
    pub dynamic_user_id: Option<String>,
    
    /// Environment ID in Dynamic Auth system
    #[serde(default)]
    pub env_id: Option<String>,
    
    /// Session ID - used for tracking specific user sessions
    #[serde(default)]
    pub sid: Option<String>,
    
    /// Any additional custom claims returned by Dynamic Auth
    #[serde(flatten)]
    pub additional_claims: HashMap<String, serde_json::Value>,
}

impl DynamicClaims {
    /// Get the project ID, if available
    pub fn project_id(&self) -> Option<&str> {
        self.project.as_deref()
    }
    
    /// Check if token is associated with a specific project
    pub fn is_for_project(&self, project_id: &str) -> bool {
        self.project_id()
            .map(|pid| pid == project_id)
            .unwrap_or(false)
    }
    
    /// Get the user role, defaulting to User if not specified
    pub fn user_role(&self) -> UserRole {
        match &self.role {
            Some(role_str) => UserRole::from_str(role_str).unwrap_or_default(),
            None => UserRole::default(),
        }
    }
    
    /// Check if the user has the required role
    pub fn has_role(&self, required_role: &UserRole) -> bool {
        self.user_role().has_permission(required_role)
    }
    
    /// Check if the user is an admin
    pub fn is_admin(&self) -> bool {
        self.has_role(&UserRole::Admin)
    }
    
    /// Check if the user is a developer (or admin)
    pub fn is_developer(&self) -> bool {
        self.has_role(&UserRole::Developer)
    }
    
    /// Get the user's email address, if available
    pub fn email(&self) -> Option<&str> {
        self.email.as_deref()
    }
    
    /// Get the user's wallet address, if available
    pub fn wallet_address(&self) -> Option<&str> {
        self.wallet_address.as_deref()
    }
    
    /// Get the user's name, if available
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }
    
    /// Check if the token is currently valid based on nbf (not before) claim
    pub fn is_valid_time(&self, current_time: u64) -> bool {
        // Check if token has expired
        if self.exp <= current_time {
            return false;
        }
        
        // Check not before time if present
        if let Some(nbf) = self.nbf {
            if current_time < nbf {
                return false;
            }
        }
        
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_deserialize_claims() {
        // Sample JWT payload with project claim
        let jwt_payload = r#"{
            "sub": "user123",
            "exp": 1703980800,
            "iat": 1703894400,
            "iss": "https://dynamic.xyz",
            "aud": "my-app",
            "project": "proj-abc123"
        }"#;
        
        // Deserialize the claims
        let claims: DynamicClaims = serde_json::from_str(jwt_payload).unwrap();
        
        // Verify the claims were properly deserialized
        assert_eq!(claims.sub, "user123");
        assert_eq!(claims.exp, 1703980800);
        assert_eq!(claims.iat, 1703894400);
        assert_eq!(claims.iss, Some("https://dynamic.xyz".to_string()));
        assert_eq!(claims.aud, Some("my-app".to_string()));
        assert_eq!(claims.project, Some("proj-abc123".to_string()));
        
        // Test the helper method
        assert!(claims.is_for_project("proj-abc123"));
        assert!(!claims.is_for_project("other-project"));
    }
    
    #[test]
    fn test_missing_project() {
        // Sample JWT payload without project claim
        let jwt_payload = r#"{
            "sub": "user123",
            "exp": 1703980800,
            "iat": 1703894400
        }"#;
        
        // Deserialize the claims
        let claims: DynamicClaims = serde_json::from_str(jwt_payload).unwrap();
        
        // Verify the project claim is None
        assert_eq!(claims.project, None);
        assert_eq!(claims.project_id(), None);
        assert!(!claims.is_for_project("any-project"));
    }
    
    #[test]
    fn test_additional_claims() {
        // Sample JWT payload with additional custom claims
        let jwt_payload = r#"{
            "sub": "user123",
            "exp": 1703980800,
            "iat": 1703894400,
            "project": "proj-abc123",
            "custom_field": "custom_value",
            "nested": {
                "field": 123
            }
        }"#;
        
        // Deserialize the claims
        let claims: DynamicClaims = serde_json::from_str(jwt_payload).unwrap();
        
        // Verify standard claims
        assert_eq!(claims.sub, "user123");
        assert_eq!(claims.project, Some("proj-abc123".to_string()));
        
        // Verify additional claims were captured
        assert!(claims.additional_claims.contains_key("custom_field"));
        assert_eq!(
            claims.additional_claims.get("custom_field").unwrap(),
            &serde_json::Value::String("custom_value".to_string())
        );
        
        assert!(claims.additional_claims.contains_key("nested"));
    }
    
    #[test]
    fn test_user_roles() {
        // Test admin role
        let admin_payload = r#"{
            "sub": "admin123",
            "exp": 1703980800,
            "iat": 1703894400,
            "role": "admin"
        }"#;
        
        let admin_claims: DynamicClaims = serde_json::from_str(admin_payload).unwrap();
        assert_eq!(admin_claims.user_role(), UserRole::Admin);
        assert!(admin_claims.is_admin());
        assert!(admin_claims.is_developer());
        assert!(admin_claims.has_role(&UserRole::User));
        
        // Test developer role
        let dev_payload = r#"{
            "sub": "dev123",
            "exp": 1703980800,
            "iat": 1703894400,
            "role": "developer"
        }"#;
        
        let dev_claims: DynamicClaims = serde_json::from_str(dev_payload).unwrap();
        assert_eq!(dev_claims.user_role(), UserRole::Developer);
        assert!(!dev_claims.is_admin());
        assert!(dev_claims.is_developer());
        assert!(dev_claims.has_role(&UserRole::User));
        assert!(!dev_claims.has_role(&UserRole::Admin));
        
        // Test user role
        let user_payload = r#"{
            "sub": "user123",
            "exp": 1703980800,
            "iat": 1703894400,
            "role": "user"
        }"#;
        
        let user_claims: DynamicClaims = serde_json::from_str(user_payload).unwrap();
        assert_eq!(user_claims.user_role(), UserRole::User);
        assert!(!user_claims.is_admin());
        assert!(!user_claims.is_developer());
        assert!(user_claims.has_role(&UserRole::User));
        assert!(!user_claims.has_role(&UserRole::Developer));
        
        // Test default when role is missing
        let no_role_payload = r#"{
            "sub": "user123",
            "exp": 1703980800,
            "iat": 1703894400
        }"#;
        
        let no_role_claims: DynamicClaims = serde_json::from_str(no_role_payload).unwrap();
        assert_eq!(no_role_claims.user_role(), UserRole::User);
        assert!(!no_role_claims.is_admin());
        assert!(!no_role_claims.is_developer());
        assert!(no_role_claims.has_role(&UserRole::User));
    }
    
    #[test]
    fn test_role_from_str() {
        assert_eq!(UserRole::from_str("admin"), Some(UserRole::Admin));
        assert_eq!(UserRole::from_str("Admin"), Some(UserRole::Admin));
        assert_eq!(UserRole::from_str("ADMIN"), Some(UserRole::Admin));
        assert_eq!(UserRole::from_str("developer"), Some(UserRole::Developer));
        assert_eq!(UserRole::from_str("user"), Some(UserRole::User));
        assert_eq!(UserRole::from_str("unknown"), None);
    }
    
    #[test]
    fn test_additional_standard_claims() {
        // Sample JWT payload with additional standard claims
        let jwt_payload = r#"{
            "sub": "user123",
            "exp": 1703980800,
            "iat": 1703894400,
            "nbf": 1703894100,
            "jti": "token-id-123",
            "email": "user@example.com",
            "name": "Test User",
            "wallet_address": "0x123456789abcdef",
            "dynamic_user_id": "dyn-user-123",
            "env_id": "env-456",
            "sid": "session-789"
        }"#;
        
        // Deserialize the claims
        let claims: DynamicClaims = serde_json::from_str(jwt_payload).unwrap();
        
        // Verify the standard claims
        assert_eq!(claims.sub, "user123");
        assert_eq!(claims.nbf, Some(1703894100));
        assert_eq!(claims.jti, Some("token-id-123".to_string()));
        
        // Verify the additional user claims
        assert_eq!(claims.email, Some("user@example.com".to_string()));
        assert_eq!(claims.name, Some("Test User".to_string()));
        assert_eq!(claims.wallet_address, Some("0x123456789abcdef".to_string()));
        assert_eq!(claims.dynamic_user_id, Some("dyn-user-123".to_string()));
        assert_eq!(claims.env_id, Some("env-456".to_string()));
        assert_eq!(claims.sid, Some("session-789".to_string()));
        
        // Test helper methods
        assert_eq!(claims.email(), Some("user@example.com"));
        assert_eq!(claims.name(), Some("Test User"));
        assert_eq!(claims.wallet_address(), Some("0x123456789abcdef"));
        
        // Test time validity
        assert!(claims.is_valid_time(1703894400)); // Valid time (equal to iat)
        assert!(claims.is_valid_time(1703895000)); // Valid time (between iat and exp)
        assert!(!claims.is_valid_time(1703894000)); // Invalid time (before nbf)
        assert!(!claims.is_valid_time(1703981000)); // Invalid time (after exp)
    }
} 