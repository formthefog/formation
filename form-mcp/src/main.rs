use std::env;
use form_mcp::{api, tools};
use std::sync::Arc;
use anyhow::Result;
use log::{info, error};
use std::process;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    env_logger::init_from_env(env_logger::Env::default().default_filter_or("info"));
    
    info!("Starting form-mcp server version {}", form_mcp::MCP_VERSION);
    
    // Get configuration path from command line arguments
    let config_path = env::args().nth(1);
    
    // Load configuration
    let settings = match form_mcp::config::load_config(config_path.as_deref()) {
        Ok(settings) => {
            info!("Loaded configuration successfully");
            settings
        },
        Err(e) => {
            error!("Failed to load configuration: {}", e);
            process::exit(1);
        }
    };
    
    // Initialize tool registry
    let registry = tools::init_registry();
    info!("Initialized tool registry with {} tools", registry.list_tools().len());
    
    // Start the API server
    match api::init_server(settings, registry).await {
        Ok(_) => {
            info!("form-mcp server stopped gracefully");
            Ok(())
        },
        Err(e) => {
            error!("Error starting form-mcp server: {}", e);
            process::exit(1);
        }
    }
}
