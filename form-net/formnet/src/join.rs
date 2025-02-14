use std::{collections::{HashSet, VecDeque}, net::{IpAddr, SocketAddr, TcpListener}, path::PathBuf, process::Command, str::FromStr, time::Duration};
use form_types::{BootCompleteRequest, PeerType, VmmResponse};
use formnet_server::ConfigFile;
use futures::{stream::FuturesUnordered, StreamExt};
use ipnet::IpNet;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use shared::{interface_config::InterfaceConfig, wg, Endpoint, NetworkOpts};
use socket2::Socket;
use url::Host;
use wireguard_control::{Device, InterfaceName, KeyPair};
use crate::{api::{BootstrapInfo, JoinResponse as BootstrapResponse, Response}, up, CONFIG_DIR, DATA_DIR, NETWORK_NAME};


#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum JoinRequest {
    UserJoinRequest(UserJoinRequest),
    OperatorJoinRequest(OperatorJoinRequest),
    InstanceJoinRequest(VmJoinRequest),
}

impl JoinRequest {
    pub fn id(&self) -> String {
        match self {
            Self::UserJoinRequest(req) => req.user_id.clone(),
            Self::OperatorJoinRequest(req) => req.operator_id.clone(),
            Self::InstanceJoinRequest(req) => req.vm_id.clone(),
        }
    }

    pub fn peer_type(&self) -> PeerType {
        match self {
            Self::UserJoinRequest(_) => PeerType::User,
            Self::OperatorJoinRequest(_) => PeerType::Operator,
            Self::InstanceJoinRequest(_) => PeerType::Instance,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VmJoinRequest {
    pub vm_id: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OperatorJoinRequest {
    pub operator_id: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UserJoinRequest {
    pub user_id: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum JoinResponse {
    Success {
        #[serde(flatten)]
        invitation: InterfaceConfig,
    },
    Error(String) 
}

pub async fn request_to_join(bootstrap: Vec<String>, address: String, peer_type: PeerType) -> Result<IpAddr, Box<dyn std::error::Error>> {
    if let Err(e) = std::fs::remove_file(PathBuf::from(DATA_DIR).join("formnet").with_extension("json")) {
        log::error!("Pre-existing datastore did not exist: {e}"); 
    }
    let client = Client::new();
    let mut iter = bootstrap.iter();
    let mut bootstrap_info: Option<BootstrapInfo> = None;
    while let Some(dial) = iter.next() {
        match client.get(format!("http://{dial}:51820/bootstrap"))
            .send().await {
                Ok(resp) => match resp.json::<Response>().await {
                    Ok(Response::Bootstrap(info)) => {
                        bootstrap_info = Some(info.clone());
                        break;
                    }
                    Err(e) => {
                        log::error!("Error deserializing response from {dial}: {e}");
                        continue;
                    }
                    _ => {
                        log::error!("Recieved invalid variant for join request");
                        continue;
                    }
                }
                Err(e) => {
                    log::error!("Error dialing {dial}: {e}");
                }
            }
    }

    if bootstrap_info.is_none() {
        return Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, "Was unable to acquire bootstrap information from any bootstrap nodes provided")));
    }

    let bootstrap_info = bootstrap_info.unwrap(); 

    let keypair = KeyPair::generate();
    let publicip = publicip::get_any(
        publicip::Preference::Ipv4
    ).ok_or(
        Box::new(
            std::io::Error::new(
                std::io::ErrorKind::Other,
                    "unable to acquire public ip"
            )
        )
    )?;


    let mut common_endpoints: FuturesUnordered<_> = bootstrap.iter().map(|dial| {
        let inner_client = client.clone();
        let inner_address = address.clone();
        async move {
            match inner_client.get(format!("http://{dial}:51820/fetch"))
                .send().await {
                    Ok(resp) => match resp.json::<Response>().await {
                    Ok(Response::Fetch(peers)) => {
                        let common_endpoint = peers.iter().filter_map(|p| {
                            p.endpoint.clone() 
                        }).collect::<Vec<Endpoint>>().iter().find_map(|ep| {
                            match ep.resolve() {
                                Ok(addr) => {
                                    if addr.ip() == publicip {
                                        Some(addr)
                                    } else {
                                        None
                                    }
                                }
                                Err(_) => None 
                            }
                        });

                        let common_id = peers.iter().find_map(|p| {
                            if &p.id == &inner_address {
                                match &p.endpoint {
                                    Some(endpoint) => {
                                        match endpoint.resolve() {
                                            Ok(addr) => Some(addr),
                                            Err(_) => None,
                                        }
                                    }
                                    None => None,
                                }
                            } else {
                                None
                            }
                        });
                        (common_endpoint, common_id)
                    }
                    Err(_) => {
                        (None, None)
                    }
                    _ => (None, None)
            }
            Err(_) => (None, None)
        }
    }}).collect();

    let mut complete_common_endpoints = vec![];
    while let Some(complete) = common_endpoints.next().await {
        complete_common_endpoints.push(complete);
    };

    let ports_used = complete_common_endpoints.iter().filter_map(|ce| {
        match ce {
            (None, None) => None,
            (Some(e1), None) => Some(vec![e1.clone()]),
            (None, Some(e2)) => Some(vec![e2.clone()]),
            (Some(e1), Some(e2)) => Some(vec![e1.clone(), e2.clone()])
        }
    }).collect::<Vec<Vec<SocketAddr>>>()
    .iter().flatten().cloned().collect::<Vec<SocketAddr>>()
    .iter().map(|addr| addr.port()).collect::<HashSet<u16>>();

    let mut next_port = (51820..64000).collect::<VecDeque<u16>>();
    next_port.retain(|n| !ports_used.contains(n));

    if next_port.len() == 0 {
        return Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, "There are no ports available on this device")));
    }

