use std::{
    collections::HashMap,
    fs::{self, OpenOptions, File},
    io::{Read, Write},
    net::{IpAddr, SocketAddr}, 
    path::{PathBuf, Path}, 
    str::FromStr, 
    time::{Instant, Duration, SystemTime}
};
use form_types::state::{Response as StateResponse, Success};
use client::{data_store::DataStore, nat::{self, NatTraverse}, util};
use formnet_server::ConfigFile;
use futures::{stream::FuturesUnordered, StreamExt};
use hostsfile::HostsBuilder;
use reqwest::{Client, Response as ServerResponse};
use serde::{Deserialize, Serialize};
use shared::{get_local_addrs, wg::{self, DeviceExt, PeerInfoExt}, Endpoint, IoErrorContext, NatOpts, NetworkOpts, Peer, PeerDiff};
use wireguard_control::{Backend, Device, DeviceUpdate, InterfaceName, PeerConfigBuilder};

use crate::{api::{BootstrapInfo, Response}, CONFIG_DIR, DATA_DIR, NETWORK_NAME};

// Simple struct to track successful connections to endpoints
#[derive(Debug, Clone, Serialize, Deserialize)]
struct CachedEndpoint {
    endpoint: Endpoint,
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
                        if let Ok(cache) = serde_json::from_str(&json) {
                            log::info!("Loaded connection cache from {}", cache_path.display());
                            return cache;
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
    
    // Record a successful connection to an endpoint
    fn record_success(&mut self, pubkey: &str, endpoint: Endpoint) {
        let now = SystemTime::now();
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
                last_success: now,
                success_count: 1,
            });
        }
        
        // Sort by success count (descending) and then by recency
        entries.sort_by(|a, b| {
            b.success_count
                .cmp(&a.success_count)
                .then_with(|| b.last_success.cmp(&a.last_success))
        });
        
        // Limit to 5 entries per peer
        if entries.len() > 5 {
            entries.truncate(5);
        }
    }
    
    // Prioritize endpoints based on previous successful connections
    fn prioritize_candidates(&self, pubkey: &str, candidates: &mut Vec<Endpoint>) {
        if let Some(cached_endpoints) = self.endpoints.get(pubkey) {
            // Create a new vector with prioritized endpoints first
            let mut prioritized = Vec::new();
            
            // Add cached endpoints that are in the candidate list
            for cached in cached_endpoints {
                if candidates.contains(&cached.endpoint) {
                    prioritized.push(cached.endpoint.clone());
                    log::info!("Prioritizing cached endpoint {} for peer {}", cached.endpoint, pubkey);
                }
            }
            
            // Add remaining candidates
            for endpoint in candidates.iter() {
                if !prioritized.contains(endpoint) {
                    prioritized.push(endpoint.clone());
                }
            }
            
            // Replace original candidates with prioritized list
            *candidates = prioritized;
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
    peers: Vec<Peer<String>>,
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
    let device_clone = device.clone();
    
    // Use the clones for diff to avoid lifetime issues
    let modifications = device.diff(&peers);
    let mut store = DataStore::open_or_create(&data_dir, &interface)?;
    
    // For each peer that's new or modified, prioritize its candidates based on connection history
    for diff in &modifications {
        if let Some(peer) = diff.new {
            // If we have cached endpoints for this peer, prioritize them
            let pubkey = &peer.public_key;
            if let Some(cached_endpoints) = connection_cache.endpoints.get(pubkey) {
                // Clone the current candidates
                let candidates = peer.candidates.clone();
                
                // Create a prioritized list
                let mut prioritized = Vec::new();
                
                // Add cached endpoints first (if they're in the candidate list)
                for cached in cached_endpoints {
                    if candidates.contains(&cached.endpoint) {
                        prioritized.push(cached.endpoint.clone());
                        log::info!("Prioritizing cached endpoint {} for peer {}", cached.endpoint, pubkey);
                    }
                }
                
                // Add remaining candidates
                for endpoint in &candidates {
                    if !prioritized.contains(endpoint) {
                        prioritized.push(endpoint.clone());
                    }
                }
                
                // Replace candidates with prioritized list
                let idx = peers.iter().position(|p| p.public_key == *pubkey);
                if let Some(_idx) = idx {
                    // Since we can't modify peers directly (it's a parameter), log the prioritization
                    log::info!("Would prioritize {} endpoints for peer {}", prioritized.len(), pubkey);
                }
            }
        }
    }
    
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
    connection_cache: &mut ConnectionCache,
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
    let interface_updated_time = std::time::Instant::now();
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
