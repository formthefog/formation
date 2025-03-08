use anyhow::{Error, anyhow};
use formnet::relay::{CacheIntegration, RelayManager, SharedRelayRegistry, set_relay_enabled, is_relay_enabled};
use formnet::nat_relay::RelayNatTraverse;
use log::{info, LevelFilter};
use shared::{Endpoint, Peer, PeerDiff, PeerContents};
use std::path::PathBuf;
use std::time::{Duration, Instant};
use wireguard_control::{Backend, InterfaceName};
use std::net::SocketAddr;
use std::str::FromStr;

#[tokio::main]
async fn main() -> Result<(), Error> {
    // Set up logging
    simple_logger::SimpleLogger::new()
        .with_level(LevelFilter::Info)
        .init()
        .unwrap();
    
    info!("Starting relay-enabled NAT traversal example");
    
    // Demonstrate both automatic and manual relay control
    
    // 1. Check the automatic detection result
    info!("Automatic relay detection result: relay {}", 
          if is_relay_enabled() { "enabled" } else { "disabled" });
    
    // 2. Override with manual control if desired (uncomment to test)
    // set_relay_enabled(true);  // Force enable relays
    // info!("After manual override: relay {}", 
    //      if is_relay_enabled() { "enabled" } else { "disabled" });
    
    // Create a relay registry and manager
    let registry = SharedRelayRegistry::new();
    // Convert hex string to byte array
    let mut local_pubkey = [0u8; 32];
    hex::decode_to_slice("AABBCCDDEEFF00112233445566778899AABBCCDDEEFF00112233445566778899", &mut local_pubkey)
        .map_err(|e| anyhow!("Failed to decode pubkey: {}", e))?;
    let relay_manager = RelayManager::new(registry, local_pubkey);
    
    // Create a mock interface
    let interface = InterfaceName::from_str("wg0").unwrap();
    let data_dir = PathBuf::from("/tmp/relay_nat_example");
    std::fs::create_dir_all(&data_dir).ok(); // Ensure directory exists
    
    // Create a cache integration
    let mut cache_integration = CacheIntegration::new(interface.clone(), data_dir.to_string_lossy().to_string());
    cache_integration.set_relay_manager(relay_manager);
    
    // Create example peers that need NAT traversal
    // In a real scenario, these would come from your peer discovery system
    let peers = vec![
        create_test_peer(
            "peer1", 
            "peer1-id", 
            "ABCDEF0123456789ABCDEF0123456789ABCDEF0123456789ABCDEF0123456789", 
            vec![
                Endpoint::from(SocketAddr::from_str("192.168.1.100:51820").unwrap()),
                Endpoint::from(SocketAddr::from_str("10.0.0.1:51820").unwrap()),
            ]
        ),
        create_test_peer(
            "peer2", 
            "peer2-id", 
            "9876543210FEDCBA9876543210FEDCBA9876543210FEDCBA9876543210FEDCBA", 
            vec![
                Endpoint::from(SocketAddr::from_str("192.168.1.200:51820").unwrap()),
                Endpoint::from(SocketAddr::from_str("10.0.0.2:51820").unwrap()),
            ]
        ),
    ];
    
    // Create peer diffs for NAT traversal
    // Using a simpler approach for the example - in real code you'd typically
    // compare against existing peers on the WireGuard interface
    let peer_diffs: Vec<_> = peers.iter()
        .filter_map(|p| {
            // Create peer diff with no old peer (simulating a new peer)
            match PeerDiff::new(None, Some(p)) {
                Ok(Some(diff)) => Some(diff),
                Ok(None) => None,
                Err(e) => {
                    info!("Error creating peer diff: {}", e);
                    None
                }
            }
        })
        .collect();
    
    info!("Created {} peer diffs for NAT traversal", peer_diffs.len());
    
    // Create NAT traversal with relay support
    let mut nat_traverse = RelayNatTraverse::new(
        &interface,
        Backend::Userspace,
        &peer_diffs,
        &cache_integration
    )?;
    
    // For demonstration purposes, we'll run the NAT traversal for a limited time
    // or until it's finished
    let start_time = Instant::now();
    let timeout = Duration::from_secs(60); // 60 second timeout
    
    while !nat_traverse.is_finished() && start_time.elapsed() < timeout {
        info!("Running NAT traversal step with relay support. Remaining peers: {}", 
              nat_traverse.remaining());
        
        // Perform a NAT traversal step with relay support
        // The method will automatically use relays only if needed based on
        // the intelligent detection system
        nat_traverse.step_with_relay_sync()?;
        
        // Wait a bit before trying again to avoid hammering the network
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
    
    if nat_traverse.is_finished() {
        info!("NAT traversal completed successfully");
    } else {
        info!("NAT traversal timed out with {} peers remaining", nat_traverse.remaining());
    }
    
    Ok(())
}

// Helper function to create a test peer
fn create_test_peer(name: &str, id: &str, pubkey: &str, endpoints: Vec<Endpoint>) -> Peer<String> {
    Peer {
        id: id.to_string(),
        contents: PeerContents {
            name: name.to_string().parse().unwrap(),
            ip: "10.0.0.100".parse().unwrap(), // Dummy IP
            cidr_id: "1".to_string(),
            public_key: pubkey.to_string(),
            endpoint: endpoints.first().cloned(),
            persistent_keepalive_interval: Some(25),
            is_admin: false,
            is_disabled: false,
            is_redeemed: true,
            invite_expires: None,
            candidates: endpoints,
        }
    }
} 