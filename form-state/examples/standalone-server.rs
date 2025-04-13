use form_state::datastore::DataStore;
use form_state::api;
use std::sync::Arc;
use tokio::sync::Mutex;
use k256::ecdsa::{SigningKey, signature::rand_core::OsRng};
use alloy_primitives::Address;
use hex;

/// This example demonstrates how to run a standalone datastore server
/// without connecting to any other Formation services.
/// 
/// The server will run on 0.0.0.0:3004 and serve all the API endpoints.
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize basic logging
    simple_logger::init_with_level(log::Level::Info).unwrap();
    
    println!("Initializing standalone datastore server...");
    
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