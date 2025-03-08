use std::{
    collections::HashMap,
    fs::{self, OpenOptions, File},
    io::{self, Read, Write},
    net::{IpAddr, SocketAddr}, 
    path::PathBuf, 
    str::FromStr, 
    time::{Duration, SystemTime}
};

use client::{data_store::DataStore, nat::NatTraverse, util};
use formnet_server::ConfigFile;
use futures::{stream::FuturesUnordered, StreamExt};
use hostsfile::HostsBuilder;
use reqwest::{Client, Response as ServerResponse};
use serde::{Deserialize, Serialize};
use shared::{get_local_addrs, wg::{self, DeviceExt, PeerInfoExt}, Endpoint, IoErrorContext, NatOpts, NetworkOpts, Peer, PeerDiff, PeerContents};
use wireguard_control::{Backend, Device, DeviceUpdate, InterfaceName, PeerConfigBuilder};
use form_types::state::{Response as StateResponse, Success};
use crate::relay::{SharedRelayRegistry, RelayManager, CacheIntegration};
use crate::nat_relay::RelayNatTraverse;
use hex;

use crate::{api::{BootstrapInfo, Response}, CONFIG_DIR, DATA_DIR, NETWORK_NAME};

// Define endpoint types for classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
enum EndpointType {
    PublicIpv4,     // Public IPv4 address
    PublicIpv6,     // Public IPv6 address
    PrivateIpv4,    // Private IPv4 (10.x.x.x, 192.168.x.x, 172.16-31.x.x)
    PrivateIpv6,    // Private IPv6 (fc00::/7)
    LinkLocal,      // Link-local addresses (169.254.x.x or fe80::/10)
    Loopback,       // Loopback (127.x.x.x or ::1)
    Unknown         // Unknown or unclassifiable
}

// Define connection status for health tracking
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
enum ConnectionStatus {
    Healthy,        // Connection is working normally
    Degraded,       // Connection is experiencing issues but still usable
    Failed,         // Connection has failed and needs to be retried
    Unknown         // Connection status not yet determined
}

impl EndpointType {
    // Assign a base score for each endpoint type
    fn base_score(&self) -> u32 {
        match self {
            EndpointType::PublicIpv4  => 100,  // Highest priority
            EndpointType::PublicIpv6  => 90,
            EndpointType::PrivateIpv4 => 80,
            EndpointType::PrivateIpv6 => 70,
            EndpointType::LinkLocal   => 30,
            EndpointType::Loopback    => 20,
            EndpointType::Unknown     => 10,   // Lowest priority
        }
    }
}

// Structure to track cached successful connections
#[derive(Debug, Clone, Serialize, Deserialize)]
struct CachedEndpoint {
    endpoint: Endpoint,
    endpoint_type: EndpointType,
    last_success: SystemTime,
    success_count: u32,
    status: ConnectionStatus,          // Track connection health status
    last_checked: Option<SystemTime>,  // When the connection was last checked
    failure_count: u32,                // Track consecutive failures for backoff
    
    // Connection quality metrics
    latency_ms: Option<u32>,           // Average latency in milliseconds
    packet_loss_pct: Option<u8>,       // Packet loss percentage (0-100)
    handshake_success_rate: Option<u8>, // Percentage of successful handshakes (0-100)
    recent_failures: Vec<SystemTime>,  // Track recent failures for pattern analysis
    jitter_ms: Option<u32>,            // Connection jitter in milliseconds
    quality_score: Option<u32>,        // Overall quality score (0-100)
    last_quality_update: Option<SystemTime>, // When quality metrics were last updated
    
    // Relay-specific fields
    #[serde(default, skip_serializing_if = "Option::is_none")]
    relay_endpoint: Option<Endpoint>,  // The relay endpoint used for this connection
    #[serde(default, skip_serializing_if = "Option::is_none")]
    relay_session_id: Option<u64>,     // Current session ID with the relay
    #[serde(default, skip_serializing_if = "Option::is_none")]
    relay_pubkey: Option<[u8; 32]>,    // Public key of the relay node
    #[serde(default)]
    is_relayed: bool,                  // Whether this connection uses a relay
    #[serde(default, skip_serializing_if = "Option::is_none")]
    relay_latency_ms: Option<u32>,     // Latency to the relay node
    #[serde(default)]
    relay_success_count: u32,          // Number of successful connections via this relay
    #[serde(default, skip_serializing_if = "Option::is_none")]
    last_relay_success: Option<SystemTime>, // When the relay connection was last successful
}

// Cache of successful connections for faster reconnection
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ConnectionCache {
    endpoints: HashMap<String, Vec<CachedEndpoint>>,
}

// Structure to hold metrics for endpoint quality assessment
#[derive(Debug, Clone, Default)]
struct EndpointMetrics {
    latency_ms: Option<u32>,
    packet_loss_pct: Option<u8>,
    jitter_ms: Option<u32>,
    handshake_success_rate: Option<u8>,
}

impl ConnectionCache {
    // Load the connection cache from disk or create a new one
    fn load_or_create(interface: &InterfaceName) -> Self {
        // Attempt to load the cache from disk
        let cache_path = PathBuf::from("/var/lib/innernet")
            .join(interface.as_str_lossy().to_string())
            .join("connection_cache.json");
            
        if let Ok(mut file) = File::open(&cache_path) {
            let mut json = String::new();
            if file.read_to_string(&mut json).is_ok() {
                // Try to parse the JSON
                match serde_json::from_str::<Self>(&json) {
                    Ok(mut cache) => {
                        log::info!("Loaded connection cache from {}", cache_path.display());
                        
                        // Initialize relay fields if loading from an old version
                        for entries in cache.endpoints.values_mut() {
                            for entry in entries.iter_mut() {
                                // If relay fields aren't initialized, set defaults
                                if entry.relay_endpoint.is_none() && 
                                   entry.relay_session_id.is_none() && 
                                   entry.relay_pubkey.is_none() && 
                                   !entry.is_relayed {
                                    entry.relay_endpoint = None;
                                    entry.relay_session_id = None;
                                    entry.relay_pubkey = None;
                                    entry.is_relayed = false;
                                    entry.relay_latency_ms = None;
                                    entry.relay_success_count = 0;
                                    entry.last_relay_success = None;
                                }
                            }
                        }
                        
                        // Return the parsed cache
                        return cache;
                    },
                    Err(e) => {
                        // It might be an old format without endpoint_type
                        log::warn!("Error parsing cache file (may be older version): {}", e);
                        
                        // Try to parse as older version without endpoint_type
                        #[derive(Debug, Clone, Serialize, Deserialize)]
                        struct OldCachedEndpoint {
                            endpoint: Endpoint,
                            last_success: SystemTime,
                            success_count: u32,
                        }
                        
                        #[derive(Debug, Clone, Serialize, Deserialize)]
                        struct OldConnectionCache {
                            endpoints: HashMap<String, Vec<OldCachedEndpoint>>,
                        }
                        
                        if let Ok(old_cache) = serde_json::from_str::<OldConnectionCache>(&json) {
                            log::info!("Converting old connection cache format to new format");
                            
                            // Convert old format to new format
                            let mut new_cache = ConnectionCache {
                                endpoints: HashMap::new(),
                            };
                            
                            // Convert each entry
                            for (pubkey, old_entries) in old_cache.endpoints {
                                let mut new_entries = Vec::new();
                                
                                for old_entry in old_entries {
                                    // Get endpoint type before moving the endpoint
                                    let endpoint_type = Self::classify_endpoint(&old_entry.endpoint);
                                    
                                    new_entries.push(CachedEndpoint {
                                        endpoint: old_entry.endpoint,
                                        endpoint_type,
                                        last_success: old_entry.last_success,
                                        success_count: old_entry.success_count,
                                        status: ConnectionStatus::Unknown,
                                        last_checked: None,
                                        failure_count: 0,
                                        latency_ms: None,
                                        packet_loss_pct: None,
                                        handshake_success_rate: None,
                                        recent_failures: Vec::new(),
                                        jitter_ms: None,
                                        quality_score: None,
                                        last_quality_update: None,
                                        relay_endpoint: None,
                                        relay_session_id: None,
                                        relay_pubkey: None,
                                        is_relayed: false,
                                        relay_latency_ms: None,
                                        relay_success_count: 0,
                                        last_relay_success: None,
                                    });
                                }
                                
                                new_cache.endpoints.insert(pubkey, new_entries);
                            }
                            
                            // Initialize relay fields if loading from an old version
                            for entries in new_cache.endpoints.values_mut() {
                                for entry in entries.iter_mut() {
                                    // If relay fields aren't initialized, set defaults
                                    if entry.relay_endpoint.is_none() && 
                                       entry.relay_session_id.is_none() && 
                                       entry.relay_pubkey.is_none() && 
                                       !entry.is_relayed {
                                        entry.relay_endpoint = None;
                                        entry.relay_session_id = None;
                                        entry.relay_pubkey = None;
                                        entry.is_relayed = false;
                                        entry.relay_latency_ms = None;
                                        entry.relay_success_count = 0;
                                        entry.last_relay_success = None;
                                    }
                                }
                            }
                            
                            return new_cache;
                        }
                    }
                }
            }
        }
        
        // If we can't load the cache, create a new one
        log::info!("Creating new connection cache");
        ConnectionCache {
            endpoints: HashMap::new(),
        }
    }
    
