use std::collections::HashMap;
use std::net::TcpListener;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;
use form_types::PeerType;
use parking_lot::RwLock;
use reqwest::Client;
use tokio::time::interval;
use std::time::Duration;
use std::{net::SocketAddr, ops::Deref};
use formnet_server::{ConfigFile, Endpoints, VERSION};
use formnet_server::{db::CrdtMap, DatabasePeer};
use ipnet::IpNet;
use shared::{get_local_addrs, wg, Endpoint, NetworkOpts, PeerContents};
use wireguard_control::{Backend, Device, DeviceUpdate, InterfaceName, PeerConfigBuilder};
use crate::api::{server, BootstrapInfo, Response};
use crate::{fetch_server, CONFIG_DIR};

pub async fn serve(
    interface: &str,
    id: String,
    bootstrap: Vec<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let interface_name = InterfaceName::from_str(interface)?;
    #[cfg(target_os = "linux")]
    let interface_up = match Device::list(Backend::Kernel) {
        Ok(interfaces) => interfaces.iter().any(|name| name == &interface_name),
        _ => false,
    };
    #[cfg(not(target_os = "linux"))]
    let interface_up = match Device::list(Backend::Userspace) {
        Ok(interfaces) => interfaces.iter().any(|name| name == &interface_name),
        _ => false,
    };
    let config_dir = PathBuf::from(CONFIG_DIR);
    log::debug!("Getting peers...");

    log::info!("Sleeping for 5 seconds to allow data to propagate...");
    let _ = tokio::time::sleep(Duration::from_secs(5)).await;

    let mut peers: Vec<DatabasePeer<String, CrdtMap>> = vec![];
    if !bootstrap.is_empty() {
        let mut iter = bootstrap.iter();
        while let Some(bootstrap) = iter.next() {
            match Client::new()
                .get(format!("http://{bootstrap}:51820/fetch"))
                .send()
                .await {
                    Ok(resp) => match resp.json::<Response>().await {
                        Ok(Response::Fetch(p)) => { 
                            peers.extend(p.iter().map(|p| {
                                DatabasePeer::<String, CrdtMap>::from(p.clone())
                            }));
                        }
                        _ => {}
                    }
                    _ => {}
                }
        }
    } else {
        peers = DatabasePeer::<String, CrdtMap>::list().await?;
    }

    log::debug!("peers listed...");
    let peer_configs = peers
        .iter()
        .map(|peer| peer.deref().into())
        .collect::<Vec<PeerConfigBuilder>>();

    let network_opts = NetworkOpts::default();
    let config = ConfigFile::from_file(config_dir.join(interface).with_extension("conf"))?;
    if !interface_up {
        log::info!("bringing up interface.");
        if let Some(info) = config.bootstrap {
            let decoded = hex::decode(&info)?;
            let info: BootstrapInfo = serde_json::from_slice(&decoded)?;
            wg::up(
                &interface_name,
                &config.private_key,
                IpNet::new(config.address, config.network_cidr_prefix)?,
                config.listen_port,
                Some((
                    &info.pubkey,
                    info.internal_endpoint.unwrap(),
                    info.external_endpoint.unwrap(),
                )),
                network_opts,
            )?;
        } else {
            wg::up(
                &interface_name,
                &config.private_key,
                IpNet::new(config.address, config.network_cidr_prefix)?,
                config.listen_port,
                None,
                network_opts,
            )?;
        }
    }

    log::info!("Adding peers {peer_configs:?} to wireguard interface"); 
    DeviceUpdate::new()
        .add_peers(&peer_configs)
        .apply(&interface_name, network_opts.backend)?;

    log::info!("{} peers added to wireguard interface.", peers.len());

    let candidates: Vec<Endpoint> = get_local_addrs()?
        .map(|addr| SocketAddr::from((addr, config.listen_port.unwrap())).into())
        .collect();
    let num_candidates = candidates.len();
    let myself = peers
        .iter_mut()
        .find(|peer| peer.contents.ip == config.address)
        .expect("Couldn't find server peer in peer list.");

    myself.update(
        PeerContents {
            candidates,
            ..myself.contents.clone()
        },
    ).await?;

    log::info!(
        "{} local candidates added to server peer config.",
        num_candidates
    );

    let public_key = wireguard_control::Key::from_base64(&config.private_key)?.get_public();
    let _endpoints = spawn_endpoint_refresher(interface_name, network_opts).await;
    spawn_expired_invite_sweeper().await;
    log::info!("formnet-server {} starting.", VERSION);
    let publicip = publicip::get_any(publicip::Preference::Ipv4).ok_or(
        Box::new(
            std::io::Error::new(
                std::io::ErrorKind::Other,
                "Unable to acquire public ip, required for operator".to_string()
            )
        )
    )?;

    let my_info = BootstrapInfo {
        id,
        peer_type: PeerType::Operator,
        cidr_id: "formnet".to_string(),
        pubkey: public_key.to_base64(),
        internal_endpoint: Some(config.address),
        external_endpoint: Some(SocketAddr::new(publicip, 51820))
    };

    tokio::spawn(async move {
        let _ = Client::new()
            .post("http://127.0.0.1:53333/queue/joined_formnet")
            .send()
            .await;
    });

    tokio::spawn(async move {
        let _ = Client::new()
            .post("http://127.0.0.1:3004/bootstrap/joined_formnet")
            .send()
            .await;
    });

    tokio::spawn(async move {
        let mut interval = interval(Duration::from_secs(20));
        loop {
            interval.tick().await;
            if let Err(e) = fetch_server().await {
                log::error!("Error fetching peers from self: {e}");
            }
        }
    });

    server(my_info).await?;

    Ok(())
}

