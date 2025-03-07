//! Relay discovery and registry
//!
//! This module handles finding, registering, and selecting relay nodes.

use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{Duration, SystemTime};

use crate::relay::{RelayNodeInfo, Result, RelayError};

/// Maximum age for relay information before it's considered stale
const MAX_RELAY_AGE: Duration = Duration::from_secs(3600); // 1 hour

/// Registry of known relay nodes
#[derive(Debug, Clone, Default)]
pub struct RelayRegistry {
    /// Map of relay public key to relay information
    relays: HashMap<String, RelayNodeInfo>,
    
    /// Map of relay public key to timestamp of last update
    last_updated: HashMap<String, SystemTime>,
}

impl RelayRegistry {
    /// Create a new empty relay registry
    pub fn new() -> Self {
        Self {
            relays: HashMap::new(),
            last_updated: HashMap::new(),
        }
    }
    
    /// Register a relay node with the registry
    pub fn register_relay(&mut self, relay: RelayNodeInfo) {
        let pubkey_hex = hex::encode(&relay.pubkey);
        self.relays.insert(pubkey_hex.clone(), relay);
        self.last_updated.insert(pubkey_hex, SystemTime::now());
    }
    
    /// Find relay nodes matching the specified criteria
    pub fn find_relays(&self, 
                      region: Option<&str>, 
                      min_capabilities: u32, 
                      max_count: usize) -> Vec<RelayNodeInfo> {
        let now = SystemTime::now();
        
        // Filter relays based on criteria
        let mut matching_relays: Vec<RelayNodeInfo> = self.relays.iter()
            .filter(|(pubkey, relay)| {
                // Check if the relay info is stale
                if let Some(last_update) = self.last_updated.get(*pubkey) {
                    if let Ok(age) = now.duration_since(*last_update) {
                        if age > MAX_RELAY_AGE {
                            return false; // Relay info is too old
                        }
                    }
                }
                
                // Check region if specified
                if let Some(region_filter) = region {
                    if let Some(relay_region) = &relay.region {
                        if !relay_region.eq_ignore_ascii_case(region_filter) {
                            return false;
                        }
                    } else {
                        // Relay has no region specified, skip if we're filtering by region
                        return false;
                    }
                }
                
                // Check capabilities
                if (relay.capabilities & min_capabilities) != min_capabilities {
                    return false;
                }
                
                true
            })
            .map(|(_, relay)| relay.clone())
            .collect();
            
        // Sort by load (ascending) then latency (ascending if available)
        matching_relays.sort_by(|a, b| {
            let load_cmp = a.load.cmp(&b.load);
            if load_cmp != std::cmp::Ordering::Equal {
                return load_cmp;
            }
            
            match (a.latency, b.latency) {
                (Some(a_latency), Some(b_latency)) => a_latency.cmp(&b_latency),
                (Some(_), None) => std::cmp::Ordering::Less,
                (None, Some(_)) => std::cmp::Ordering::Greater,
                (None, None) => std::cmp::Ordering::Equal,
            }
        });
        
        // Limit to max_count
        if matching_relays.len() > max_count {
            matching_relays.truncate(max_count);
        }
        
        matching_relays
    }
    
    /// Get a specific relay by its public key
    pub fn get_relay(&self, pubkey: &[u8]) -> Option<RelayNodeInfo> {
        let pubkey_hex = hex::encode(pubkey);
        self.relays.get(&pubkey_hex).cloned()
    }
    
    /// Remove stale relay nodes
    pub fn prune(&mut self) {
        let now = SystemTime::now();
        
        // Find stale relay keys
        let stale_keys: Vec<String> = self.last_updated.iter()
            .filter(|(_, last_update)| {
                if let Ok(age) = now.duration_since(**last_update) {
                    age > MAX_RELAY_AGE
                } else {
                    false
                }
            })
            .map(|(key, _)| key.clone())
            .collect();
            
        // Remove stale relays
        for key in stale_keys {
            self.relays.remove(&key);
            self.last_updated.remove(&key);
        }
    }
    
    /// Get the number of relays in the registry
    pub fn count(&self) -> usize {
        self.relays.len()
    }
    
