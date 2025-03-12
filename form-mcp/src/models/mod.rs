// Models module for the MCP server
//
// This module defines the data models used throughout the MCP server,
// including MCP protocol structures and internal data representations.

pub mod operations;

/// Represents a resource in the MCP protocol
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Resource {
    /// Resource ID
    pub id: String,
    /// Resource type
    pub resource_type: String,
    /// Resource name
    pub name: String,
    /// Resource metadata
    pub metadata: std::collections::HashMap<String, String>,
}

// Additional models will be added in future sub-tasks 