use form_dns::geolocation::{GeoLocation, GeoLocationError, GeoResolver, calculate_distance};
use form_dns::geo_resolver::{GeoDnsResolver, GeoResolverConfig};
use std::net::{IpAddr, Ipv4Addr};
use std::path::Path;
use std::env;

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
    
    // Create geolocation resolver directly
    match GeoResolver::new(Path::new(&db_path)) {
        Ok(resolver) => {
            // Test IP geolocation (Google DNS)
            let test_ip = IpAddr::V4(Ipv4Addr::new(8, 8, 8, 8));
            match resolver.get_location(test_ip) {
                Ok(location) => {
                    println!("Location for {}: {:?}", test_ip, location);
                    println!("Country: {:?}", location.country_code);
                    println!("Region: {:?}", location.region_code);
                    println!("Coordinates: {}, {}", location.latitude, location.longitude);
                }
                Err(e) => {
                    println!("Error looking up location for {}: {}", test_ip, e);
                }
            }
            
            // Test distance calculation
            let new_york = GeoLocation {
                latitude: 40.7128,
                longitude: -74.0060,
                country_code: Some("US".to_string()),
                region_code: Some("NY".to_string()),
            };
            
            let los_angeles = GeoLocation {
                latitude: 34.0522,
                longitude: -118.2437,
                country_code: Some("US".to_string()),
                region_code: Some("CA".to_string()),
            };
            
            let distance = calculate_distance(&new_york, &los_angeles);
            println!("Distance between New York and Los Angeles: {:.2} km", distance);
            
            // Test finding nearest IP
            let test_ips = vec![
                IpAddr::V4(Ipv4Addr::new(8, 8, 8, 8)),      // Google DNS (US)
                IpAddr::V4(Ipv4Addr::new(1, 1, 1, 1)),      // Cloudflare DNS (US)
                IpAddr::V4(Ipv4Addr::new(205, 251, 192, 64)), // Amazon AWS (US)
                IpAddr::V4(Ipv4Addr::new(185, 228, 168, 9)),  // CleanBrowsing (EU)
            ];
            
            let london = GeoLocation {
                latitude: 51.5074,
                longitude: -0.1278,
                country_code: Some("GB".to_string()),
                region_code: Some("ENG".to_string()),
            };
            
            println!("\nTesting proximity from London:");
            let nearest = resolver.find_nearest(&london, &test_ips);
            for (idx, (ip, distance)) in nearest.iter().enumerate() {
                println!("#{}: {} - Distance: {:?} km", idx + 1, ip, distance);
            }
        }
        Err(e) => {
            println!("Failed to initialize GeoResolver: {}", e);
        }
    }
    
    // Now demonstrate the DNS specific resolver
    println!("\nTesting GeoDnsResolver:");
    let config = GeoResolverConfig {
        db_path,
        enabled: true,
        ..GeoResolverConfig::default()
    };
    
    let geo_dns = GeoDnsResolver::new(config);
    
    // Test IPs to sort by proximity
    let test_ips = vec![
        IpAddr::V4(Ipv4Addr::new(8, 8, 8, 8)),      // Google DNS (US)
        IpAddr::V4(Ipv4Addr::new(1, 1, 1, 1)),      // Cloudflare DNS (US)
        IpAddr::V4(Ipv4Addr::new(205, 251, 192, 64)), // Amazon AWS (US)
        IpAddr::V4(Ipv4Addr::new(185, 228, 168, 9)),  // CleanBrowsing (EU)
    ];
    
    // Client location - pretend we're in London
    let client_ip = IpAddr::V4(Ipv4Addr::new(185, 228, 168, 10)); // Pretend this is from London
    
    println!("Sorting IPs by proximity to client IP: {}", client_ip);
    let sorted_ips = geo_dns.get_geo_sorted_ips(
        "example.com",
        trust_dns_proto::rr::RecordType::A,
        Some(client_ip),
        test_ips.clone()
    );
    
    for (idx, ip) in sorted_ips.iter().enumerate() {
        println!("#{}: {}", idx + 1, ip);
    }
    
    Ok(())
} 