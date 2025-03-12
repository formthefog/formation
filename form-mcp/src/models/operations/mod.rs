// Operations module
//
// This module provides types and functions for managing long-running operations.

mod repository;
#[cfg(test)]
mod tests;

use std::time::{Duration, SystemTime};
use serde::{Serialize, Deserialize};
use uuid::Uuid;

pub use repository::{OperationsRepository, create_repository};

/// Status of an operation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum OperationStatus {
    /// Operation is queued but not yet started
    Queued,
    /// Operation is currently running
    Running,
    /// Operation completed successfully
    Completed,
    /// Operation failed
    Failed,
    /// Operation was cancelled
    Cancelled,
}

impl std::fmt::Display for OperationStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OperationStatus::Queued => write!(f, "queued"),
            OperationStatus::Running => write!(f, "running"),
            OperationStatus::Completed => write!(f, "completed"),
            OperationStatus::Failed => write!(f, "failed"),
            OperationStatus::Cancelled => write!(f, "cancelled"),
        }
    }
}

/// Represents a long-running operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Operation {
    /// Unique identifier for the operation
    pub id: String,
    /// User ID that initiated the operation
    pub user_id: String,
    /// Tool name that is being executed
    pub tool_name: String,
    /// Current status of the operation
    pub status: OperationStatus,
    /// Progress of the operation (0.0 to 1.0)
    pub progress: Option<f32>,
    /// Result of the operation (if completed)
    pub result: Option<serde_json::Value>,
    /// Error message (if failed)
    pub error: Option<String>,
    /// When the operation was created
    pub created_at: SystemTime,
    /// When the operation was last updated
    pub updated_at: SystemTime,
    /// When the operation completed (if completed)
    pub completed_at: Option<SystemTime>,
    /// Time-to-live for the operation record
    pub ttl: Duration,
}

impl Operation {
    /// Create a new operation
    pub fn new(user_id: String, tool_name: String) -> Self {
        let now = SystemTime::now();
        Self {
            id: Uuid::new_v4().to_string(),
            user_id,
            tool_name,
            status: OperationStatus::Queued,
            progress: Some(0.0),
            result: None,
            error: None,
            created_at: now,
            updated_at: now,
            completed_at: None,
            ttl: Duration::from_secs(3600), // 1 hour by default
        }
    }
    
    /// Mark the operation as running
    pub fn mark_running(&mut self) {
        self.status = OperationStatus::Running;
        self.updated_at = SystemTime::now();
    }
    
    /// Update the progress of the operation
    pub fn update_progress(&mut self, progress: f32) {
        self.progress = Some(progress.clamp(0.0, 1.0));
        self.updated_at = SystemTime::now();
    }
    
    /// Mark the operation as completed
    pub fn mark_completed(&mut self, result: serde_json::Value) {
        let now = SystemTime::now();
        self.status = OperationStatus::Completed;
        self.progress = Some(1.0);
        self.result = Some(result);
        self.updated_at = now;
        self.completed_at = Some(now);
    }
    
    /// Mark the operation as failed
    pub fn mark_failed(&mut self, error: String) {
        let now = SystemTime::now();
        self.status = OperationStatus::Failed;
        self.error = Some(error);
        self.updated_at = now;
        self.completed_at = Some(now);
    }
    
    /// Mark the operation as cancelled
    pub fn mark_cancelled(&mut self) {
        let now = SystemTime::now();
        self.status = OperationStatus::Cancelled;
        self.updated_at = now;
        self.completed_at = Some(now);
    }
    
    /// Check if the operation record has expired
    pub fn is_expired(&self) -> bool {
        match self.status {
            OperationStatus::Completed | 
            OperationStatus::Failed | 
            OperationStatus::Cancelled => {
                if let Some(completed_at) = self.completed_at {
                    match completed_at.elapsed() {
                        Ok(elapsed) => elapsed > self.ttl,
                        Err(_) => false,
                    }
                } else {
                    false
                }
            },
            _ => false,
        }
    }
    
    /// Convert to API response format
    pub fn to_api_response(&self) -> crate::api::handlers::operations::OperationStatus {
        crate::api::handlers::operations::OperationStatus {
            id: self.id.clone(),
            status: self.status.to_string(),
            progress: self.progress,
            result: self.result.clone(),
            error: self.error.clone(),
        }
    }
} 