use std::{net::{IpAddr, SocketAddr}, path::PathBuf, str::FromStr, thread, time::Duration};
use form_types::{BootCompleteRequest, PeerType, VmmResponse};
use formnet_server::ConfigFile;
use ipnet::IpNet;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use shared::{interface_config::InterfaceConfig, wg, NetworkOpts};
use wireguard_control::{Device, InterfaceName, KeyPair};
use crate::{api::{BootstrapInfo, JoinResponse as BootstrapResponse, Response}, fetch, report_initial_candidates, up, CONFIG_DIR, DATA_DIR, NETWORK_NAME};


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

async fn try_holepunch_fetch(bootstrap: Vec<String>, my_ip: String) -> bool {
    if let Ok(_) = report_initial_candidates(bootstrap, my_ip).await {
        let mut fetch_success = false;
        for _ in 0..3 {
            if fetch(None).await.is_ok() {
                fetch_success = true;
                break;
            }
            thread::sleep(Duration::from_secs(1));
        }
        return fetch_success
    }
    false
}

async fn check_already_joined(bootstrap: Vec<String>, id: &str) -> Result<(bool, Option<IpAddr>), Box<dyn std::error::Error>> {
    let mut iter = bootstrap.iter();
    while let Some(dial) = iter.next() {
        match Client::new().get(format!("http://{dial}:51820/fetch")).send().await {
            Ok(resp) => {
                let r = resp.json::<Response>().await;
                match r {
                    Ok(Response::Fetch(peers)) => {
                        if let Some(p) = peers.iter().find(|p| p.id == id) {
                            let config = ConfigFile::from_file(PathBuf::from(CONFIG_DIR).join(NETWORK_NAME).with_extension("conf"))?;
                            if let Some(admin) = peers.iter().find(|p| p.is_admin) {
                                wg::up(
                                    &InterfaceName::from_str(NETWORK_NAME)?,
                                    &config.private_key,
                                    IpNet::new(p.ip, 8)?, 
                                    None,
                                    Some((&admin.public_key, admin.ip, admin.endpoint.clone().unwrap().resolve()?)), 
                                    NetworkOpts::default(),
                                )?;
                            }
                            if !try_holepunch_fetch(bootstrap, p.ip.to_string()).await {
                                eprintln!(
                                    "Failed to fetch peers from server, you will need to manually run the 'up' command."
                                );
                            }
                            return Ok((true, Some(p.ip)));
                        }
                    }
                    Err(e) => {
                        log::error!(
                            "Could not deserialize response from {dial}: {e}"
                        )
                    }
                    Ok(resp_val) => {
                        log::error!(
                            "Received invalid response type from {dial}/fetch endpoint: {resp_val:?}" 
                        )
                    }
                }
            }
            Err(e) => {
                log::error!(
                    "API call to {dial}/fetch failed: {e}"
                )
            }
        }
    }

    Ok((false, None))
}

async fn try_get_bootstrap_info(bootstrap: Vec<String>) -> Result<BootstrapInfo, Box<dyn std::error::Error>> {
    let client = Client::new();
    let mut iter = bootstrap.iter();
    let mut bootstrap_info: Option<BootstrapInfo> = None;
    while let Some(dial) = iter.next() {
        match client.get(format!("http://{dial}:51820/bootstrap"))
            .send().await {
                Ok(resp) => match resp.json::<Response>().await {
                    Ok(Response::Bootstrap(info)) => {
                        log::info!("Received bootstrap info from bootstrap node {dial}");
                        log::info!("Bootstrap info: {info:?}");
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
        return Err(
            Box::new(
                std::io::Error::new(
                    std::io::ErrorKind::Other, 
                    "Was unable to acquire bootstrap information from any bootstrap nodes provided"
                )
            )
        );
    }

    Ok(bootstrap_info.unwrap())
}

fn write_config_file(
    keypair: KeyPair,
    request: BootstrapInfo,
    ip: IpAddr,
    bootstrap_info: BootstrapInfo,
) -> Result<(), Box<dyn std::error::Error>> {
    let config_file = ConfigFile {
        private_key: keypair.private.to_base64(),
        address: ip.clone(),
        listen_port: match request.external_endpoint {
            Some(endpoint) => {
                Some(endpoint.port())
            },
            None => None
        },
        network_cidr_prefix: 8,
        bootstrap: Some(hex::encode(&serde_json::to_vec(&bootstrap_info)?)) 
    };

    std::fs::create_dir_all(PathBuf::from(CONFIG_DIR))?;
    config_file.write_to_path(
        PathBuf::from(CONFIG_DIR).join(NETWORK_NAME).with_extension("conf")
    )?;
    log::info!("Wrote config file");
    Ok(())
}

fn try_bring_formnet_up(
    keypair: KeyPair,
    ip: IpAddr,
    request: BootstrapInfo,
    bootstrap_info: BootstrapInfo,
) -> Result<(), Box<dyn std::error::Error>> {
    wg::up(
        &InterfaceName::from_str("formnet")?,
        &keypair.private.to_base64(), 
        IpNet::new(ip.clone(), 8)?,
        match request.external_endpoint {
            Some(addr) => Some(addr.port()),
            None => None
        },
        Some((
            &bootstrap_info.pubkey,
            bootstrap_info.internal_endpoint.unwrap(),
            bootstrap_info.external_endpoint.unwrap(),
        )), 
        NetworkOpts::default(),
    )?;

    Ok(())
}

fn log_initial_endpoints() {
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
}


async fn try_join_formnet(
    bootstrap_info: BootstrapInfo,
    request: BootstrapInfo,
    keypair: KeyPair
) -> Result<IpAddr, Box<dyn std::error::Error>> {
    let dial = bootstrap_info.external_endpoint.unwrap();
    log::info!("Attempting to dial {dial}");
    match Client::new().post(&format!("http://{dial}/join"))
    .json(&request)
    .send()
    .await?.json::<Response>().await {
        Ok(Response::Join(BootstrapResponse::Success(ip))) => {
            log::info!("Bringing Wireguard interface up...");
            write_config_file(keypair.clone(), request.clone(), ip.clone(), bootstrap_info.clone())?;
            thread::sleep(Duration::from_secs(5));
            try_bring_formnet_up(keypair, ip, request, bootstrap_info)?; 

            if !try_holepunch_fetch(vec![dial.to_string()], ip.to_string()).await {
                eprintln!(
                    "Failed to fetch peers from server, you will need to manually run the 'up' command."
                );
            };
            log_initial_endpoints();
            log::info!("Wireguard interface is up, saved config file");
            return Ok(ip.clone());
        }
        Err(e) => {
            log::error!("Error attempting to join network: {e}");
        }
        Ok(r) => {
            log::error!("Received invalid response type when trying to join network: {r:?}");
        }
    }
    eprintln!("Didn't receive a valid response from bootstrap, unable to join formnet: {:?}", bootstrap_info.external_endpoint);
    return Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, "Did not receive a valid invitation")));
}

