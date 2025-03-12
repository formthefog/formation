use crate::geolocation::{GeoLocation, GeoResolver};
use std::net::IpAddr;
use std::path::Path;
use trust_dns_proto::rr::RecordType;
use log::{debug, error, info};

/// Add a new ProximityEntry struct for holding IP addresses with their geo attributes
#[derive(Debug, Clone)]
struct ProximityEntry {
    ip: IpAddr,
    distance: Option<f64>,
    region_code: Option<String>,
    country_code: Option<String>,
    score: f64,
}

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
    pub region_bias_factor: f64,    // Adjustment factor for same-region preference (0.0-1.0)
    pub max_distance_km: Option<f64>, // Maximum distance in km to consider (None = unlimited)
    pub distance_weights: DistanceWeightStrategy, // Strategy for weighting distances
}

/// Strategy for how distance affects node selection
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DistanceWeightStrategy {
    /// Linear - distance directly affects weight
    Linear,
    /// Quadratic - closer nodes are significantly favored
    Quadratic,
    /// Logarithmic - closer nodes have advantage but far nodes still considered
    Logarithmic,
    /// Step function - nodes within certain distance thresholds grouped together
    Stepped,
}

impl Default for GeoResolverConfig {
    fn default() -> Self {
        Self {
            db_path: "/etc/formation/geo/GeoLite2-City.mmdb".to_string(),
            enabled: true,
            prefer_same_region: true,
            max_unhealthy_score: 0.5,
            max_results: 3,
            region_bias_factor: 0.8,
            max_distance_km: None,
            distance_weights: DistanceWeightStrategy::Linear,
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
    
    /// Select and sort IPs by geographic proximity using configured strategy
    pub fn select_by_proximity(&self, 
                              client_location: &GeoLocation,
                              ips: Vec<IpAddr>,
                              limit: Option<usize>) -> Vec<IpAddr> {
        if ips.is_empty() || self.geo_resolver.is_none() {
            return ips;
        }
        
        let resolver = self.geo_resolver.as_ref().unwrap();
        
        // Step 1: Map IPs to ProximityEntry with geo data
        let mut entries: Vec<ProximityEntry> = Vec::with_capacity(ips.len());
        for ip in ips {
            let (distance, region_code, country_code) = match resolver.get_location(ip) {
                Ok(location) => {
                    let distance = crate::geolocation::calculate_distance(client_location, &location);
                    (Some(distance), location.region_code, location.country_code)
                },
                Err(_) => (None, None, None),
            };
            
            entries.push(ProximityEntry {
                ip,
                distance,
                region_code,
                country_code,
                score: 0.0, // Will be calculated in next step
            });
        }
        
        // Step 2: Calculate proximity scores based on distance and region
        for entry in &mut entries {
            entry.score = self.calculate_proximity_score(entry, client_location);
        }
        
        // Step 3: Sort by proximity score (highest first)
        entries.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        
        // Step 4: Apply limit if specified
        let limit = limit.unwrap_or(self.config.max_results);
        if entries.len() > limit {
            entries.truncate(limit);
        }
        
        // Return sorted IPs
        entries.into_iter().map(|e| e.ip).collect()
    }
    
    /// Calculate a proximity score for an IP based on distance, region, and configuration
    fn calculate_proximity_score(&self, entry: &ProximityEntry, client_location: &GeoLocation) -> f64 {
        // Base case: if no distance is available, return minimum score
        let distance = match entry.distance {
            Some(d) => d,
            None => return 0.01, // Very low but not zero to allow for fallback
        };
        
        // Check if the entry is within the maximum allowed distance
        if let Some(max_distance) = self.config.max_distance_km {
            if distance > max_distance {
                return 0.0; // Too far, exclude from results
            }
        }
        
        // Base score from distance using the configured strategy
        let distance_score = match self.config.distance_weights {
            DistanceWeightStrategy::Linear => {
                // Linear falloff, max score = 1.0 at 0 km
                // We use 20,000 km (approx half earth circumference) as the max distance reference
                let normalized = 1.0 - (distance / 20_000.0).min(1.0);
                normalized.max(0.0) // Ensure non-negative
            },
            DistanceWeightStrategy::Quadratic => {
                // Quadratic falloff - more aggressive preference for close locations
                let normalized = 1.0 - (distance / 20_000.0).min(1.0);
                normalized * normalized
            },
            DistanceWeightStrategy::Logarithmic => {
                // Logarithmic falloff - gentler falloff for distant locations
                if distance < 1.0 {
                    1.0 // Avoid log(0) issues
                } else {
                    1.0 - (distance.ln() / 10.0).min(1.0).max(0.0)
                }
            },
            DistanceWeightStrategy::Stepped => {
                // Step function - group into distance bands
                if distance < 100.0 { 
                    1.0 // Very close (same city)
                } else if distance < 500.0 { 
                    0.8 // Nearby (same region)
                } else if distance < 2000.0 { 
                    0.6 // Medium distance (same continent)
                } else if distance < 5000.0 { 
                    0.4 // Far (different continent)
                } else { 
                    0.2 // Very far
                }
            }
        };
        
        // Region bonus: Apply a bonus if in the same region or country
        let region_bonus = if self.config.prefer_same_region {
            let same_region = entry.region_code.is_some() && 
                             client_location.region_code.is_some() && 
                             entry.region_code == client_location.region_code;
                             
            let same_country = entry.country_code.is_some() && 
                              client_location.country_code.is_some() &&
                              entry.country_code == client_location.country_code;
            
            if same_region {
                self.config.region_bias_factor // Full bonus for same region
            } else if same_country {
                self.config.region_bias_factor * 0.5 // Partial bonus for same country
            } else {
                0.0
            }
        } else {
            0.0
        };
        
        // Combine distance score and region bonus, cap at 1.0
        let combined_score = distance_score * (1.0 - self.config.region_bias_factor) + region_bonus;
        combined_score.min(1.0)
    }
    
    /// Get a list of IPs sorted by proximity to the client using enhanced proximity scoring
    /// This replaces the simpler get_geo_sorted_ips but maintains the same interface
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
            debug!("Selecting and sorting {} IPs by proximity to client at {:?}", 
                  ips.len(), client_location);
            
            self.select_by_proximity(&client_location, ips, None)
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
    
    #[test]
    fn test_proximity_selection_strategies() {
        let resolver = get_test_resolver();
        
        if resolver.geo_resolver.is_none() {
            println!("Skipping test_proximity_selection_strategies: No MaxMind database available");
            return;
        }
        
        // Create test IPs (using real IPs that should be in MaxMind DB)
        let test_ips = vec![
            IpAddr::V4(Ipv4Addr::new(8, 8, 8, 8)),       // Google DNS (US)
            IpAddr::V4(Ipv4Addr::new(1, 1, 1, 1)),       // Cloudflare DNS (US)
            IpAddr::V4(Ipv4Addr::new(185, 228, 168, 9)), // CleanBrowsing (EU)
            IpAddr::V4(Ipv4Addr::new(208, 67, 222, 222)),// OpenDNS (US)
        ];
        
        // Set up London location as the client
        let london_ip = IpAddr::V4(Ipv4Addr::new(178, 62, 0, 1)); // DigitalOcean London
        let london_location = resolver.get_client_location(london_ip);
        if london_location.is_none() {
            println!("Could not get location for London test IP");
            return;
        }
        let london = london_location.unwrap();
        
        // Test each distance weight strategy
        let strategies = vec![
            DistanceWeightStrategy::Linear,
            DistanceWeightStrategy::Quadratic,
            DistanceWeightStrategy::Logarithmic,
            DistanceWeightStrategy::Stepped,
        ];
        
        for strategy in strategies {
            println!("Testing strategy: {:?}", strategy);
            
            // Create custom config with the specific strategy
            let mut config = GeoResolverConfig::default();
            config.distance_weights = strategy;
            config.db_path = resolver.config.db_path.clone();
            
            let strategy_resolver = GeoDnsResolver::new(config);
            if strategy_resolver.geo_resolver.is_none() {
                continue;
            }
            
            // Get sorted IPs
            let sorted_ips = strategy_resolver.select_by_proximity(&london, test_ips.clone(), None);
            
            // At minimum, verify we get results back
            assert!(!sorted_ips.is_empty());
            
            // Print results for inspection
            println!("Sorted results for {:?}:", strategy);
            for (idx, ip) in sorted_ips.iter().enumerate() {
                println!("  #{}: {}", idx + 1, ip);
            }
            
            // With London as client, European IPs should appear before US IPs
            // Find EU IP position
            let eu_ip = IpAddr::V4(Ipv4Addr::new(185, 228, 168, 9));
            if let Some(eu_pos) = sorted_ips.iter().position(|&ip| ip == eu_ip) {
                if eu_pos > 0 {
                    // For all strategies, EU IP should be either first or second
                    assert!(eu_pos <= 1, 
                            "EU IP should be first or second for London client, was position {}", 
                            eu_pos + 1);
                }
            }
        }
    }
    
    #[test]
    fn test_region_bias() {
        let resolver = get_test_resolver();
        
        if resolver.geo_resolver.is_none() {
            println!("Skipping test_region_bias: No MaxMind database available");
            return;
        }
        
        // Create test IPs (using real IPs that should be in MaxMind DB)
        let test_ips = vec![
            IpAddr::V4(Ipv4Addr::new(8, 8, 8, 8)),       // Google DNS (US)
            IpAddr::V4(Ipv4Addr::new(1, 1, 1, 1)),       // Cloudflare DNS (US)
            IpAddr::V4(Ipv4Addr::new(185, 228, 168, 9)), // CleanBrowsing (EU)
            IpAddr::V4(Ipv4Addr::new(208, 67, 222, 222)),// OpenDNS (US)
        ];
        
        // Set up London location as the client
        let london_ip = IpAddr::V4(Ipv4Addr::new(178, 62, 0, 1)); // DigitalOcean London
        let london_location = resolver.get_client_location(london_ip);
        if london_location.is_none() {
            println!("Could not get location for London test IP");
            return;
        }
        let london = london_location.unwrap();
        
        // Test with and without region bias
        let test_configs = vec![
            (true, 0.9),   // Strong region bias
            (true, 0.5),   // Moderate region bias
            (false, 0.0),  // No region bias
        ];
        
        for (prefer_region, bias_factor) in test_configs {
            println!("Testing region bias: prefer_region={}, bias_factor={}", prefer_region, bias_factor);
            
            // Create custom config with specific region bias settings
            let mut config = GeoResolverConfig::default();
            config.prefer_same_region = prefer_region;
            config.region_bias_factor = bias_factor;
            config.db_path = resolver.config.db_path.clone();
            
            let bias_resolver = GeoDnsResolver::new(config);
            if bias_resolver.geo_resolver.is_none() {
                continue;
            }
            
            // Get sorted IPs
            let sorted_ips = bias_resolver.select_by_proximity(&london, test_ips.clone(), None);
            
            // Print results for inspection
            println!("Results with region_bias={}, factor={}:", prefer_region, bias_factor);
            for (idx, ip) in sorted_ips.iter().enumerate() {
                println!("  #{}: {}", idx + 1, ip);
            }
            
            // With strong region bias, EU IP should be first for London client
            if prefer_region && bias_factor > 0.8 {
                let eu_ip = IpAddr::V4(Ipv4Addr::new(185, 228, 168, 9));
                if let Some(eu_pos) = sorted_ips.iter().position(|&ip| ip == eu_ip) {
                    assert_eq!(eu_pos, 0, "EU IP should be first with strong region bias");
                }
            }
        }
    }
} 