#[allow(unused)]
#[cfg(target_os = "linux")]
fn get_listener(addr: SocketAddr, interface: &InterfaceName) -> Result<TcpListener, Box<dyn std::error::Error>> {
    let listener = TcpListener::bind(addr)?;
    listener.set_nonblocking(true)?;
    let sock = socket2::Socket::from(listener);
    sock.bind_device(Some(interface.as_str_lossy().as_bytes()))?;
    Ok(sock.into())
}

#[cfg(not(target_os = "linux"))]
fn get_listener(addr: SocketAddr, _interface: &InterfaceName) -> Result<TcpListener, Box<dyn std::error::Error>> {
    let listener = TcpListener::bind(addr)?;
    listener.set_nonblocking(true)?;
    Ok(listener)
}


async fn spawn_endpoint_refresher(interface: InterfaceName, network: NetworkOpts) -> Endpoints {
    let endpoints = Arc::new(RwLock::new(HashMap::new()));
    tokio::task::spawn({
        log::info!("Spawning endpoint refresher");
        let endpoints = endpoints.clone();
        async move {
            let mut interval = tokio::time::interval(Duration::from_secs(10));
            loop {
                interval.tick().await;
                if let Ok(info) = Device::get(&interface, network.backend) {
                    log::info!("Acquired device info");
                    for peer in info.peers {
                        log::info!("Acquired device info for peer {peer:?}");
                        if let Some(endpoint) = peer.config.endpoint {
                            log::info!("Acquired endpoint {endpoint:?} for peer... Updating...");
                            endpoints
                                .write()
                                .insert(peer.config.public_key.to_base64(), endpoint);
                        }
                    }
                }
            }
        }
    });
    endpoints
}

async fn spawn_expired_invite_sweeper() {
    tokio::task::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(10));
        loop {
            interval.tick().await;
            match DatabasePeer::<String, CrdtMap>::delete_expired_invites().await {
                Ok(_) => {
                    log::info!("Deleted expired peer invitations.")
                },
                Err(e) => log::error!("Failed to delete expired peer invitations: {}", e),
            }
        }
    });
}

/*
pub fn create_router() -> axum::Router {
    axum::Router::new()
        .route("/user/redeem", axum::routing::post(handle_redeem))
        .route("/admin/", axum::routing::post(handle_redeem))
        //TODO: Add routes to request custom cidr, request custom assoc
        //Add routes to delete peer, delete custom cidr, delete assoc
}

async fn handle_redeem(
) {}
*/
