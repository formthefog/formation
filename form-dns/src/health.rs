use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::RwLock;
use log::{debug, info, warn};

/// Simple health status - binary available/unavailable
#[derive(Debug, Clone)]
pub enum IpHealthStatus {
    Available,
    Unavailable { since: SystemTime, reason: String },
}

/// Repository for tracking IP health status
#[derive(Debug)]
pub struct IpHealthRepository {
    /// Map of IP addresses to their health status
    health_map: HashMap<IpAddr, IpHealthStatus>,
    /// Maximum time since last heartbeat before a node is considered unhealthy
    heartbeat_timeout: Duration,
}

impl IpHealthRepository {
    pub fn new(heartbeat_timeout: Duration) -> Self {
        Self {
            health_map: HashMap::new(),
            heartbeat_timeout,
        }
    }

    /// Mark an IP as available
    pub fn mark_available(&mut self, ip: IpAddr) {
        debug!("Marking IP {} as available", ip);
        self.health_map.insert(ip, IpHealthStatus::Available);
    }

    /// Mark an IP as unavailable with a reason
    pub fn mark_unavailable(&mut self, ip: IpAddr, reason: String) {
        warn!("Marking IP {} as unavailable: {}", ip, reason);
        self.health_map.insert(ip, IpHealthStatus::Unavailable { 
            since: SystemTime::now(), 
            reason,
        });
    }

    /// Check if an IP is available
    pub fn is_available(&self, ip: &IpAddr) -> bool {
        match self.health_map.get(ip) {
            Some(IpHealthStatus::Available) => true,
            Some(IpHealthStatus::Unavailable { .. }) => false,
            None => true, // By default, consider unknown IPs as available
        }
    }

    /// Get detailed health status for an IP
    pub fn get_status(&self, ip: &IpAddr) -> Option<&IpHealthStatus> {
        self.health_map.get(ip)
    }

    /// Filter a list of IPs, returning only those that are available
    pub fn filter_available_ips(&self, ips: &[IpAddr]) -> Vec<IpAddr> {
        let filtered = ips.iter()
            .filter(|ip| self.is_available(ip))
            .cloned()
            .collect::<Vec<IpAddr>>();
        
        if filtered.len() < ips.len() {
            debug!(
                "Filtered out {} unhealthy IPs, {} remaining", 
                ips.len() - filtered.len(), 
                filtered.len()
            );
        }
        
        filtered
    }

    /// Get all unavailable IPs
    pub fn get_unavailable_ips(&self) -> Vec<(IpAddr, &IpHealthStatus)> {
        self.health_map
            .iter()
            .filter_map(|(ip, status)| {
                match status {
                    IpHealthStatus::Unavailable { .. } => Some((*ip, status)),
                    _ => None,
                }
            })
            .collect()
    }

    /// Clear unavailable IPs that have been in that state longer than the specified duration
    pub fn clear_stale_unavailable(&mut self, stale_after: Duration) {
        let now = SystemTime::now();
        let to_clear = self.health_map
            .iter()
            .filter_map(|(ip, status)| {
                match status {
                    IpHealthStatus::Unavailable { since, .. } => {
                        match now.duration_since(*since) {
                            Ok(duration) if duration > stale_after => Some(*ip),
                            _ => None,
                        }
                    },
                    _ => None,
                }
            })
            .collect::<Vec<IpAddr>>();
        
        for ip in to_clear {
            info!("Clearing stale unavailability status for IP {}", ip);
            self.health_map.remove(&ip);
        }
    }
}

/// Global shared health repository
pub type SharedIpHealthRepository = Arc<RwLock<IpHealthRepository>>;

