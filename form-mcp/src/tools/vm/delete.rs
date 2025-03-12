// VM Delete Tool
//
// This tool allows deleting existing VMs by ID or name.

use std::sync::Arc;
use async_trait::async_trait;
use serde::{Serialize, Deserialize};
use serde_json::{json, Value};
use reqwest::Client;
use tiny_keccak::{Hasher, Sha3};

use crate::errors::ToolError;
use crate::tools::{Tool, ToolContext, ToolDefinition, ToolParameter, ToolResult};
use crate::tools::registry::ToolRegistry;

// Constants for API endpoints
const QUEUE_PORT: u16 = 53333;
const STATE_PORT: u16 = 3004;
const STATE_TOPIC: &str = "state";
const INSTANCE_SUBTOPIC: u8 = 4;

/// Queue Message Formats
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum QueueRequest {
    Write {
        content: Vec<u8>,
        topic: String,
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum QueueResponse {
    OpSuccess,
    Failure { reason: Option<String> },
    // Other variants not needed for our use case
}

/// Instance status enum
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum InstanceStatus {
    Building,
    Created,
    Started,
    Stopped,
    Deleting,
    Deleted,
}

/// VM Instance for state storage
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Instance {
    pub instance_id: String,
    pub node_id: String,
    pub build_id: String,
    pub instance_owner: String,
    pub status: InstanceStatus,
    // Other fields not needed for delete operation
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum InstanceRequest {
    Delete { instance_id: String, force: bool },
    // Other variants not needed for our use case
}

/// Response when using state API
#[derive(Clone, Debug, Serialize, Deserialize)]
struct Response<T> {
    success: bool,
    data: Option<T>,
    message: Option<String>,
}

/// VM Deletion Tool Implementation
pub struct VMDeleteTool {
    http_client: Client,
}

impl VMDeleteTool {
    /// Create a new VM deletion tool
    pub fn new() -> Self {
        Self {
            http_client: Client::new(),
        }
    }
    
    /// Register this tool with the registry
    pub fn register(registry: &ToolRegistry) -> Result<(), ToolError> {
        registry.register_tool(Arc::new(Self::new()))
    }
    
    /// Delete VM through state API
    async fn delete_vm_api(
        &self, 
        instance_id: &str, 
        force: bool,
        context: &ToolContext
    ) -> Result<Value, ToolError> {
        // Ensure user has admin access for force delete
        if force && !context.is_admin {
            return Err(ToolError::Forbidden("Force deletion requires admin privileges".to_string()));
        }
        
        // Create the delete request
        let request = InstanceRequest::Delete {
            instance_id: instance_id.to_string(),
            force,
        };

        // Send direct API request
        let response = self.http_client
            .delete(format!("http://127.0.0.1:{}/instances/delete", STATE_PORT))
            .json(&request)
            .send()
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Instance API request failed: {}", e)))?;
            
        // Check if request was successful
        if !response.status().is_success() {
            return Err(ToolError::ExecutionFailed(
                format!("State API returned error status: {}", response.status())
            ));
        }
        
        // Parse response
        let api_response: Response<()> = response
            .json()
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to parse API response: {}", e)))?;
            
        if !api_response.success {
            return Err(ToolError::ExecutionFailed(
                api_response.message.unwrap_or_else(|| "Unknown error from state API".to_string())
            ));
        }
        
        // Return success response
        Ok(json!({
            "success": true,
            "message": format!("VM '{}' deletion has been initiated", instance_id)
        }))
    }
    
    /// Delete VM through message queue (fallback)
    async fn delete_vm_queue(
        &self, 
        instance_id: &str, 
        force: bool,
        context: &ToolContext
    ) -> Result<Value, ToolError> {
        // Ensure user has admin access for force delete
        if force && !context.is_admin {
            return Err(ToolError::Forbidden("Force deletion requires admin privileges".to_string()));
        }
        
        // Create the delete request
        let request = InstanceRequest::Delete {
            instance_id: instance_id.to_string(),
            force,
        };
        
        // Write to the queue
        self.write_to_queue(request).await
            .map_err(|e| ToolError::ExecutionFailed(format!("Queue error: {}", e)))?;
            
        // Return success response
        Ok(json!({
            "success": true,
            "message": format!("VM '{}' deletion has been queued", instance_id)
        }))
    }
    
    /// Helper function to write to the message queue
    async fn write_to_queue(
        &self,
        message: impl Serialize + Clone
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Create the topic hash
        let mut hasher = Sha3::v256();
        let mut topic_hash = [0u8; 32];
        hasher.update(STATE_TOPIC.as_bytes());
        hasher.finalize(&mut topic_hash);
        
        // Create the message content with subtopic
        let mut message_code = vec![INSTANCE_SUBTOPIC];
        message_code.extend(serde_json::to_vec(&message)?);
        
        // Create the queue request
        let request = QueueRequest::Write { 
            content: message_code, 
            topic: hex::encode(topic_hash) 
        };

        // Send the request to the queue
        let response = self.http_client
            .post(format!("http://127.0.0.1:{}/queue/write_local", QUEUE_PORT))
            .json(&request)
            .send().await?
            .json::<QueueResponse>().await?;
            
        match response {
            QueueResponse::OpSuccess => Ok(()),
            QueueResponse::Failure { reason } => {
                Err(Box::new(std::io::Error::new(
                    std::io::ErrorKind::Other, 
                    format!("Queue error: {:?}", reason)
                )))
            },
            // No default case needed as we've covered all variants
        }
    }
    
    /// Get VM instance details by name or ID
    async fn get_instance_id_by_name(
        &self, 
        name: &str, 
        context: &ToolContext
    ) -> Result<String, ToolError> {
        // First check if the name is actually an instance ID
        if name.contains('-') {
            return Ok(name.to_string());
        }
        
        // Query state API to get instance by name
        let response = self.http_client
            .get(format!("http://127.0.0.1:{}/instances/list?user_id={}", STATE_PORT, context.user_id))
            .send()
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("State API request failed: {}", e)))?;
        
        // Check if request was successful
        if !response.status().is_success() {
            return Err(ToolError::ExecutionFailed(
                format!("State API returned error status: {}", response.status())
            ));
        }
        
        // Parse response
        let api_response: Response<Vec<Instance>> = response
            .json()
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to parse API response: {}", e)))?;
            
        // Find instance with matching name in build_id
        let instance_id = api_response.data
            .ok_or_else(|| ToolError::ExecutionFailed("No instances found".to_string()))?
            .iter()
            .filter(|vm| vm.instance_owner == context.user_id)
            .find(|vm| vm.build_id.starts_with(&format!("{}-", name)))
            .map(|vm| vm.instance_id.clone())
            .ok_or_else(|| ToolError::ExecutionFailed(format!("VM with name '{}' not found", name)))?;
            
        Ok(instance_id)
    }
}

#[async_trait]
impl Tool for VMDeleteTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "vm.delete".to_string(),
            description: "Delete an existing VM by ID or name".to_string(),
            version: "1.0".to_string(),
            parameters: vec![
                ToolParameter {
                    name: "id".to_string(),
                    description: "ID or name of the VM to delete".to_string(),
                    required: true,
                    parameter_type: "string".to_string(),
                    default: None,
                    enum_values: None,
                },
                ToolParameter {
                    name: "force".to_string(),
                    description: "Force deletion even if VM is running (admin only)".to_string(),
                    required: false,
                    parameter_type: "boolean".to_string(),
                    default: Some(json!(false)),
                    enum_values: None,
                },
            ],
            return_type: "object".to_string(),
            tags: vec!["vm".to_string(), "delete".to_string()],
            is_long_running: Some(true),
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
            
        let force = params.get("force")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
            
        // Convert name to instance_id if needed
        let instance_id = self.get_instance_id_by_name(vm_id, &context).await?;
        
        // Try direct API endpoint first
        match self.delete_vm_api(&instance_id, force, &context).await {
            Ok(result) => Ok(result),
            Err(e) => {
                log::warn!("API endpoint failed, falling back to queue: {}", e);
                // Fall back to queue if API fails
                self.delete_vm_queue(&instance_id, force, &context).await
            }
        }
    }
} 