    /// Update a relay's information
    pub fn update_relay(&mut self, pubkey: &[u8], update_fn: impl FnOnce(&mut RelayNodeInfo)) -> Result<()> {
        let pubkey_hex = hex::encode(pubkey);
        
        if let Some(relay) = self.relays.get_mut(&pubkey_hex) {
            update_fn(relay);
            self.last_updated.insert(pubkey_hex, SystemTime::now());
            Ok(())
        } else {
            Err(RelayError::Protocol(format!("Relay not found: {}", hex::encode(pubkey))))
        }
    }
}

/// A thread-safe relay registry that can be shared between threads
#[derive(Debug, Clone, Default)]
pub struct SharedRelayRegistry {
    inner: Arc<RwLock<RelayRegistry>>,
}

impl SharedRelayRegistry {
    /// Create a new empty shared relay registry
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(RelayRegistry::new())),
        }
    }
    
    /// Register a relay node with the registry
    pub fn register_relay(&self, relay: RelayNodeInfo) -> Result<()> {
        match self.inner.write() {
            Ok(mut registry) => {
                registry.register_relay(relay);
                Ok(())
            },
            Err(_) => Err(RelayError::Protocol("Failed to acquire write lock on relay registry".into())),
        }
    }
    
    /// Find relay nodes matching the specified criteria
    pub fn find_relays(&self, 
                     region: Option<&str>, 
                     min_capabilities: u32, 
                     max_count: usize) -> Result<Vec<RelayNodeInfo>> {
        match self.inner.read() {
            Ok(registry) => {
                Ok(registry.find_relays(region, min_capabilities, max_count))
            },
            Err(_) => Err(RelayError::Protocol("Failed to acquire read lock on relay registry".into())),
        }
    }
    
    /// Get a specific relay by its public key
    pub fn get_relay(&self, pubkey: &[u8]) -> Result<Option<RelayNodeInfo>> {
        match self.inner.read() {
            Ok(registry) => {
                Ok(registry.get_relay(pubkey))
            },
            Err(_) => Err(RelayError::Protocol("Failed to acquire read lock on relay registry".into())),
        }
    }
    
    /// Remove stale relay nodes
    pub fn prune(&self) -> Result<()> {
        match self.inner.write() {
            Ok(mut registry) => {
                registry.prune();
                Ok(())
            },
            Err(_) => Err(RelayError::Protocol("Failed to acquire write lock on relay registry".into())),
        }
    }
    
    /// Get the number of relays in the registry
    pub fn count(&self) -> Result<usize> {
        match self.inner.read() {
            Ok(registry) => {
                Ok(registry.count())
            },
            Err(_) => Err(RelayError::Protocol("Failed to acquire read lock on relay registry".into())),
        }
    }
    
    /// Update a relay's information
    pub fn update_relay(&self, pubkey: &[u8], update_fn: impl FnOnce(&mut RelayNodeInfo)) -> Result<()> {
        match self.inner.write() {
            Ok(mut registry) => {
                registry.update_relay(pubkey, update_fn)
            },
            Err(_) => Err(RelayError::Protocol("Failed to acquire write lock on relay registry".into())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::relay::{RELAY_CAP_IPV4, RELAY_CAP_IPV6};
    
    // Helper to create a test relay node
    fn create_test_relay(id: u8, endpoints: Vec<&str>, max_sessions: u32) -> RelayNodeInfo {
        let mut pubkey = [0u8; 32];
        pubkey[0] = id; // Simple way to create different pubkeys
        
        let endpoints = endpoints.into_iter().map(String::from).collect();
        
        RelayNodeInfo::new(pubkey, endpoints, max_sessions)
    }
    
    #[test]
    fn test_relay_registry_basic_operations() {
        let mut registry = RelayRegistry::new();
        
        // Initial state
        assert_eq!(registry.count(), 0);
        
        // Add a relay
        let relay1 = create_test_relay(1, vec!["192.168.1.1:8080"], 10);
        registry.register_relay(relay1.clone());
        
        // Verify it was added
        assert_eq!(registry.count(), 1);
        let retrieved = registry.get_relay(&[1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().pubkey[0], 1);
        
        // Add another relay
        let relay2 = create_test_relay(2, vec!["192.168.1.2:8080"], 20);
        registry.register_relay(relay2);
        
        // Verify count increased
        assert_eq!(registry.count(), 2);
        
        // Update a relay
        registry.update_relay(&[1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0], |relay| {
            relay.load = 50;
            relay.add_capability(RELAY_CAP_IPV6);
        }).unwrap();
        
        // Verify update was applied
        let updated = registry.get_relay(&[1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]).unwrap();
        assert_eq!(updated.load, 50);
        assert!(updated.has_capability(RELAY_CAP_IPV6));
    }
    
    #[test]
    fn test_relay_registry_find_relays() {
        let mut registry = RelayRegistry::new();
        
        // Add relays with different capabilities and regions
        let mut relay1 = create_test_relay(1, vec!["192.168.1.1:8080"], 10);
        relay1 = relay1.with_region("us-east");
        relay1.add_capability(RELAY_CAP_IPV6);
        registry.register_relay(relay1);
        
        let mut relay2 = create_test_relay(2, vec!["192.168.1.2:8080"], 20);
        relay2 = relay2.with_region("us-west");
        relay2.add_capability(RELAY_CAP_IPV6);
        relay2.load = 30; // Higher load
        registry.register_relay(relay2);
        
        let mut relay3 = create_test_relay(3, vec!["192.168.1.3:8080"], 30);
        relay3 = relay3.with_region("eu-central");
        registry.register_relay(relay3);
        
        // Find by region
        let us_relays = registry.find_relays(Some("us-west"), 0, 10);
        assert_eq!(us_relays.len(), 1);
        assert_eq!(us_relays[0].pubkey[0], 2);
        
        // Find by capability
        let ipv6_relays = registry.find_relays(None, RELAY_CAP_IPV6, 10);
        assert_eq!(ipv6_relays.len(), 2);
        
        // Find with multiple criteria and limit
        let limited_relays = registry.find_relays(None, RELAY_CAP_IPV4, 1);
        assert_eq!(limited_relays.len(), 1);
        
        // Sort by load (relay1 has lower load than relay2)
        let sorted_relays = registry.find_relays(None, RELAY_CAP_IPV6, 10);
        assert_eq!(sorted_relays.len(), 2);
        assert_eq!(sorted_relays[0].pubkey[0], 1); // Lower load should be first
    }
    
    #[test]
    fn test_relay_registry_pruning() {
        let mut registry = RelayRegistry::new();
        
        // Add a relay
        let relay = create_test_relay(1, vec!["192.168.1.1:8080"], 10);
        registry.register_relay(relay);
        assert_eq!(registry.count(), 1);
        
        // Manually set last_updated to be older than MAX_RELAY_AGE
        let pubkey_hex = hex::encode(&[1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        let old_time = SystemTime::now().checked_sub(Duration::from_secs(MAX_RELAY_AGE.as_secs() + 60)).unwrap();
        registry.last_updated.insert(pubkey_hex, old_time);
        
        // Prune should remove the stale relay
        registry.prune();
        assert_eq!(registry.count(), 0);
    }
    
    #[test]
    fn test_shared_relay_registry() {
        let registry = SharedRelayRegistry::new();
        
        // Add a relay
        let relay = create_test_relay(1, vec!["192.168.1.1:8080"], 10);
        registry.register_relay(relay).unwrap();
        
        // Verify it was added
        assert_eq!(registry.count().unwrap(), 1);
        let retrieved = registry.get_relay(&[1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]).unwrap();
        assert!(retrieved.is_some());
        
        // Find relays
        let relays = registry.find_relays(None, 0, 10).unwrap();
        assert_eq!(relays.len(), 1);
        
        // Update a relay
        registry.update_relay(&[1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0], |r| {
            r.load = 25;
        }).unwrap();
        
        // Verify update
        let updated = registry.get_relay(&[1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]).unwrap().unwrap();
        assert_eq!(updated.load, 25);
        
        // Prune
        registry.prune().unwrap();
    }
} 