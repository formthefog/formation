// Pack Ship Tool
//
// This tool allows deploying built workloads to Formation instances.

use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use async_trait::async_trait;
use serde::{Serialize, Deserialize};
use serde_json::{json, Value};
use reqwest::Client;
use uuid::Uuid;

use crate::errors::ToolError;
use crate::tools::{Tool, ToolContext, ToolDefinition, ToolParameter, ToolResult};
use crate::tools::registry::ToolRegistry;

// Constants for API endpoints
const QUEUE_PORT: u16 = 53333;

/// Network configuration for the VM (copied from VM tools)
#[derive(Debug, Serialize, Deserialize)]
pub struct VMNetworkConfig {
    /// Whether to join the formnet network
    pub join_formnet: Option<bool>,
    /// External networks to connect to
    pub external_networks: Option<Vec<String>>,
}

/// Pack deploy request format
#[derive(Debug, Serialize, Deserialize)]
pub struct PackShipRequest {
    /// Build ID of the package to deploy
    pub build_id: String,
    /// Instance name for the deployment
    pub instance_name: String,
    /// VM configuration details
    pub vm_config: Option<VMConfig>,
    /// User ID owning the deployment
    pub user_id: String,
    /// Request signature
    pub signature: Option<String>,
    /// Nonce for request validation
    pub nonce: String,
    /// Timestamp of the request
    pub timestamp: u64,
}

/// VM Configuration for the deployment
#[derive(Debug, Serialize, Deserialize)]
pub struct VMConfig {
    /// Number of vCPUs
    pub vcpus: Option<u8>,
    /// Memory size in MB
    pub memory_mb: Option<u64>,
    /// Network configuration
    pub network: Option<VMNetworkConfig>,
    /// Additional metadata
    pub metadata: Option<Value>,
}

