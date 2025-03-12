// Tool handlers for the MCP server API
//
// This module contains handlers for tool-related API endpoints,
// such as listing available tools and executing a tool.

use actix_web::{web, HttpResponse, Responder};
use serde::Deserialize;
use std::sync::Arc;
use uuid::Uuid;
use tokio::task;
use serde_json::json;

use crate::tools::{ToolRegistry, ToolRequest, ToolContext, ToolResponse};
use crate::api::handlers::ApiResponse;
use crate::errors::ToolError;
use crate::models::operations::{OperationsRepository, Operation};

/// Query parameters for tool listing
#[derive(Deserialize, Default)]
pub struct ToolListParams {
    /// Optional category filter
    pub category: Option<String>,
}

/// Data structure for tool list response
#[derive(serde::Serialize)]
pub struct ToolListResponse {
    pub tools: Vec<serde_json::Value>,
    pub categories: Vec<String>,
}

/// Handler for listing available tools
pub async fn list_tools(
    registry: web::Data<Arc<ToolRegistry>>,
    query: web::Query<ToolListParams>,
) -> impl Responder {
    let tools = registry.list_tools();
    let categories = registry.get_categories();
    
    // Filter by category if specified
    let filtered_tools: Vec<_> = if let Some(category) = &query.category {
        tools.into_iter()
            .filter(|tool| tool.tags.contains(category))
            .map(|tool| serde_json::to_value(&tool).unwrap_or_default())
            .collect()
    } else {
        tools.into_iter()
            .map(|tool| serde_json::to_value(&tool).unwrap_or_default())
            .collect()
    };
    
    HttpResponse::Ok().json(ApiResponse::success(ToolListResponse {
        tools: filtered_tools,
        categories,
    }))
}

/// Request for executing a tool
#[derive(Deserialize)]
pub struct ExecuteToolRequest {
    /// Parameters for the tool
    pub parameters: serde_json::Value,
    /// Optional context data
    pub context: Option<std::collections::HashMap<String, String>>,
}

/// Response for an asynchronous tool execution
#[derive(serde::Serialize)]
pub struct AsyncToolResponse {
    pub operation_id: String,
    pub status: String,
    pub message: String,
}

/// Handler for executing a specific tool
pub async fn execute_tool(
    registry: web::Data<Arc<ToolRegistry>>,
    operations_repo: web::Data<Arc<OperationsRepository>>,
    path: web::Path<String>,
    req: web::Json<ExecuteToolRequest>,
    // Later, we would add user authentication info here
) -> impl Responder {
    let tool_name = path.into_inner();
    
    // Get the tool to check if it's long running
    let tool = match registry.get_tool(&tool_name) {
        Some(tool) => tool,
        None => return HttpResponse::NotFound().json(ApiResponse::<()>::error(
            format!("Tool '{}' not found", tool_name)
        )),
    };
    
    // Create a tool request
    let tool_request = ToolRequest {
        name: tool_name.clone(),
        parameters: req.parameters.clone(),
        context: req.context.clone(),
    };
    
    // Create a tool context (in a real implementation, this would use authentication data)
    let context = ToolContext {
        user_id: "test_user".to_string(), // Placeholder, would come from auth
        request_id: Uuid::new_v4().to_string(),
        context: req.context.clone().unwrap_or_default(),
        is_admin: true, // Placeholder, would come from auth
    };
    
    // Check if the tool is marked as long running
    let is_long_running = tool.definition().is_long_running.unwrap_or(false);
    
    if is_long_running {
        // Create an operation to track the tool execution
        let operation = Operation::new(context.user_id.clone(), tool_name.clone());
        let operation_id = operation.id.clone();
        
        // Store the operation
        operations_repo.add_operation(operation).await;
        
        // Clone dependencies for async task
        let registry_clone = registry.get_ref().clone();
        let tool_request_clone = tool_request.clone();
        let context_clone = context.clone();
        let operations_repo_clone = operations_repo.get_ref().clone();
        let operation_id_clone = operation_id.clone();
        
        // Spawn an async task to execute the tool
        task::spawn(async move {
            // Get the operation and mark it as running
            if let Some(mut operation) = operations_repo_clone.get_operation(&operation_id_clone).await {
                operation.mark_running();
                if let Err(e) = operations_repo_clone.update_operation(operation).await {
                    log::error!("Failed to update operation status to running: {}", e);
                    return;
                }
                
                // Execute the tool
                let result = crate::tools::execute_tool(registry_clone, tool_request_clone, context_clone).await;
                
                // Update operation with result
                if let Some(mut operation) = operations_repo_clone.get_operation(&operation_id_clone).await {
                    match result {
                        Ok(response) => {
                            operation.mark_completed(json!(response));
                            if let Err(e) = operations_repo_clone.update_operation(operation).await {
                                log::error!("Failed to update operation status to completed: {}", e);
                            }
                        },
                        Err(error) => {
                            let error_msg = format!("Tool execution failed: {}", error);
                            operation.mark_failed(error_msg);
                            if let Err(e) = operations_repo_clone.update_operation(operation).await {
                                log::error!("Failed to update operation status to failed: {}", e);
                            }
                        }
                    }
                }
            }
        });
        
        // Return immediate response with operation ID
        HttpResponse::Accepted().json(ApiResponse::success(AsyncToolResponse {
            operation_id,
            status: "queued".to_string(),
            message: format!("Tool '{}' execution has been queued", tool_name),
        }))
    } else {
        // Execute the tool synchronously for non-long-running tools
        match crate::tools::execute_tool(registry.get_ref().clone(), tool_request, context).await {
            Ok(response) => HttpResponse::Ok().json(ApiResponse::success(response)),
            Err(error) => {
                match error {
                    ToolError::NotFound(_) => 
                        HttpResponse::NotFound().json(ApiResponse::<()>::error(format!("Tool '{}' not found", tool_name))),
                    ToolError::InvalidParameters(msg) => 
                        HttpResponse::BadRequest().json(ApiResponse::<()>::error(msg)),
                    ToolError::ExecutionFailed(msg) => 
                        HttpResponse::InternalServerError().json(ApiResponse::<()>::error(msg)),
                    ToolError::Timeout => 
                        HttpResponse::GatewayTimeout().json(ApiResponse::<()>::error("Tool execution timed out")),
                    ToolError::Forbidden(msg) =>
                        HttpResponse::Forbidden().json(ApiResponse::<()>::error(msg)),
                    ToolError::RegistrationFailed(_) => 
                        HttpResponse::InternalServerError().json(ApiResponse::<()>::error("Internal server error")),
                }
            }
        }
    }
} 