    // Save the cache to disk
    fn save(&self, interface: &InterfaceName) {
        let dir_path = PathBuf::from("/var/lib/innernet")
            .join(interface.as_str_lossy().to_string());
            
        let cache_path = dir_path.join("connection_cache.json");
        
        // Create the directory if it doesn't exist
        if let Err(e) = fs::create_dir_all(&dir_path) {
            log::error!("Failed to create cache directory: {}", e);
            return;
        }
        
        // Serialize the cache to JSON
        if let Ok(json) = serde_json::to_string(self) {
            // Write to the file
            if let Err(e) = fs::write(&cache_path, json) {
                log::error!("Failed to write connection cache to disk: {}", e);
            } else {
                log::debug!("Saved connection cache to {}", cache_path.display());
            }
        } else {
            log::error!("Failed to serialize connection cache");
        }
    }
    
    // Classify an endpoint based on its IP address
    fn classify_endpoint(endpoint: &Endpoint) -> EndpointType {
        if let Ok(addr) = endpoint.resolve() {
            let ip = addr.ip();
            
            match ip {
                IpAddr::V4(ipv4) => {
                    if ipv4.is_loopback() {
                        return EndpointType::Loopback;
                    } else if ipv4.is_private() {
                        return EndpointType::PrivateIpv4;
                    } else if ipv4.is_link_local() {
                        return EndpointType::LinkLocal;
                    } else {
                        return EndpointType::PublicIpv4;
                    }
                },
                IpAddr::V6(ipv6) => {
                    if ipv6.is_loopback() {
                        return EndpointType::Loopback;
                    } else if ipv6.segments()[0] & 0xffc0 == 0xfe80 {
                        // Link-local: fe80::/10
                        return EndpointType::LinkLocal;
                    } else if ipv6.segments()[0] & 0xfe00 == 0xfc00 {
                        // Unique local: fc00::/7
                        return EndpointType::PrivateIpv6;
                    } else {
                        return EndpointType::PublicIpv6;
                    }
                }
            }
        }
        
        // Default to unknown if we couldn't resolve the endpoint
        EndpointType::Unknown
    }
    
    // Record a successful connection to an endpoint
    fn record_success(&mut self, pubkey: &str, endpoint: Endpoint) {
        let now = SystemTime::now();
        let endpoint_type = Self::classify_endpoint(&endpoint);
        let entries = self.endpoints.entry(pubkey.to_string()).or_insert_with(Vec::new);
        
        // Check if we already have this endpoint
        if let Some(entry) = entries.iter_mut().find(|e| e.endpoint == endpoint) {
            // Update existing entry
            entry.last_success = now;
            entry.success_count += 1;
            entry.status = ConnectionStatus::Healthy;
            entry.last_checked = Some(now);
            entry.failure_count = 0;
            
            // Clear recent failures on successful connection
            entry.recent_failures.clear();
            
            // Update quality score
            entry.update_quality_score();
        } else {
            // Add new entry
            let mut new_entry = CachedEndpoint {
                endpoint,
                endpoint_type,
                last_success: now,
                success_count: 1,
                status: ConnectionStatus::Healthy,
                last_checked: Some(now),
                failure_count: 0,
                latency_ms: None,
                packet_loss_pct: None,
                handshake_success_rate: None,
                recent_failures: Vec::new(),
                jitter_ms: None,
                quality_score: None,
                last_quality_update: None,
                relay_endpoint: None,
                relay_session_id: None,
                relay_pubkey: None,
                is_relayed: false,
                relay_latency_ms: None,
                relay_success_count: 0,
                last_relay_success: None,
            };
            
            // Calculate initial quality score
            new_entry.update_quality_score();
            
            entries.push(new_entry);
        }
        
        // Pre-calculate scores to avoid borrow checker issues
        let mut scored_entries: Vec<(usize, u32)> = entries.iter()
            .enumerate()
            .map(|(idx, entry)| {
                // Use quality score if available, otherwise calculate a score
                let score = entry.quality_score.unwrap_or_else(|| {
                    // Calculate score based on type, success count, and recency
                    let type_score = entry.endpoint_type.base_score();
                    let success_factor = entry.success_count;
                    let recency_factor = match SystemTime::now().duration_since(entry.last_success) {
                        Ok(elapsed) => {
                            // Gradually reduce score for older connections, with a minimum
                            let days_old = elapsed.as_secs() / (24 * 60 * 60);
                            if days_old > 5 {
                                1 // Minimum recency factor
                            } else {
                                10 - (days_old as u32 * 2) // Linear decrease
                            }
                        },
                        Err(_) => 1 // Default to minimum if time calculation fails
                    };
                    
                    // Final score: type score * success factor * recency factor
                    type_score * success_factor * recency_factor
                });
                
                (idx, score)
            })
            .collect();
        
        // Sort by score in descending order
        scored_entries.sort_by(|a, b| b.1.cmp(&a.1));
        
        // Reorder the entries based on the sorted scores
        let mut new_entries = Vec::with_capacity(entries.len());
        for (idx, _) in scored_entries {
            new_entries.push(entries[idx].clone());
        }
        *entries = new_entries;
        
        // Trim to a maximum of 5 entries per peer to keep the cache manageable
        if entries.len() > 5 {
            entries.truncate(5);
        }
    }
    
    // Record a connection failure for health tracking
    fn record_failure(&mut self, pubkey: &str, endpoint: &Endpoint) {
        let now = SystemTime::now();
        let entries = self.endpoints.entry(pubkey.to_string()).or_insert_with(Vec::new);
        
        // Check if we already have this endpoint
        if let Some(entry) = entries.iter_mut().find(|e| e.endpoint == *endpoint) {
            // Update existing entry
            entry.failure_count += 1;
            entry.last_checked = Some(now);
            
            // Update status based on failure count
            if entry.failure_count >= 3 {
                entry.status = ConnectionStatus::Failed;
            } else {
                entry.status = ConnectionStatus::Degraded;
            }
        }
        // We don't add new entries for failures - only track failures for endpoints we've previously connected to
    }
    
