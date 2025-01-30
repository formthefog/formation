use std::time::Duration;
use colored::*;
use axum::Json;
use daemonize::Daemonize;
use form_types::{BootCompleteRequest, PeerType, VmmResponse};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use shared::{interface_config::InterfaceConfig, NetworkOpts};
use crate::{add_peer::add_peer, handle_leave_request, redeem, up};

pub fn create_router() -> axum::Router {
    axum::Router::new()
        .route("/join", axum::routing::post(handle_join_request))
        .route("/leave", axum::routing::post(handle_leave_request))
        //TODO: Add routes to request custom cidr, request custom assoc
        //Add routes to delete peer, delete custom cidr, delete assoc
}

async fn handle_join_request(
    Json(join_request): Json<JoinRequest>,
) -> axum::Json<JoinResponse> {
    match add_peer(
        &NetworkOpts::default(),
        &join_request.peer_type(),
        &join_request.id()
    ).await {
        Ok(invitation) => {
            let resp = JoinResponse::Success { invitation };
            log::info!("SUCCESS! Sending Response: {resp:?}");
            return Json(resp)
        },
        Err(e) => {
            Json(JoinResponse::Error(e.to_string()))
        }
    }
}

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


pub async fn request_to_join(bootstrap: Vec<String>, address: String, peer_type: PeerType) -> Result<InterfaceConfig, Box<dyn std::error::Error>> {
    let request = match peer_type { 
        PeerType::Operator => JoinRequest::OperatorJoinRequest(
            OperatorJoinRequest {
                operator_id: address,
            }
        ),
        PeerType::User => JoinRequest::UserJoinRequest(
            UserJoinRequest {
                user_id: address
            }
        ),
        PeerType::Instance => JoinRequest::InstanceJoinRequest(
            VmJoinRequest {
                vm_id: address
            }
        )
    };

    while let Some(dial) = bootstrap.iter().next() {
        match Client::new()
        .post(&format!("http://{dial}/join"))
        .json(&request)
        .send()
        .await {
            Ok(response) => match response.json::<JoinResponse>().await {
                Ok(JoinResponse::Success { invitation }) => return Ok(invitation),
                _ => {}
            }
            _ => {}
        }
    }
    return Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, "Did not receive a valid invitation")));
}

pub async fn user_join_formnet(address: String, provider: String, formnet_port: u16) -> Result<(), Box<dyn std::error::Error>> {
    let invitation = request_to_join(vec![format!("{}:{}", provider, formnet_port)], address, PeerType::User).await?;
    println!("{}", "Attempting to redeem formnet invite".yellow());
    if let Err(e) = redeem(invitation) {
        println!("{}: {}", "Error trying to redeem invite".yellow(), e.to_string().red());
    } 

    let daemon = Daemonize::new()
        .pid_file("/run/formnet.pid")
        .chown_pid_file(true)
        .working_directory("/")
        .umask(0o027)
        .stdout(std::fs::File::create("/var/log/formnet.log").unwrap())
        .stderr(std::fs::File::create("/var/log/formnet.log").unwrap());

    match daemon.start() {
        Ok(_) => {
            if let Err(e) = up(
                Some(Duration::from_secs(60)),
                None,
            ) {
                println!("{}: {}", "Error trying to bring formnet up".yellow(), e.to_string().red());
            }
        }
        Err(e) => {
            println!("{}: {}", "Error trying to daemonize formnet".yellow(), e.to_string().red());
        }
    }
    Ok(())
}

pub async fn vm_join_formnet() -> Result<(), Box<dyn std::error::Error>> {
    simple_logger::SimpleLogger::new().init().unwrap();
    let host_public_ip = reqwest::blocking::get(
        "https://api.ipify.org"
    )?.text()?;
    // Get name
    let name = std::fs::read_to_string("/etc/vm_name")?;
    log::info!("Requesting formnet invite for vm {}", name);
    log::info!("Building VmJoinRequest");
    let join_request = VmJoinRequest { vm_id: name.clone() };
    log::info!("Wrapping VmJoinRequest in a JoinRequest");
    let join_request = JoinRequest::InstanceJoinRequest(join_request);
    log::info!("Getting a new client");
    let client = reqwest::Client::new();
    log::info!("Posting request to endpoint using client, awaiting response...");
    // We should be able to access formnet, and the VMM over the bridge gateway
    let resp = client.post(&format!("http://{host_public_ip}:3001/join"))
        .json(&join_request)
        .send().await.map_err(|e| {
            other_err(&e.to_string())
        })?.json::<JoinResponse>().await.map_err(|e| {
            other_err(&e.to_string())
        })?;

    log::info!("Response text: {resp:?}");

    match resp {
        JoinResponse::Success { invitation } => {
            log::info!("Received invitation");
            let invite = invitation;
            let formnet_ip = invite.interface.address.addr().to_string();
            log::info!("extracted formnet IP for {name}");
            log::info!("Attempting to redeem invite");
            if let Err(e) = redeem(invite).map_err(|e| {
                other_err(&e.to_string())
            }) {
                log::error!("Error attempting to redeem invite: {e}");
            }

            log::info!("Successfully redeemed invite");
            log::info!("Spawning thread to bring formnet up");
            let handle = tokio::spawn(async move {
                if let Err(e) = up(
                    Some(Duration::from_secs(60)),
                    None,
                ) {
                    log::error!("Error bringing formnet up: {e}");
                }
            });

            log::info!("Building request to inform VMM service that the boot process has completed for {name}");

            // Send message to VMM api.
            let request = BootCompleteRequest {
                name: name.clone(),
                formnet_ip
            };

            log::info!("Sending BootCompleteRequest {request:?} to http://{host_public_ip}:3002/{name}/boot_complete endpoint");
            let resp = client.post(&format!("http://{host_public_ip}:3002/{}/boot_complete", name))
                .json(&request)
                .send()
                .await?
                .json::<VmmResponse>().await;

            log::info!("BootCompleteRequest Response: {resp:?}");

            log::info!("Formnet up, awaiting kill signal");
            handle.await?;

            Ok(())
        },
        JoinResponse::Error(reason) => return Err(other_err(&reason.to_string()))
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
