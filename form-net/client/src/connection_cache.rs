use std::{
    collections::HashMap,
    fs::OpenOptions,
    path::{Path, PathBuf},
    time::{Duration, SystemTime},
};

use anyhow::Result;
use serde::{Deserialize, Serialize};
use shared::{chmod, ensure_dirs_exist, Endpoint, IoErrorContext, WrappedIoError};
use wireguard_control::InterfaceName;

/// Maximum age of a cached endpoint (7 days)
const MAX_CACHE_AGE: Duration = Duration::from_secs(7 * 24 * 60 * 60);

/// Struct representing a successful connection to a peer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionEntry {
    /// The endpoint that was successfully connected to
    pub endpoint: Endpoint,
    /// When the successful connection was established
    pub last_success: SystemTime,
    /// Number of successful connections through this endpoint
    pub success_count: u32,
}

/// Manages cached information about successful peer connections
#[derive(Debug, Serialize, Deserialize)]
pub struct ConnectionCache {
    /// Maps peer public keys to their successful connection entries
    cache: HashMap<String, Vec<ConnectionEntry>>,
}

impl ConnectionCache {
    /// Creates a new empty connection cache
    pub fn new() -> Self {
        Self {
            cache: HashMap::new(),
        }
    }

    /// Opens an existing cache file or creates a new one if it doesn't exist
    pub fn open_or_create(data_dir: &Path, interface: &InterfaceName) -> Result<Self, WrappedIoError> {
        ensure_dirs_exist(&[data_dir])?;
        let path = Self::get_path(data_dir, interface);
        
        if path.exists() {
            let mut file = OpenOptions::new()
                .read(true)
                .open(&path)
                .with_path(&path)?;
                
            shared::warn_on_dangerous_mode(&path).with_path(&path)?;
            
            let mut json = String::new();
            std::io::Read::read_to_string(&mut file, &mut json).with_path(&path)?;
            
            match serde_json::from_str(&json) {
                Ok(cache) => Ok(cache),
                Err(_) => {
                    log::warn!("Could not parse connection cache, creating new one");
                    Ok(Self::new())
                }
            }
        } else {
            Ok(Self::new())
        }
    }

    /// Get the path to the cache file
    pub fn get_path(data_dir: &Path, interface: &InterfaceName) -> PathBuf {
        data_dir
            .join(interface.to_string())
            .with_extension("connection-cache.json")
    }

    /// Record a successful connection to a peer
    pub fn record_success(&mut self, public_key: &str, endpoint: Endpoint) {
        let now = SystemTime::now();
        
        let entries = self.cache.entry(public_key.to_string()).or_insert_with(Vec::new);
        
        // Check if we already have this endpoint
        if let Some(entry) = entries.iter_mut().find(|e| e.endpoint == endpoint) {
            // Update the existing entry
            entry.last_success = now;
            entry.success_count += 1;
        } else {
            // Add a new entry
            entries.push(ConnectionEntry {
                endpoint,
                last_success: now,
                success_count: 1,
            });
        }
        
        // Prune old entries
        self.prune();
    }

    /// Clean up old entries from the cache
    pub fn prune(&mut self) {
        let now = SystemTime::now();
        
        for entries in self.cache.values_mut() {
            // Remove entries older than MAX_CACHE_AGE
            entries.retain(|entry| {
                match now.duration_since(entry.last_success) {
                    Ok(age) => age < MAX_CACHE_AGE,
                    Err(_) => {
                        // Clock went backwards, keep the entry to be safe
                        true
                    }
                }
            });
            
            // Sort entries by success count (descending) and then by recency (newest first)
            entries.sort_by(|a, b| {
                b.success_count
                    .cmp(&a.success_count)
                    .then_with(|| b.last_success.cmp(&a.last_success))
            });
            
            // Keep only the top 5 entries per peer to avoid unbounded growth
            if entries.len() > 5 {
                entries.truncate(5);
            }
        }
        
        // Remove peers with no entries
        self.cache.retain(|_, entries| !entries.is_empty());
    }

    /// Get the best endpoints for a peer (ordered by likelihood of success)
    pub fn get_best_endpoints(&self, public_key: &str) -> Vec<Endpoint> {
        match self.cache.get(public_key) {
            Some(entries) => entries.iter().map(|e| e.endpoint.clone()).collect(),
            None => Vec::new(),
        }
    }

