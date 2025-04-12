use form_state::datastore::DataStore;
use form_state::api;
use std::sync::Arc;
use std::env;
use tokio::sync::Mutex;
use k256::ecdsa::{SigningKey, signature::rand_core::OsRng};
use alloy_primitives::Address;
use hex;

/// This example demonstrates how to run a standalone datastore server
/// with JWT audience validation correctly configured.
/// 
/// The server will run on 0.0.0.0:3004 and serve all the API endpoints.
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize basic logging
    simple_logger::init_with_level(log::Level::Info).unwrap();
    
    println!("Initializing standalone datastore server with auth configuration...");
    
    // Set up JWT authentication configuration
    // The audience must match what's in your JWT token
    env::set_var("DYNAMIC_JWT_AUDIENCE", "https://formation-cloud-git-dynamic-versatus.vercel.app");
    
    // You can also set issuer if needed
    env::set_var("DYNAMIC_JWT_ISSUER", "app.dynamicauth.com/3f53e601-17c7-419b-8a13-4c5e25c0bde9");
    
    // Set the JWKS URL for Dynamic Auth
    env::set_var("DYNAMIC_JWKS_URL", "https://app.dynamic.xyz/api/v0/sdk/3f53e601-17c7-419b-8a13-4c5e25c0bde9/.well-known/jwks");
    
    // Set token validation leeway (in seconds)
    env::set_var("DYNAMIC_JWT_LEEWAY", "60");
    
    // Create a proper signing key for the node
    let signing_key = SigningKey::random(&mut OsRng);
    let private_key = hex::encode(signing_key.to_bytes());
    let address = hex::encode(Address::from_private_key(&signing_key));
    
    println!("Generated node address: {}", address);
    
    // Create a new DataStore instance with the generated credentials
    let datastore = Arc::new(Mutex::new(DataStore::new(address, private_key)));
    
    // Create a shutdown channel
    let (shutdown_sender, shutdown_receiver) = tokio::sync::broadcast::channel(1);
    
    println!("Starting API server on 0.0.0.0:3004");
    println!("JWT auth configured with audience: {}", env::var("DYNAMIC_JWT_AUDIENCE").unwrap());
    
    // Create a clone of the shutdown sender for signal handling
    let shutdown_sender_clone = shutdown_sender.clone();
    
    // Handle CTRL+C to gracefully shutdown
    tokio::spawn(async move {
        match tokio::signal::ctrl_c().await {
            Ok(()) => {
                println!("Shutdown signal received, shutting down...");
                let _ = shutdown_sender_clone.send(());
            }
            Err(err) => {
                eprintln!("Error setting up signal handler: {}", err);
            }
        }
    });
    
    // Run the API server
    // This will block until shutdown signal is received
    if let Err(e) = api::run(datastore, shutdown_receiver).await {
        eprintln!("Error running API server: {}", e);
    }
    
    println!("Server shutdown complete");
    Ok(())
} 