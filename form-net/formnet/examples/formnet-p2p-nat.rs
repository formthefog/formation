use std::{
    collections::{HashMap, HashSet},
    net::{IpAddr, Ipv4Addr, SocketAddr},
    str::FromStr,
    sync::Arc,
    time::Duration,
};

use axum::{
    extract::{ConnectInfo, State},
    routing::{get, post, put},
    Json, Router,
};
use clap::Parser;
use ipnet::IpNet;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use shared::{get_local_addrs, wg, REDEEM_TRANSITION_WAIT};
use tokio::{net::TcpListener, sync::RwLock, time::interval};
use wireguard_control::{Backend, Device, DeviceUpdate, InterfaceName, Key, KeyPair, PeerConfigBuilder};

// Track both internal and external endpoints, plus NAT candidates
#[derive(Clone, Debug, Serialize, Deserialize)]
struct PeerInfo {
    pub pubkey: String,
    pub internal_ip: IpAddr,
    pub candidates: Vec<SocketAddr>,
}

// Bootstrap node maintains essential network information
#[derive(Clone, Debug, Serialize, Deserialize)]
struct BootstrapInfo {
    pub pubkey: String,
    pub internal_endpoint: IpAddr,
    pub external_endpoint: SocketAddr,
}

// Response types for different operations
#[derive(Clone, Debug, Serialize, Deserialize)]
enum Response {
    Join(JoinResponse),
    Bootstrap(BootstrapInfo),
    Ping,
    Error(String),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
enum JoinResponse {
    Success { ip: IpAddr },
    Failure { reason: String },
}

// State maintained by the bootstrap node
#[derive(Clone)]
struct BootstrapState {
    // Bootstrap node's own info
    info: BootstrapInfo,
    // Track assigned IPs
    used_ips: Arc<RwLock<HashSet<IpAddr>>>,
    // Track real endpoints seen from connections
    endpoints: Arc<RwLock<HashMap<String, SocketAddr>>>,
}

#[derive(Clone, Debug, Parser)]
struct Cli {
    #[clap(long, short)]
    bootstrap: Option<String>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    simple_logger::SimpleLogger::new().init().unwrap();
    
    let parser = Cli::parse();
    if let Some(bs) = parser.bootstrap {
        // Run as a joining peer
        peer_node(&bs).await?;
    } else {
        // Run as the bootstrap node
        bootstrap_node().await?;
    }

