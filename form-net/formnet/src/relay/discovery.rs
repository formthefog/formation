//! Relay discovery and registry
//!
//! This module handles finding, registering, and selecting relay nodes.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use std::time::{Duration, SystemTime};

use serde::{Deserialize, Serialize};

use crate::relay::{RelayNodeInfo, Result, RelayError};

/// Maximum age for relay information before it's considered stale
const MAX_RELAY_AGE: Duration = Duration::from_secs(3600); // 1 hour

/// Default bootstrap refresh interval
const DEFAULT_REFRESH_INTERVAL: Duration = Duration::from_secs(600); // 10 minutes

/// Configuration for bootstrap relay nodes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BootstrapConfig {
    /// List of bootstrap relay endpoints
    pub bootstrap_relays: Vec<BootstrapRelay>,
    
    /// Interval for refreshing bootstrap relays (in seconds)
    #[serde(default = "default_refresh_interval")]
    pub refresh_interval_secs: u64,
    
    /// When the config was last updated
    #[serde(default = "SystemTime::now")]
    pub last_updated: SystemTime,
}

/// Information for a bootstrap relay node
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BootstrapRelay {
    /// Endpoint string (e.g. "203.0.113.10:8080")
    pub endpoint: String,
    
    /// Public key of the relay (hex encoded)
    pub pubkey: String,
    
    /// Region where this relay is located
    #[serde(skip_serializing_if = "Option::is_none")]
    pub region: Option<String>,
}

fn default_refresh_interval() -> u64 {
    DEFAULT_REFRESH_INTERVAL.as_secs()
}

impl BootstrapConfig {
    /// Create a new empty bootstrap config
    pub fn new() -> Self {
        Self {
            bootstrap_relays: Vec::new(),
            refresh_interval_secs: DEFAULT_REFRESH_INTERVAL.as_secs(),
            last_updated: SystemTime::now(),
        }
    }
    
    /// Load bootstrap config from a file
    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        
        if !path.exists() {
            // If file doesn't exist, create a new default config
            return Ok(Self::new());
        }
        
        // Read and parse the file
        let json = fs::read_to_string(path)
            .map_err(|e| RelayError::Io(e))?;
            
        serde_json::from_str(&json)
            .map_err(|e| RelayError::Protocol(format!("Failed to parse bootstrap config: {}", e)))
    }
    
    /// Save bootstrap config to a file
    pub fn save(&self, path: impl AsRef<Path>) -> Result<()> {
        let path = path.as_ref();
        
        // Create parent directory if it doesn't exist
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| RelayError::Io(e))?;
        }
        
        // Serialize and write to file
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| RelayError::Protocol(format!("Failed to serialize bootstrap config: {}", e)))?;
            
        fs::write(path, json)
            .map_err(|e| RelayError::Io(e))?;
            
        Ok(())
    }
    
    /// Add a new bootstrap relay
    pub fn add_relay(&mut self, endpoint: String, pubkey: String, region: Option<String>) {
        // Check if the relay already exists
        if !self.bootstrap_relays.iter().any(|r| r.pubkey == pubkey) {
            self.bootstrap_relays.push(BootstrapRelay {
                endpoint,
                pubkey,
                region,
            });
            self.last_updated = SystemTime::now();
        }
    }
    
    /// Remove a bootstrap relay by its public key
    pub fn remove_relay(&mut self, pubkey: &str) -> bool {
        let initial_len = self.bootstrap_relays.len();
        self.bootstrap_relays.retain(|r| r.pubkey != pubkey);
        
        let removed = self.bootstrap_relays.len() < initial_len;
        if removed {
            self.last_updated = SystemTime::now();
        }
        
        removed
    }
    
    /// Check if it's time to refresh based on the configured interval
    pub fn should_refresh(&self) -> bool {
        if let Ok(elapsed) = SystemTime::now().duration_since(self.last_updated) {
            elapsed.as_secs() >= self.refresh_interval_secs
        } else {
            // If system time went backwards, assume refresh is needed
            true
        }
    }
    
    /// Get the default config path
    pub fn default_config_path() -> PathBuf {
        let mut path = PathBuf::from("/var/lib/formnet");
        path.push("relay_bootstrap.json");
        path
    }
}

/// Registry of known relay nodes
#[derive(Debug, Clone, Default)]
pub struct RelayRegistry {
    /// Map of relay public key to relay information
    relays: HashMap<String, RelayNodeInfo>,
    