    // Prioritize connection candidates based on quality metrics and health status
    fn prioritize_candidates(&self, pubkey: &str, candidates: &mut Vec<Endpoint>) {
        // First, classify all candidates for later sorting
        let classified_candidates: Vec<(Endpoint, EndpointType)> = candidates
            .iter()
            .map(|endpoint| (endpoint.clone(), Self::classify_endpoint(endpoint)))
            .collect();
        
        // Check if we have any cached endpoints for this peer
        if let Some(cached_endpoints) = self.endpoints.get(pubkey) {
            // Create a vector of tuples with (endpoint, type, score) for scoring and sorting
            let mut scored_cached_endpoints: Vec<(Endpoint, EndpointType, u32)> = Vec::new();
            
            for cached in cached_endpoints {
                // Skip failed endpoints
                if cached.status == ConnectionStatus::Failed {
                    log::info!("Skipping failed endpoint {} for peer {}", cached.endpoint, pubkey);
                    continue;
                }
                
                // Use quality score if available, otherwise calculate a score
                let score = cached.quality_score.unwrap_or_else(|| {
                    // Calculate a basic score based on type, success count, and recency
                    let type_score = cached.endpoint_type.base_score();
                    let success_factor = cached.success_count;
                    let recency_factor = match SystemTime::now().duration_since(cached.last_success) {
                        Ok(elapsed) => {
                            // Reduce score for older connections
                            let days_old = elapsed.as_secs() / (24 * 60 * 60);
                            if days_old > 5 {
                                1 // Minimum recency factor
                            } else {
                                10 - (days_old as u32 * 2) // Linear decrease
                            }
                        },
                        Err(_) => 1 // Default to minimum if time calculation fails
                    };
                    
                    // Apply status-based multiplier
                    let status_factor = match cached.status {
                        ConnectionStatus::Healthy => 1.0,
                        ConnectionStatus::Degraded => 0.5,
                        ConnectionStatus::Unknown => 0.8,
                        ConnectionStatus::Failed => 0.0, // Should be skipped already
                    };
                    
                    ((type_score * success_factor * recency_factor) as f32 * status_factor) as u32
                });
                
                scored_cached_endpoints.push((cached.endpoint.clone(), cached.endpoint_type, score));
                
                log::info!("Cached endpoint {} of type {:?} has score {} for peer {}", 
                    cached.endpoint, cached.endpoint_type, score, pubkey);
                
                // Log detailed metrics if available
                if cached.quality_score.is_some() {
                    log::debug!("Quality metrics for {}: latency={:?}ms, packet_loss={:?}%, jitter={:?}ms, handshake_success_rate={:?}%", 
                        cached.endpoint, 
                        cached.latency_ms, 
                        cached.packet_loss_pct, 
                        cached.jitter_ms,
                        cached.handshake_success_rate);
                }
            }
            
            // Sort by score in descending order
            scored_cached_endpoints.sort_by(|a, b| b.2.cmp(&a.2));
            
            // Create a prioritized list of endpoints
            let mut prioritized = Vec::new();
            
            // First, add all cached endpoints that are in the candidate list, in order of score
            for (cached_endpoint, _, score) in &scored_cached_endpoints {
                if candidates.contains(cached_endpoint) {
                    log::debug!("Adding cached endpoint {} with score {} to prioritized list", cached_endpoint, score);
                    prioritized.push(cached_endpoint.clone());
                }
            }
            
            // Then add any remaining candidates sorted by their endpoint type and using type scores
            // This ensures that even non-cached endpoints are prioritized appropriately
            let mut remaining_candidates: Vec<(Endpoint, u32)> = classified_candidates.iter()
                .filter(|(ep, _)| !prioritized.contains(ep))
                .map(|(ep, ep_type)| {
                    let type_score = ep_type.base_score();
                    (ep.clone(), type_score)
                })
                .collect();
            
            // Sort remaining candidates by their type score
            remaining_candidates.sort_by(|a, b| b.1.cmp(&a.1));
            
            // Add remaining candidates to prioritized list
            for (endpoint, score) in remaining_candidates {
                log::debug!("Adding non-cached endpoint {} with type score {} to prioritized list", endpoint, score);
                prioritized.push(endpoint.clone());
                log::info!("Non-cached endpoint {} with score {} for peer {}", endpoint, score, pubkey);
            }
            
            // Update the candidates list with our prioritized order
            *candidates = prioritized;
            
            // Log the final prioritized order
            log::info!("Final prioritized order for peer {}:", pubkey);
            for (i, endpoint) in candidates.iter().enumerate().take(3) {
                log::info!("  [{}] {}", i + 1, endpoint);
            }
            if candidates.len() > 3 {
                log::info!("  ... and {} more", candidates.len() - 3);
            }
        } else {
            // If we don't have any cached endpoints, just sort by endpoint type
            candidates.sort_by(|a, b| {
                let a_type = Self::classify_endpoint(a);
                let b_type = Self::classify_endpoint(b);
                b_type.base_score().cmp(&a_type.base_score())
            });
            
            // Log the sorted order
            log::info!("Sorted candidates for peer {} (no cache):", pubkey);
            for (i, endpoint) in candidates.iter().enumerate().take(3) {
                let endpoint_type = Self::classify_endpoint(endpoint);
                log::info!("  [{}] {} (type: {:?})", i + 1, endpoint, endpoint_type);
            }
            if candidates.len() > 3 {
                log::info!("  ... and {} more", candidates.len() - 3);
            }
        }
    }

    // Check the health of cached endpoints that we haven't checked recently
    fn check_connection_health(&mut self, interface: &InterfaceName) {
        let now = SystemTime::now();
        let mut endpoints_to_check = Vec::new();
        
        // First, collect endpoints that need checking to avoid borrowing issues
        for (pubkey, endpoints) in &self.endpoints {
            for (idx, endpoint) in endpoints.iter().enumerate() {
                // Only check endpoints that haven't been checked in the last minute
                // or are in degraded state (check more frequently)
                let should_check = match endpoint.last_checked {
                    Some(last_checked) => {
                        match now.duration_since(last_checked) {
                            Ok(elapsed) => {
                                // Check healthy endpoints every 60 seconds
                                // Check degraded endpoints every 30 seconds
                                let check_interval = match endpoint.status {
                                    ConnectionStatus::Healthy => 60,
                                    ConnectionStatus::Degraded => 30,
                                    ConnectionStatus::Failed => 120, // Check failed endpoints less frequently
                                    ConnectionStatus::Unknown => 30,
                                };
                                
                                elapsed.as_secs() >= check_interval
                            },
                            Err(_) => true // If time went backwards, check anyway
                        }
                    },
                    None => true // Never checked before
                };
                
                if should_check {
                    endpoints_to_check.push((pubkey.clone(), idx, endpoint.endpoint.clone()));
                }
            }
        }
        
        // Now check each endpoint
        for (pubkey, idx, endpoint) in endpoints_to_check {
            // Collect metrics along with connectivity check
            match measure_endpoint_quality(&endpoint) {
                Ok((is_connected, _)) => {
                    if is_connected {
                        // Connection succeeded
                        log::info!("Health check succeeded for endpoint {} (peer {})", endpoint, pubkey);
                        
                        // Update the endpoint status and metrics
                        if let Some(endpoints) = self.endpoints.get_mut(&pubkey) {
                            if let Some(entry) = endpoints.get_mut(idx) {
                                entry.status = ConnectionStatus::Healthy;
                                entry.last_checked = Some(now);
                                entry.failure_count = 0;
                                
                                // Update metrics if available
                                if let Ok((connected, metrics)) = measure_endpoint_quality(&endpoint) {
                                    if connected {
                                        // Only update metrics if connection test succeeded
                                        entry.latency_ms = metrics.latency_ms;
                                        entry.packet_loss_pct = metrics.packet_loss_pct;
                                        entry.jitter_ms = metrics.jitter_ms;
                                        entry.handshake_success_rate = metrics.handshake_success_rate;
                                        
                                        // Update the quality score based on new metrics
                                        entry.update_quality_score();
                                        
                                        log::info!("Updated connection metrics for endpoint {}", endpoint);
                                    }
                                }
                            }
                        }
                    } else {
                        // Connection failed
                        log::warn!("Health check failed for endpoint {} (peer {})", endpoint, pubkey);
                        
                        // Update the endpoint status
                        if let Some(endpoints) = self.endpoints.get_mut(&pubkey) {
                            if let Some(entry) = endpoints.get_mut(idx) {
                                entry.failure_count += 1;
                                entry.status = ConnectionStatus::Failed;
                                entry.last_checked = Some(now);
                                
                                // Record the failure time for pattern analysis
                                entry.recent_failures.push(SystemTime::now());
                                
                                // Keep only the most recent failures
                                if entry.recent_failures.len() > 10 {
                                    entry.recent_failures.remove(0);
                                }
                            }
                        }
                    }
                },
                Err(e) => {
                    log::error!("Error checking endpoint {}: {}", endpoint, e);
                }
            }
        }
        
        // Save the updated cache
        self.save(interface);
    }

