use form_pack::manager::FormPackManager;
use std::net::SocketAddr;
use tokio::sync::broadcast;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Create a channel for shutdown signal
    let (shutdown_sender, shutdown_receiver) = broadcast::channel(1);
    
    // Handle Ctrl+C signal for graceful shutdown
    let shutdown_sender_clone = shutdown_sender.clone();
    tokio::spawn(async move {
        tokio::signal::ctrl_c().await.expect("Failed to listen for ctrl+c");
        println!("Received shutdown signal, shutting down...");
        let _ = shutdown_sender_clone.send(());
    });
    
    // Get environment variables
    let node_id = std::env::var("NODE_ID")
        .unwrap_or_else(|_| "form-pack-default".to_string());
    
    let host = std::env::var("API_HOST")
        .unwrap_or_else(|_| "0.0.0.0".to_string());
    
    let port = std::env::var("API_PORT")
        .unwrap_or_else(|_| "3001".to_string())
        .parse::<u16>()
        .unwrap_or(3001);
    
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    
    // Create and run the FormPackManager
    let manager = FormPackManager::new(addr, node_id);
    manager.run(shutdown_receiver).await?;
    
    println!("Form Pack service gracefully shut down");
    Ok(())
} 