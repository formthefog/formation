use crate::geolocation::{GeoLocation, GeoResolver};
use std::net::IpAddr;
use std::path::Path;
use trust_dns_proto::rr::RecordType;
use log::{debug, error, info};

/// Represents a healthy server with its weight and location information
#[derive(Debug, Clone)]
pub struct GeoNode {
    pub ip: IpAddr,
    pub health_score: f64,  // 0.0 to 1.0, where 1.0 is perfectly healthy
    pub location: Option<GeoLocation>,
    pub region: Option<String>,
}

/// Configuration for geo-based DNS resolution
#[derive(Debug, Clone)]
pub struct GeoResolverConfig {
    pub db_path: String,
    pub enabled: bool,
    pub prefer_same_region: bool,
    pub max_unhealthy_score: f64,  // Servers with health scores below this are excluded
    pub max_results: usize,         // Maximum number of results to return
}

impl Default for GeoResolverConfig {
    fn default() -> Self {
        Self {
            db_path: "/etc/formation/geo/GeoLite2-City.mmdb".to_string(),
            enabled: true,
            prefer_same_region: true,
            max_unhealthy_score: 0.5,
            max_results: 3,
        }
    }
}

/// GeoDnsResolver wraps a GeoResolver with DNS-specific functionality
pub struct GeoDnsResolver {
    geo_resolver: Option<GeoResolver>,
    config: GeoResolverConfig,
}

impl GeoDnsResolver {
    /// Create a new GeoResolver
    pub fn new(config: GeoResolverConfig) -> Self {
        let geo_resolver = if config.enabled {
            match GeoResolver::new(Path::new(&config.db_path)) {
                Ok(resolver) => {
                    info!("GeoResolver initialized successfully");
                    Some(resolver)
                },
                Err(e) => {
                    error!("Failed to initialize GeoResolver: {}", e);
                    None
                }
            }
        } else {
            info!("GeoResolver disabled in configuration");
            None
        };
        
        Self {
            geo_resolver,
            config,
        }
    }
    
    /// Get client location from IP address
    pub fn get_client_location(&self, client_ip: IpAddr) -> Option<GeoLocation> {
        if let Some(resolver) = &self.geo_resolver {
            match resolver.get_location(client_ip) {
                Ok(location) => {
                    debug!("Client location resolved: {:?} for IP {}", location, client_ip);
                    return Some(location);
                },
                Err(e) => {
                    debug!("Could not resolve client location for IP {}: {}", client_ip, e);
                }
            }
        }
        None
    }
    
    /// Sort IPs by proximity to client location
    pub fn sort_ips_by_proximity(&self, 
                              client_location: &GeoLocation,
                              ips: Vec<IpAddr>) -> Vec<IpAddr> {
        if let Some(resolver) = &self.geo_resolver {
            let sorted = resolver.find_nearest(client_location, &ips);
            return sorted.into_iter().map(|(ip, _)| ip).collect();
        }
        ips
    }
    
    /// Get a list of IPs sorted by proximity to the client
    /// This function can be called from DNS resolution logic
    pub fn get_geo_sorted_ips(&self, 
                           _domain: &str,
                           _record_type: RecordType, 
                           client_ip: Option<IpAddr>,
                           ips: Vec<IpAddr>) -> Vec<IpAddr> {
        // If no client IP or no IPs to sort, return the original list
        if ips.is_empty() || client_ip.is_none() {
            return ips;
        }
        
        let client_ip = client_ip.unwrap();
        
        // Get client location
        if let Some(client_location) = self.get_client_location(client_ip) {
            debug!("Sorting {} IPs by proximity to client at {:?}", ips.len(), client_location);
            self.sort_ips_by_proximity(&client_location, ips)
        } else {
            debug!("Client location could not be determined for {}, returning unsorted IPs", client_ip);
            ips
        }
    }
}

/// Factory function to add GeoResolving capabilities to the DNS resolution
pub fn create_geo_resolver(config: GeoResolverConfig) -> GeoDnsResolver {
    GeoDnsResolver::new(config)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{IpAddr, Ipv4Addr};
    
    // These tests will be skipped unless a database file is present
    fn get_test_resolver() -> GeoDnsResolver {
        let test_paths = vec![
            "./GeoLite2-City.mmdb".to_string(),
            "/etc/formation/geo/GeoLite2-City.mmdb".to_string(),
        ];
        
        for path in test_paths {
            let config = GeoResolverConfig {
                db_path: path,
                enabled: true,
                ..GeoResolverConfig::default()
            };
            
            let resolver = GeoDnsResolver::new(config);
            if resolver.geo_resolver.is_some() {
                return resolver;
            }
        }
        
        // Return a disabled resolver if no database is found
        GeoDnsResolver::new(GeoResolverConfig {
            enabled: false,
            ..GeoResolverConfig::default()
        })
    }
    
    #[test]
    fn test_get_client_location() {
        let resolver = get_test_resolver();
        
        if resolver.geo_resolver.is_none() {
            println!("Skipping test_get_client_location: No MaxMind database available");
            return;
        }
        
        // This is a Google DNS IP, it should resolve to a US location
        let google_dns = IpAddr::V4(Ipv4Addr::new(8, 8, 8, 8));
        
        if let Some(location) = resolver.get_client_location(google_dns) {
            assert_eq!(location.country_code.as_deref(), Some("US"));
        }
    }
} 