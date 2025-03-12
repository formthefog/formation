use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Arc;
use std::time::Duration;
use form_dns::authority::FormAuthority;
use form_dns::health;
use form_dns::store::{DnsStore, SharedStore, FormDnsRecord, VerificationStatus};
use tokio::sync::RwLock;
use trust_dns_client::client::{AsyncClient, ClientHandle};
use trust_dns_proto::rr::{Name, Record, RecordType, LowerName};
use trust_dns_proto::udp::{UdpClientConnect, UdpClientStream};
use tokio::net::UdpSocket;
use trust_dns_server::authority::{Authority, LookupObject, LookupOptions};
use std::str::FromStr;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    env_logger::init();
    
    println!("====== Bootstrap Domain Example with Health-Based Filtering ======");
    
    // Create a shared DNS store
    let store: SharedStore = Arc::new(RwLock::new(DnsStore::default()));
    
    // Create health repository
    let health_repo = health::create_shared_repository(Duration::from_secs(60));
    
    // Setup fallback client for DNS resolution
    let fallback = "8.8.8.8:53".parse().unwrap();
    let stream = UdpClientStream::<UdpSocket>::new(fallback);
    let (fallback_client, bg) = AsyncClient::connect(stream).await?;
    tokio::spawn(bg);
    
    // Create DNS authority with health repository
    let origin = Name::from_str("formation.cloud.")?;
    let authority = FormAuthority::new(origin, store.clone(), fallback_client)
        .with_health_repository(health_repo.clone());
    
    // Step 1: Add bootstrap domain with sample bootstrap nodes from different regions
    {
        println!("\n1. Adding bootstrap domain with sample bootstrap nodes...");
        let mut store_guard = store.write().await;
        
        // Create the bootstrap domain record
        let bootstrap_domain = "bootstrap.formation.cloud";
        let bootstrap_record = FormDnsRecord {
            domain: bootstrap_domain.to_string(),
            record_type: RecordType::A,
            public_ip: vec![
                // Sample bootstrap nodes from different regions
                SocketAddr::new(IpAddr::V4(Ipv4Addr::new(198, 51, 100, 1)), 4000), // US East
                SocketAddr::new(IpAddr::V4(Ipv4Addr::new(198, 51, 100, 2)), 4000), // US West
                SocketAddr::new(IpAddr::V4(Ipv4Addr::new(198, 51, 100, 3)), 4000), // Europe
                SocketAddr::new(IpAddr::V4(Ipv4Addr::new(198, 51, 100, 4)), 4000), // Asia
            ],
            formnet_ip: vec![],
            cname_target: None,
            ssl_cert: false,
            ttl: 60, // Lower TTL for bootstrap domain to allow faster failover
            verification_status: Some(VerificationStatus::Verified),
            verification_timestamp: Some(std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |d| d.as_secs())),
        };
        
        // Add the bootstrap domain to the DNS store
        store_guard.insert(bootstrap_domain, bootstrap_record).await;
        println!("Bootstrap domain configured with 4 nodes");
    }
    
    // Step 2: Query DNS with all nodes healthy
    println!("\n2. Querying bootstrap domain (all nodes healthy)...");
    let result = perform_dns_query(&authority, "bootstrap.formation.cloud").await?;
    println!("Resolved IPs: {}", result.len());
    display_result(&result);
    
    // Step 3: Mark some nodes as unhealthy
    {
        println!("\n3. Marking some bootstrap nodes as unhealthy...");
        let mut health_guard = health_repo.write().await;
        
        // Mark two nodes as unhealthy
        health_guard.mark_unavailable(
            IpAddr::V4(Ipv4Addr::new(198, 51, 100, 1)), 
            "Node down for maintenance".to_string()
        );
        health_guard.mark_unavailable(
            IpAddr::V4(Ipv4Addr::new(198, 51, 100, 3)),
            "Connection timeout".to_string()
        );
        
        println!("Marked 2 nodes as unhealthy: 198.51.100.1, 198.51.100.3");
    }
    
    // Step 4: Query DNS again with some nodes unhealthy
    println!("\n4. Querying bootstrap domain with unhealthy nodes filtered...");
    let result = perform_dns_query(&authority, "bootstrap.formation.cloud").await?;
    println!("Resolved IPs (should only show healthy nodes): {}", result.len());
    display_result(&result);
    
    // Step 5: Mark all nodes as unhealthy to demonstrate fallback behavior
    {
        println!("\n5. Marking all bootstrap nodes as unhealthy...");
        let mut health_guard = health_repo.write().await;
        
        // Mark remaining nodes as unhealthy
        health_guard.mark_unavailable(
            IpAddr::V4(Ipv4Addr::new(198, 51, 100, 2)), 
            "Region outage".to_string()
        );
        health_guard.mark_unavailable(
            IpAddr::V4(Ipv4Addr::new(198, 51, 100, 4)),
            "Network partition".to_string()
        );
        
        println!("Marked all nodes as unhealthy");
    }
    
    // Step 6: Query DNS with all nodes unhealthy to demonstrate failback behavior
    println!("\n6. Querying bootstrap domain with all nodes unhealthy...");
    println!("   (Should return IPs anyway to avoid complete service disruption)");
    let result = perform_dns_query(&authority, "bootstrap.formation.cloud").await?;
    println!("Resolved IPs: {}", result.len());
    display_result(&result);
    
    println!("\nExample completed successfully!");
    Ok(())
}

// Helper function to perform DNS query
async fn perform_dns_query(
    authority: &FormAuthority,
    domain: &str,
) -> Result<Vec<Record>, Box<dyn std::error::Error>> {
    // Convert domain to LowerName for DNS lookup
    let name = LowerName::from_str(format!("{}.", domain).as_str())?;
    
    // Use client IP from US East for location-based sorting
    let client_ip = Some(IpAddr::V4(Ipv4Addr::new(50, 16, 0, 1)));
    
    // Create lookup options with client IP
    let lookup_options = LookupOptions::default();
    
    // Perform DNS lookup with the client IP
    let lookup = authority.lookup(
        &name,
        RecordType::A,
        lookup_options,
    ).await?;
    
    // Extract records from lookup
    let records: Vec<Record> = lookup.iter().cloned().collect();
    
    Ok(records)
}

// Helper function to display DNS results
fn display_result(records: &[Record]) {
    if records.is_empty() {
        println!("No records found");
        return;
    }
    
    for (i, record) in records.iter().enumerate() {
        let ip = match record.data() {
            Some(trust_dns_proto::rr::RData::A(addr)) => format!("{}", addr),
            Some(other) => format!("{:?}", other),
            None => "No data".to_string(),
        };
        
        println!("  {}. {} (TTL: {}s)", i+1, ip, record.ttl());
    }
} 