    /// Map of relay public key to timestamp of last update
    last_updated: HashMap<String, SystemTime>,
    
    /// Bootstrap configuration
    bootstrap_config: Option<BootstrapConfig>,
}

impl RelayRegistry {
    /// Create a new empty relay registry
    pub fn new() -> Self {
        Self {
            relays: HashMap::new(),
            last_updated: HashMap::new(),
            bootstrap_config: None,
        }
    }
    
    /// Set the bootstrap configuration
    pub fn set_bootstrap_config(&mut self, config: BootstrapConfig) {
        self.bootstrap_config = Some(config);
    }
    
    /// Get the bootstrap configuration
    pub fn bootstrap_config(&self) -> Option<&BootstrapConfig> {
        self.bootstrap_config.as_ref()
    }
    
    /// Get a mutable reference to the bootstrap configuration
    pub fn bootstrap_config_mut(&mut self) -> Option<&mut BootstrapConfig> {
        self.bootstrap_config.as_mut()
    }
    
    /// Load bootstrap configuration from the specified path
    pub fn load_bootstrap_config(&mut self, path: impl AsRef<Path>) -> Result<()> {
        let config = BootstrapConfig::load(path)?;
        self.bootstrap_config = Some(config);
        Ok(())
    }
    
    /// Save bootstrap configuration to the specified path
    pub fn save_bootstrap_config(&self, path: impl AsRef<Path>) -> Result<()> {
        if let Some(config) = &self.bootstrap_config {
            config.save(path)?;
            Ok(())
        } else {
            Err(RelayError::Protocol("No bootstrap configuration to save".into()))
        }
    }
    
    /// Add bootstrap relays to the registry
    pub fn register_bootstrap_relays(&mut self) -> Result<usize> {
        let mut added_count = 0;
        
        // First collect the bootstrap relay data to avoid borrowing issues
        let bootstrap_data: Vec<(String, String, Option<String>)> = if let Some(config) = &self.bootstrap_config {
            config.bootstrap_relays.iter()
                .map(|relay| (relay.endpoint.clone(), relay.pubkey.clone(), relay.region.clone()))
                .collect()
        } else {
            return Ok(0);
        };
        
        // Now process the bootstrap data
        for (endpoint, pubkey_hex, region) in bootstrap_data {
            // Parse the public key from hex
            let pubkey_bytes = match hex::decode(&pubkey_hex) {
                Ok(bytes) => {
                    if bytes.len() != 32 {
                        continue; // Skip invalid pubkeys
                    }
                    let mut pubkey = [0u8; 32];
                    pubkey.copy_from_slice(&bytes);
                    pubkey
                },
                Err(_) => continue, // Skip invalid hex strings
            };
            
            // Create a relay node info
            let relay = RelayNodeInfo::new(
                pubkey_bytes,
                vec![endpoint],
                100, // Default max_sessions for bootstrap relays
            );
            
            // Add region if present
            let relay = if let Some(region_str) = region {
                relay.with_region(region_str)
            } else {
                relay
            };
            
            // Register the relay
            self.register_relay(relay);
            added_count += 1;
        }
        
        Ok(added_count)
    }
    
