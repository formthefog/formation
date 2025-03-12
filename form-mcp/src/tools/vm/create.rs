// VM Create Tool
//
// This tool allows creating new VMs with specified configurations.

use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use async_trait::async_trait;
use serde::{Serialize, Deserialize};
use serde_json::{json, Value};
use reqwest::Client;
use tiny_keccak::{Hasher, Sha3};
use rand::Rng;

use crate::errors::ToolError;
use crate::tools::{Tool, ToolContext, ToolDefinition, ToolParameter, ToolResult};
use crate::tools::registry::ToolRegistry;

// Constants for API endpoints
const QUEUE_PORT: u16 = 53333;
const STATE_PORT: u16 = 3004;
const STATE_TOPIC: &str = "state";
const INSTANCE_SUBTOPIC: u8 = 4;

/// VM configuration for creation
#[derive(Debug, Serialize, Deserialize)]
pub struct VMCreateConfig {
    /// Name of the VM
    pub name: String,
    /// Number of vCPUs
    pub vcpus: Option<u8>,
    /// Memory size in MB
    pub memory_mb: Option<u64>,
    /// Root disk size in GB
    pub disk_gb: Option<u64>,
    /// Base image to use
    pub image: Option<String>,
    /// Network configuration
    pub network: Option<VMNetworkConfig>,
    /// Additional metadata
    pub metadata: Option<serde_json::Map<String, Value>>,
}

/// Network configuration for the VM
#[derive(Debug, Serialize, Deserialize)]
pub struct VMNetworkConfig {
    /// Whether to join the formnet network
    pub join_formnet: Option<bool>,
    /// External networks to connect to
    pub external_networks: Option<Vec<String>>,
}

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

