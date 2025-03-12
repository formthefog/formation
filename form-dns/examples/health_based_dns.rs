use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Arc;
use std::time::Duration;
use form_dns::authority::FormAuthority;
use form_dns::health;
use form_dns::store::{DnsStore, SharedStore, FormDnsRecord, VerificationStatus};
use tokio::sync::RwLock;
use trust_dns_client::client::AsyncClient;
use trust_dns_proto::rr::{Name, Record, RecordType, LowerName};
use trust_dns_proto::udp::{UdpClientConnect, UdpClientStream};
use tokio::net::UdpSocket;
use trust_dns_server::authority::{Authority, LookupObject, LookupOptions};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    env_logger::init();
    
    println!("====== Health-Based DNS Filtering Example ======");
    
    // Create a shared DNS store
    let (tx, _) = tokio::sync::mpsc::channel(1024);
    let store: SharedStore = Arc::new(RwLock::new(DnsStore::new(tx)));
    
    // Set up a test domain with multiple IPs
    let test_domain = "test.example.com";
    let test_ips = vec![
        SocketAddr::new(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 10)), 80),
        SocketAddr::new(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 11)), 80),
        SocketAddr::new(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 12)), 80),
        SocketAddr::new(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 13)), 80),
    ];
    
    // Add the test domain to the DNS store
    {
        let mut store_guard = store.write().await;
        let record = FormDnsRecord {
            domain: test_domain.to_string(),
            record_type: RecordType::A,
            public_ip: test_ips.clone(),
            formnet_ip: vec![],
            cname_target: None,
            ssl_cert: false,
            ttl: 300,
            verification_status: Some(VerificationStatus::NotVerified),
            verification_timestamp: Some(0),
        };
        
        store_guard.insert(test_domain, record).await;
        println!("Added test domain '{}' with {} IPs to DNS store", test_domain, test_ips.len());
    }
    
    // Create a fallback client (though we won't use it in this example)
    let fallback = "8.8.8.8:53".parse().unwrap();
    let stream: UdpClientConnect<UdpSocket> = UdpClientStream::new(fallback);
    let (fallback_client, bg) = AsyncClient::connect(stream).await?;
    tokio::spawn(bg);
    
    // Create the FormAuthority
    let origin = Name::root();
    let auth = FormAuthority::new(origin, store.clone(), fallback_client);
    
    // First, query without health repository
    let query_result = perform_dns_query(&auth, test_domain).await?;
    println!("\n=== DNS Query without health filtering ===");
    display_result(&query_result);
    
    // Create health repository and mark some IPs as unhealthy
    println!("\n=== Creating health repository and marking IPs ===");
    let health_repo = health::create_shared_repository(Duration::from_secs(60));
    {
        // Mark IP 192.168.1.10 as available
        let ip1 = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 10));
        
        // Mark IP 192.168.1.11 as unavailable
        let ip2 = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 11));
        
        // Mark IP 192.168.1.12 as unavailable
        let ip3 = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 12));
        
        // We'll leave 192.168.1.13 unmarked (it should be considered available by default)
        
        let mut health_guard = health_repo.write().await;
        health_guard.mark_available(ip1);
        health_guard.mark_unavailable(ip2, "Node offline".to_string());
        health_guard.mark_unavailable(ip3, "Node unreachable".to_string());
        
        println!("Marked 192.168.1.10 as available");
        println!("Marked 192.168.1.11 as unavailable (Node offline)");
        println!("Marked 192.168.1.12 as unavailable (Node unreachable)");
        println!("Left 192.168.1.13 unmarked (should be available by default)");
    }
    
    // Create a new FormAuthority with health repository
    let auth_with_health = auth.with_health_repository(health_repo.clone());
    
    // Query with health repository
    let query_result_with_health = perform_dns_query(&auth_with_health, test_domain).await?;
    println!("\n=== DNS Query with health filtering ===");
    display_result(&query_result_with_health);
    
    // Now mark all IPs as unavailable
    println!("\n=== Marking all IPs as unavailable ===");
    {
        let mut health_guard = health_repo.write().await;
        for ip in test_ips.iter() {
            health_guard.mark_unavailable(ip.ip(), "All nodes down".to_string());
            println!("Marked {} as unavailable", ip.ip());
        }
    }
    
    // Query again with all IPs unavailable
    let query_result_all_unavailable = perform_dns_query(&auth_with_health, test_domain).await?;
    println!("\n=== DNS Query with all IPs unavailable ===");
    display_result(&query_result_all_unavailable);
    println!("Note: Even though all IPs are unhealthy, they may still be returned to avoid service disruption");
    
    Ok(())
}

async fn perform_dns_query(
    authority: &FormAuthority,
    domain: &str,
) -> Result<Vec<Record>, Box<dyn std::error::Error>> {
    // Create the lower name for lookup
    let name = Name::from_ascii(domain)?;
    let lower_name = LowerName::new(&name);
    
    // Use the public lookup method
    let options = LookupOptions::default();
    let lookup_result = Authority::lookup(authority, &lower_name, RecordType::A, options).await?;
    
    // Convert lookup result to Vec<Record>
    let mut records = Vec::new();
    for record in lookup_result.iter() {
        records.push(record.clone());
    }
    
    Ok(records)
}

fn display_result(records: &[Record]) {
    println!("Response contains {} records", records.len());
    for (i, record) in records.iter().enumerate() {
        println!("Record {}: {:?} (TTL: {})", i + 1, record.data(), record.ttl());
    }
} 