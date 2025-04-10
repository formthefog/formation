//! Project Resource Access Control
//! 
//! This module provides models and utilities for managing access control
//! between projects and marketplace resources (agents and models).

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use crate::auth::UserRole;
use axum::response::{IntoResponse, Response};
use axum::http::StatusCode;
use serde_json::json;

/// Represents the type of marketplace resource
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ResourceType {
    /// AI Agent resource
    Agent,
    /// AI Model resource
    Model,
}

/// Represents the level of access a project has to a resource
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AccessLevel {
    /// Read-only access
    ReadOnly,
    /// Full access including deployment and modification
    FullAccess,
    /// Owner-level access (can grant access to others)
    Owner,
}

/// Represents an access grant between a project and a marketplace resource
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectResourceAccess {
    /// ID of the project granted access
    pub project_id: String,
    
    /// ID of the resource (agent_id or model_id)
    pub resource_id: String,
    
    /// Type of resource (Agent or Model)
    pub resource_type: ResourceType,
    
    /// Level of access granted
    pub access_level: AccessLevel,
    
    /// User ID who granted this access
    pub granted_by: String,
    
    /// Timestamp when access was granted
    pub granted_at: i64,
    
    /// Optional expiration time for temporary access
    pub expires_at: Option<i64>,
    
    /// Optional metadata associated with this access grant
    #[serde(default)]
    pub metadata: HashMap<String, String>,
}

impl ProjectResourceAccess {
    /// Create a new project resource access grant
    pub fn new(
        project_id: String,
        resource_id: String,
        resource_type: ResourceType,
        access_level: AccessLevel,
        granted_by: String,
    ) -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;
            
        Self {
            project_id,
            resource_id,
            resource_type,
            access_level,
            granted_by,
            granted_at: now,
            expires_at: None,
            metadata: HashMap::new(),
        }
    }
    
    /// Check if this access grant is currently valid
    pub fn is_valid(&self) -> bool {
        if let Some(expires_at) = self.expires_at {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() as i64;
                
            return now < expires_at;
        }
        
        true // No expiration means always valid
    }
    
    /// Set an expiration time for this access grant
    pub fn with_expiration(mut self, expires_at: i64) -> Self {
        self.expires_at = Some(expires_at);
        self
    }
    
    /// Add metadata to this access grant
    pub fn with_metadata(mut self, key: &str, value: &str) -> Self {
        self.metadata.insert(key.to_string(), value.to_string());
        self
    }
}

/// In-memory store for project resource access records
/// 
/// This is a simple implementation - in a real system, this would be stored
/// in a database with proper indexing.
#[derive(Debug, Clone, Default)]
pub struct ProjectAccessStore {
    /// Map of resource_type+resource_id -> set of project accesses
    access_records: HashMap<(ResourceType, String), Vec<ProjectResourceAccess>>,
}

impl ProjectAccessStore {
    /// Create a new empty store
    pub fn new() -> Self {
        Self {
            access_records: HashMap::new(),
        }
    }
    
    /// Grant access for a project to a resource
    pub fn grant_access(&mut self, access: ProjectResourceAccess) {
        let key = (access.resource_type, access.resource_id.clone());
        let records = self.access_records.entry(key).or_insert_with(Vec::new);
        
        // Remove any existing access record for this project and resource
        records.retain(|r| r.project_id != access.project_id);
        
        // Add the new access record
        records.push(access);
    }
    
    /// Revoke access for a project to a resource
    pub fn revoke_access(
        &mut self,
        project_id: &str,
        resource_type: ResourceType,
        resource_id: &str
    ) -> bool {
        let key = (resource_type, resource_id.to_string());
        
        if let Some(records) = self.access_records.get_mut(&key) {
            let original_len = records.len();
            records.retain(|r| r.project_id != project_id);
            return original_len != records.len();
        }
        
        false
    }
    
    /// Check if a project has specific access to a resource
    pub fn has_access(
        &self,
        project_id: &str,
        resource_type: ResourceType,
        resource_id: &str,
        minimum_access: AccessLevel
    ) -> bool {
        let key = (resource_type, resource_id.to_string());
        
        if let Some(records) = self.access_records.get(&key) {
            for record in records {
                if record.project_id == project_id && record.is_valid() {
                    return match (record.access_level, minimum_access) {
                        // Owner can do anything
                        (AccessLevel::Owner, _) => true,
                        // Full access can do read-only and full access operations
                        (AccessLevel::FullAccess, AccessLevel::ReadOnly | AccessLevel::FullAccess) => true,
                        // Read-only access can only do read-only operations
                        (AccessLevel::ReadOnly, AccessLevel::ReadOnly) => true,
                        // Otherwise, insufficient access
                        _ => false,
                    };
                }
            }
        }
        
        false
    }
    
    /// List all projects that have access to a resource
    pub fn list_projects_with_access(
        &self,
        resource_type: ResourceType,
        resource_id: &str
    ) -> Vec<String> {
        let key = (resource_type, resource_id.to_string());
        
        if let Some(records) = self.access_records.get(&key) {
            return records
                .iter()
                .filter(|r| r.is_valid())
                .map(|r| r.project_id.clone())
                .collect();
        }
        
        Vec::new()
    }
    
