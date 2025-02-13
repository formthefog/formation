use std::{net::{IpAddr, SocketAddr, TcpListener}, path::PathBuf, str::FromStr, time::Duration};
use colored::*;
use daemonize::Daemonize;
use form_types::{BootCompleteRequest, PeerType, VmmResponse};
use formnet_server::ConfigFile;
use ipnet::IpNet;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use shared::{interface_config::InterfaceConfig, wg, NetworkOpts};
use wireguard_control::{Device, InterfaceName, KeyPair};
use crate::{api::{BootstrapInfo, JoinResponse as BootstrapResponse, Response}, up, CONFIG_DIR, NETWORK_NAME};


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
                external_endpoint: { 
                    let mut port: u16 = 0;
                    for p in 51821..64000 {
                        if let Ok(listener) = TcpListener::bind(("0.0.0.0", port)) {
                            drop(listener);
                            port = p;
                            break;
                        }
                    }
                    if port == 0 {
                        panic!("Unable to find a valid listening port in the formnet range");
                    }
                    Some(
                        SocketAddr::new(publicip, port)
                    )
                },
            }
        },
        PeerType::Instance => {
            BootstrapInfo {
                id: address.to_string(),
                peer_type: PeerType::Instance,
                cidr_id: "formnet".to_string(),
                pubkey: keypair.public.to_base64(),
                internal_endpoint: None,
                external_endpoint: {
                    let mut port: u16 = 0;
                    for p in 51821..64000 {
                        if let Ok(listener) = TcpListener::bind(("0.0.0.0", port)) {
                            drop(listener);
                            port = p;
                            break;
                        }
                    }
                    if port == 0 {
                        panic!("Unable to find a valid listening port in the formnet range");
                    }
                    Some(
                        SocketAddr::new(publicip, port)
                    )
                },
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

    #[cfg(target_os = "linux")]
    let daemon = Daemonize::new()
        .pid_file("/run/formnet.pid")
        .chown_pid_file(true)
        .working_directory("/")
        .umask(0o027)
        .stdout(std::fs::File::create("/var/log/formnet.log").unwrap())
        .stderr(std::fs::File::create("/var/log/formnet.log").unwrap());

    #[cfg(target_os = "linux")]
    match daemon.start() {
        Ok(_) => {
            if let Err(e) = up(
                Some(Duration::from_secs(60)),
                None,
            ).await {
                println!("{}: {}", "Error trying to bring formnet up".yellow(), e.to_string().red());
            }
        }
        Err(e) => {
            println!("{}: {}", "Error trying to daemonize formnet".yellow(), e.to_string().red());
        }
    }

    #[cfg(not(target_os = "linux"))]
    if let Err(e) = up(
        Some(Duration::from_secs(60)),
        None,
    ).await {
        println!("{}: {}", "Error trying to bring formnet up".yellow(), e.to_string().red());
    }

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
