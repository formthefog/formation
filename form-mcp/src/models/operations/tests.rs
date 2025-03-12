#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tokio::time::sleep;
    use std::time::Duration;
    use crate::models::operations::{Operation, OperationStatus, create_repository};

    #[tokio::test]
    async fn test_operation_repository_basic() {
        let repo = create_repository();
        
        // Create a test operation
        let user_id = "test-user".to_string();
        let tool_name = "test-tool".to_string();
        let mut op = Operation::new(user_id.clone(), tool_name.clone());
        let op_id = op.id.clone();
        
        // Add operation to repository
        repo.add_operation(op.clone()).await;
        
        // Retrieve operation
        let retrieved = repo.get_operation(&op_id).await;
        assert!(retrieved.is_some());
        
        // Check operation fields
        let retrieved = retrieved.unwrap();
        assert_eq!(retrieved.user_id, user_id);
        assert_eq!(retrieved.tool_name, tool_name);
        assert_eq!(retrieved.status, OperationStatus::Queued);
        
        // Update operation status
        {
            let mut op = retrieved.clone();
            op.mark_running();
            repo.update_operation(op).await;
        }
        
        // Retrieve updated operation
        let retrieved = repo.get_operation(&op_id).await.unwrap();
        assert_eq!(retrieved.status, OperationStatus::Running);
        
        // Complete operation
        {
            let mut op = retrieved.clone();
            op.mark_completed(json!({"result": "success"}));
            repo.update_operation(op).await;
        }
        
        // Retrieve completed operation
        let retrieved = repo.get_operation(&op_id).await.unwrap();
        assert_eq!(retrieved.status, OperationStatus::Completed);
        assert!(retrieved.result.is_some());
        
        // List operations by user
        let ops = repo.get_operations_by_user(&user_id).await;
        assert_eq!(ops.len(), 1);
        assert_eq!(ops[0].id, op_id);
        
        // Remove operation
        repo.remove_operation(&op_id).await;
        let retrieved = repo.get_operation(&op_id).await;
        assert!(retrieved.is_none());
    }
    
    #[tokio::test]
    async fn test_operation_expiration() {
        let repo = create_repository();
        
        // Create a test operation with short TTL
        let mut op = Operation::new("test-user".to_string(), "test-tool".to_string());
        op.ttl = Duration::from_millis(10);  // Very short TTL for testing
        let op_id = op.id.clone();
        
        // Add operation to repository
        repo.add_operation(op.clone()).await;
        
        // Complete operation to start TTL timer
        {
            let mut op = repo.get_operation(&op_id).await.unwrap();
            op.mark_completed(json!({"result": "success"}));
            repo.update_operation(op).await;
        }
        
        // Wait for TTL to expire
        sleep(Duration::from_millis(20)).await;
        
        // Clean up expired operations
        repo.cleanup_expired_operations().await;
        
        // Operation should now be removed
        let retrieved = repo.get_operation(&op_id).await;
        assert!(retrieved.is_none(), "Operation should have been cleaned up after expiration");
    }
} 