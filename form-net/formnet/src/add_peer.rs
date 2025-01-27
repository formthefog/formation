use std::{net::SocketAddr, path::PathBuf, str::FromStr, time::{Duration, SystemTime}};
use form_types::PeerType;
use formnet_server::{db::CrdtMap, ConfigFile, DatabaseCidr, DatabasePeer, ServerError};
use ipnet::IpNet;
use shared::{interface_config::{InterfaceConfig, InterfaceInfo, ServerInfo}, Cidr, CidrTree, Hostname, IpNetExt, NetworkOpts, Peer, PeerContents, Timestring, PERSISTENT_KEEPALIVE_INTERVAL_SECS};
use wireguard_control::{Device, DeviceUpdate, InterfaceName, KeyPair, PeerConfigBuilder};

use crate::{CONFIG_DIR, NETWORK_NAME};

pub async fn add_peer(
    network: &NetworkOpts,
    peer_type: &PeerType,
    peer_id: &str,
) -> Result<InterfaceConfig, Box<dyn std::error::Error>> {
    let config = ConfigFile::from_file(PathBuf::from(CONFIG_DIR).join(NETWORK_NAME).with_extension("conf"))?;
    let interface = InterfaceName::from_str(NETWORK_NAME)?;
    let peers = DatabasePeer::<String, CrdtMap>::list().await?
        .into_iter()
        .map(|dp| dp.inner)
        .collect::<Vec<_>>();
    let cidrs = DatabaseCidr::<String, CrdtMap>::list().await?;
    let cidr_tree = CidrTree::new(&cidrs[..]);
    let server_id = {
        match peers.iter().find(|p| p.is_admin) {
            Some(peer) => {
                &peer.id
            }
            None => {
                return Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, "No admins, no servers, cannot add peer")));
            }
        }
    };

    let (peer_request, keypair) = build_peer(
        &peers,
        &peer_type,
        peer_id
    ).await?; 
    log::info!("Received results from prompts, attempting to create peer in database");
    let peer = DatabasePeer::<String, CrdtMap>::create(peer_request).await?;
    if cfg!(not(test)) && Device::get(&interface, network.backend).is_ok() {
        // Update the current WireGuard interface with the new peers.
        DeviceUpdate::new()
            .add_peer(PeerConfigBuilder::from(&*peer))
            .apply(&interface, network.backend)
            .map_err(|_| ServerError::WireGuard)?;

        log::info!("adding to WireGuard interface: {}", &*peer);
    }

    let server_peer = DatabasePeer::<String, CrdtMap>::get(server_id.clone()).await?;
    let invitation = build_peer_invitation(
        &interface,
        &peer,
        &server_peer,
        &cidr_tree,
        keypair,
        &SocketAddr::new(config.address, config.listen_port),
    )?;
    return Ok(invitation)
}

pub async fn build_peer(
    peers: &[Peer<String>],
    peer_type: &PeerType,
    peer_id: &str,
) -> Result<(PeerContents<String>, KeyPair), Box<dyn std::error::Error>> {
    let cidr = DatabaseCidr::<String, CrdtMap>::get("peer-1".to_string()).await?; 
    let mut available_ip = None;
    let candidate_ips = cidr.hosts().filter(|ip| cidr.is_assignable(ip));
    for ip in candidate_ips {
        if !peers.iter().any(|peer| peer.ip == ip) {
            available_ip = Some(ip);
            break;
        }
    }

    let available_ip = available_ip.expect("No IPs in this CIDR are avavilable");

    let name = peer_id.to_string(); 

    valid_hostname(&Hostname::from_str(&name)?, &peer_type)?;
    let is_admin = match peer_type {
        PeerType::Operator => true,
        _ => false,
    }; 

    let invite_expires: Timestring = "1d".parse()?;
    let invite_expires: Duration = invite_expires.into();

    let default_keypair = KeyPair::generate();
    let peer_request = PeerContents {
        name: Hostname::from_str(&name)?,
        ip: available_ip,
        cidr_id: cidr.id.clone(),
        public_key: default_keypair.public.to_base64(),
        endpoint: None,
        is_admin,
        is_disabled: false,
        is_redeemed: false,
        persistent_keepalive_interval: Some(PERSISTENT_KEEPALIVE_INTERVAL_SECS),
        invite_expires: Some(SystemTime::now() + invite_expires),
        candidates: vec![],
    };

    Ok((peer_request, default_keypair))
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
            address: IpNet::new(peer.ip, root_cidr.prefix_len())?,
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

pub fn valid_hostname(_name: &Hostname, _peer_type: &PeerType) -> Result<(), Box<dyn std::error::Error>> {
    Ok(())
}
