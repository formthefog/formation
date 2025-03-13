// Operations handlers for the MCP server API
//
// This module contains handlers for operation-related API endpoints,
// such as checking the status of long-running operations.

use actix_web::{web, HttpResponse, Responder};
use serde::Serialize;
use std::sync::Arc;

use crate::api::handlers::ApiResponse;
use crate::models::operations::OperationsRepository;

/// Data structure for operation status response
#[derive(Serialize)]
pub struct OperationStatus {
    pub id: String,
    pub status: String,
    pub progress: Option<f32>,
    pub result: Option<serde_json::Value>,
    pub error: Option<String>,
}

/// Handler for checking the status of a long-running operation
pub async fn get_operation_status(
    repository: web::Data<Arc<OperationsRepository>>,
    path: web::Path<String>,
) -> impl Responder {
    let operation_id = path.into_inner();
    
    // Get the operation from the repository
    match repository.get_operation(&operation_id).await {
        Some(operation) => {
            // Convert the operation to an API response
            let status = operation.to_api_response();
            HttpResponse::Ok().json(ApiResponse::success(status))
        },
        None => {
            // Operation not found
            HttpResponse::NotFound().json(ApiResponse::<()>::error(
                format!("Operation with ID '{}' not found", operation_id)
            ))
        }
    }
}

/// Query parameters for listing operations
#[derive(serde::Deserialize, Default)]
pub struct ListOperationsParams {
    /// Filter by user ID
    pub user_id: Option<String>,
}

/// Data structure for operation list response
#[derive(Serialize)]
pub struct OperationListResponse {
    pub operations: Vec<OperationStatus>,
}

/// Handler for listing operations
pub async fn list_operations(
    repository: web::Data<Arc<OperationsRepository>>,
    query: web::Query<ListOperationsParams>,
) -> impl Responder {
    // Get operations by user ID, or all operations if admin
    let operations = if let Some(user_id) = &query.user_id {
        repository.get_operations_by_user(user_id).await
    } else {
        // For now, just return an empty list if no user ID is provided
        // In a real implementation, we would check if the user is an admin
        Vec::new()
    };
    
    // Convert operations to API responses
    let operation_statuses = operations
        .into_iter()
        .map(|op| op.to_api_response())
        .collect();
    
    HttpResponse::Ok().json(ApiResponse::success(OperationListResponse {
        operations: operation_statuses,
    }))
} 