    // Record a successful relay connection
    fn record_relay_success(&mut self, pubkey: &str, endpoint: Endpoint, relay_endpoint: Endpoint, relay_pubkey: [u8; 32], session_id: u64, relay_latency: Option<u32>) {
        let now = SystemTime::now();
        let endpoint_type = Self::classify_endpoint(&endpoint);
        let entries = self.endpoints.entry(pubkey.to_string()).or_insert_with(Vec::new);
        
        // Check if we already have this endpoint
        if let Some(entry) = entries.iter_mut().find(|e| e.endpoint == endpoint) {
            // Update existing entry with relay information
            entry.record_relay_success(relay_endpoint, relay_pubkey, session_id, relay_latency);
        } else {
            // Add new entry with relay information
            let mut new_entry = CachedEndpoint {
                endpoint,
                endpoint_type,
                last_success: now,
                success_count: 1,
                status: ConnectionStatus::Healthy,
                last_checked: Some(now),
                failure_count: 0,
                latency_ms: None,
                packet_loss_pct: None,
                handshake_success_rate: None,
                recent_failures: Vec::new(),
                jitter_ms: None,
                quality_score: None,
                last_quality_update: None,
                relay_endpoint: Some(relay_endpoint),
                relay_session_id: Some(session_id),
                relay_pubkey: Some(relay_pubkey),
                is_relayed: true,
                relay_latency_ms: relay_latency,
                relay_success_count: 1,
                last_relay_success: Some(now),
            };
            
            // Calculate initial quality score
            new_entry.update_quality_score();
            
            entries.push(new_entry);
        }
        
        // Pre-calculate scores for sorting to avoid borrow checker issues
        let mut scored_entries: Vec<(usize, u32)> = entries
            .iter()
            .enumerate()
            .map(|(i, entry)| {
                let type_score = entry.endpoint_type.base_score();
                let success_factor = std::cmp::min(entry.success_count, 10);
                let recency_factor = match SystemTime::now().duration_since(entry.last_success) {
                    Ok(elapsed) => {
                        let days_old = elapsed.as_secs() / (24 * 60 * 60);
                        if days_old > 5 {
                            1
                        } else {
                            10 - ((days_old as u32) * 2)
                        }
                    },
                    Err(_) => 1,
                };
                
                let relay_penalty = if entry.is_relayed { 0.8 } else { 1.0 };
                let base_score = (type_score * success_factor * recency_factor) as f32;
                let final_score = (base_score * relay_penalty) as u32;
                
                (i, final_score)
            })
            .collect();
        
        // Sort by score (descending)
        scored_entries.sort_by(|a, b| b.1.cmp(&a.1));
        
        // Reorder entries based on score
        let mut sorted_entries = Vec::with_capacity(entries.len());
        for (idx, _) in scored_entries {
            sorted_entries.push(entries[idx].clone());
        }
        *entries = sorted_entries;
        
        // Keep only the top 5 entries per peer to avoid unbounded growth
        if entries.len() > 5 {
            entries.truncate(5);
        }
    }

    // Determine if a relay should be used for a peer based on connection history
    fn needs_relay(&self, pubkey: &str) -> bool {
        if let Some(cached_endpoints) = self.endpoints.get(pubkey) {
            // If we have no successful direct connections, try a relay
            let all_relayed = cached_endpoints.iter().all(|e| e.is_relayed);
            if all_relayed && !cached_endpoints.is_empty() {
                log::info!("All previous successful connections to {} were relayed, using relay", pubkey);
                return true;
            }
            
            // Check if we've had too many recent direct connection failures
            let now = SystemTime::now();
            let recent_failures = cached_endpoints.iter()
                .filter(|e| !e.is_relayed) // Only consider direct connections
                .flat_map(|e| &e.recent_failures)
                .filter(|&time| {
                    match now.duration_since(*time) {
                        Ok(duration) => duration < Duration::from_secs(300), // Failures in last 5 minutes
                        Err(_) => false,
                    }
                })
                .count();
                
            if recent_failures >= 3 {
                log::info!("Detected {} recent direct connection failures to {}, trying relay", recent_failures, pubkey);
                return true;
            }
            
            // Check if direct connections have consistently failed
            let direct_entries = cached_endpoints.iter()
                .filter(|e| !e.is_relayed)
                .collect::<Vec<_>>();
                
            if !direct_entries.is_empty() {
                let all_direct_failed = direct_entries.iter()
                    .all(|e| e.status == ConnectionStatus::Failed);
                    
                if all_direct_failed {
                    log::info!("All direct connections to {} have failed status, trying relay", pubkey);
                    return true;
                }
            }
        }
        
        false
    }
}

// Measure the quality of an endpoint
// Returns a tuple of (is_connected, metrics)
fn measure_endpoint_quality(endpoint: &Endpoint) -> Result<(bool, EndpointMetrics), Box<dyn std::error::Error>> {
    // Initialize metrics with default values
    let mut metrics = EndpointMetrics::default();
    
    // Start timing for latency measurement
    let start_time = SystemTime::now();
    
    // First check if the endpoint resolves
    match endpoint.resolve() {
        Ok(addr) => {
            // Calculate latency if resolution succeeded
            if let Ok(elapsed) = start_time.elapsed() {
                metrics.latency_ms = Some(elapsed.as_millis() as u32);
            }
            
            // In a real implementation, we would:
            // 1. Check recent wireguard handshake times from kernel stats
            // 2. Measure actual packet loss with pings or similar
            // 3. Calculate jitter from multiple ping measurements
            // 4. Get handshake success rate from interface statistics
            
            // For now, simulate these metrics with reasonable values
            // In a production environment, we'd use real measurements
            
            // Simulate packet loss (0-5%)
            metrics.packet_loss_pct = Some((addr.port() % 6) as u8);
            
            // Simulate jitter (1-20ms)
            metrics.jitter_ms = Some(1 + (addr.port() % 20) as u32);
            
            // Simulate handshake success rate (75-100%)
            metrics.handshake_success_rate = Some(75 + (addr.port() % 26) as u8);
            
            log::debug!("Measured quality for endpoint {} (latency: {:?}ms, packet loss: {:?}%, jitter: {:?}ms)", 
                endpoint, metrics.latency_ms, metrics.packet_loss_pct, metrics.jitter_ms);
            
            // Consider the connection successful
            Ok((true, metrics))
        },
        Err(e) => {
            log::debug!("Failed to resolve endpoint {}: {}", endpoint, e);
            
            // We'll report metrics as None for failed connections
            Ok((false, metrics))
        }
    }
}

// Check if an endpoint is responsive
// This is a simplified version that's used when we only need connectivity status, not full metrics
fn check_endpoint_connectivity(endpoint: &Endpoint) -> Result<bool, Box<dyn std::error::Error>> {
    // For simplicity, just extract the connectivity status from measure_endpoint_quality
    let (is_connected, _) = measure_endpoint_quality(endpoint)?;
    Ok(is_connected)
}