/// Create a new shared IP health repository
pub fn create_shared_repository(heartbeat_timeout: Duration) -> SharedIpHealthRepository {
    Arc::new(RwLock::new(IpHealthRepository::new(heartbeat_timeout)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::Ipv4Addr;
    use std::thread;

    #[test]
    fn test_mark_available() {
        let mut repo = IpHealthRepository::new(Duration::from_secs(60));
        let ip = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1));
        
        repo.mark_available(ip);
        assert!(repo.is_available(&ip));
    }

    #[test]
    fn test_mark_unavailable() {
        let mut repo = IpHealthRepository::new(Duration::from_secs(60));
        let ip = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 2));
        
        repo.mark_unavailable(ip, "Test reason".to_string());
        assert!(!repo.is_available(&ip));
    }

    #[test]
    fn test_filter_available_ips() {
        let mut repo = IpHealthRepository::new(Duration::from_secs(60));
        let ip1 = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1));
        let ip2 = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 2));
        let ip3 = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 3));
        
        repo.mark_available(ip1);
        repo.mark_unavailable(ip2, "Test reason".to_string());
        // ip3 is unknown, should be considered available by default
        
        let filtered = repo.filter_available_ips(&[ip1, ip2, ip3]);
        assert_eq!(filtered, vec![ip1, ip3]);
    }

    #[test]
    fn test_clear_stale_unavailable() {
        let mut repo = IpHealthRepository::new(Duration::from_secs(60));
        let ip = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 2));
        
        repo.mark_unavailable(ip, "Test reason".to_string());
        assert!(!repo.is_available(&ip));
        
        // Sleep for a short time, then try to clear with a longer stale time
        thread::sleep(Duration::from_millis(10));
        repo.clear_stale_unavailable(Duration::from_secs(1));
        // Should still be unavailable
        assert!(!repo.is_available(&ip));
        
        // Now clear with a very short stale time
        repo.clear_stale_unavailable(Duration::from_millis(5));
        // Should be removed from the map and therefore available
        assert!(repo.is_available(&ip));
    }
    
    #[test]
    fn test_comprehensive_functionality() {
        // Create repository
        let mut repo = IpHealthRepository::new(Duration::from_secs(30));
        
        // Define test IPs
        let healthy_ip1 = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1));
        let healthy_ip2 = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 2));
        let unhealthy_ip1 = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 3));
        let unhealthy_ip2 = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 4));
        let unknown_ip = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 5));
        
        // Initially all IPs should be considered available (default behavior)
        assert!(repo.is_available(&healthy_ip1));
        assert!(repo.is_available(&unhealthy_ip1));
        assert!(repo.is_available(&unknown_ip));
        
        // Mark some IPs as explicitly available/unavailable
        repo.mark_available(healthy_ip1);
        repo.mark_available(healthy_ip2);
        repo.mark_unavailable(unhealthy_ip1, "Testing unavailable status".to_string());
        repo.mark_unavailable(unhealthy_ip2, "Another reason".to_string());
        
        // Check individual status
        assert!(repo.is_available(&healthy_ip1));
        assert!(repo.is_available(&healthy_ip2));
        assert!(!repo.is_available(&unhealthy_ip1));
        assert!(!repo.is_available(&unhealthy_ip2));
        assert!(repo.is_available(&unknown_ip));  // Unknown IPs are available by default
        
        // Test detailed status retrieval
        match repo.get_status(&unhealthy_ip1) {
            Some(IpHealthStatus::Unavailable { reason, .. }) => {
                assert_eq!(reason, "Testing unavailable status");
            },
            _ => panic!("Expected unavailable status for unhealthy_ip1"),
        }
        
        // Test filtering
        let all_ips = vec![healthy_ip1, healthy_ip2, unhealthy_ip1, unhealthy_ip2, unknown_ip];
        let filtered = repo.filter_available_ips(&all_ips);
        
        // Should include healthy and unknown IPs (3 total)
        assert_eq!(filtered.len(), 3);
        assert!(filtered.contains(&healthy_ip1));
        assert!(filtered.contains(&healthy_ip2));
        assert!(filtered.contains(&unknown_ip));
        assert!(!filtered.contains(&unhealthy_ip1));
        assert!(!filtered.contains(&unhealthy_ip2));
        
        // Test retrieving unavailable IPs
        let unavailable = repo.get_unavailable_ips();
        assert_eq!(unavailable.len(), 2);
        
        // Test clearing stale entries
        thread::sleep(Duration::from_millis(10));
        repo.clear_stale_unavailable(Duration::from_millis(5));
        
        // All unavailable IPs should now be cleared (and therefore available)
        assert!(repo.is_available(&unhealthy_ip1));
        assert!(repo.is_available(&unhealthy_ip2));
        
        // Make unhealthy_ip1 unavailable again
        repo.mark_unavailable(unhealthy_ip1, "New reason".to_string());
        assert!(!repo.is_available(&unhealthy_ip1));
        
        // Mark previously unavailable IP as available
        repo.mark_available(unhealthy_ip1);
        assert!(repo.is_available(&unhealthy_ip1));
    }
} 