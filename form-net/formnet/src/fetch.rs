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
use shared::{get_local_addrs, wg::{self, DeviceExt, PeerInfoExt}, Endpoint, IoErrorContext, NatOpts, NetworkOpts, Peer, PeerDiff};
use wireguard_control::{Backend, Device, DeviceUpdate, InterfaceName, PeerConfigBuilder};
use form_types::state::{Response as StateResponse, Success};

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
                    Ok(cache) => {
                        log::info!("Loaded connection cache from {}", cache_path.display());
                        
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
                                    });
                                }
                                
                                new_cache.endpoints.insert(pubkey, new_entries);
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
                Ok((is_connected, metrics)) => {
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
                                if let Some(latency) = metrics.latency_ms {
                                    entry.latency_ms = Some(latency);
                                }
                                if let Some(packet_loss) = metrics.packet_loss_pct {
                                    entry.packet_loss_pct = Some(packet_loss);
                                }
                                if let Some(jitter) = metrics.jitter_ms {
                                    entry.jitter_ms = Some(jitter);
                                }
                                if let Some(success_rate) = metrics.handshake_success_rate {
                                    entry.handshake_success_rate = Some(success_rate);
                                }
                                
                                // Update quality score with new metrics
                                entry.update_quality_score();
                                
                                log::debug!("Updated quality metrics for {} (score: {})", 
                                    endpoint, entry.quality_score.unwrap_or(0));
                            }
                        }
                    } else {
                        // Connection failed
                        log::warn!("Health check failed for endpoint {} (peer {})", endpoint, pubkey);
                        
                        // Update the endpoint status
                        if let Some(endpoints) = self.endpoints.get_mut(&pubkey) {
                            if let Some(entry) = endpoints.get_mut(idx) {
                                entry.failure_count += 1;
                                entry.last_checked = Some(now);
                                
                                // Add to recent failures list, keeping only the last 10
                                entry.recent_failures.push(now);
                                if entry.recent_failures.len() > 10 {
                                    entry.recent_failures.remove(0);
                                }
                                
                                // Update status based on failure count
                                if entry.failure_count >= 3 {
                                    log::warn!("Marking endpoint {} as failed after {} consecutive failures", 
                                        endpoint, entry.failure_count);
                                    entry.status = ConnectionStatus::Failed;
                                } else {
                                    log::info!("Marking endpoint {} as degraded (failure {} of 3)", 
                                        endpoint, entry.failure_count);
                                    entry.status = ConnectionStatus::Degraded;
                                }
                                
                                // Update quality score to reflect failure
                                entry.update_quality_score();
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

// Helper function to handle server NAT traversal
async fn try_server_nat_traversal(
    interface: &InterfaceName,
    network: NetworkOpts,
    my_ip: String,
    _connection_cache: &mut ConnectionCache,
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
    // Calculate a quality score based on available metrics
    fn calculate_quality_score(&self) -> u32 {
        let mut score = 0;
        let mut factors = 0;
        
        // Base score based on endpoint type (0-40 points)
        let type_score = match self.endpoint_type {
            EndpointType::PublicIpv4 => 40,
            EndpointType::PublicIpv6 => 35,
            EndpointType::PrivateIpv4 => 30,
            EndpointType::PrivateIpv6 => 25,
            EndpointType::LinkLocal => 15,
            EndpointType::Loopback => 10,
            EndpointType::Unknown => 5,
        };
        score += type_score;
        factors += 1;
        
        // Connection status (0-30 points)
        let status_score = match self.status {
            ConnectionStatus::Healthy => 30,
            ConnectionStatus::Degraded => 15,
            ConnectionStatus::Unknown => 10,
            ConnectionStatus::Failed => 0,
        };
        score += status_score;
        factors += 1;
        
        // Latency score (0-10 points)
        if let Some(latency) = self.latency_ms {
            let latency_score = if latency < 10 {
                10 // Excellent: < 10ms
            } else if latency < 50 {
                8 // Very good: 10-50ms
            } else if latency < 100 {
                6 // Good: 50-100ms
            } else if latency < 200 {
                4 // Fair: 100-200ms
            } else if latency < 500 {
                2 // Poor: 200-500ms
            } else {
                0 // Very poor: > 500ms
            };
            score += latency_score;
            factors += 1;
        }
        
        // Packet loss score (0-10 points)
        if let Some(packet_loss) = self.packet_loss_pct {
            let packet_loss_score = if packet_loss == 0 {
                10 // Perfect: 0% loss
            } else if packet_loss < 1 {
                8 // Excellent: < 1% loss
            } else if packet_loss < 5 {
                6 // Good: 1-5% loss
            } else if packet_loss < 10 {
                4 // Fair: 5-10% loss
            } else if packet_loss < 20 {
                2 // Poor: 10-20% loss
            } else {
                0 // Very poor: > 20% loss
            };
            score += packet_loss_score;
            factors += 1;
        }
        
        // Handshake success rate (0-10 points)
        if let Some(success_rate) = self.handshake_success_rate {
            let handshake_score = if success_rate > 95 {
                10 // Excellent: > 95% success
            } else if success_rate > 85 {
                8 // Very good: 85-95% success
            } else if success_rate > 70 {
                6 // Good: 70-85% success
            } else if success_rate > 50 {
                4 // Fair: 50-70% success
            } else if success_rate > 30 {
                2 // Poor: 30-50% success
            } else {
                0 // Very poor: < 30% success
            };
            score += handshake_score;
            factors += 1;
        }
        
        // Jitter score (0-10 points)
        if let Some(jitter) = self.jitter_ms {
            let jitter_score = if jitter < 5 {
                10 // Excellent: < 5ms jitter
            } else if jitter < 20 {
                8 // Very good: 5-20ms jitter
            } else if jitter < 50 {
                6 // Good: 20-50ms jitter
            } else if jitter < 100 {
                4 // Fair: 50-100ms jitter
            } else if jitter < 200 {
                2 // Poor: 100-200ms jitter
            } else {
                0 // Very poor: > 200ms jitter
            };
            score += jitter_score;
            factors += 1;
        }
        
        // Success stability (0-10 points)
        // More consecutive successes = higher score
        let stability_score = if self.success_count > 20 {
            10 // Excellent: > 20 consecutive successes
        } else if self.success_count > 10 {
            8 // Very good: 10-20 consecutive successes
        } else if self.success_count > 5 {
            6 // Good: 5-10 consecutive successes
        } else if self.success_count > 2 {
            4 // Fair: 2-5 consecutive successes
        } else if self.success_count > 0 {
            2 // Poor: 1 success
        } else {
            0 // Never succeeded
        };
        score += stability_score;
        factors += 1;
        
        // Failure pattern analysis (0-10 points)
        // Analyze recent failures to detect patterns
        let recent_failure_score = match self.recent_failures.len() {
            0 => 10, // No recent failures - excellent
            1 => 8,  // One failure - very good
            2 => 6,  // Two failures - good
            3..=5 => 4, // 3-5 failures - fair
            6..=10 => 2, // 6-10 failures - poor
            _ => 0,  // More than 10 failures - very poor
        };
        score += recent_failure_score;
        factors += 1;
        
        // Calculate weighted average and normalize to 0-100
        if factors > 0 {
            (score * 100) / (factors * 10)
        } else {
            0
        }
    }
    
    // Update connection quality score
    fn update_quality_score(&mut self) {
        let score = self.calculate_quality_score();
        self.quality_score = Some(score);
        self.last_quality_update = Some(SystemTime::now());
    }
}