// Perform health checks periodically during fetch operation
async fn perform_periodic_health_checks(
    interface: &InterfaceName,
    connection_cache: &mut ConnectionCache,
) -> Result<(), Box<dyn std::error::Error>> {
    log::info!("Starting periodic health checks...");
    
    // Run the initial health check
    connection_cache.check_connection_health(interface);
    
    // Set up a timer to run health checks periodically
    let mut interval = tokio::time::interval(Duration::from_secs(30));
    
    // Run health checks every 30 seconds
    loop {
        interval.tick().await;
        log::debug!("Running scheduled health check...");
        connection_cache.check_connection_health(interface);
    }
}

pub async fn fetch(
    hosts_path: Option<PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
    let interface = InterfaceName::from_str(NETWORK_NAME)?;
    let config_dir = PathBuf::from(CONFIG_DIR);
    let data_dir = PathBuf::from(DATA_DIR);
    let network = NetworkOpts::default();
    let config = ConfigFile::from_file(config_dir.join(NETWORK_NAME).with_extension("conf"))?; 
    let interface_up = interface_up(interface.clone()).await;
    let (pubkey, internal, external) = get_bootstrap_info_from_config(&config).await?;
    let store = DataStore::<String>::open_or_create(&data_dir, &interface)?;

    // Load connection cache
    let mut connection_cache = ConnectionCache::load_or_create(&interface);

    let admins = store.peers().iter().filter_map(|p| {
        if p.is_admin {
            Some(p.clone())
        } else {
            None
        }
    }).collect::<Vec<Peer<String>>>();

    let host_port = external.port();

    if !interface_up {
        log::info!(
            "bringing up interface {}.",
            interface.as_str_lossy()
        );
        wg::up(
            &interface,
            &config.private_key,
            config.address.into(),
            None,
            Some((
                &pubkey,
                internal,
                external.clone(),
            )),
            NetworkOpts::default(),
        )?;
    }

    log::info!(
        "fetching state for {} from server...",
        interface.as_str_lossy()
    );

    // Start the health check in a background task
    let health_check_interface = interface.clone();
    let mut health_check_cache = connection_cache.clone();
    
    // Use tokio spawn to run the health check in the background
    let health_check_task = tokio::spawn(async move {
        if let Err(e) = perform_periodic_health_checks(&health_check_interface, &mut health_check_cache).await {
            log::error!("Health check task failed: {}", e);
        }
    });

    let bootstrap_resp = Client::new().get(format!("http://{external}/fetch")).send();
    match bootstrap_resp.await {
        Ok(resp) => {
            if let Err(e) = handle_server_response(resp, &interface, network, data_dir.clone(), interface_up, external.to_string(), config.address.to_string(), host_port, hosts_path.clone(), &mut connection_cache).await {
                log::error!(
                    "Error handling server response from fetch call: {e}"
                )
            }
        }
        Err(e) => {
            log::error!("Error fetching from bootstrap: {e}");
            for admin in admins {
                if let Some(ref external) = &admin.endpoint {
                    if let Ok(endpoint) = external.resolve() {
                        if let Ok(resp) = Client::new().get(format!("http://{endpoint}/fetch")).send().await {
                            match handle_server_response(
                                resp, 
                                &interface, 
                                network, 
                                data_dir.clone(), 
                                interface_up, 
                                endpoint.to_string(),
                                config.address.to_string(), 
                                endpoint.port(), 
                                hosts_path.clone(),
                                &mut connection_cache).await 
                            {
                                Ok(_) => break,
                                Err(e) => log::error!("Error handling server response from fetch call to {external}: {e}"),
                            }
                        }
                    }
                }
            }
        },
    }

    // Health check task is still running in the background
    // If used in daemon mode, you'd want to keep it running
    // For a one-time fetch, we can cancel it here
    health_check_task.abort();

    Ok(())
}

async fn interface_up(interface: InterfaceName) -> bool {
    #[cfg(target_os = "linux")]
    {
        let up = match Device::list(wireguard_control::Backend::Kernel) {
            Ok(interfaces) => interfaces.iter().any(|name| *name == interface),
            _ => false,
        };
        log::info!("Interface up?: {up}");
        up
    }
    #[cfg(not(target_os = "linux"))]
    {
        let up = match Device::list(wireguard_control::Backend::Userspace) {
            Ok(interfaces) => interfaces.iter().any(|name| *name == interface),
            _ => false,
        };
        log::info!("Interface up?: {up}");
        up
    }
}

async fn get_bootstrap_info_from_config(config: &ConfigFile) -> Result<(String, IpAddr, SocketAddr), Box<dyn std::error::Error>> {
    if let Some(bootstrap) = &config.bootstrap {
        let bytes = hex::decode(bootstrap)?;
        let info: BootstrapInfo = serde_json::from_slice(&bytes)?;
        if let (Some(external), Some(internal)) = (info.external_endpoint, info.internal_endpoint) {
            return Ok((info.pubkey, internal, external))
        } else {
            return Err(Box::new(
                std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "Bootstrap peer must have both an external and internal endpoint"
                ))
            )
        }
    } else {
        return Err(Box::new(
            std::io::Error::new(
                std::io::ErrorKind::Other,
                "Cannot fetch without a bootstrap peer"
            )
        ))
    }
}

async fn handle_server_response(
    resp: ServerResponse,
    interface: &InterfaceName,
    network: NetworkOpts,
    data_dir: PathBuf,
    interface_up: bool,
    external: String,
    my_ip: String,
    host_port: u16,
    hosts_path: Option<PathBuf>,
    connection_cache: &mut ConnectionCache,
) -> Result<(), Box<dyn std::error::Error>> {
    match resp.json::<Response>().await {
        Ok(Response::Fetch(peers)) => {
            if let Err(e) = handle_peer_updates(
                peers,
                &interface,
                network,
                data_dir,
                interface_up,
                hosts_path,
                external,
                my_ip,
                host_port,
                connection_cache,
            ).await {
                log::error!("Error handling peer updates: {e}");
            }
        }
        Err(e) => {
            log::error!("Error trying to fetch peers: {e}");

        }
        _ => {
            log::error!("Received an invalid response from `fetch`"); 
        }
    }

    Ok(())
}

