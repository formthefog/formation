use std::path::PathBuf;
use std::sync::Arc;
use std::env;

use clap::Parser;
use dotenv::dotenv;
use k256::ecdsa::SigningKey;
use tokio::sync::Mutex;
use rand::{thread_rng, RngCore};
use uuid::Uuid;

use form_state::datastore::DataStore;
use form_state::api;

/// CLI arguments for Form-State Mock Server
#[derive(Parser, Debug)]
struct Cli {
    /// Port to listen on
    #[clap(long, default_value = "3004")]
    port: u16,
    
    /// JWT audience for auth validation
    #[clap(long)]
    jwt_audience: Option<String>,
    
    /// JWT issuer for auth validation
    #[clap(long)]
    jwt_issuer: Option<String>,
    
    /// JWKS URL for auth validation
    #[clap(long)]
    jwks_url: Option<String>,
    
    /// JWT leeway in seconds
    #[clap(long, default_value = "60")]
    jwt_leeway: Option<String>,
    
    /// Path to .env file
    #[clap(long)]
    env_file: Option<PathBuf>,
    
    /// Skip JWT validation (for local development)
    #[clap(long)]
    skip_jwt: bool,
    
    /// Generate verbose logs
    #[clap(long)]
    verbose: bool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Parse CLI arguments
    let args = Cli::parse();
    
    // Initialize logger with appropriate level
    if args.verbose {
        simple_logger::init_with_level(log::Level::Debug)?;
    } else {
        simple_logger::init_with_level(log::Level::Info)?;
    }
    
    log::info!("Starting Form-State Mock Server");
    
    // Load environment variables from .env file if provided
    if let Some(env_path) = args.env_file {
        log::info!("Loading environment from file: {:?}", env_path);
        match dotenv::from_path(env_path) {
            Ok(_) => log::info!("Successfully loaded environment from file"),
            Err(e) => log::warn!("Failed to load environment from file: {}", e),
        }
    } else {
        // Try to load from default .env file if it exists
        match dotenv() {
            Ok(_) => log::info!("Loaded environment from .env file"),
            Err(_) => log::debug!("No .env file found or failed to load it"),
        }
    }
    
    // Set JWT authentication environment variables from CLI args
    if let Some(audience) = args.jwt_audience {
        log::info!("Setting JWT audience to: {}", audience);
        env::set_var("DYNAMIC_JWT_AUDIENCE", audience);
    }
    
    if let Some(issuer) = args.jwt_issuer {
        log::info!("Setting JWT issuer to: {}", issuer);
        env::set_var("DYNAMIC_JWT_ISSUER", issuer);
    }
    
    if let Some(jwks_url) = args.jwks_url {
        log::info!("Setting JWKS URL to: {}", jwks_url);
        env::set_var("DYNAMIC_JWKS_URL", jwks_url);
    }
    
    if let Some(leeway) = args.jwt_leeway {
        log::info!("Setting JWT leeway to: {}", leeway);
        env::set_var("DYNAMIC_JWT_LEEWAY", leeway);
    }
    
    if args.skip_jwt {
        log::warn!("JWT validation is disabled! This should only be used for local development.");
        env::set_var("SKIP_JWT_VALIDATION", "true");
    }
    
    // Set the API port
    env::set_var("API_PORT", args.port.to_string());
    
    // Log JWT configuration
    log::info!("JWT Configuration:");
    log::info!("  Audience: {:?}", env::var("DYNAMIC_JWT_AUDIENCE").ok());
    log::info!("  Issuer: {:?}", env::var("DYNAMIC_JWT_ISSUER").ok());
    log::info!("  JWKS URL: {:?}", env::var("DYNAMIC_JWKS_URL").ok());
    log::info!("  Leeway: {:?}", env::var("DYNAMIC_JWT_LEEWAY").ok());
    
    // Generate random key for the node
    let mut key_bytes = [0u8; 32];
    thread_rng().fill_bytes(&mut key_bytes);
    let private_key = hex::encode(&key_bytes);
    
    // Generate a node ID
    let node_id = Uuid::new_v4().to_string();
    log::info!("Generated node ID: {}", node_id);
    
    // Create a new datastore with the random key
    let datastore = DataStore::new(node_id, private_key);
    
    // Wrap datastore in Arc<Mutex<>>
    let datastore = Arc::new(Mutex::new(datastore));
    
    // Create a shutdown channel
    let (tx, rx) = tokio::sync::broadcast::channel(1);
    
    // Handle Ctrl+C signal
    let tx_clone = tx.clone();
    tokio::spawn(async move {
        match tokio::signal::ctrl_c().await {
            Ok(()) => {
                log::info!("Shutdown signal received");
                let _ = tx_clone.send(());
            }
            Err(err) => {
                log::error!("Error setting up Ctrl+C handler: {}", err);
            }
        }
    });
    
    log::info!("Starting API server on port {}", args.port);
    
    // Start the API server
    if let Err(e) = api::run(datastore, rx).await {
        log::error!("Error running API server: {}", e);
    }
    
    log::info!("Server shutdown complete");
    Ok(())
} 