/// Queue Message Formats
#[derive(Debug, Serialize, Deserialize)]
pub enum QueueRequest {
    Write {
        content: Vec<u8>,
        topic: String,
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub enum QueueResponse {
    OpSuccess,
    Failure { reason: Option<String> },
    // Other variants not needed for our use case
}

/// Pack deploy status response
#[derive(Debug, Serialize, Deserialize)]
pub struct ShipStatusResponse {
    /// Deployment ID
    pub deploy_id: String,
    /// Instance ID where the workload is deployed
    pub instance_id: Option<String>,
    /// Current status of the deployment
    pub status: DeploymentStatus,
    /// Status message or error details
    pub message: Option<String>,
}

/// Deployment status enumeration
#[derive(Debug, Serialize, Deserialize)]
pub enum DeploymentStatus {
    /// Request queued
    Queued,
    /// Deployment in process
    InProgress,
    /// Deployment completed successfully
    Completed,
    /// Deployment failed
    Failed,
}

/// Pack ship tool implementation
pub struct PackShipTool {
    http_client: Client,
}

impl PackShipTool {
    /// Create a new pack ship tool
    pub fn new() -> Self {
        Self {
            http_client: Client::new(),
        }
    }
    
    /// Register this tool with the registry
    pub fn register(registry: &ToolRegistry) -> Result<(), ToolError> {
        let tool = Arc::new(Self::new());
        registry.register_tool(tool)
    }
    
    /// Send a deployment request
    async fn submit_ship_request(
        &self, 
        build_id: &str, 
        instance_name: &str,
        vm_config: Option<VMConfig>,
        context: &ToolContext
    ) -> Result<Value, ToolError> {
        // Generate deployment request
        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)
            .map_err(|_| ToolError::ExecutionFailed("Failed to get system time".to_string()))?
            .as_secs();
        
        let nonce = Uuid::new_v4().to_string();
        
        let ship_request = PackShipRequest {
            build_id: build_id.to_string(),
            instance_name: instance_name.to_string(),
            vm_config,
            user_id: context.user_id.clone(),
            signature: None, // Placeholder for future signature implementation
            nonce,
            timestamp,
        };
        
        // Serialize the request
        let request_json = serde_json::to_vec(&ship_request)
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to serialize ship request: {}", e)))?;
        
        // Create queue request
        let queue_request = QueueRequest::Write {
            content: request_json,
            topic: "pack.ship".to_string(),
        };
        
        // Serialize queue request
        let queue_json = serde_json::to_vec(&queue_request)
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to serialize queue request: {}", e)))?;
        
        // Send to queue
        let endpoint = format!("http://127.0.0.1:{}/queue", QUEUE_PORT);
        let response = self.http_client.post(&endpoint)
            .body(queue_json)
            .send()
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to send ship request: {}", e)))?;
        
        // Handle response
        let queue_response: QueueResponse = response.json()
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to parse response: {}", e)))?;
        
        match queue_response {
            QueueResponse::OpSuccess => {
                // Generate deploy ID for tracking
                let deploy_id = Uuid::new_v4().to_string();
                
                // Create status response
                let status_response = ShipStatusResponse {
                    deploy_id: deploy_id.clone(),
                    instance_id: None, // Will be populated once VM is created
                    status: DeploymentStatus::Queued,
                    message: Some("Deployment request queued successfully".to_string()),
                };
                
                Ok(json!({
                    "status": "success",
                    "deploy_id": deploy_id,
                    "message": "Deployment request queued successfully",
                    "details": status_response
                }))
            },
            QueueResponse::Failure { reason } => {
                Err(ToolError::ExecutionFailed(format!("Deployment request failed: {}", 
                    reason.unwrap_or_else(|| "Unknown reason".to_string()))))
            }
        }
    }
}

#[async_trait]
impl Tool for PackShipTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "form_pack_ship".to_string(),
            description: "Deploys a built workload package to a Formation instance".to_string(),
            version: "0.1.0".to_string(),
            parameters: vec![
                ToolParameter {
                    name: "build_id".to_string(),
                    description: "ID of the built package to deploy".to_string(),
                    required: true,
                    parameter_type: "string".to_string(),
                    default: None,
                    enum_values: None,
                },
                ToolParameter {
                    name: "instance_name".to_string(),
                    description: "Name for the instance running the workload".to_string(),
                    required: true,
                    parameter_type: "string".to_string(),
                    default: None,
                    enum_values: None,
                },
                ToolParameter {
                    name: "vm_config".to_string(),
                    description: "Virtual machine configuration for the deployment".to_string(),
                    required: false,
                    parameter_type: "object".to_string(),
                    default: None,
                    enum_values: None,
                },
            ],
            return_type: "Deployment ID and status for tracking the deployment process".to_string(),
            tags: vec![
                "pack".to_string(),
                "ship".to_string(),
                "deploy".to_string(),
                "workload".to_string(),
            ],
            is_long_running: Some(true),
        }
    }
    
    async fn execute(&self, params: Value, context: ToolContext) -> ToolResult {
        // Validate parameters
        self.validate_params(&params)?;
        
        // Extract parameters
        let build_id = params["build_id"].as_str()
            .ok_or_else(|| ToolError::InvalidParameters("Missing required parameter: build_id".to_string()))?;
            
        let instance_name = params["instance_name"].as_str()
            .ok_or_else(|| ToolError::InvalidParameters("Missing required parameter: instance_name".to_string()))?;
        
        // Extract optional VM configuration
        let vm_config = if params["vm_config"].is_object() {
            let vm_config_value = &params["vm_config"];
            
            let vcpus = vm_config_value["vcpus"].as_u64().map(|v| v as u8);
            let memory_mb = vm_config_value["memory_mb"].as_u64();
            
            // Extract network config if available
            let network = if vm_config_value["network"].is_object() {
                Some(serde_json::from_value::<VMNetworkConfig>(vm_config_value["network"].clone())
                    .map_err(|e| ToolError::InvalidParameters(format!("Invalid network configuration: {}", e)))?)
            } else {
                None
            };
            
            // Extract metadata if available
            let metadata = if vm_config_value["metadata"].is_object() {
                Some(vm_config_value["metadata"].clone())
            } else {
                None
            };
            
            Some(VMConfig {
                vcpus,
                memory_mb,
                network,
                metadata,
            })
        } else {
            None
        };
        
        // Submit ship request
        self.submit_ship_request(build_id, instance_name, vm_config, &context).await
    }
} 