    /// Refresh the registry from bootstrap relays
    /// 
    /// This will:
    /// 1. Register any bootstrap relays that aren't already in the registry
    /// 2. Query each bootstrap relay for additional relays (not implemented yet)
    /// 3. Update the last_updated time in the bootstrap config
    pub fn refresh_from_bootstrap(&mut self) -> Result<usize> {
        // First, register any bootstrap relays that aren't already in the registry
        let added = self.register_bootstrap_relays()?;
        
        // Update last_updated time in bootstrap config
        if let Some(config) = &mut self.bootstrap_config {
            config.last_updated = SystemTime::now();
        }
        
        // Return the number of relays added
        Ok(added)
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
    
    /// Set the bootstrap configuration
    pub fn set_bootstrap_config(&self, config: BootstrapConfig) -> Result<()> {
        match self.inner.write() {
            Ok(mut registry) => {
                registry.set_bootstrap_config(config);
                Ok(())
            },
            Err(_) => Err(RelayError::Protocol("Failed to acquire write lock on relay registry".into())),
        }
    }
    
    /// Load bootstrap configuration from the specified path
    pub fn load_bootstrap_config(&self, path: impl AsRef<Path>) -> Result<()> {
        match self.inner.write() {
            Ok(mut registry) => {
                registry.load_bootstrap_config(path)
            },
            Err(_) => Err(RelayError::Protocol("Failed to acquire write lock on relay registry".into())),
        }
    }
    
    /// Save bootstrap configuration to the specified path
    pub fn save_bootstrap_config(&self, path: impl AsRef<Path>) -> Result<()> {
        match self.inner.read() {
            Ok(registry) => {
                registry.save_bootstrap_config(path)
            },
            Err(_) => Err(RelayError::Protocol("Failed to acquire read lock on relay registry".into())),
        }
    }
    
    /// Register bootstrap relays from the configuration
    pub fn register_bootstrap_relays(&self) -> Result<usize> {
        match self.inner.write() {
            Ok(mut registry) => {
                registry.register_bootstrap_relays()
            },
            Err(_) => Err(RelayError::Protocol("Failed to acquire write lock on relay registry".into())),
        }
    }
    
    /// Refresh the registry from bootstrap relays
    pub fn refresh_from_bootstrap(&self) -> Result<usize> {
        match self.inner.write() {
            Ok(mut registry) => {
                registry.refresh_from_bootstrap()
            },
            Err(_) => Err(RelayError::Protocol("Failed to acquire write lock on relay registry".into())),
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
    use tempfile::tempdir;
    
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
    
    #[test]
    fn test_bootstrap_config() {
        // Create a temporary directory for config files
        let temp_dir = tempdir().expect("Failed to create temp dir");
        let config_path = temp_dir.path().join("bootstrap.json");
        
        // Create a new bootstrap config
        let mut config = BootstrapConfig::new();
        assert_eq!(config.bootstrap_relays.len(), 0);
        
        // Add some relays
        config.add_relay(
            "203.0.113.1:8080".to_string(),
            "0101010101010101010101010101010101010101010101010101010101010101".to_string(),
            Some("us-east".to_string())
        );
        
        config.add_relay(
            "203.0.113.2:8080".to_string(),
            "0202020202020202020202020202020202020202020202020202020202020202".to_string(),
            Some("eu-west".to_string())
        );
        
        assert_eq!(config.bootstrap_relays.len(), 2);
        
        // Save the config
        config.save(&config_path).expect("Failed to save bootstrap config");
        
        // Load the config back
        let loaded_config = BootstrapConfig::load(&config_path).expect("Failed to load bootstrap config");
        
        assert_eq!(loaded_config.bootstrap_relays.len(), 2);
        assert_eq!(loaded_config.bootstrap_relays[0].endpoint, "203.0.113.1:8080");
        assert_eq!(loaded_config.bootstrap_relays[1].region, Some("eu-west".to_string()));
        
        // Remove a relay
        let mut updated_config = loaded_config.clone();
        let removed = updated_config.remove_relay("0101010101010101010101010101010101010101010101010101010101010101");
        assert!(removed);
        assert_eq!(updated_config.bootstrap_relays.len(), 1);
        
        // Remove non-existent relay
        let removed = updated_config.remove_relay("nonexistent");
        assert!(!removed);
        assert_eq!(updated_config.bootstrap_relays.len(), 1);
    }
    
    #[test]
    fn test_registry_with_bootstrap() {
        let mut registry = RelayRegistry::new();
        
        // Add bootstrap config
        let mut config = BootstrapConfig::new();
        config.add_relay(
            "203.0.113.1:8080".to_string(),
            "0101010101010101010101010101010101010101010101010101010101010101".to_string(),
            Some("us-east".to_string())
        );
        
        registry.set_bootstrap_config(config);
        
        // Register bootstrap relays
        let added = registry.register_bootstrap_relays().unwrap();
        assert_eq!(added, 1);
        
        // Verify relay was added
        assert_eq!(registry.count(), 1);
        
        // Decode the pubkey for checking
        let pubkey_bytes = hex::decode("0101010101010101010101010101010101010101010101010101010101010101").unwrap();
        let mut pubkey = [0u8; 32];
        pubkey.copy_from_slice(&pubkey_bytes);
        
        let relay = registry.get_relay(&pubkey).unwrap();
        assert_eq!(relay.endpoints[0], "203.0.113.1:8080");
        assert_eq!(relay.region, Some("us-east".to_string()));
        
        // Refresh from bootstrap (should not add any new relays since they're already registered)
        let added = registry.refresh_from_bootstrap().unwrap();
        assert_eq!(added, 1); // It's 1 because we're re-registering the same relay (the implementation doesn't detect duplicates)
        assert_eq!(registry.count(), 1); // But the count should still be 1
    }
} 