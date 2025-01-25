use std::collections::HashMap;
use std::net::TcpListener;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;
use parking_lot::RwLock;
use std::time::Duration;
use std::{net::SocketAddr, ops::Deref};
use formnet_server::{crdt_service, ConfigFile, Endpoints, VERSION};
use formnet_server::{db::CrdtMap, CrdtContext, DatabasePeer};
use ipnet::IpNet;
use shared::{get_local_addrs, wg, Endpoint, NetworkOpts, PeerContents};
use wireguard_control::{Device, DeviceUpdate, InterfaceName, PeerConfigBuilder};
use hyper::{http, server::conn::AddrStream, Body, Request};
use crate::CONFIG_DIR;


pub async fn serve(
    interface: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let config_dir = PathBuf::from(CONFIG_DIR);
    log::debug!("opening database connection...");

    let mut peers = DatabasePeer::<String, CrdtMap>::list().await?;
    log::debug!("peers listed...");
    let peer_configs = peers
        .iter()
        .map(|peer| peer.deref().into())
        .collect::<Vec<PeerConfigBuilder>>();

    let interface_name = InterfaceName::from_str(interface)?;
    let network_opts = NetworkOpts::default();
    let config = ConfigFile::from_file(config_dir.join(interface).with_extension("conf"))?;
    log::info!("bringing up interface.");
    wg::up(
        &interface_name,
        &config.private_key,
        IpNet::new(config.address, config.network_cidr_prefix)?,
        Some(config.listen_port),
        None,
        network_opts,
    )?;

    DeviceUpdate::new()
        .add_peers(&peer_configs)
        .apply(&interface_name, network_opts.backend)?;

    log::info!("{} peers added to wireguard interface.", peers.len());

    let candidates: Vec<Endpoint> = get_local_addrs()?
        .map(|addr| SocketAddr::from((addr, config.listen_port)).into())
        .collect();
    let num_candidates = candidates.len();
    let myself = peers
        .iter_mut()
        .find(|peer| peer.ip == config.address)
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
    let endpoints = spawn_endpoint_refresher(interface_name, network_opts).await;
    spawn_expired_invite_sweeper().await;

    let context = CrdtContext {
        endpoints,
        interface: interface_name,
        public_key,
        backend: network_opts.backend,
    };

    log::info!("formnet-server {} starting.", VERSION);

    let listener = get_listener((config.address, config.listen_port).into(), &interface_name)?;

    let make_svc = hyper::service::make_service_fn(move |socket: &AddrStream| {
        let remote_addr = socket.remote_addr();
        let context = context.clone();
        async move {
            Ok::<_, http::Error>(hyper::service::service_fn(move |req: Request<Body>| {
                log::debug!("{} - {} {}", &remote_addr, req.method(), req.uri());
                crdt_service::hyper_service(req, context.clone(), remote_addr)
            }))
        }
    });

    let server = hyper::Server::from_tcp(listener)?.serve(make_svc);

    server.await?;

    Ok(())
}

fn get_listener(addr: SocketAddr, interface: &InterfaceName) -> Result<TcpListener, Box<dyn std::error::Error>> {
    let listener = TcpListener::bind(addr)?;
    listener.set_nonblocking(true)?;
    let sock = socket2::Socket::from(listener);
    sock.bind_device(Some(interface.as_str_lossy().as_bytes()))?;
    Ok(sock.into())
}


async fn spawn_endpoint_refresher(interface: InterfaceName, network: NetworkOpts) -> Endpoints {
    let endpoints = Arc::new(RwLock::new(HashMap::new()));
    tokio::task::spawn({
        let endpoints = endpoints.clone();
        async move {
            let mut interval = tokio::time::interval(Duration::from_secs(10));
            loop {
                interval.tick().await;
                if let Ok(info) = Device::get(&interface, network.backend) {
                    for peer in info.peers {
                        if let Some(endpoint) = peer.config.endpoint {
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
