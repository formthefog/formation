// VM List Tool
//
// This tool lists all VMs for the current user or for all users if requested.

use std::sync::Arc;
use async_trait::async_trait;
use serde::{Serialize, Deserialize};
use serde_json::{json, Value};
use reqwest::Client;

use crate::errors::ToolError;
use crate::tools::{Tool, ToolContext, ToolDefinition, ToolParameter, ToolResult};
use crate::tools::registry::ToolRegistry;

// Reuse the Instance structures from the create tool
use super::create::{Instance, InstanceStatus, InstanceResources};

// Constants for the state API
const STATE_PORT: u16 = 3004;

/// State datastore response wrapper
#[derive(Clone, Debug, Serialize, Deserialize)]
struct Response<T> {
    success: bool,
    data: Option<Vec<T>>,
    message: Option<String>,
}

/// VM List Tool Implementation
pub struct VMListTool {
    http_client: Client,
}

impl VMListTool {
    /// Create a new VM list tool
    pub fn new() -> Self {
        Self {
            http_client: Client::new(),
        }
    }
    
    /// Register this tool with the registry
    pub fn register(registry: &ToolRegistry) -> Result<(), ToolError> {
        registry.register_tool(Arc::new(Self::new()))
    }
    
    /// Get VMs from the state datastore
    async fn get_vms(&self, context: &ToolContext, all_users: bool) -> Result<Value, ToolError> {
        // Determine which endpoint to use based on whether we want all users' VMs or just the current user's
        let endpoint = if all_users {
            format!("http://127.0.0.1:{}/instances", STATE_PORT)
        } else {
            // Get instances for the current user only
            format!("http://127.0.0.1:{}/instances/owner/{}", STATE_PORT, context.user_id)
        };
        
        // Send request to the state datastore
        let response = self.http_client
            .get(&endpoint)
            .send()
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("State API request failed: {}", e)))?;
            
        // Check if request was successful
        if !response.status().is_success() {
            return Err(ToolError::ExecutionFailed(
                format!("State API returned error status: {}", response.status())
            ));
        }
        
        // Parse the response
        let api_response: Response<Instance> = response
            .json()
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to parse state API response: {}", e)))?;
            
        if !api_response.success {
            return Err(ToolError::ExecutionFailed(
                api_response.message.unwrap_or_else(|| "Unknown error from state API".to_string())
            ));
        }
        
        // Extract the instances
        let instances = api_response.data.unwrap_or_default();
        
        // Create a user-friendly list of VMs
        let vm_list: Vec<Value> = instances.into_iter().map(|instance| {
            json!({
                "id": instance.instance_id,
                "name": instance.build_id,
                "owner": instance.instance_owner,
                "status": format!("{:?}", instance.status),
                "created": instance.created_at,
                "resources": {
                    "vcpus": instance.resources.vcpus,
                    "memory_mb": instance.resources.memory_mb,
                    "disk_gb": instance.resources.disk_gb,
                }
            })
        }).collect();
        
        Ok(json!({
            "success": true,
            "count": vm_list.len(),
            "vms": vm_list
        }))
    }
}

#[async_trait]
impl Tool for VMListTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "vm.list".to_string(),
            description: "List all VMs for the current user or all users if requested".to_string(),
            version: "1.0".to_string(),
            parameters: vec![
                ToolParameter {
                    name: "all_users".to_string(),
                    description: "Whether to list VMs for all users (admin only)".to_string(),
                    required: false,
                    parameter_type: "boolean".to_string(),
                    default: Some(json!(false)),
                    enum_values: None,
                },
            ],
            return_type: "object".to_string(),
            tags: vec!["vm".to_string(), "list".to_string()],
            is_long_running: Some(false),
        }
    }
    
    async fn execute(&self, params: Value, context: ToolContext) -> ToolResult {
        // Validate parameters
        self.validate_params(&params)?;
        
        // Extract parameters
        let all_users = params
            .get("all_users")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        
        // Check permissions for all_users
        if all_users && !context.is_admin {
            return Err(ToolError::Forbidden(
                "Admin permission required to list VMs for all users".to_string()
            ));
        }
        
        // Get VMs from state
        self.get_vms(&context, all_users).await
    }
} 