    Ok(())
}

// Bootstrap node setup and operation
async fn bootstrap_node() -> Result<(), Box<dyn std::error::Error>> {
    // Generate keys and get external IP
    let keypair = KeyPair::generate();
    let pubkey = keypair.public.to_base64();
    let external_ip = publicip::get_any(publicip::Preference::Ipv4)
        .ok_or("Could not determine external IP")?;

    // Create bootstrap info
    let info = BootstrapInfo {
        pubkey: pubkey.clone(),
        internal_endpoint: "10.0.0.1".parse()?,
        external_endpoint: SocketAddr::new(external_ip, 51820),
    };

    // Initialize WireGuard interface
    wg::up(
        &InterfaceName::from_str("formnet")?,
        &keypair.private.to_base64(),
        IpNet::new(info.internal_endpoint, 8)?,
        Some(51820),
        None,
        shared::NetworkOpts::default(),
    )?;

    // Create shared state
    let state = BootstrapState {
        info,
        used_ips: Arc::new(RwLock::new(HashSet::new())),
        endpoints: Arc::new(RwLock::new(HashMap::new())),
    };

    // Start endpoint refresh task
    spawn_endpoint_refresher(state.clone());

    // Start server
    server("0.0.0.0", 51820, state).await?;

    Ok(())
}

// Peer node setup and operation
async fn peer_node(bootstrap: &str) -> Result<(), Box<dyn std::error::Error>> {
    // Get bootstrap node info
    log::info!("Fetching bootstrap info from {bootstrap}");
    let bootstrap_info = Client::new()
        .get(format!("http://{bootstrap}/bootstrap"))
        .send()
        .await?
        .json::<Response>()
        .await?;

    let bootstrap_info = match bootstrap_info {
        Response::Bootstrap(info) => info,
        _ => return Err("Invalid bootstrap response".into()),
    };

    // Generate our keys
    let keypair = KeyPair::generate();
    
    // Try to join the network
    let peer_info = PeerInfo {
        pubkey: keypair.public.to_base64(),
        internal_ip: "0.0.0.0".parse()?, // Will be assigned by bootstrap
        candidates: vec![], // Will be updated after joining
    };

    log::info!("Sending join request to bootstrap node");
    let resp = Client::new()
        .put(format!("http://{bootstrap}/join"))
        .json(&peer_info)
        .send()
        .await?
        .json::<Response>()
        .await?;

    let assigned_ip = match resp {
        Response::Join(JoinResponse::Success { ip }) => ip,
        Response::Join(JoinResponse::Failure { reason }) => {
            return Err(format!("Failed to join: {reason}").into())
        }
        _ => return Err("Invalid join response".into()),
    };

    // Configure WireGuard with bootstrap node as peer
    wg::up(
        &InterfaceName::from_str("formnet")?,
        &keypair.private.to_base64(),
        IpNet::new(assigned_ip, 8)?,
        None,
        Some((
            &bootstrap_info.pubkey,
            bootstrap_info.internal_endpoint,
            bootstrap_info.external_endpoint,
        )),
        shared::NetworkOpts::default(),
    )?;
    std::thread::sleep(REDEEM_TRANSITION_WAIT);
    log::info!("Spawning candidate updates");

    // Start NAT candidate updates
    spawn_candidate_updates(bootstrap.to_string());

    // Test internal connectivity
    let mut interval = interval(Duration::from_secs(10));
    loop {
        interval.tick().await;
        let device = Device::get(&InterfaceName::from_str("formnet")?, Backend::default())?;
        for peer in device.peers {
            log::info!("Peer Info: {:?}", peer.config);
            log::info!("Peer Stats: {:?}", peer.stats);
        }
        match Client::new()
            .get(format!("http://{}:{}/ping", bootstrap_info.internal_endpoint, 51820))
            .send()
            .await
        {
            Ok(_) => log::info!("Successfully pinged bootstrap node over internal network"),
            Err(e) => log::error!("Failed to ping bootstrap: {e}"),
        }
        tokio::time::sleep(Duration::from_secs(5)).await;
    }
}

// Server setup and API routes
async fn server(
    address: &str,
    port: u16,
    state: BootstrapState,
) -> Result<(), Box<dyn std::error::Error>> {
    let state = Arc::new(state);

    let router = Router::new()
        .route("/bootstrap", get(get_bootstrap_info))
        .route("/join", put(handle_join))
        .route("/candidates", post(handle_candidates))
        .route("/ping", get(handle_ping))
        .with_state(state);

    let listener = TcpListener::bind(format!("{address}:{port}")).await?;
    axum::serve(listener, router.into_make_service_with_connect_info::<SocketAddr>()).await?;

    Ok(())
}

// Handler implementations
async fn handle_join(
    State(state): State<Arc<BootstrapState>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Json(peer_info): Json<PeerInfo>,
) -> Json<Response> {
    // Find next available IP
    let ip = {
        let mut used_ips = state.used_ips.write().await;
        let ip = (2..255)
            .map(|n| IpAddr::V4(Ipv4Addr::new(10, 0, 0, n)))
            .find(|ip| !used_ips.contains(ip))
            .expect("No IPs available");
        used_ips.insert(ip);
        ip
    };

    // Record real endpoint from connection
    state.endpoints.write().await.insert(peer_info.pubkey.clone(), addr);

    // Add to WireGuard interface
    let pubkey = Key::from_base64(&peer_info.pubkey).unwrap();
    let config_builder = PeerConfigBuilder::new(&pubkey)
        .replace_allowed_ips()
        .add_allowed_ip(ip, 32)
        .set_persistent_keepalive_interval(25)
        .set_endpoint(addr);

    if let Err(e) = DeviceUpdate::new()
        .add_peer(config_builder)
        .apply(&InterfaceName::from_str("formnet").unwrap(), Backend::default())
    {
        return Json(Response::Join(JoinResponse::Failure {
            reason: e.to_string(),
        }));
    }

    Json(Response::Join(JoinResponse::Success { ip }))
}

async fn handle_candidates(
    State(state): State<Arc<BootstrapState>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Json(candidates): Json<Vec<SocketAddr>>,
) -> Json<Response> {
    // Find peer by their current endpoint
    let endpoints = state.endpoints.read().await;
    if let Some((pubkey, _)) = endpoints.iter().find(|(_, ep)| **ep == addr) {
        // Update WireGuard peer configuration with new candidates
        let mut builder = PeerConfigBuilder::new(&Key::from_base64(pubkey).unwrap());
        if let Some(&candidate) = candidates.first() {
            builder = builder.set_endpoint(candidate);
        }

        if let Err(e) = DeviceUpdate::new()
            .add_peer(builder)
            .apply(&InterfaceName::from_str("formnet").unwrap(), Backend::default())
        {
            return Json(Response::Error(e.to_string()));
        }
    }
    Json(Response::Ping)
}

async fn get_bootstrap_info(
    State(state): State<Arc<BootstrapState>>,
) -> Json<Response> {
    Json(Response::Bootstrap(state.info.clone()))
}

async fn handle_ping() -> Json<Response> {
    Json(Response::Ping)
}

// Background tasks
fn spawn_endpoint_refresher(state: BootstrapState) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(10));
        loop {
            interval.tick().await;
            if let Ok(info) = Device::get(&InterfaceName::from_str("formnet").unwrap(), Backend::default())
            {
                let mut endpoints = state.endpoints.write().await;
                for peer in info.peers {
                    log::info!("Peer config: {:?}", peer.config);
                    log::info!("Peer stats: {:?}", peer.stats);
                    if let Some(endpoint) = peer.config.endpoint {
                        endpoints.insert(peer.config.public_key.to_base64(), endpoint);
                    }
                }
            }
        }
    });
}

fn spawn_candidate_updates(bootstrap: String) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(10));
        loop {
            interval.tick().await;
            // Get our local addresses as candidates
            if let Ok(device) = Device::get(&InterfaceName::from_str("formnet").unwrap(), Backend::default())
            {
                let candidates = get_local_addrs()
                    .unwrap()
                    .map(|addr| SocketAddr::new(addr, device.listen_port.unwrap_or(51820)))
                    .collect::<Vec<_>>();

                log::info!("Reporting candidates: {candidates:?}");

                if let Err(e) = Client::new()
                    .post(format!("http://{bootstrap}/candidates"))
                    .json(&candidates)
                    .send()
                    .await
                {
                    log::error!("Failed to update candidates: {e}");
                }
            }
        }
    });
}
