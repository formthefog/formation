use std::{net::{IpAddr, SocketAddr}, str::FromStr, time::{Duration, SystemTime}};
use form_types::PeerType;
use formnet_server::{db::CrdtMap, DatabaseCidr, DatabasePeer, ServerError};
use ipnet::IpNet;
use shared::{interface_config::{InterfaceConfig, InterfaceInfo, ServerInfo}, Cidr, Hostname, IpNetExt, NetworkOpts, Peer, PeerContents, Timestring, PERSISTENT_KEEPALIVE_INTERVAL_SECS, REDEEM_TRANSITION_WAIT};
use wireguard_control::{Device, DeviceUpdate, InterfaceName, KeyPair, PeerConfigBuilder};

use crate::NETWORK_NAME;

pub async fn add_peer(
    _network: &NetworkOpts,
    peer_type: &PeerType,
    peer_id: &str,
    client_endpoint_info: Option<SocketAddr>,
    client_pubkey: String,
    _client_conn_addr: SocketAddr,
) -> Result<shared::interface_config::InterfaceConfig, Box<dyn std::error::Error>> {
    log::warn!("ATTEMPTING TO ADD PEER {peer_id}...");
    let interface_name = InterfaceName::from_str(NETWORK_NAME)?;

    let peers_from_db = DatabasePeer::<String, CrdtMap>::list().await?
        .into_iter()
        .map(|dp| dp.inner)
        .collect::<Vec<_>>();

    let root_cidr_obj = DatabaseCidr::<String, CrdtMap>::get(NETWORK_NAME.to_string()).await
        .map_err(|e| Box::new(std::io::Error::new(std::io::ErrorKind::NotFound, format!("Root CIDR '{}' not found in datastore: {}", NETWORK_NAME, e))))?;
    let root_ipnet = root_cidr_obj.cidr;

    if let Some(existing_peer) = peers_from_db.iter().find(|p| p.id == peer_id) {
        if existing_peer.public_key != client_pubkey {
            return Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, "Peer ID Match: peer already exists, but public key provided does not match")));
        }
        let server_peer_info = peers_from_db.iter().find(|p| p.is_admin)
            .ok_or_else(|| Box::new(std::io::Error::new(std::io::ErrorKind::NotFound, "Server peer info not found")))?;
        
        let server_api_socket_addr = SocketAddr::new(server_peer_info.ip, 51820);

        return Ok(InterfaceConfig {
            interface: InterfaceInfo {
                network_name: NETWORK_NAME.to_string(),
                address: IpNet::new(existing_peer.ip, root_ipnet.prefix_len())?,
                private_key: String::new(),
                listen_port: client_endpoint_info.map(|s| s.port()),
            },
            server: ServerInfo {
                public_key: server_peer_info.public_key.clone(),
                external_endpoint: server_peer_info.endpoint.clone().ok_or_else(|| Box::new(std::io::Error::new(std::io::ErrorKind::NotFound, "Server external endpoint not found")))?, 
                internal_endpoint: server_api_socket_addr,
            },
        });
    }

    let peer_contents_req = build_peer(
        &peers_from_db,
        peer_type,
        peer_id,
        client_endpoint_info,
        client_pubkey.clone(),
        _client_conn_addr,
        &root_ipnet
    ).await?;

    let assigned_ip = peer_contents_req.ip;

    let _peer_db_entry = DatabasePeer::<String, CrdtMap>::create(peer_contents_req).await?;

    let mut server_peer_info_opt = peers_from_db.iter().find(|p| p.is_admin).cloned();
    if server_peer_info_opt.is_none() {
        if let Ok(updated_peers) = DatabasePeer::<String, CrdtMap>::list().await {
            server_peer_info_opt = updated_peers.into_iter().find(|p|p.is_admin).map(|p|p.inner);
        }
    }
    let server_peer_info = server_peer_info_opt
        .ok_or_else(|| Box::new(std::io::Error::new(std::io::ErrorKind::NotFound, "Server peer info not found after peer creation")))?;
    
    let server_api_socket_addr = SocketAddr::new(server_peer_info.ip, 51820);

    Ok(InterfaceConfig {
        interface: InterfaceInfo {
            network_name: NETWORK_NAME.to_string(),
            address: IpNet::new(assigned_ip, root_ipnet.prefix_len())?,
            private_key: String::new(),
            listen_port: client_endpoint_info.map(|s| s.port()),
        },
        server: ServerInfo {
            public_key: server_peer_info.public_key.clone(),
            external_endpoint: server_peer_info.endpoint.clone().ok_or_else(|| Box::new(std::io::Error::new(std::io::ErrorKind::NotFound, "Server external endpoint not found")))?, 
            internal_endpoint: server_api_socket_addr,
        },
    })
}

pub async fn build_peer(
    peers: &[Peer<String>],
    peer_type: &PeerType,
    peer_id: &str,
    endpoint: Option<SocketAddr>,
    pubkey: String,
    _addr: SocketAddr,
    root_ipnet: &IpNet
) -> Result<PeerContents<String>, Box<dyn std::error::Error>> {
    let mut available_ip = None;
    let candidate_ips = root_ipnet.hosts().filter(|ip| root_ipnet.is_assignable(ip));
    for ip in candidate_ips {
        if !peers.iter().any(|peer| peer.ip == ip) {
            available_ip = Some(ip);
            break;
        }
    }
    let available_ip = available_ip.ok_or_else(|| Box::new(std::io::Error::new(std::io::ErrorKind::Other, "No IPs in this CIDR are available")))?;

    let is_admin = matches!(peer_type, PeerType::Operator);

    let invite_expires: Timestring = "1d".parse()?;
    let invite_duration: Duration = invite_expires.into();

    Ok(PeerContents {
        name: Hostname::from_str(peer_id)?,
        ip: available_ip,
        cidr_id: NETWORK_NAME.to_string(),
        public_key: pubkey,
        endpoint: endpoint.map(Into::into),
        is_admin,
        is_disabled: false,
        is_redeemed: true,
        persistent_keepalive_interval: Some(PERSISTENT_KEEPALIVE_INTERVAL_SECS),
        invite_expires: Some(SystemTime::now() + invite_duration),
        candidates: vec![],
    })
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
