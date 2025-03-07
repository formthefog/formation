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

impl EndpointType {
    // Get a base score for this endpoint type (higher is better)
    fn base_score(&self) -> u32 {
        match self {
            EndpointType::PublicIpv4 => 90,    // Highest priority for remote connections
            EndpointType::PublicIpv6 => 85,    // Slightly lower than IPv4 due to compatibility
            EndpointType::PrivateIpv4 => 70,   // Good for local network
            EndpointType::PrivateIpv6 => 65,   // Slightly lower than IPv4
            EndpointType::LinkLocal => 30,     // Low priority, only works in local segment
            EndpointType::Loopback => 10,      // Lowest priority, only works on same machine
            EndpointType::Unknown => 50,       // Middle priority when we're not sure
        }
    }
}

// Simple struct to track successful connections to endpoints
#[derive(Debug, Clone, Serialize, Deserialize)]
struct CachedEndpoint {
    endpoint: Endpoint,
    endpoint_type: EndpointType,  // New field for classification
    last_success: SystemTime,
    success_count: u32,
}

// Simple cache for successful connections
#[derive(Debug, Default, Serialize, Deserialize)]
struct ConnectionCache {
    endpoints: HashMap<String, Vec<CachedEndpoint>>,
}

impl ConnectionCache {
    // Load cache from disk or create a new one
    fn load_or_create(interface: &InterfaceName) -> Self {
        let cache_path = PathBuf::from(DATA_DIR).join(format!("{}-connection-cache.json", interface));
        
        if cache_path.exists() {
            match File::open(&cache_path) {
                Ok(mut file) => {
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
                                
                                #[derive(Debug, Default, Serialize, Deserialize)]
                                struct OldConnectionCache {
                                    endpoints: HashMap<String, Vec<OldCachedEndpoint>>,
                                }
                                
                                if let Ok(old_cache) = serde_json::from_str::<OldConnectionCache>(&json) {
                                    // Convert old format to new format
                                    let mut new_cache = Self::default();
                                    
                                    for (pubkey, old_entries) in old_cache.endpoints {
                                        let mut new_entries = Vec::new();
                                        
                                        for old_entry in old_entries {
                                            new_entries.push(CachedEndpoint {
                                                endpoint: old_entry.endpoint.clone(),
                                                endpoint_type: Self::classify_endpoint(&old_entry.endpoint),
                                                last_success: old_entry.last_success,
                                                success_count: old_entry.success_count,
                                            });
                                        }
                                        
                                        new_cache.endpoints.insert(pubkey, new_entries);
                                    }
                                    
                                    log::info!("Successfully converted old cache format to new format");
                                    return new_cache;
                                }
                            }
                        }
                    }
                },
                Err(e) => log::warn!("Could not open connection cache: {}", e),
            }
        }
        
        log::info!("Creating new connection cache");
        Self::default()
    }
    
    // Save cache to disk
    fn save(&self, interface: &InterfaceName) {
        let cache_path = PathBuf::from(DATA_DIR).join(format!("{}-connection-cache.json", interface));
        
        // Ensure the directory exists
        if let Some(parent) = cache_path.parent() {
            if !parent.exists() {
                if let Err(e) = fs::create_dir_all(parent) {
                    log::error!("Could not create directory for connection cache: {}", e);
                    return;
                }
            }
        }
        
        match OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&cache_path) {
                Ok(mut file) => {
                    if let Ok(json) = serde_json::to_string_pretty(self) {
                        if let Err(e) = file.write_all(json.as_bytes()) {
                            log::error!("Could not write connection cache: {}", e);
                        } else {
                            log::info!("Saved connection cache to {}", cache_path.display());
                        }
                    }
                },
                Err(e) => log::error!("Could not create connection cache file: {}", e),
            }
    }
    
    // Classify an IP address into an endpoint type
    fn classify_endpoint(endpoint: &Endpoint) -> EndpointType {
        if let Ok(socket_addr) = endpoint.resolve() {
            match socket_addr.ip() {
                IpAddr::V4(ip) => {
                    let octets = ip.octets();
                    
                    // Check for loopback (127.x.x.x)
                    if octets[0] == 127 {
                        return EndpointType::Loopback;
                    }
                    
                    // Check for link-local (169.254.x.x)
                    if octets[0] == 169 && octets[1] == 254 {
                        return EndpointType::LinkLocal;
                    }
                    
                    // Check for private IP ranges
                    if (octets[0] == 10) ||                                         // 10.0.0.0/8
                       (octets[0] == 172 && octets[1] >= 16 && octets[1] <= 31) ||  // 172.16.0.0/12
                       (octets[0] == 192 && octets[1] == 168) {                     // 192.168.0.0/16
                        return EndpointType::PrivateIpv4;
                    }
                    
                    // If none of the above, it's a public IP
                    return EndpointType::PublicIpv4;
                },
                IpAddr::V6(ip) => {
                    let segments = ip.segments();
                    
                    // Check for loopback (::1)
                    if segments == [0, 0, 0, 0, 0, 0, 0, 1] {
                        return EndpointType::Loopback;
                    }
                    
                    // Check for link-local (fe80::/10)
                    if segments[0] & 0xffc0 == 0xfe80 {
                        return EndpointType::LinkLocal;
                    }
                    
                    // Check for unique local addresses (fc00::/7)
                    if segments[0] & 0xfe00 == 0xfc00 {
                        return EndpointType::PrivateIpv6;
                    }
                    
                    // If none of the above, it's a public IP
                    return EndpointType::PublicIpv6;
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
        } else {
            // Add new entry
            entries.push(CachedEndpoint {
                endpoint,
                endpoint_type,
                last_success: now,
                success_count: 1,
            });
        }
        
        // Pre-calculate scores to avoid borrow checker issues
        let mut scored_entries: Vec<(usize, u32)> = entries.iter()
            .enumerate()
            .map(|(idx, entry)| {
                // Calculate score without borrowing self
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
                    Err(_) => 5, // Default if time went backwards
                };
                let score = type_score * 100 + recency_factor * 10 + success_factor.min(10);
                (idx, score)
            })
            .collect();
        
        // Sort by score (higher is better)
        scored_entries.sort_by(|(_, score_a), (_, score_b)| score_b.cmp(score_a));
        
        // Reorder entries based on scores
        let mut new_entries = Vec::with_capacity(entries.len());
        for (idx, _) in scored_entries {
            new_entries.push(entries[idx].clone());
        }
        *entries = new_entries;
        
        // Limit to 5 entries per peer
        if entries.len() > 5 {
            entries.truncate(5);
        }
    }
    
    // Prioritize endpoints based on previous successful connections
    fn prioritize_candidates(&self, pubkey: &str, candidates: &mut Vec<Endpoint>) {
        // First, classify all candidates
        let mut classified_candidates: Vec<(Endpoint, EndpointType)> = candidates
            .iter()
            .map(|e| (e.clone(), Self::classify_endpoint(e)))
            .collect();
        
        if let Some(cached_endpoints) = self.endpoints.get(pubkey) {
            // Use cached information for prioritization
            let mut prioritized = Vec::new();
            
            // Create tuples of (endpoint, score) for sorting
            let mut scored_cached_endpoints: Vec<(Endpoint, u32)> = Vec::new();
            
            // Calculate score for each cached endpoint
            for cached in cached_endpoints {
                if candidates.contains(&cached.endpoint) {
                    // Calculate score directly
                    let type_score = cached.endpoint_type.base_score();
                    let success_factor = cached.success_count;
                    let recency_factor = match SystemTime::now().duration_since(cached.last_success) {
                        Ok(elapsed) => {
                            let days_old = elapsed.as_secs() / (24 * 60 * 60);
                            if days_old > 5 {
                                1
                            } else {
                                10 - (days_old as u32 * 2)
                            }
                        },
                        Err(_) => 5,
                    };
                    let score = type_score * 100 + recency_factor * 10 + success_factor.min(10);
                    
                    scored_cached_endpoints.push((cached.endpoint.clone(), score));
                    
                    log::info!("Scoring cached endpoint {} (type: {:?}, score: {}) for peer {}", 
                        cached.endpoint, cached.endpoint_type, score, pubkey);
                }
            }
            
            // Sort cached endpoints by score
            scored_cached_endpoints.sort_by(|(_, score_a), (_, score_b)| score_b.cmp(score_a));
            
            // Add cached endpoints in order of score
            for (endpoint, _) in scored_cached_endpoints {
                prioritized.push(endpoint);
            }
            
            // Now add remaining candidates based on endpoint type
            classified_candidates.sort_by(|(_, type_a), (_, type_b)| {
                type_b.base_score().cmp(&type_a.base_score())
            });
            
            for (endpoint, endpoint_type) in classified_candidates {
                if !prioritized.contains(&endpoint) {
                    log::info!("Adding non-cached endpoint {} (type: {:?}, score: {}) for peer {}", 
                        endpoint, endpoint_type, endpoint_type.base_score(), pubkey);
                    prioritized.push(endpoint);
                }
            }
            
            // Replace the original candidates list
            *candidates = prioritized;
        } else {
            // No cache for this peer, just sort by endpoint type
            classified_candidates.sort_by(|(_, type_a), (_, type_b)| {
                type_b.base_score().cmp(&type_a.base_score())
            });
            
            *candidates = classified_candidates.into_iter()
                .map(|(endpoint, endpoint_type)| {
                    log::info!("Sorting endpoint {} by type: {:?}, score: {}", 
                        endpoint, endpoint_type, endpoint_type.base_score());
                    endpoint
                })
                .collect();
        }
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
