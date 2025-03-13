// VM Status Tool
//
// This tool retrieves the status of a specific VM.

use std::sync::Arc;
use async_trait::async_trait;
use serde::{Serialize, Deserialize};
use serde_json::{json, Value};
use reqwest::Client;

use crate::errors::ToolError;
use crate::tools::{Tool, ToolContext, ToolDefinition, ToolParameter, ToolResult};
use crate::tools::registry::ToolRegistry;

// Reuse the Instance structures from the create tool
use super::create::{Instance, InstanceStatus};

// Constants for the state API
const STATE_PORT: u16 = 3004;

/// State datastore response wrapper
#[derive(Clone, Debug, Serialize, Deserialize)]
struct Response<T> {
    success: bool,
    data: Option<T>,
    message: Option<String>,
}

/// VM Status Tool Implementation
pub struct VMStatusTool {
    http_client: Client,
}

impl VMStatusTool {
    /// Create a new VM status tool
    pub fn new() -> Self {
        Self {
            http_client: Client::new(),
        }
    }
    
    /// Register this tool with the registry
    pub fn register(registry: &ToolRegistry) -> Result<(), ToolError> {
        registry.register_tool(Arc::new(Self::new()))
    }
    
    /// Retrieve VM status from the state datastore
    async fn get_vm_status(&self, vm_id: &str, context: &ToolContext) -> Result<Value, ToolError> {
        // Try both ways to look up the VM - by instance_id and by build_id
        let by_instance_id = self.get_vm_by_instance_id(vm_id, &context.user_id).await;
        if by_instance_id.is_ok() {
            return by_instance_id;
        }
        
        // If we didn't find it by instance_id, try by build_id
        self.get_vm_by_build_id(vm_id, &context.user_id).await
    }
    
    /// Get VM by instance ID
    async fn get_vm_by_instance_id(&self, instance_id: &str, user_id: &str) -> Result<Value, ToolError> {
        // Get endpoint URL for instance by ID
        let endpoint = format!("http://127.0.0.1:{}/instances/{}", STATE_PORT, instance_id);
        self.query_state_api(endpoint, user_id, instance_id).await
    }
    
    /// Get VM by build ID
    async fn get_vm_by_build_id(&self, build_id: &str, user_id: &str) -> Result<Value, ToolError> {
        // Get endpoint URL for instance by build ID
        let endpoint = format!("http://127.0.0.1:{}/instances/build/{}", STATE_PORT, build_id);
        self.query_state_api(endpoint, user_id, build_id).await
    }
    
    /// Common function to query the state API
    async fn query_state_api(&self, endpoint: String, user_id: &str, vm_id: &str) -> Result<Value, ToolError> {
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
                api_response.message.unwrap_or_else(|| format!("VM '{}' not found", vm_id))
            ));
        }
        
        let instance = match api_response.data {
            Some(instance) => instance,
            None => return Err(ToolError::ExecutionFailed(format!("VM '{}' not found", vm_id))),
        };
        
        // Check ownership
        if instance.instance_owner != user_id {
            return Err(ToolError::Forbidden(
                format!("VM '{}' is not owned by the current user", vm_id)
            ));
        }
        
        // Format the result
        Ok(json!({
            "success": true,
            "id": instance.instance_id,
            "name": instance.build_id,
            "owner": instance.instance_owner,
            "status": format!("{:?}", instance.status),
            "created": instance.created_at,
            "updated": instance.updated_at,
            "resources": {
                "vcpus": instance.resources.vcpus,
                "memory_mb": instance.resources.memory_mb,
                "disk_gb": instance.resources.disk_gb,
            },
            "network": {
                "formnet_ip": instance.formnet_ip,
            }
        }))
    }
}

#[async_trait]
impl Tool for VMStatusTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "vm.status".to_string(),
            description: "Get status of a VM by ID or name".to_string(),
            version: "1.0".to_string(),
            parameters: vec![
                ToolParameter {
                    name: "id".to_string(),
                    description: "ID or name of the VM".to_string(),
                    required: true,
                    parameter_type: "string".to_string(),
                    default: None,
                    enum_values: None,
                },
            ],
            return_type: "object".to_string(),
            tags: vec!["vm".to_string(), "status".to_string()],
            is_long_running: Some(false),
        }
    }
    
    async fn execute(&self, params: Value, context: ToolContext) -> ToolResult {
        // Validate parameters
        self.validate_params(&params)?;
        
        // Extract parameters
        let params = params.as_object().ok_or_else(|| {
            ToolError::InvalidParameters("Parameters must be an object".to_string())
        })?;
        
        let vm_id = params.get("id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                ToolError::InvalidParameters("'id' parameter is required".to_string())
            })?;
            
        // Get VM status from state
        self.get_vm_status(vm_id, &context).await
    }
} 