    /// List all resources a project has access to
    pub fn list_resources_for_project(
        &self,
        project_id: &str,
        resource_type: Option<ResourceType>
    ) -> Vec<(ResourceType, String, AccessLevel)> {
        let mut results = Vec::new();
        
        for ((res_type, res_id), records) in &self.access_records {
            if let Some(req_type) = resource_type {
                if *res_type != req_type {
                    continue;
                }
            }
            
            for record in records {
                if record.project_id == project_id && record.is_valid() {
                    results.push((*res_type, res_id.clone(), record.access_level));
                }
            }
        }
        
        results
    }
    
    /// Get the access level a project has for a resource
    pub fn get_access_level(
        &self,
        project_id: &str,
        resource_type: ResourceType,
        resource_id: &str
    ) -> Option<AccessLevel> {
        let key = (resource_type, resource_id.to_string());
        
        if let Some(records) = self.access_records.get(&key) {
            for record in records {
                if record.project_id == project_id && record.is_valid() {
                    return Some(record.access_level);
                }
            }
        }
        
        None
    }
}

/// Error types for project resource access
#[derive(Debug)]
pub enum ProjectAccessError {
    /// Project does not have access to the resource
    AccessDenied {
        project_id: String,
        resource_type: ResourceType,
        resource_id: String,
        required_access: AccessLevel,
    },
    
    /// Resource not found
    ResourceNotFound {
        resource_type: ResourceType,
        resource_id: String,
    },
    
    /// Project not found
    ProjectNotFound {
        project_id: String,
    },
    
    /// User doesn't have permission to manage access
    PermissionDenied {
        reason: String,
    },
    
    /// Other errors
    Other(String),
}

impl std::fmt::Display for ProjectAccessError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AccessDenied { project_id, resource_type, resource_id, required_access } => {
                write!(f, "Project {} does not have {:?} access to {:?} {}", 
                       project_id, required_access, resource_type, resource_id)
            }
            Self::ResourceNotFound { resource_type, resource_id } => {
                write!(f, "{:?} with ID {} not found", resource_type, resource_id)
            }
            Self::ProjectNotFound { project_id } => {
                write!(f, "Project with ID {} not found", project_id)
            }
            Self::PermissionDenied { reason } => {
                write!(f, "Permission denied: {}", reason)
            }
            Self::Other(msg) => {
                write!(f, "Project access error: {}", msg)
            }
        }
    }
}

impl std::error::Error for ProjectAccessError {}

impl IntoResponse for ProjectAccessError {
    fn into_response(self) -> Response {
        let (status, json_body) = match &self {
            Self::AccessDenied { project_id, resource_type, resource_id, required_access } => {
                (StatusCode::FORBIDDEN, json!({
                    "error": "resource_access_denied",
                    "message": format!("Project does not have sufficient access to this resource"),
                    "details": {
                        "project_id": project_id,
                        "resource_type": format!("{:?}", resource_type),
                        "resource_id": resource_id,
                        "required_access": format!("{:?}", required_access),
                    }
                }))
            }
            Self::ResourceNotFound { resource_type, resource_id } => {
                (StatusCode::NOT_FOUND, json!({
                    "error": "resource_not_found",
                    "message": format!("The requested resource was not found"),
                    "details": {
                        "resource_type": format!("{:?}", resource_type),
                        "resource_id": resource_id,
                    }
                }))
            }
            Self::ProjectNotFound { project_id } => {
                (StatusCode::NOT_FOUND, json!({
                    "error": "project_not_found",
                    "message": format!("Project with ID {} not found", project_id),
                }))
            }
            Self::PermissionDenied { reason } => {
                (StatusCode::FORBIDDEN, json!({
                    "error": "permission_denied",
                    "message": format!("You don't have permission to perform this operation"),
                    "details": reason
                }))
            }
            Self::Other(msg) => {
                (StatusCode::INTERNAL_SERVER_ERROR, json!({
                    "error": "project_access_error",
                    "message": msg
                }))
            }
        };
        
        let body = axum::body::Body::from(serde_json::to_string(&json_body).unwrap_or_default());
        axum::response::Response::builder()
            .status(status)
            .header(axum::http::header::CONTENT_TYPE, "application/json")
            .body(body)
            .unwrap_or_else(|_| (status, self.to_string()).into_response())
    }
}

/// Helper functions for project resource access

/// Verify a project has required access to a resource
pub fn verify_project_resource_access(
    access_store: &ProjectAccessStore,
    project_id: &str,
    resource_type: ResourceType,
    resource_id: &str,
    required_access: AccessLevel
) -> Result<(), ProjectAccessError> {
    if access_store.has_access(project_id, resource_type, resource_id, required_access) {
        Ok(())
    } else {
        Err(ProjectAccessError::AccessDenied {
            project_id: project_id.to_string(),
            resource_type,
            resource_id: resource_id.to_string(),
            required_access,
        })
    }
} 