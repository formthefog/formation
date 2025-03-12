// Permissions module for authorization
//
// This module handles role-based permissions for the MCP server.

use std::collections::{HashMap, HashSet};
use std::sync::RwLock;
use once_cell::sync::Lazy;
use crate::errors::AuthError;

/// Permission represents a specific action that can be performed
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct Permission {
    /// The resource being accessed (e.g., "vm", "network")
    pub resource: String,
    /// The action being performed (e.g., "read", "write", "create")
    pub action: String,
}

impl Permission {
    /// Create a new permission
    pub fn new(resource: impl Into<String>, action: impl Into<String>) -> Self {
        Self {
            resource: resource.into(),
            action: action.into(),
        }
    }
    
    /// Convert the permission to a string representation
    pub fn to_string(&self) -> String {
        format!("{}:{}", self.resource, self.action)
    }
}

/// Role represents a set of permissions
#[derive(Debug, Clone)]
pub struct Role {
    /// Name of the role
    pub name: String,
    /// Description of the role
    pub description: String,
    /// Set of permissions granted by this role
    pub permissions: HashSet<Permission>,
}

impl Role {
    /// Create a new role
    pub fn new(name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            permissions: HashSet::new(),
        }
    }
    
    /// Add a permission to the role
    pub fn add_permission(&mut self, permission: Permission) {
        self.permissions.insert(permission);
    }
    
    /// Check if the role has a specific permission
    pub fn has_permission(&self, resource: &str, action: &str) -> bool {
        self.permissions.contains(&Permission::new(resource, action))
    }
}

/// Global roles registry
static ROLES: Lazy<RwLock<HashMap<String, Role>>> = Lazy::new(|| {
    let mut roles = HashMap::new();
    
    // Add default roles
    let mut admin = Role::new("admin", "Administrator with full access");
    admin.add_permission(Permission::new("*", "*"));
    roles.insert("admin".to_string(), admin);
    
    let mut user = Role::new("user", "Standard user with limited access");
    user.add_permission(Permission::new("vm", "read"));
    user.add_permission(Permission::new("network", "read"));
    roles.insert("user".to_string(), user);
    
    RwLock::new(roles)
});

/// Check if a user has a specific permission
pub fn has_permission(role_name: &str, resource: &str, action: &str) -> Result<bool, AuthError> {
    let roles = ROLES.read().map_err(|_| AuthError::Internal("Failed to read roles".to_string()))?;
    
    match roles.get(role_name) {
        Some(role) => Ok(role.has_permission(resource, action)),
        None => Err(AuthError::InvalidRole(role_name.to_string())),
    }
}

/// Get all available roles
pub fn get_all_roles() -> Result<Vec<Role>, AuthError> {
    let roles = ROLES.read().map_err(|_| AuthError::Internal("Failed to read roles".to_string()))?;
    Ok(roles.values().cloned().collect())
} 