    /// Prioritize a list of candidate endpoints based on connection history
    pub fn prioritize_endpoints(&self, public_key: &str, mut candidates: Vec<Endpoint>) -> Vec<Endpoint> {
        // Get the best known endpoints for this peer
        let best_endpoints = self.get_best_endpoints(public_key);
        
        // If we have no history for this peer, return the original candidates
        if best_endpoints.is_empty() {
            return candidates;
        }
        
        // Since Endpoint doesn't implement Hash, we can't use a HashSet
        // Instead, we'll use a Vec and check for membership explicitly
        
        // Start with known good endpoints that are in the candidate list
        let mut result: Vec<_> = best_endpoints
            .into_iter()
            .filter(|e| candidates.iter().any(|c| c == e))
            .collect();
            
        // Add any remaining candidates that weren't in our best endpoints list
        for endpoint in candidates {
            if !result.contains(&endpoint) {
                result.push(endpoint);
            }
        }
        
        result
    }

    /// Save the cache to disk
    pub fn save(&self, data_dir: &Path, interface: &InterfaceName) -> Result<(), WrappedIoError> {
        let path = Self::get_path(data_dir, interface);
        
        let mut file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&path)
            .with_path(&path)?;
            
        chmod(&file, 0o600).with_path(&path)?;
        
        let json = serde_json::to_string_pretty(&self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))
            .with_path(&path)?;
            
        std::io::Write::write_all(&mut file, json.as_bytes()).with_path(&path)?;
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    fn create_test_endpoint(host: &str, port: u16) -> Endpoint {
        Endpoint::from_str(&format!("{}:{}", host, port)).unwrap()
    }

    #[test]
    fn test_record_and_get_best_endpoints() {
        let mut cache = ConnectionCache::new();
        let pubkey = "test_key";
        let endpoint1 = create_test_endpoint("192.168.1.1", 51820);
        let endpoint2 = create_test_endpoint("10.0.0.1", 51820);

        // Record some successes
        cache.record_success(pubkey, endpoint1.clone());
        cache.record_success(pubkey, endpoint1.clone()); // Second success for endpoint1
        cache.record_success(pubkey, endpoint2.clone());

        // Get the best endpoints
        let best = cache.get_best_endpoints(pubkey);
        
        // Endpoint1 should be first (2 successes vs 1)
        assert_eq!(best.len(), 2);
        assert_eq!(best[0], endpoint1);
        assert_eq!(best[1], endpoint2);
    }

    #[test]
    fn test_prioritize_endpoints() {
        let mut cache = ConnectionCache::new();
        let pubkey = "test_key";
        let endpoint1 = create_test_endpoint("192.168.1.1", 51820);
        let endpoint2 = create_test_endpoint("10.0.0.1", 51820);
        let endpoint3 = create_test_endpoint("172.16.0.1", 51820);

        // Record some successes
        cache.record_success(pubkey, endpoint1.clone());
        cache.record_success(pubkey, endpoint3.clone());
        cache.record_success(pubkey, endpoint3.clone());

        // Create a list of candidates (note endpoint2 hasn't been seen before)
        let candidates = vec![endpoint2.clone(), endpoint1.clone(), endpoint3.clone()];
        
        // Prioritize the candidates
        let prioritized = cache.prioritize_endpoints(pubkey, candidates);
        
        // endpoint3 should be first (2 successes), then endpoint1 (1 success), then endpoint2 (0 successes)
        assert_eq!(prioritized.len(), 3);
        assert_eq!(prioritized[0], endpoint3);
        assert_eq!(prioritized[1], endpoint1);
        assert_eq!(prioritized[2], endpoint2);
    }

    #[test]
    fn test_prune_old_entries() {
        let mut cache = ConnectionCache::new();
        let pubkey = "test_key";
        let endpoint = create_test_endpoint("192.168.1.1", 51820);

        // Add an old entry (manually setting the timestamp)
        let old_time = SystemTime::now() - Duration::from_secs(8 * 24 * 60 * 60); // 8 days ago
        let entries = cache.cache.entry(pubkey.to_string()).or_insert_with(Vec::new);
        entries.push(ConnectionEntry {
            endpoint: endpoint.clone(),
            last_success: old_time,
            success_count: 1,
        });

        // Prune the cache
        cache.prune();
        
        // The entry should have been removed
        assert!(cache.cache.get(pubkey).is_none() || cache.cache.get(pubkey).unwrap().is_empty());
    }
} 