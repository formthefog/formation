// Pack Build Tool
//
// This tool allows building workloads from a Formfile specification.

use std::sync::Arc;
use std::collections::HashMap;
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
const FORMPACK_PORT: u16 = 3003;
const QUEUE_PORT: u16 = 53333;

/// Formfile content representation
#[derive(Debug, Serialize, Deserialize)]
pub struct Formfile {
    /// Base image information
    pub from: String,
    /// Workload name
    pub name: Option<String>,
    /// Commands to run during build
    pub run: Option<Vec<String>>,
    /// Files to include
    pub include: Option<Vec<String>>,
    /// Environment variables
    pub env: Option<HashMap<String, String>>,
    /// Exposed ports
    pub expose: Option<Vec<u16>>,
    /// Entry point command
    pub entrypoint: Option<String>,
    /// Resources configuration
    pub resources: Option<ResourcesConfig>,
    /// Network configuration
    pub network: Option<NetworkConfig>,
    /// Additional metadata
    pub metadata: Option<HashMap<String, String>>,
}

/// Resources configuration for the workload
#[derive(Debug, Serialize, Deserialize)]
pub struct ResourcesConfig {
    /// Number of vCPUs
    pub vcpus: Option<u8>,
    /// Memory size in MB
    pub memory_mb: Option<u64>,
    /// Root disk size in GB
    pub disk_gb: Option<u64>,
}

/// Network configuration for the workload
#[derive(Debug, Serialize, Deserialize)]
pub struct NetworkConfig {
    /// Whether to join the formnet network
    pub join_formnet: Option<bool>,
    /// External networks to connect to
    pub external_networks: Option<Vec<String>>,
}

/// Pack build request format
#[derive(Debug, Serialize, Deserialize)]
pub struct PackBuildRequest {
    /// Formfile content
    pub formfile: Formfile,
    /// Context data (files content)
    pub context: HashMap<String, String>,
    /// User ID owning the build
    pub user_id: String,
    /// Request signature
    pub signature: Option<String>,
    /// Nonce for request validation
    pub nonce: String,
    /// Timestamp of the request
    pub timestamp: u64,
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

/// Pack build tool implementation
pub struct PackBuildTool {
    http_client: Client,
}

impl PackBuildTool {
    /// Create a new pack build tool
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
    
    /// Send a build request to the pack manager
    async fn submit_build_request(
        &self, 
        formfile_content: &str, 
        context_files: HashMap<String, String>,
        context: &ToolContext
    ) -> Result<Value, ToolError> {
        // Parse the Formfile content
        let formfile: Formfile = serde_json::from_str(formfile_content)
            .map_err(|e| ToolError::InvalidParameters(format!("Invalid Formfile format: {}", e)))?;
        
        // Generate build request
        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)
            .map_err(|_| ToolError::ExecutionFailed("Failed to get system time".to_string()))?
            .as_secs();
        
        let nonce = Uuid::new_v4().to_string();
        
        let build_request = PackBuildRequest {
            formfile,
            context: context_files,
            user_id: context.user_id.clone(),
            signature: None, // Placeholder for future signature implementation
            nonce,
            timestamp,
        };
        
        // Serialize the request
        let request_json = serde_json::to_vec(&build_request)
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to serialize build request: {}", e)))?;
        
        // Create queue request
        let queue_request = QueueRequest::Write {
            content: request_json,
            topic: "pack.build".to_string(),
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
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to send build request: {}", e)))?;
        
        // Handle response
        let queue_response: QueueResponse = response.json()
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to parse response: {}", e)))?;
        
        match queue_response {
            QueueResponse::OpSuccess => {
                // Return build ID for tracking
                let build_id = Uuid::new_v4().to_string();
                
                Ok(json!({
                    "status": "success",
                    "build_id": build_id,
                    "message": "Build request accepted successfully"
                }))
            },
            QueueResponse::Failure { reason } => {
                Err(ToolError::ExecutionFailed(format!("Build request failed: {}", 
                    reason.unwrap_or_else(|| "Unknown reason".to_string()))))
            }
        }
    }
}

#[async_trait]
impl Tool for PackBuildTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "form_pack_build".to_string(),
            description: "Builds a workload from a Formfile specification".to_string(),
            version: "0.1.0".to_string(),
            parameters: vec![
                ToolParameter {
                    name: "formfile_content".to_string(),
                    description: "Content of the Formfile in JSON or YAML format".to_string(),
                    required: true,
                    parameter_type: "string".to_string(),
                    default: None,
                    enum_values: None,
                },
                ToolParameter {
                    name: "context_files".to_string(),
                    description: "Map of filename to file content for files to include in the build context".to_string(),
                    required: false,
                    parameter_type: "object".to_string(),
                    default: None,
                    enum_values: None,
                },
            ],
            return_type: "Build ID and status for tracking the build process".to_string(),
            tags: vec![
                "pack".to_string(),
                "build".to_string(),
                "workload".to_string(),
            ],
            is_long_running: Some(true),
        }
    }
    
    async fn execute(&self, params: Value, context: ToolContext) -> ToolResult {
        // Validate parameters
        self.validate_params(&params)?;
        
        // Extract parameters
        let formfile_content = params["formfile_content"].as_str()
            .ok_or_else(|| ToolError::InvalidParameters("Missing required parameter: formfile_content".to_string()))?;
        
        // Extract optional context files
        let mut context_files = HashMap::new();
        if let Some(files) = params["context_files"].as_object() {
            for (key, value) in files {
                if let Some(content) = value.as_str() {
                    context_files.insert(key.clone(), content.to_string());
                }
            }
        }
        
        // Submit build request
        self.submit_build_request(formfile_content, context_files, &context).await
    }
} 