// Operations repository
//
// This module provides a repository for managing operation state.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use std::time::{Duration, SystemTime};

use super::Operation;

/// Repository for managing operations
#[derive(Debug, Clone)]
pub struct OperationsRepository {
    operations: Arc<RwLock<HashMap<String, Operation>>>,
    cleanup_interval: Duration,
}

impl OperationsRepository {
    /// Create a new operations repository
    pub fn new() -> Self {
        let repo = Self {
            operations: Arc::new(RwLock::new(HashMap::new())),
            cleanup_interval: Duration::from_secs(300), // 5 minutes
        };
        
        // Start background cleanup task
        repo.start_cleanup_task();
        
        repo
    }
    
    /// Add a new operation to the repository
    pub async fn add_operation(&self, operation: Operation) -> String {
        let id = operation.id.clone();
        let mut operations = self.operations.write().await;
        operations.insert(id.clone(), operation);
        id
    }
    
    /// Get an operation by ID
    pub async fn get_operation(&self, id: &str) -> Option<Operation> {
        let operations = self.operations.read().await;
        operations.get(id).cloned()
    }
    
    /// Get operations by user ID
    pub async fn get_operations_by_user(&self, user_id: &str) -> Vec<Operation> {
        let operations = self.operations.read().await;
        operations
            .values()
            .filter(|op| op.user_id == user_id)
            .cloned()
            .collect()
    }
    
    /// Update an operation
    pub async fn update_operation(&self, operation: Operation) -> Result<(), String> {
        let mut operations = self.operations.write().await;
        if operations.contains_key(&operation.id) {
            operations.insert(operation.id.clone(), operation);
            Ok(())
        } else {
            Err(format!("Operation with ID '{}' not found", operation.id))
        }
    }
    
    /// Remove an operation from the repository
    pub async fn remove_operation(&self, id: &str) -> Option<Operation> {
        let mut operations = self.operations.write().await;
        operations.remove(id)
    }
    
    /// Clean up expired operations
    pub async fn cleanup(&self) {
        let mut operations = self.operations.write().await;
        operations.retain(|_, op| !op.is_expired());
    }
    
    /// Clean up expired operations (alias for cleanup)
    pub async fn cleanup_expired_operations(&self) {
        self.cleanup().await;
    }
    
    /// Start the background cleanup task
    fn start_cleanup_task(&self) {
        let repo = self.clone();
        tokio::spawn(async move {
            let interval = repo.cleanup_interval;
            loop {
                tokio::time::sleep(interval).await;
                repo.cleanup().await;
            }
        });
    }
}

/// Create a new shared operations repository
pub fn create_repository() -> Arc<OperationsRepository> {
    Arc::new(OperationsRepository::new())
} 