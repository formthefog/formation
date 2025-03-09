use std::error::Error;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::time::Duration;
use log::info;
use formnet::relay::{RelayService, RelayConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Set up logging
    simple_logger::init_with_level(log::Level::Info).unwrap();
    
    info!("Starting relay service example");
    
    // Generate a relay keypair (in a real application, this would be persistent)
    let relay_pubkey = [0u8; 32]; // In real usage, this would be a proper public key
    
    // Configure the relay service
    let listen_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), 51820);
    let config = RelayConfig::new(listen_addr, relay_pubkey)
        .with_region("us-west")
        .with_capabilities(formnet::relay::RELAY_CAP_IPV4);
    
    info!("Creating relay service with configuration: {:?}", config);
    
    // Create the relay service
    let mut relay_service = RelayService::new(config);
    
    // Start the relay service
    info!("Starting relay service...");
    match relay_service.start() {
        Ok(_) => info!("Relay service started successfully"),
        Err(e) => {
            eprintln!("Failed to start relay service: {}", e);
            return Err(e.into());
        }
    }
    
    // Run for 5 seconds then exit
    info!("Relay service is running. Will exit in 5 seconds...");
    tokio::time::sleep(Duration::from_secs(5)).await;
    
    // Stop the relay service
    info!("Stopping relay service...");
    relay_service.stop();
    info!("Relay service stopped");
    
    Ok(())
} 