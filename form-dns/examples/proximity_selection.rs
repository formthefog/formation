use form_dns::geo_resolver::{GeoDnsResolver, GeoResolverConfig, DistanceWeightStrategy};
use form_dns::geolocation::{GeoLocation, GeoResolver};
use std::net::{IpAddr, Ipv4Addr};
use std::path::Path;
use std::env;
use std::collections::HashMap;
use trust_dns_proto::rr::RecordType;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Setup logging
    env_logger::init();
    
    println!("=== Enhanced Geographic Proximity-Based DNS Selection Demo ===\n");
    
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
    
    println!("Using MaxMind database at: {}\n", db_path);
    
    // Create test IP addresses (well-known DNS servers in different regions)
    let test_ips = vec![
        IpAddr::V4(Ipv4Addr::new(8, 8, 8, 8)),        // Google DNS (US)
        IpAddr::V4(Ipv4Addr::new(1, 1, 1, 1)),        // Cloudflare DNS (US) 
        IpAddr::V4(Ipv4Addr::new(208, 67, 222, 222)), // OpenDNS (US)
        IpAddr::V4(Ipv4Addr::new(185, 228, 168, 9)),  // CleanBrowsing (EU)
        IpAddr::V4(Ipv4Addr::new(199, 85, 126, 10)),  // Norton ConnectSafe (US)
        IpAddr::V4(Ipv4Addr::new(8, 26, 56, 26)),     // Comodo Secure (US)
        IpAddr::V4(Ipv4Addr::new(64, 6, 64, 6)),      // Verisign (US)
        IpAddr::V4(Ipv4Addr::new(156, 154, 70, 1)),   // Neustar DNS (US)
        IpAddr::V4(Ipv4Addr::new(9, 9, 9, 9)),        // Quad9 (Global)
    ];
    
    // Create test clients from different locations
    let test_clients = vec![
        // Format: IP, description
        (IpAddr::V4(Ipv4Addr::new(178, 62, 0, 1)), "London, UK"),
        (IpAddr::V4(Ipv4Addr::new(35, 180, 0, 1)), "Paris, France"),
        (IpAddr::V4(Ipv4Addr::new(13, 36, 0, 1)), "Frankfurt, Germany"),
        (IpAddr::V4(Ipv4Addr::new(54, 193, 0, 1)), "California, US"),
        (IpAddr::V4(Ipv4Addr::new(18, 232, 0, 1)), "Virginia, US"),
        (IpAddr::V4(Ipv4Addr::new(13, 208, 0, 1)), "Tokyo, Japan"),
        (IpAddr::V4(Ipv4Addr::new(43, 250, 0, 1)), "Mumbai, India"),
        (IpAddr::V4(Ipv4Addr::new(13, 54, 0, 1)), "Sydney, Australia"),
        (IpAddr::V4(Ipv4Addr::new(54, 233, 0, 1)), "São Paulo, Brazil"),
    ];
    
    // Create the resolver with the Default strategy
    let resolver = GeoResolver::new(Path::new(&db_path))?;
    
    // Verify and get locations for all servers
    let mut server_locations = HashMap::new();
    println!("Resolving locations for test servers:");
    for &ip in &test_ips {
        match resolver.get_location(ip) {
            Ok(location) => {
                let description = format!("{}{}, {}",
                                         location.region_code.clone().unwrap_or_default(),
                                         if location.region_code.is_some() { ", " } else { "" },
                                         location.country_code.clone().unwrap_or_default());
                println!("  {} => {}", ip, description);
                server_locations.insert(ip, (location, description));
            },
            Err(e) => {
                println!("  {} => Error resolving location: {}", ip, e);
            }
        }
    }
    println!();
    
    // Test different weighting strategies for each client
    let strategies = vec![
        DistanceWeightStrategy::Linear,
        DistanceWeightStrategy::Quadratic,
        DistanceWeightStrategy::Logarithmic,
        DistanceWeightStrategy::Stepped,
    ];
    
    for (client_ip, client_desc) in &test_clients {
        println!("==== Client: {} ({}) ====", client_ip, client_desc);
        
        // Get client location
        let client_location = match resolver.get_location(*client_ip) {
            Ok(loc) => {
                println!("  Location: {}{}, {}",
                         loc.region_code.clone().unwrap_or_default(),
                         if loc.region_code.is_some() { ", " } else { "" },
                         loc.country_code.clone().unwrap_or_default());
                loc
            },
            Err(e) => {
                println!("  Error resolving client location: {}", e);
                continue;
            }
        };
        
        // Print distances to each server
        println!("\n  Distances to servers:");
        for &ip in &test_ips {
            if let Some((location, desc)) = server_locations.get(&ip) {
                let distance = form_dns::geolocation::calculate_distance(&client_location, location);
                println!("    {} ({}): {:.1} km", ip, desc, distance);
            }
        }
        
        // Test each strategy
        for strategy in &strategies {
            println!("\n  === Strategy: {:?} ===", strategy);
            
            // Configure resolver
            let geo_config = GeoResolverConfig {
                db_path: db_path.clone(),
                enabled: true,
                distance_weights: *strategy,
                ..GeoResolverConfig::default()
            };
            
            let geo_dns = GeoDnsResolver::new(geo_config);
            
            // Get sorted results
            let sorted_ips = geo_dns.get_geo_sorted_ips(
                "example.com",
                RecordType::A,
                Some(*client_ip),
                test_ips.clone()
            );
            
            // Print results
            println!("  Selection results:");
            for (i, ip) in sorted_ips.iter().enumerate() {
                if let Some((_, desc)) = server_locations.get(ip) {
                    println!("    #{}: {} ({})", i+1, ip, desc);
                } else {
                    println!("    #{}: {}", i+1, ip);
                }
            }
        }
        
        // Also test region bias
        println!("\n  === Testing Region Bias ===");
        
        // Strong region bias
        let config_with_bias = GeoResolverConfig {
            db_path: db_path.clone(),
            enabled: true,
            prefer_same_region: true,
            region_bias_factor: 0.9,
            ..GeoResolverConfig::default()
        };
        
        let bias_dns = GeoDnsResolver::new(config_with_bias);
        let biased_ips = bias_dns.get_geo_sorted_ips(
            "example.com",
            RecordType::A,
            Some(*client_ip),
            test_ips.clone()
        );
        
        println!("  Strong region bias (0.9) results:");
        for (i, ip) in biased_ips.iter().enumerate() {
            if let Some((_, desc)) = server_locations.get(ip) {
                println!("    #{}: {} ({})", i+1, ip, desc);
            } else {
                println!("    #{}: {}", i+1, ip);
            }
        }
        
        println!("\n");
    }
    
    Ok(())
} 