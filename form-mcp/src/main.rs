use std::env;
use form_mcp::start_server;
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
    
    // Start the server
    match start_server(config_path.as_deref()).await {
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