/// VM Instance for state storage
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Instance {
    pub instance_id: String,
    pub node_id: String,
    pub build_id: String,
    pub instance_owner: String,
    pub formnet_ip: Option<std::net::IpAddr>,
    pub created_at: i64,
    pub updated_at: i64,
    pub status: InstanceStatus,
    pub resources: InstanceResources,
    // Additional fields would be here in a complete implementation
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum InstanceStatus {
    Building,
    Created,
    Started,
    Stopped,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct InstanceResources {
    pub vcpus: u8,
    pub memory_mb: u64,
    pub disk_gb: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum InstanceRequest {
    Create(Instance),
    // Other variants not needed for our use case
}

/// Response when creating VM via API
#[derive(Clone, Debug, Serialize, Deserialize)]
struct Response<T> {
    success: bool,
    data: Option<T>,
    message: Option<String>,
}

/// VM Creation Tool Implementation
pub struct VMCreateTool {
    http_client: Client,
}

impl VMCreateTool {
    /// Create a new VM creation tool
    pub fn new() -> Self {
        Self {
            http_client: Client::new(),
        }
    }
    
    /// Register this tool with the registry
    pub fn register(registry: &ToolRegistry) -> Result<(), ToolError> {
        registry.register_tool(Arc::new(Self::new()))
    }
    
    /// Submit a VM creation request
    async fn submit_create_request(
        &self, 
        config: &VMCreateConfig,
        context: &ToolContext,
        build_id: String
    ) -> Result<Value, ToolError> {
        // Create user ID from context
        let user_id = &context.user_id;
        
        // Create current timestamp
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| ToolError::ExecutionFailed(format!("Time error: {}", e)))?
            .as_secs() as i64;
            
        // Create a basic VM Instance
        let instance = Instance {
            instance_id: format!("{}-{}", user_id, build_id),
            node_id: "mcp-server".to_string(), // This would be determined by the node allocation logic
            build_id: build_id.clone(),
            instance_owner: user_id.clone(),
            formnet_ip: None, // Will be assigned during VM creation
            created_at: timestamp,
            updated_at: timestamp,
            status: InstanceStatus::Building,
            resources: InstanceResources {
                vcpus: config.vcpus.unwrap_or(1),
                memory_mb: config.memory_mb.unwrap_or(1024),
                disk_gb: config.disk_gb.unwrap_or(10),
            },
        };
        
        // Try direct API endpoint first (preferred)
        match self.create_vm_api(instance.clone()).await {
            Ok(result) => return Ok(result),
            Err(e) => {
                log::warn!("API endpoint failed, falling back to queue: {}", e);
                // Fall back to queue if API fails
                self.create_vm_queue(instance).await
            }
        }
    }
    
    /// Create VM using state datastore HTTP API
    async fn create_vm_api(&self, instance: Instance) -> Result<Value, ToolError> {
        // Create the instance request
        let request = InstanceRequest::Create(instance.clone());
        
        // Send direct API request
        let response = self.http_client
            .post(format!("http://127.0.0.1:{}/instances/create", STATE_PORT))
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
        let api_response: Response<Instance> = response
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
            "vm_id": instance.build_id,
            "status": "creating",
            "message": format!("VM '{}' creation has been initiated", instance.build_id)
        }))
    }
    
    /// Create VM using message queue (fallback)
    async fn create_vm_queue(&self, instance: Instance) -> Result<Value, ToolError> {
        // Create a state update request
        let request = InstanceRequest::Create(instance.clone());
        
        // Write to the queue using the proper format
        self.write_to_queue(request).await
            .map_err(|e| ToolError::ExecutionFailed(format!("Queue error: {}", e)))?;
            
        // Return success response
        Ok(json!({
            "success": true,
            "vm_id": instance.build_id,
            "status": "creating",
            "message": format!("VM '{}' creation has been queued", instance.build_id)
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
}

#[async_trait]
impl Tool for VMCreateTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "vm.create".to_string(),
            description: "Create a new virtual machine".to_string(),
            version: "1.0".to_string(),
            parameters: vec![
                ToolParameter {
                    name: "name".to_string(),
                    description: "Name of the VM".to_string(),
                    required: true,
                    parameter_type: "string".to_string(),
                    default: None,
                    enum_values: None,
                },
                ToolParameter {
                    name: "vcpus".to_string(),
                    description: "Number of vCPUs".to_string(),
                    required: false,
                    parameter_type: "number".to_string(),
                    default: Some(json!(1)),
                    enum_values: None,
                },
                ToolParameter {
                    name: "memory_mb".to_string(),
                    description: "Memory size in MB".to_string(),
                    required: false,
                    parameter_type: "number".to_string(),
                    default: Some(json!(1024)),
                    enum_values: None,
                },
                ToolParameter {
                    name: "disk_gb".to_string(),
                    description: "Root disk size in GB".to_string(),
                    required: false,
                    parameter_type: "number".to_string(),
                    default: Some(json!(10)),
                    enum_values: None,
                },
                ToolParameter {
                    name: "image".to_string(),
                    description: "Base image to use".to_string(),
                    required: false,
                    parameter_type: "string".to_string(),
                    default: Some(json!("ubuntu-22.04")),
                    enum_values: None,
                },
                ToolParameter {
                    name: "join_formnet".to_string(),
                    description: "Whether to join the Formnet network".to_string(),
                    required: false,
                    parameter_type: "boolean".to_string(),
                    default: Some(json!(true)),
                    enum_values: None,
                },
                ToolParameter {
                    name: "metadata".to_string(),
                    description: "Additional metadata for the VM".to_string(),
                    required: false,
                    parameter_type: "object".to_string(),
                    default: None,
                    enum_values: None,
                },
            ],
            return_type: "object".to_string(),
            tags: vec!["vm".to_string(), "compute".to_string()],
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
        
        let name = params.get("name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                ToolError::InvalidParameters("'name' parameter is required".to_string())
            })?
            .to_string();
        
        let vcpus = params.get("vcpus")
            .and_then(|v| v.as_u64())
            .map(|v| v as u8);
            
        let memory_mb = params.get("memory_mb")
            .and_then(|v| v.as_u64());
            
        let disk_gb = params.get("disk_gb")
            .and_then(|v| v.as_u64());
            
        let image = params.get("image")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
            
        let join_formnet = params.get("join_formnet")
            .and_then(|v| v.as_bool());
            
        let metadata = params.get("metadata")
            .and_then(|v| v.as_object())
            .map(|obj| obj.clone());
        
        // Create VM config
        let network_config = VMNetworkConfig {
            join_formnet,
            external_networks: None,
        };
        
        let vm_config = VMCreateConfig {
            name: name.clone(),
            vcpus,
            memory_mb,
            disk_gb,
            image,
            network: Some(network_config),
            metadata,
        };
        
        // Generate a unique build ID for this VM
        let random_id = rand::random::<u32>();
        let build_id = format!("{}-{}", name, random_id);
        
        // Submit create request with the generated build_id
        self.submit_create_request(&vm_config, &context, build_id).await
    }
} 