    let next_port = next_port.pop_front().unwrap();

    let request = match peer_type { 
        PeerType::Operator => {
            BootstrapInfo {
                id: address.to_string(),
                peer_type: PeerType::Operator,
                cidr_id: "formnet".to_string(),
                pubkey: keypair.public.to_base64(),
                internal_endpoint: None,
                external_endpoint: Some(
                    SocketAddr::new(publicip, 51820)
                ),
            }
        },
        PeerType::User => {
            BootstrapInfo {
                id: address.to_string(),
                peer_type: PeerType::User,
                cidr_id: "formnet".to_string(),
                pubkey: keypair.public.to_base64(),
                internal_endpoint: None,
                external_endpoint: Some(SocketAddr::new(publicip, next_port))
            }
        },
        PeerType::Instance => {
            BootstrapInfo {
                id: address.to_string(),
                peer_type: PeerType::Instance,
                cidr_id: "formnet".to_string(),
                pubkey: keypair.public.to_base64(),
                internal_endpoint: None,
                external_endpoint: Some(SocketAddr::new(publicip, next_port))
            }
        }
    };

    log::info!("Built join request: {request:?}");

    let mut iter = bootstrap.iter();
    while let Some(dial) = iter.next() {
        log::info!("Attemptiing to dial {dial} to request to join the network");
        match Client::new().post(&format!("http://{dial}:51820/join"))
        .json(&request)
        .send()
        .await {
            Ok(response) => match response.json::<Response>().await {
                Ok(Response::Join(BootstrapResponse::Success(ip))) => {
                    log::info!("Bringing Wireguard interface up...");
                    match wg::up(
                        &InterfaceName::from_str("formnet")?,
                        &keypair.private.to_base64(), 
                        IpNet::new(ip.clone(), 8)?,
                        Some(request.external_endpoint.unwrap().port()),
                        Some((
                            &bootstrap_info.pubkey,
                            bootstrap_info.internal_endpoint.unwrap(),
                            bootstrap_info.external_endpoint.unwrap(),
                        )), 
                        NetworkOpts::default(),
                    ) {
                        Ok(()) => {
                            let config_file = ConfigFile {
                                private_key: keypair.private.to_base64(),
                                address: ip.clone(),
                                listen_port: Some(request.external_endpoint.unwrap().port()),
                                network_cidr_prefix: 8,
                                bootstrap: Some(hex::encode(&serde_json::to_vec(&bootstrap_info)?)) 
                            };
                            std::fs::create_dir_all(PathBuf::from(CONFIG_DIR))?;
                            config_file.write_to_path(
                                PathBuf::from(CONFIG_DIR).join(NETWORK_NAME).with_extension("conf")
                            )?;
                            log::info!("Wireguard interface is up, saved config file");
                            #[cfg(target_os = "linux")]
                            if let Ok(info) = Device::get(&InterfaceName::from_str("formnet").unwrap(), wireguard_control::Backend::Kernel) {
                                log::info!("Current device info: {info:?}");
                                for peer in info.peers {
                                    log::info!("Acquired device info for peer {peer:?}");
                                    if let Some(endpoint) = peer.config.endpoint {
                                        log::info!("Acquired endpoint {endpoint:?} for peer..."); 
                                    }
                                }
                            }
                            #[cfg(not(target_os = "linux"))]
                            if let Ok(info) = Device::get(&InterfaceName::from_str("formnet").unwrap(), wireguard_control::Backend::Userspace) {
                                log::info!("Current device info: {info:?}");
                                for peer in info.peers {
                                    log::info!("Acquired device info for peer {peer:?}");
                                    if let Some(endpoint) = peer.config.endpoint {
                                        log::info!("Acquired endpoint {endpoint:?} for peer..."); 
                                    }
                                }
                            }
                            return Ok(ip.clone());
                        }
                        Err(e) => {
                            return Err(Box::new(e))
                        }
                    }
                }
                Err(e) => {
                    log::error!("Error attempting to join network: {e}");
                }
                Ok(r) => {
                    log::error!("Received invalid response type when trying to join network: {r:?}");
                }
            }
            Err(e) => {
                log::error!("Didn't receive a response: {e}")
            }
        }
    }
    log::info!("Didn't receive a valid response from any bootstraps, unable to join formnet: {bootstrap:?}");
    return Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, "Did not receive a valid invitation")));
}