fn build_join_request(
    peer_type: PeerType,
    keypair: KeyPair,
    address: String,
    public_ip: Option<String>
) -> Result<BootstrapInfo, Box<dyn std::error::Error>> {
    match peer_type { 
        PeerType::Operator => {
            let publicip = if let Some(ip) = public_ip {
                    ip.parse::<IpAddr>()?
            } else {
                publicip::get_any(
                    publicip::Preference::Ipv4
                ).ok_or(
                    Box::new(
                        std::io::Error::new(
                            std::io::ErrorKind::Other,
                                "unable to acquire public ip"
                        )
                    )
                )?
            };

            Ok(BootstrapInfo {
                id: address.to_string(),
                peer_type: PeerType::Operator,
                cidr_id: "formnet".to_string(),
                pubkey: keypair.public.to_base64(),
                internal_endpoint: None,
                external_endpoint: Some(SocketAddr::new(publicip, 51820)),
            })
        },
        PeerType::User => {
            Ok(BootstrapInfo {
                id: address.to_string(),
                peer_type: PeerType::User,
                cidr_id: "formnet".to_string(),
                pubkey: keypair.public.to_base64(),
                internal_endpoint: None,
                external_endpoint: None, 
            })
        },
        PeerType::Instance => {
            Ok(BootstrapInfo {
                id: address.to_string(),
                peer_type: PeerType::Instance,
                cidr_id: "formnet".to_string(),
                pubkey: keypair.public.to_base64(),
                internal_endpoint: None,
                external_endpoint: None, 
            })
        }
    }
}

pub async fn request_to_join(bootstrap: Vec<String>, address: String, peer_type: PeerType, public_ip: Option<String>) -> Result<IpAddr, Box<dyn std::error::Error>> {
    if let Err(e) = std::fs::remove_file(PathBuf::from(DATA_DIR).join("formnet").with_extension("json")) {
        log::error!("Pre-existing datastore did not exist: {e}"); 
    }

    if let Ok((true, Some(ip))) = check_already_joined(bootstrap.clone(), &address).await {
        if !try_holepunch_fetch(bootstrap, ip.to_string()).await {
            println!("initial attempt to fetch failed, you will need to call `form manage formnet-up`") 
        }
        return Ok(ip);
    } 

    let bootstrap_info = try_get_bootstrap_info(bootstrap.clone()).await?;
    let keypair = KeyPair::generate();
    let request = build_join_request(peer_type, keypair.clone(), address, public_ip)?;

    log::info!("Built join request: {request:?}");

    try_join_formnet(bootstrap_info, request, keypair).await

}

pub async fn user_join_formnet(address: String, provider: String, public_ip: Option<String>) -> Result<(), Box<dyn std::error::Error>> {
    request_to_join(vec![provider], address, PeerType::User, public_ip).await?;
    Ok(())
}

pub async fn vm_join_formnet() -> Result<(), Box<dyn std::error::Error>> {
    let host_public_ip = std::env::var("HOST_BRIDGE_IP").unwrap(); 
    log::info!("HOST IP: {host_public_ip}");

    let name = std::fs::read_to_string("/etc/vm_name")?;
    let build_id = std::fs::read_to_string("/etc/build_id")?;
    match request_to_join(vec![host_public_ip.clone()], name.clone(), form_types::PeerType::Instance, None).await {
        Ok(ip)=> {
            log::info!("Received invitation");
            let formnet_ip = ip; 
            log::info!("extracted formnet IP for {name}: {formnet_ip}");
            log::info!("Attempting to redeem invite");
            log::info!("Spawning thread to bring formnet up");
            let _ = tokio::time::sleep(Duration::from_secs(5)).await;
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
