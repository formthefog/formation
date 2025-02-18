use std::{net::{IpAddr, SocketAddr}, str::FromStr, time::{Duration, SystemTime}};
use form_types::PeerType;
use formnet_server::{db::CrdtMap, DatabaseCidr, DatabasePeer, ServerError};
use ipnet::IpNet;
use shared::{interface_config::{InterfaceConfig, InterfaceInfo, ServerInfo}, Cidr, Endpoint, Hostname, IpNetExt, NetworkOpts, Peer, PeerContents, Timestring, PERSISTENT_KEEPALIVE_INTERVAL_SECS, REDEEM_TRANSITION_WAIT};
use wireguard_control::{Device, DeviceUpdate, InterfaceName, KeyPair, PeerConfigBuilder};

use crate::NETWORK_NAME;

pub async fn add_peer(
    network: &NetworkOpts,
    peer_type: &PeerType,
    peer_id: &str,
    pubkey: String,
    addr: SocketAddr,
) -> Result<IpAddr, Box<dyn std::error::Error>> {
    log::warn!("ATTEMPTING TO ADD PEER {peer_id}...");
    log::info!("Getting config from file...");
    log::info!("Getting interface name...");
    let interface = InterfaceName::from_str(NETWORK_NAME)?;

    log::info!("Gathering peers...");
    let mut peers = DatabasePeer::<String, CrdtMap>::list().await?
        .into_iter()
        .map(|dp| dp.inner)
        .collect::<Vec<_>>();

    if let Some(ref mut peer) = peers.iter_mut().find(|p| p.id == peer_id) {
        if peer.public_key != pubkey {
            return Err(
                Box::new(
                    std::io::Error::new(
                        std::io::ErrorKind::Other, 
                        "Peer ID Match: peer already exists, but public key provided does not match"
                    )
                )
            );
        }

        return Ok(peer.ip.clone());
    }

    log::info!("Building peer...");
    let peer_request = build_peer(
        &peers,
        &peer_type,
        peer_id,
        pubkey,
        addr
    ).await?; 

    let ip = peer_request.ip;

    log::info!("Built peer, attempting to add peer {peer_id} to datastore");
    let peer = DatabasePeer::<String, CrdtMap>::create(peer_request).await?;
    log::info!("Added peer {peer_id} to datastore");
    if Device::get(&interface, network.backend).is_ok() {
        // Update the current WireGuard interface with the new peers.
        log::info!("Adding peer to device");
        DeviceUpdate::new()
            .add_peer(PeerConfigBuilder::from(&*peer))
            .apply(&interface, network.backend)
            .map_err(|_| ServerError::WireGuard)?;
        tokio::time::sleep(REDEEM_TRANSITION_WAIT).await;

        log::info!("adding to WireGuard interface: {}", &*peer);
    }

    return Ok(ip)
}

pub async fn build_peer(
    peers: &[Peer<String>],
    peer_type: &PeerType,
    peer_id: &str,
    pubkey: String,
    _addr: SocketAddr,
) -> Result<PeerContents<String>, Box<dyn std::error::Error>> {
    let cidr = DatabaseCidr::<String, CrdtMap>::get("formnet".to_string()).await?; 
    let mut available_ip = None;
    let candidate_ips = cidr.hosts().filter(|ip| cidr.is_assignable(ip));
    for ip in candidate_ips {
        if !peers.iter().any(|peer| peer.ip == ip) {
            available_ip = Some(ip);
            break;
        }
    }

    let available_ip = available_ip.expect("No IPs in this CIDR are avavilable");

    log::info!("Checking valid host name for {peer_id}");
    let is_admin = match peer_type {
        PeerType::Operator => true,
        _ => false,
    }; 

    let invite_expires: Timestring = "1d".parse()?;
    let invite_expires: Duration = invite_expires.into();

    let peer_request = PeerContents {
        name: Hostname::from_str(peer_id)?,
        ip: available_ip,
        cidr_id: cidr.id.clone(),
        public_key: pubkey,
        endpoint: None,
        is_admin,
        is_disabled: false,
        is_redeemed: true,
        persistent_keepalive_interval: Some(PERSISTENT_KEEPALIVE_INTERVAL_SECS),
        invite_expires: Some(SystemTime::now() + invite_expires),
        candidates: vec![],
    };

    Ok(peer_request)
}


pub fn build_peer_invitation(
    network_name: &InterfaceName,
    peer: &Peer<String>,
    server_peer: &Peer<String>,
    root_cidr: &Cidr<String>,
    keypair: KeyPair,
    server_api_addr: &SocketAddr,
) -> Result<InterfaceConfig, Box<dyn std::error::Error>> {
    let peer_invitation = InterfaceConfig {
        interface: InterfaceInfo {
            network_name: network_name.to_string(),
            private_key: keypair.private.to_base64(),
            address: IpNet::new(peer.ip, root_cidr.max_prefix_len())?,
            listen_port: None,
        },
        server: ServerInfo {
            external_endpoint: server_peer
                .endpoint
                .clone()
                .expect("The innernet server should have a WireGuard endpoint"),
            internal_endpoint: *server_api_addr,
            public_key: server_peer.public_key.clone(),
        },
    };

    Ok(peer_invitation)
}