pub async fn user_join_formnet(address: String, provider: String) -> Result<(), Box<dyn std::error::Error>> {
    request_to_join(vec![provider], address, PeerType::User).await?;
    Ok(())
}

pub async fn vm_join_formnet() -> Result<(), Box<dyn std::error::Error>> {
    let host_public_ip = std::env::var("HOST_BRIDGE_IP").unwrap(); 
    log::info!("HOST IP: {host_public_ip}");

    let name = std::fs::read_to_string("/etc/vm_name")?;
    let build_id = std::fs::read_to_string("/etc/build_id")?;
    match request_to_join(vec![host_public_ip.clone()], name.clone(), form_types::PeerType::Instance).await {
        Ok(ip)=> {
            log::info!("Received invitation");
            let formnet_ip = ip; 
            log::info!("extracted formnet IP for {name}: {formnet_ip}");
            log::info!("Attempting to redeem invite");
            log::info!("Spawning thread to bring formnet up");
            let handle = tokio::spawn(async move {
                if let Err(e) = up(
                    Some(Duration::from_secs(60)),
                    None,
                ).await {
                    log::error!("Error bringing formnet up: {e}");
                }
            });

            log::info!("Building request to inform VMM service that the boot process has completed for {name}");
            // Send message to VMM api.
            let request = BootCompleteRequest {
                name: name.clone(),
                build_id: build_id.clone(),
                formnet_ip: formnet_ip.to_string()
            };

            log::info!("Sending BootCompleteRequest {request:?} to http://{host_public_ip}:3002/vm/boot_complete endpoint");

            match Client::new().post(&format!("http://{host_public_ip}:3002/vm/boot_complete"))
                .json(&request)
                .send()
                .await {

                Ok(r) => {
                    log::info!("recevied response from {host_public_ip}:3002");
                    log::info!("Response: {r:?}");
                    log::info!("Response status: {:?}", r.status());
                    log::info!("Response contents: {:?}", r.json::<VmmResponse>().await?);
                }
                Err(e) => {
                    log::info!("Error sending BootCompleteRequest to {host_public_ip}:3002: {e}");
                }
            }


            log::info!("Formnet up, awaiting kill signal");
            handle.await?;

            Ok(())
        },
        Err(reason) => {
            log::info!("Error trying to join formnet: {reason}");
            return Err(other_err(&reason.to_string()))
        }
    }
}

pub fn other_err(msg: &str) -> Box<dyn std::error::Error> {
    Box::new(
        std::io::Error::new(
            std::io::ErrorKind::Other,
            msg
        )
    )
}