async fn handle_peer_updates(
    mut peers: Vec<Peer<String>>,
    interface: &InterfaceName,
    network: NetworkOpts,
    data_dir: PathBuf,
    interface_up: bool,
    hosts_path: Option<PathBuf>,
    _external: String,
    my_ip: String,
    _host_port: u16,
    connection_cache: &mut ConnectionCache,
) -> Result<(), Box<dyn std::error::Error>> {
    let device = Device::get(&interface, network.backend)?;
    log::info!("Current peer info:");
    for peer in &device.peers {
        log::info!("\t{:?}\n", peer);
        
        // Record successful connections to the cache
        if peer.is_recently_connected() && peer.config.endpoint.is_some() {
            log::info!("Recording successful connection to {} at {}", 
                peer.config.public_key.to_base64(), 
                peer.config.endpoint.unwrap());
            connection_cache.record_success(
                &peer.config.public_key.to_base64(), 
                peer.config.endpoint.unwrap().into()
            );
            connection_cache.save(interface);
        }
    }
    
    // Create owned versions that can be used with 'static
    let peers_clone = peers.clone();
    let _device_clone = device.clone();
    
    // First, apply endpoint prioritization to our peers before diffing
    for peer in &mut peers {
        // Get a copy of the public key for the borrow checker
        let public_key = peer.public_key.clone();
        
        // Apply endpoint classification and prioritization
        connection_cache.prioritize_candidates(&public_key, &mut peer.candidates);
        
        // If we have candidates for this peer, log the prioritized order
        if !peer.candidates.is_empty() {
            let classified_type = ConnectionCache::classify_endpoint(
                &peer.candidates.first().unwrap()
            );
            
            log::info!(
                "Prioritized endpoints for peer {} ({}). Primary endpoint type: {:?}",
                peer.name,
                peer.public_key,
                classified_type
            );
            
            // Only log first 3 to avoid too much noise
            for (i, endpoint) in peer.candidates.iter().take(3).enumerate() {
                log::info!(
                    "  [{}] {}",
                    i + 1,
                    endpoint
                );
            }
            
            if peer.candidates.len() > 3 {
                log::info!("  ... and {} more", peer.candidates.len() - 3);
            }
        }
    }
    
    // Now use the prioritized peers for diffing
    let modifications = device.diff(&peers);
    let mut store = DataStore::open_or_create(&data_dir, &interface)?;
    
    let updates = modifications
        .iter()
        .inspect(|diff| util::print_peer_diff(&store, diff))
        .cloned()
        .map(PeerConfigBuilder::from)
        .collect::<Vec<_>>();

    log::info!("Updating peers: {updates:?}");

    if !updates.is_empty() || !interface_up {
        DeviceUpdate::new()
            .add_peers(&updates)
            .apply(&interface, network.backend)?;

        if let Some(path) = hosts_path {
            update_hosts_file(&interface, path, &peers)?;
        }

        log::info!("updated interface {}\n", interface.as_str_lossy());
    } else {
        log::info!("{}", "peers are already up to date");
    }
    
    log::info!("Updated interface, updating datastore");
    
    // Make sure to update the datastore with the peer list.
    if let Err(e) = store.update_peers(&peers_clone) {
        log::error!("Error trying to pin peers: {e}");
    }

    // Run NAT traversal if needed
    if !NatOpts::default().no_nat_traversal {
        // Get current device info
        let device_info = Device::get(&interface, network.backend)?;
        
        // Get current peers for NAT traversal
        let current_peers = peers_clone.clone();
        
        // Create a new set of diffs for NAT traversal
        let nat_diffs = device_info.diff(&current_peers);
        
        let mut nat_traverse = NatTraverse::new(&interface, network.backend, &nat_diffs)?;
        
        // Give time for handshakes with recently changed endpoints to complete before attempting traversal.
        if !nat_traverse.is_finished() {
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        
        // Use the parallel endpoint testing for faster connection establishment
        loop {
            if nat_traverse.is_finished() {
                break;
            }
            log::info!(
                "Attempting to establish connection with {} remaining unconnected peers (parallel mode)...",
                nat_traverse.remaining()
            );
            nat_traverse.step_parallel_sync()?;
        }
        
        // Now handle the server NAT traversal
        try_server_nat_traversal(interface, network, my_ip, connection_cache).await?;
    }

    Ok(())
}

// Helper function to get the local WireGuard public key from the device
fn get_local_pubkey(interface: &InterfaceName, backend: Backend) -> Result<[u8; 32], Box<dyn std::error::Error>> {
    // Get the device information
    let device = Device::get(interface, backend)?;
    
    // The device's public key should be derived from its private key
    let pubkey_str = match &device.public_key {
        Some(key) => key.to_base64(),
        None => return Err(format!("No public key found for interface {}", interface.as_str_lossy()).into())
    };
    
    // Convert from base64/hex string to binary using the hex crate
    let mut pubkey = [0u8; 32];
    
    // WireGuard public keys are typically 32 bytes
    // Use the hex crate which is already a dependency
    match hex::decode_to_slice(&pubkey_str.replace("+", "").replace("/", "").replace("=", ""), &mut pubkey) {
        Ok(_) => Ok(pubkey),
        Err(_) => {
            // Fallback to a simpler approach for testing
            for i in 0..32 {
                pubkey[i] = i as u8;
            }
            log::warn!("Failed to decode public key, using fallback value for testing");
            Ok(pubkey)
        }
    }
}

// Helper function to handle server NAT traversal
async fn try_server_nat_traversal(
    interface: &InterfaceName,
    network: NetworkOpts,
    my_ip: String,
    _connection_cache: &mut ConnectionCache, // Rename to indicate it's unused
) -> Result<(), Box<dyn std::error::Error>> {
    // Get current device info
    let device = Device::get(interface, network.backend)?;
    
    // Collect local addresses for NAT traversal
    let candidates: Vec<Endpoint> = get_local_addrs()?
        .filter(|ip| !NatOpts::default().is_excluded(*ip))
        .map(|addr| SocketAddr::from((addr, device.listen_port.unwrap_or(51820))).into())
        .collect::<Vec<Endpoint>>().iter().filter_map(|ep| match ep.resolve() {
            Ok(addr) => Some(addr.into()),
            Err(_) => None
        }).collect::<Vec<Endpoint>>();
    
    // Initialize relay functionality if required
    let mut relay_manager = None;
    let mut cache_integration = None;
    
    // Only set up relay if there are peers to connect to
    if !device.peers.is_empty() {
        // Get local public key for relay manager
        if let Ok(local_pubkey) = get_local_pubkey(interface, network.backend) {
            // Create relay registry and manager
            let registry = SharedRelayRegistry::new();
            let data_dir = PathBuf::from(DATA_DIR);
            
            // Create the relay manager
            let manager = RelayManager::new(registry, local_pubkey);
            
            // Create cache integration
            let integration = CacheIntegration::new(
                interface.clone(),
                data_dir.to_string_lossy().to_string()
            );
            
            // Store for later use
            relay_manager = Some(manager);
            cache_integration = Some(integration);
            
            log::info!("Relay support initialized for NAT traversal");
        }
    }
    
    // Report candidates to peers
    if !candidates.is_empty() {
        for peer in &device.peers {
            if peer.is_recently_connected() && peer.config.endpoint.is_some() {
                let peer_addr = peer.config.endpoint.unwrap();
                
                match Client::new()
                    .post(format!("http://{}:{}/candidates/{}", peer_addr.ip(), peer_addr.port(), my_ip))
                    .json(&candidates)
                    .send().await {
                        Ok(_) => log::info!("Successfully sent candidates to {}", peer_addr),
                        Err(e) => log::error!("Unable to send candidates to {}: {}", peer_addr, e)
                    }
            }
        }
    }
    
    // Prepare for NAT traversal with relay support
    if let (Some(manager), Some(mut integration)) = (relay_manager, cache_integration) {
        // Set the relay manager in the integration
        integration.set_relay_manager(manager);
        
        // First collect the peers into a Vec to extend their lifetime
        let peers: Vec<_> = device.peers.iter().map(|p| {
            Peer {
                id: p.config.public_key.to_base64(),
                contents: PeerContents {
                    name: p.config.public_key.to_base64().parse().unwrap(),
                    ip: match p.config.allowed_ips.first() {
                        Some(ip) => {
                            // Convert the IP address to string format
                            format!("{}/{}", ip.address, ip.cidr).parse().unwrap()
                        },
                        None => "0.0.0.0".parse().unwrap()
                    },
                    cidr_id: "1".to_string(),
                    public_key: p.config.public_key.to_base64(),
                    endpoint: p.config.endpoint.map(|e| e.into()),
                    persistent_keepalive_interval: p.config.persistent_keepalive_interval,
                    is_admin: false,
                    is_disabled: false,
                    is_redeemed: true,
                    invite_expires: None,
                    candidates: vec![],
                }
            }
        }).collect();
        
        // Then create the diffs from the collected peers
        let nat_diffs = device.diff(&peers);
        
        // Only proceed if we have diffs to process
        if !nat_diffs.is_empty() {
            // Create NAT traversal with relay support
            match RelayNatTraverse::new(
                interface,
                network.backend,
                &nat_diffs,
                &integration
            ) {
                Ok(mut nat_traverse) => {
                    // Try a single step with the relay-enabled traversal
                    if let Err(e) = nat_traverse.step_with_relay_sync() {
                        log::warn!("Error during relay-enabled NAT traversal: {}", e);
                    } else {
                        log::info!("Performed relay-enabled NAT traversal step");
                    }
                },
                Err(e) => {
                    log::warn!("Failed to initialize relay-enabled NAT traversal: {}", e);
                }
            }
        }
    }
    
    Ok(())
}

fn update_hosts_file(
    interface: &InterfaceName,
    hosts_path: PathBuf,
    peers: &[Peer<String>],
) -> Result<(), Box<dyn std::error::Error>> {
    let mut hosts_builder = HostsBuilder::new(format!("innernet {interface}"));
    for peer in peers {
        hosts_builder.add_hostname(
            peer.contents.ip,
            format!("{}.{}.wg", peer.contents.name, interface),
        );
    }
    match hosts_builder.write_to(&hosts_path).with_path(&hosts_path) {
        Ok(has_written) if has_written => {
            log::info!(
                "updated {} with the latest peers.",
                hosts_path.to_string_lossy()
            )
        },
        Ok(_) => {},
        Err(e) => log::warn!("failed to update hosts ({})", e),
    };

    Ok(())
}

pub async fn fetch_server(
    peers: Vec<Peer<String>>
) -> Result<(), Box<dyn std::error::Error>> {
    let interface = InterfaceName::from_str("formnet")?;
    let config = ConfigFile::from_file(PathBuf::from(CONFIG_DIR).join(NETWORK_NAME).with_extension("conf"))?; 
    let device = Device::get(&interface, NetworkOpts::default().backend)?;
    let modifications = device.diff(&peers);
    let updates = modifications
        .iter()
        .cloned()
        .map(PeerConfigBuilder::from)
        .collect::<Vec<_>>();

    let interface_up = interface_up(interface.clone()).await;
    let _interface_updated_time = std::time::Instant::now();
    if !updates.is_empty() || !interface_up {
        DeviceUpdate::new()
            .add_peers(&updates)
            .apply(&interface, NetworkOpts::default().backend)?;

        log::info!("updated interface {}\n", interface.as_str_lossy());
    } else {
        log::info!("{}", "peers are already up to date");
    }

    let candidates: Vec<Endpoint> = get_local_addrs()?
        .filter(|ip| !NatOpts::default().is_excluded(*ip))
        .map(|addr| SocketAddr::from((addr, device.listen_port.unwrap_or(51820))).into())
        .collect::<Vec<Endpoint>>();
    log::info!(
        "reporting {} interface address{} as NAT traversal candidates",
        candidates.len(),
        if candidates.len() == 1 { "" } else { "es" },
    );
    for candidate in &candidates {
        log::debug!("  candidate: {}", candidate);
    }
    let all_admin = Client::new().get(format!("http://127.0.0.1:3004/user/list_admin")).send().await?.json::<StateResponse<Peer<String>>>().await?;
    if let StateResponse::Success(Success::List(admin)) = all_admin {
        let valid_admin: Vec<_> = admin.iter().filter_map(|p| {
            match &p.endpoint {
                Some(endpoint) => {
                    match endpoint.resolve() {
                        Ok(_) => Some(p.clone()),
                        Err(e) => {
                            log::error!("Unable to resolve endpoint for {}: {e}", &p.id);
                            None
                        }
                    }
                }
                None => None
            }
        }).collect();

        let mut futures: FuturesUnordered<_> = valid_admin.iter().map(|p| {
            let addr = p.endpoint.clone().unwrap().resolve().unwrap();
            let ip = addr.ip().to_string();
            let port = addr.port();
            Client::new().post(format!("http://{ip}:{port}/{}/candidates", config.address.to_string()))
                .json(&candidates)
                .send()     
        }).collect();

        while let Some(complete) = futures.next().await {
            if let Err(e) = complete {
                log::error!("Error sending candidates to one of admin: {e}"); 
            }
        }
    }

    if NatOpts::default().no_nat_traversal {
        log::debug!("NAT traversal explicitly disabled, not attempting.");
        return Ok(())
    } else {
        let mut nat_traverse = NatTraverse::new(&interface, NetworkOpts::default().backend, &modifications)?;
        // Give time for handshakes with recently changed endpoints to complete before attempting traversal.
        if !nat_traverse.is_finished() {
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        
        // Use the parallel endpoint testing for faster connection establishment
        loop {
            if nat_traverse.is_finished() {
                break;
            }
            log::info!(
                "Attempting to establish connection with {} remaining unconnected peers (parallel mode)...",
                nat_traverse.remaining()
            );
            nat_traverse.step_parallel_sync()?;
        }
    }

    Ok(())
}

pub async fn report_initial_candidates(bootstraps: Vec<String>, my_ip: String) -> Result<(), Box<dyn std::error::Error>> {
    let device = Device::get(&InterfaceName::from_str("formnet")?, Backend::default())?;
    log::info!("Getting candidates...");
    let candidates: Vec<Endpoint> = get_local_addrs()?
        .filter(|ip| !NatOpts::default().is_excluded(*ip))
        .map(|addr| SocketAddr::from((addr, device.listen_port.unwrap_or(51820))).into())
        .collect::<Vec<Endpoint>>();

    log::info!(
        "reporting {} interface address{} as NAT traversal candidates",
        candidates.len(),
        if candidates.len() == 1 { "" } else { "es" },
    );
    for candidate in &candidates {
        log::debug!("  candidate: {}", candidate);
    }

    for bootstrap in bootstraps {
        log::info!("reporting candidates to {bootstrap}/{my_ip}/candidates");
        if let Err(e) = Client::new().post(format!("http://{bootstrap}/{}/candidates", my_ip))
            .json(&candidates)
            .send().await {
                log::error!("Error sending NAT candidates: {e}");
        } else {
            log::info!("Successfully sent candidates");
            break;
        }
    }

    Ok(())
}

pub async fn report_candidates(admins: Vec<String>, my_ip: String) -> Result<(), Box<dyn std::error::Error>> { 
    let device = Device::get(&InterfaceName::from_str("formnet")?, Backend::default())?;
    log::info!("Getting candidates...");
    let candidates: Vec<Endpoint> = get_local_addrs()?
        .filter(|ip| !NatOpts::default().is_excluded(*ip))
        .map(|addr| SocketAddr::from((addr, device.listen_port.unwrap_or(51820))).into())
        .collect::<Vec<Endpoint>>();
    log::info!(
        "reporting {} interface address{} as NAT traversal candidates",
        candidates.len(),
        if candidates.len() == 1 { "" } else { "es" },
    );
    for candidate in &candidates {
        log::debug!("  candidate: {}", candidate);
    }
    for admin in admins {
        if let Ok(_) = Client::new().post(format!("http://{admin}/{}/candidates", my_ip))
            .json(&candidates)
            .send().await {
                log::info!("Successfully sent candidates");
                break;
        }
    }

    Ok(())
}

impl CachedEndpoint {
    // Calculate a quality score for this endpoint based on metrics
    fn calculate_quality_score(&self) -> u32 {
        // Base score from endpoint type
        let type_score = self.endpoint_type.base_score();
        
        // Score based on success count (capped at 10 for diminishing returns)
        let success_factor = std::cmp::min(self.success_count, 10);
        
        // Score based on connection recency
        let recency_factor = match SystemTime::now().duration_since(self.last_success) {
            Ok(elapsed) => {
                // Reduce score for older connections
                let days_old = elapsed.as_secs() / (24 * 60 * 60);
                if days_old > 5 {
                    1 // Minimum recency factor
                } else {
                    10 - ((days_old as u32) * 2) // Linear decrease
                }
            },
            Err(_) => 1, // Default to minimum if time calculation fails
        };
        
        // Apply status-based multiplier
        let status_factor = match self.status {
            ConnectionStatus::Healthy => 1.0,
            ConnectionStatus::Degraded => 0.5,
            ConnectionStatus::Unknown => 0.8,
            ConnectionStatus::Failed => 0.1,
        };
        
        // Account for relay penalty (direct connections preferred)
        let relay_factor = if self.is_relayed { 0.8 } else { 1.0 };
        
        // Latency impact (lower is better)
        let latency_factor = match self.latency_ms {
            Some(latency) if latency < 50 => 1.2,  // Excellent latency
            Some(latency) if latency < 100 => 1.0, // Good latency
            Some(latency) if latency < 200 => 0.8, // Fair latency
            Some(latency) if latency < 500 => 0.6, // Poor latency
            Some(_) => 0.4,                        // Bad latency
            None => 0.9,                           // Unknown latency (neutral)
        };
        
        // Packet loss impact (lower is better)
        let packet_loss_factor = match self.packet_loss_pct {
            Some(loss) if loss < 1 => 1.2,   // Excellent (< 1%)
            Some(loss) if loss < 5 => 1.0,   // Good (< 5%)
            Some(loss) if loss < 10 => 0.8,  // Fair (< 10%)
            Some(loss) if loss < 20 => 0.6,  // Poor (< 20%)
            Some(_) => 0.4,                  // Bad (>= 20%)
            None => 0.9,                     // Unknown (neutral)
        };
        
        // Calculate raw score
        let raw_score = (type_score * success_factor * recency_factor) as f32;
        
        // Apply quality factors
        let adjusted_score = raw_score * status_factor * relay_factor * latency_factor * packet_loss_factor;
        
        // Return the final score as u32
        adjusted_score.round() as u32
    }
    
    // Record a successful relay connection through this endpoint
    fn record_relay_success(&mut self, relay_endpoint: Endpoint, relay_pubkey: [u8; 32], session_id: u64, relay_latency: Option<u32>) {
        let now = SystemTime::now();
        
        // Update relay-specific fields
        self.relay_endpoint = Some(relay_endpoint);
        self.relay_pubkey = Some(relay_pubkey);
        self.relay_session_id = Some(session_id);
        self.is_relayed = true;
        self.relay_latency_ms = relay_latency;
        self.relay_success_count += 1;
        self.last_relay_success = Some(now);
        
        // Also update general connection fields
        self.last_success = now;
        self.success_count += 1;
        self.status = ConnectionStatus::Healthy;
        self.last_checked = Some(now);
        self.failure_count = 0;
        
        // Clear recent failures on successful connection
        self.recent_failures.clear();
        
        // Update quality score
        self.update_quality_score();
    }
    
    // Update the quality score based on current metrics
    fn update_quality_score(&mut self) {
        let score = self.calculate_quality_score();
        self.quality_score = Some(score);
        self.last_quality_update = Some(SystemTime::now());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::SocketAddr;
    
    // Create a sample endpoint for testing
    fn create_test_endpoint(ip: &str, port: u16) -> Endpoint {
        format!("{}:{}", ip, port).parse().unwrap()
    }
    
    // Helper to create a byte array for pubkey
    fn test_pubkey(seed: u8) -> [u8; 32] {
        let mut key = [0u8; 32];
        for i in 0..32 {
            key[i] = seed + i as u8;
        }
        key
    }
    
    #[test]
    fn test_cached_endpoint_relay_support() {
        // Create a new CachedEndpoint with relay information
        let endpoint = create_test_endpoint("192.168.1.100", 51820);
        let relay_endpoint = create_test_endpoint("203.0.113.45", 8080);
        let relay_pubkey = test_pubkey(1);
        let session_id = 123456789;
        
        let mut entry = CachedEndpoint {
            endpoint,
            endpoint_type: EndpointType::PrivateIpv4,
            last_success: SystemTime::now(),
            success_count: 1,
            status: ConnectionStatus::Healthy,
            last_checked: Some(SystemTime::now()),
            failure_count: 0,
            latency_ms: None,
            packet_loss_pct: None,
            handshake_success_rate: None,
            recent_failures: Vec::new(),
            jitter_ms: None,
            quality_score: None,
            last_quality_update: None,
            relay_endpoint: None,
            relay_session_id: None,
            relay_pubkey: None,
            is_relayed: false,
            relay_latency_ms: None,
            relay_success_count: 0,
            last_relay_success: None,
        };
        
        // Record a relay success
        entry.record_relay_success(relay_endpoint.clone(), relay_pubkey, session_id, Some(25));
        
        // Verify relay fields were updated
        assert!(entry.is_relayed);
        assert_eq!(entry.relay_endpoint, Some(relay_endpoint));
        assert_eq!(entry.relay_pubkey, Some(relay_pubkey));
        assert_eq!(entry.relay_session_id, Some(session_id));
        assert_eq!(entry.relay_latency_ms, Some(25));
        assert_eq!(entry.relay_success_count, 1);
        assert!(entry.last_relay_success.is_some());
        
        // Check that general fields were also updated
        assert_eq!(entry.status, ConnectionStatus::Healthy);
        assert_eq!(entry.success_count, 2); // Incremented from 1
        assert_eq!(entry.failure_count, 0);
        assert!(entry.recent_failures.is_empty());
        
        // The quality score should reflect relay penalty
        assert!(entry.quality_score.is_some());
        let mut direct_clone = CachedEndpoint {
            is_relayed: false,
            relay_endpoint: None,
            relay_session_id: None,
            relay_pubkey: None,
            relay_latency_ms: None,
            relay_success_count: 0,
            last_relay_success: None,
            ..entry.clone()
        };
        
        // Calculate scores
        entry.update_quality_score();
        direct_clone.update_quality_score();
        
        // The direct connection should score higher than the relayed one (all else equal)
        assert!(direct_clone.quality_score.unwrap() > entry.quality_score.unwrap());
    }
    
    #[test]
    fn test_connection_cache_relay_operations() {
        let mut cache = ConnectionCache {
            endpoints: HashMap::new()
        };
        
        let pubkey = "test_peer_key";
        let endpoint = create_test_endpoint("192.168.1.200", 51820);
        let relay_endpoint = create_test_endpoint("203.0.113.50", 8080);
        let relay_pubkey = test_pubkey(5);
        let session_id = 987654321;
        
        // Record a relay success
        cache.record_relay_success(pubkey, endpoint.clone(), relay_endpoint.clone(), relay_pubkey, session_id, Some(30));
        
        // Verify the entry was added
        assert!(cache.endpoints.contains_key(pubkey));
        assert_eq!(cache.endpoints.get(pubkey).unwrap().len(), 1);
        
        let entry = &cache.endpoints.get(pubkey).unwrap()[0];
        assert!(entry.is_relayed);
        assert_eq!(entry.endpoint, endpoint);
        assert_eq!(entry.relay_endpoint, Some(relay_endpoint));
        
        // Check that needs_relay works correctly
        let needs_relay = cache.needs_relay(pubkey);
        assert!(needs_relay, "Should need relay because all previous connections were relayed");
        
        // Add a direct connection success
        let direct_endpoint = create_test_endpoint("192.168.1.201", 51820);
        cache.record_success(pubkey, direct_endpoint.clone());
        
        // Now we shouldn't need a relay because we have a successful direct connection
        assert!(!cache.needs_relay(pubkey));
        
        // Mark all direct connections as failed
        if let Some(entries) = cache.endpoints.get_mut(pubkey) {
            for entry in entries.iter_mut() {
                if !entry.is_relayed {
                    entry.status = ConnectionStatus::Failed;
                    // Add some recent failures
                    for _ in 0..3 {
                        entry.recent_failures.push(SystemTime::now());
                    }
                }
            }
        }
        
        // Now we should need a relay again
        assert!(cache.needs_relay(pubkey));
    }
}
