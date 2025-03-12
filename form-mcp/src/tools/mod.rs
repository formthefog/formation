// Tools module for the MCP server
//
// This module implements the tool registration and execution system
// for the MCP server.

pub mod vm;
pub mod network;
pub mod metrics;
mod registry;

pub use registry::{ToolRegistry, Tool, ToolDefinition, ToolParameter, ToolResult};

use std::sync::Arc;
use serde::{Serialize, Deserialize};
use crate::errors::ToolError;

/// ToolContext holds contextual information for tool execution
#[derive(Clone)]
pub struct ToolContext {
    /// User ID of the requester
    pub user_id: String,
    /// Request ID for tracking
    pub request_id: String,
    /// Additional contextual data
    pub context: std::collections::HashMap<String, String>,
    /// Whether the user has admin privileges
    pub is_admin: bool,
}

/// ToolRequest represents a request to execute a tool
#[derive(Deserialize, Clone)]
pub struct ToolRequest {
    /// Name of the tool to execute
    pub name: String,
    /// Parameters for the tool execution
    pub parameters: serde_json::Value,
    /// Optional contextual data
    pub context: Option<std::collections::HashMap<String, String>>,
}

/// ToolResponse represents the response from a tool execution
#[derive(Serialize)]
pub struct ToolResponse {
    /// Status of the tool execution
    pub status: String,
    /// Result of the tool execution
    pub result: Option<serde_json::Value>,
    /// Error message if the tool execution failed
    pub error: Option<String>,
}

/// Initialize the tool registry
pub fn init_registry() -> Arc<registry::ToolRegistry> {
    let registry = registry::ToolRegistry::new();
    
    // Register VM management tools
    vm::register_tools(&registry);
    
    // Register network management tools
    network::register_tools(&registry);
    
    // Register metrics tools
    metrics::register_tools(&registry);
    
    Arc::new(registry)
}

/// Execute a tool with the given request
pub async fn execute_tool(
    registry: Arc<registry::ToolRegistry>,
    request: ToolRequest,
    context: ToolContext,
) -> Result<ToolResponse, ToolError> {
    let tool = registry.get_tool(&request.name)
        .ok_or_else(|| ToolError::NotFound(request.name.clone()))?;
    
    match tool.execute(request.parameters, context).await {
        Ok(result) => Ok(ToolResponse {
            status: "success".to_string(),
            result: Some(result),
            error: None,
        }),
        Err(err) => Ok(ToolResponse {
            status: "error".to_string(),
            result: None,
            error: Some(err.to_string()),
        }),
    }
}

/// List all available tools
pub fn list_tools(registry: Arc<registry::ToolRegistry>) -> Vec<ToolDefinition> {
    registry.list_tools()
} 