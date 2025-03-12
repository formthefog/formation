use form_dns::geo_resolver::{GeoDnsResolver, GeoResolverConfig};
use form_dns::geo_util;
use form_dns::geolocation::{GeoLocation, GeoResolver};
use std::net::{IpAddr, Ipv4Addr};
use std::path::Path;
use std::env;
use trust_dns_proto::rr::RecordType;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Setup logging
    env_logger::init();
    
    // Path to MaxMind database - can be passed as a command line argument
    let db_path = env::args().nth(1).unwrap_or_else(|| {
        // Default locations to check
        for path in &[
            "./GeoLite2-City.mmdb",
            "/etc/formation/geo/GeoLite2-City.mmdb",
        ] {
            if Path::new(path).exists() {
                return path.to_string();
            }
        }
        
        // If no file found, return the first path as default
        "./GeoLite2-City.mmdb".to_string()
    });
    
    println!("Using MaxMind database at: {}", db_path);
    
    // Initialize the global geolocation resolver
    let config = GeoResolverConfig {
        db_path: db_path.clone(),
        enabled: true,
        ..GeoResolverConfig::default()
    };
    geo_util::init_geo_resolver(config);
    
    // Test IPs to sort by proximity
    let test_ips = vec![
        IpAddr::V4(Ipv4Addr::new(8, 8, 8, 8)),      // Google DNS (US)
        IpAddr::V4(Ipv4Addr::new(1, 1, 1, 1)),      // Cloudflare DNS (US)
        IpAddr::V4(Ipv4Addr::new(205, 251, 192, 64)), // Amazon AWS (US)
        IpAddr::V4(Ipv4Addr::new(185, 228, 168, 9)),  // CleanBrowsing (EU)
    ];
    
    // Sample client locations
    let client_ips = vec![
        // London location
        IpAddr::V4(Ipv4Addr::new(185, 228, 168, 10)),
        // Singapore location 
        IpAddr::V4(Ipv4Addr::new(203, 114, 116, 78)),
        // New York location
        IpAddr::V4(Ipv4Addr::new(74, 125, 203, 99)),
    ];
    
    for (idx, client_ip) in client_ips.iter().enumerate() {
        println!("\n--- Test Case {} - Client IP: {} ---", idx + 1, client_ip);
        
        // Get the client's location
        if let Some(location) = geo_util::get_client_location(*client_ip) {
            println!("Client location: {:?}, {:?} ({}, {})", 
                location.country_code, location.region_code, 
                location.latitude, location.longitude);
        } else {
            println!("Could not determine client location");
            continue;
        }
        
        // Sort IPs by proximity
        let sorted_ips = geo_util::sort_ips_by_client_location(
            "example.com",
            RecordType::A,
            Some(*client_ip),
            test_ips.clone()
        );
        
        println!("Sorted IPs by proximity:");
        for (position, ip) in sorted_ips.iter().enumerate() {
            println!("  #{}: {}", position + 1, ip);
        }
    }
    
    println!("\nDemonstrating IP sorting for DNS requests...");
    
    // Simulate a DNS request with test socket addresses
    let domain = "bootstrap.formation.network";
    let record_type = RecordType::A;
    let socket_addrs = vec![
        format!("{}:80", test_ips[0]).parse()?,  // Google DNS with port 80
        format!("{}:80", test_ips[1]).parse()?,  // Cloudflare DNS with port 80
        format!("{}:80", test_ips[2]).parse()?,  // AWS with port 80
        format!("{}:80", test_ips[3]).parse()?,  // CleanBrowsing with port 80
    ];
    
    for (idx, client_ip) in client_ips.iter().enumerate() {
        println!("\n--- DNS Request {} - Client IP: {} ---", idx + 1, client_ip);
        
        // Apply geolocation sorting to the DNS results
        let sorted_addrs = geo_util::apply_geo_sorting(
            domain,
            record_type,
            Some(*client_ip),
            socket_addrs.clone()
        );
        
        println!("DNS response IPs (sorted by proximity):");
        for (position, addr) in sorted_addrs.iter().enumerate() {
            println!("  #{}: {}", position + 1, addr);
        }
    }
    
    Ok(())
} 