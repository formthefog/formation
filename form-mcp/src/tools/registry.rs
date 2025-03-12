// Tool registry module
//
// This module defines the tool registry system which manages tool registration
// and discovery for the MCP server.

use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use async_trait::async_trait;
use serde::{Serialize, Deserialize};
use serde_json::Value;

use crate::errors::ToolError;
use crate::tools::ToolContext;

/// ToolParameter defines a parameter for a tool
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ToolParameter {
    /// Name of the parameter
    pub name: String,
    /// Description of the parameter
    pub description: String,
    /// Whether the parameter is required
    pub required: bool,
    /// Type of the parameter (string, number, boolean, object, array)
    pub parameter_type: String,
    /// Default value for the parameter
    pub default: Option<Value>,
    /// Enum values for the parameter (if applicable)
    pub enum_values: Option<Vec<Value>>,
}

/// ToolDefinition defines a tool available in the MCP server
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ToolDefinition {
    /// Name of the tool
    pub name: String,
    /// Description of the tool
    pub description: String,
    /// Version of the tool
    pub version: String,
    /// Parameters for the tool
    pub parameters: Vec<ToolParameter>,
    /// Return type description
    pub return_type: String,
    /// Tags for categorizing the tool
    pub tags: Vec<String>,
    /// Whether this tool execution is potentially long-running and should use the operations system
    pub is_long_running: Option<bool>,
}

/// Type alias for tool execution results
pub type ToolResult = Result<Value, ToolError>;

/// Tool trait for implementing tool functionality
#[async_trait]
pub trait Tool: Send + Sync {
    /// Get the tool definition
    fn definition(&self) -> ToolDefinition;
    
    /// Execute the tool with the given parameters and context
    async fn execute(&self, params: Value, context: ToolContext) -> ToolResult;
    
    /// Validate the parameters for the tool
    fn validate_params(&self, params: &Value) -> Result<(), ToolError> {
        let definition = self.definition();
        
        // Check required parameters
        for param in definition.parameters.iter().filter(|p| p.required) {
            if let Value::Object(map) = params {
                if !map.contains_key(&param.name) {
                    return Err(ToolError::InvalidParameters(
                        format!("Missing required parameter: {}", param.name)
                    ));
                }
            } else {
                return Err(ToolError::InvalidParameters(
                    "Parameters must be an object".to_string()
                ));
            }
        }
        
        Ok(())
    }
}

/// ToolRegistry manages tool registration and discovery
pub struct ToolRegistry {
    tools: RwLock<HashMap<String, Arc<dyn Tool>>>,
}

impl ToolRegistry {
    /// Create a new tool registry
    pub fn new() -> Self {
        Self {
            tools: RwLock::new(HashMap::new()),
        }
    }
    
    /// Register a tool with the registry
    pub fn register_tool(&self, tool: Arc<dyn Tool>) -> Result<(), ToolError> {
        let definition = tool.definition();
        let name = definition.name.clone();
        
        let mut tools = self.tools.write().map_err(|_| {
            ToolError::RegistrationFailed("Failed to acquire write lock".to_string())
        })?;
        
        if tools.contains_key(&name) {
            return Err(ToolError::RegistrationFailed(
                format!("Tool with name '{}' already registered", name)
            ));
        }
        
        tools.insert(name, tool);
        Ok(())
    }
    
    /// Get a tool by name
    pub fn get_tool(&self, name: &str) -> Option<Arc<dyn Tool>> {
        self.tools.read().ok()?.get(name).cloned()
    }
    
    /// List all registered tools
    pub fn list_tools(&self) -> Vec<ToolDefinition> {
        self.tools.read()
            .map(|tools| {
                tools.values()
                    .map(|tool| tool.definition())
                    .collect()
            })
            .unwrap_or_default()
    }
    
    /// Get tool categories (based on tool tags)
    pub fn get_categories(&self) -> Vec<String> {
        let mut categories = std::collections::HashSet::new();
        
        if let Ok(tools) = self.tools.read() {
            for tool in tools.values() {
                let def = tool.definition();
                for tag in def.tags.clone() {
                    categories.insert(tag);
                }
            }
        }
        
        categories.into_iter().collect()
    }
} 