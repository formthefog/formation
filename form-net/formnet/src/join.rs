use axum::Json;
use form_types::PeerType;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use shared::{interface_config::InterfaceConfig, NetworkOpts};

use crate::add_peer::add_peer;

pub fn create_router() -> axum::Router {
    axum::Router::new().route("/join", axum::routing::post(handle_join_request))
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
