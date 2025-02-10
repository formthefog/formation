use std::{net::{IpAddr, SocketAddr}, str::FromStr, sync::Arc, time::Duration};

use axum::{extract::State, routing::{put, get}, Json, Router};
use clap::Parser;
use ipnet::IpNet;
use reqwest::Client;
use shared::{wg, NetworkOpts};
use tokio::{net::TcpListener, sync::RwLock};
use wireguard_control::{Backend, Device, DeviceUpdate, InterfaceName, Key, KeyPair, PeerConfigBuilder};
use serde::{Serialize, Deserialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
enum JoinResponse {
    Success,
    Failure
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct BootstrapInfo {
    pub pubkey: String,
    pub internal_endpoint: IpAddr,
    pub external_endpoint: SocketAddr,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct PeerInfo {
    pub pubkey: String,
    pub internal_endpoint: IpAddr,
    pub external_endpoint: SocketAddr,
}

#[derive(Clone, Debug, Parser)]
pub struct Cli {
    #[clap(long, short)]
    bootstrap: Option<String>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let parser = Cli::parse();
    if let Some(bs) = parser.bootstrap {
        peer_wg_up(&bs).await?;
    } else {
        bootstrap_wg_up().await?;
    }

    Ok(())
}

async fn bootstrap_wg_up() -> Result<(), Box<dyn std::error::Error>> {
    let keypair = KeyPair::generate(); 
    let pubkey = keypair.public.to_base64();
    let endpoint = publicip::get_any(publicip::Preference::Ipv4);
    let info = BootstrapInfo {
        pubkey,
        internal_endpoint: "10.0.0.1".parse()?,
        external_endpoint: SocketAddr::new(endpoint.unwrap(), 51820)
    };
    wg::up(
        &InterfaceName::from_str("formnet")?,
        &keypair.private.to_base64(), 
        IpNet::new(info.internal_endpoint, 8)?,
        Some(51820),
        None, 
        NetworkOpts::default(),
    )?;

    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(10));
        loop {
            interval.tick().await;
            if let Ok(info) = Device::get(&InterfaceName::from_str("formnet").unwrap(), Backend::Kernel) {
                log::info!("Acquired device info");
                for peer in info.peers {
                    log::info!("Acquired device info for peer {peer:?}");
                    if let Some(endpoint) = peer.config.endpoint {
                        log::info!("Acquired endpoint {endpoint:?} for peer... Updating...");
                    }
                }
            }
        }
    });

    server("0.0.0.0", 51820, info).await?;

    Ok(())
}

async fn peer_wg_up(bootstrap: &str) -> Result<(), Box<dyn std::error::Error>> {

    let bootstrap_info = Client::new()
        .get(format!("http://{}/bootstrap", bootstrap))
        .send().await?.json::<BootstrapInfo>().await?;

    let keypair = KeyPair::generate(); 

    let peer_info = PeerInfo {
        pubkey: keypair.public.to_base64(),
        internal_endpoint: "10.0.0.2".parse()?,
        external_endpoint: SocketAddr::new(
            publicip::get_any(
                publicip::Preference::Ipv4
            ).unwrap(),
            51820
        )
    };

    wg::up(
        &InterfaceName::from_str("formnet")?,
        &keypair.private.to_base64(), 
        IpNet::new("10.0.0.2".parse()?, 8)?,
        None,
        Some((
            &bootstrap_info.pubkey,
            bootstrap_info.internal_endpoint,
            bootstrap_info.external_endpoint,
        )), 
        NetworkOpts::default(),
    )?;

    let resp = Client::new()
        .put(format!("http://{}/join", bootstrap))
        .json(&peer_info)
        .send().await?
        .json::<JoinResponse>().await?;

    println!("Response: {resp:?}");

    Ok(())
}

async fn server(address: &str, port: u16, bootstrap_info: BootstrapInfo) -> Result<(), Box<dyn std::error::Error>> {
    let router = Router::new()
        .route("/bootstrap", get(get_bootstrap_info))
        .route("/join", put(handle_join))
        .with_state(Arc::new(RwLock::new(bootstrap_info)));

    let listener = TcpListener::bind(format!("{}:{}", address, port)).await?;

    axum::serve(listener, router).await?;

    Ok(())
}

async fn handle_join(
    Json(peer_info): Json<PeerInfo>
) -> Json<JoinResponse> {
    let pubkey = Key::from_base64(&peer_info.pubkey).unwrap();
    let config_builder = PeerConfigBuilder::new(&pubkey)
        .replace_allowed_ips()
        .add_allowed_ip(peer_info.internal_endpoint, 32);

    DeviceUpdate::new()
        .add_peer(config_builder)
        .apply(
            &InterfaceName::from_str("formnet").unwrap(), 
            Backend::Kernel
        ).unwrap();

    Json(JoinResponse::Success)
}

async fn get_bootstrap_info(
    State(info): State<Arc<RwLock<BootstrapInfo>>>,
) -> Json<BootstrapInfo> {
    let info_clone = info.read().await.clone();
    Json(info_clone)
}
