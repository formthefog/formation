// form-mcp: Model Context Protocol Server for Formation Network
//
// This library implements a Model Context Protocol (MCP) server which provides
// a standardized interface for AI agents to manage Formation Network resources.

pub mod api;
pub mod auth;
pub mod tools;
pub mod events;
pub mod models;
pub mod config;
pub mod billing;
pub mod errors;

use std::sync::Arc;
use tokio::sync::RwLock;

/// Version of the MCP specification implemented by this server
pub const MCP_VERSION: &str = "0.1.0";

/// Default server configuration constants
pub mod defaults {
    /// Default port for the MCP server
    pub const SERVER_PORT: u16 = 3010;
    /// Default host address to bind to
    pub const SERVER_HOST: &str = "127.0.0.1";
    /// Default timeout for requests in seconds
    pub const REQUEST_TIMEOUT_SECS: u64 = 60;
    /// Default number of worker threads (0 = auto)
    pub const WORKERS: usize = 0;
}

/// Gracefully shuts down the MCP server
pub async fn shutdown_server() -> Result<(), Box<dyn std::error::Error>> {
    // Placeholder for shutdown logic
    // This will be implemented in a future sub-task
    Ok(())
} 