use maxminddb::geoip2;
use std::net::IpAddr;
use std::path::Path;
use std::sync::Arc;
use log::{debug, error, info, warn};

/// Represents a geographic location with latitude and longitude
#[derive(Debug, Clone)]
pub struct GeoLocation {
    pub latitude: f64,
    pub longitude: f64,
    pub country_code: Option<String>,
    pub region_code: Option<String>,
}

/// Error types for geolocation operations
#[derive(Debug, thiserror::Error)]
pub enum GeoLocationError {
    #[error("Database error: {0}")]
    Database(#[from] maxminddb::MaxMindDBError),
    
    #[error("Location data not found for IP")]
    LocationNotFound,
    
    #[error("Database file not found at: {0}")]
    DatabaseNotFound(String),
}

/// Resolver for IP geolocation using MaxMind database
pub struct GeoResolver {
    reader: Arc<maxminddb::Reader<Vec<u8>>>,
}

impl GeoResolver {
    /// Create a new GeoResolver with the specified database file
    pub fn new(db_path: &Path) -> Result<Self, GeoLocationError> {
        if !db_path.exists() {
            return Err(GeoLocationError::DatabaseNotFound(
                db_path.to_string_lossy().to_string()
            ));
        }
        
        let reader = maxminddb::Reader::open_readfile(db_path)
            .map_err(GeoLocationError::Database)?;
        
        info!("Loaded MaxMind database from: {}", db_path.display());
        
        Ok(Self {
            reader: Arc::new(reader),
        })
    }
    
    /// Get location information for an IP address
    pub fn get_location(&self, ip: IpAddr) -> Result<GeoLocation, GeoLocationError> {
        let city: geoip2::City = self.reader.lookup(ip)
            .map_err(|e| {
                debug!("MaxMind lookup failed for IP {}: {}", ip, e);
                GeoLocationError::Database(e)
            })?;
            
        let location = city.location.as_ref()
            .ok_or(GeoLocationError::LocationNotFound)?;
            
        let latitude = location.latitude
            .ok_or(GeoLocationError::LocationNotFound)?;
            
        let longitude = location.longitude
            .ok_or(GeoLocationError::LocationNotFound)?;
            
        let country_code = city.country
            .as_ref()
            .and_then(|c| c.iso_code)
            .map(|s| s.to_string());
            
        let region_code = city.subdivisions
            .as_ref()
            .and_then(|s| s.get(0))
            .and_then(|s| s.iso_code)
            .map(|s| s.to_string());
            
        Ok(GeoLocation {
            latitude,
            longitude,
            country_code,
            region_code,
        })
    }
    
    /// Find the nearest location from a list of candidates
    pub fn find_nearest(&self, client_location: &GeoLocation, candidate_ips: &[IpAddr]) 
        -> Vec<(IpAddr, Option<f64>)> 
    {
        let mut result: Vec<(IpAddr, Option<f64>)> = Vec::with_capacity(candidate_ips.len());
        
        for &ip in candidate_ips {
            let distance = match self.get_location(ip) {
                Ok(location) => {
                    Some(calculate_distance(client_location, &location))
                },
                Err(e) => {
                    warn!("Could not get location for IP {}: {}", ip, e);
                    None
                }
            };
            
            result.push((ip, distance));
        }
        
        // Sort by distance (None values at the end)
        result.sort_by(|a, b| {
            match (a.1, b.1) {
                (Some(dist_a), Some(dist_b)) => dist_a.partial_cmp(&dist_b).unwrap_or(std::cmp::Ordering::Equal),
                (Some(_), None) => std::cmp::Ordering::Less,
                (None, Some(_)) => std::cmp::Ordering::Greater,
                (None, None) => std::cmp::Ordering::Equal,
            }
        });
        
        result
    }
}

/// Calculate the distance between two geographic points using the Haversine formula
pub fn calculate_distance(loc1: &GeoLocation, loc2: &GeoLocation) -> f64 {
    const EARTH_RADIUS_KM: f64 = 6371.0;
    
    let lat1_rad = loc1.latitude.to_radians();
    let lat2_rad = loc2.latitude.to_radians();
    
    let delta_lat = (loc2.latitude - loc1.latitude).to_radians();
    let delta_lon = (loc2.longitude - loc1.longitude).to_radians();
    
    let a = (delta_lat / 2.0).sin().powi(2) + 
            lat1_rad.cos() * lat2_rad.cos() * (delta_lon / 2.0).sin().powi(2);
    let c = 2.0 * a.sqrt().atan2((1.0 - a).sqrt());
    
    EARTH_RADIUS_KM * c
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_calculate_distance() {
        // New York
        let new_york = GeoLocation {
            latitude: 40.7128,
            longitude: -74.0060,
            country_code: Some("US".to_string()),
            region_code: Some("NY".to_string()),
        };
        
        // Los Angeles
        let los_angeles = GeoLocation {
            latitude: 34.0522,
            longitude: -118.2437,
            country_code: Some("US".to_string()),
            region_code: Some("CA".to_string()),
        };
        
        let distance = calculate_distance(&new_york, &los_angeles);
        
        // Approximate distance between NY and LA is about 3,944 km
        assert!((distance - 3940.0).abs() < 50.0);
    }
} 