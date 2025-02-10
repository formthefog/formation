use std::{path::PathBuf, str::FromStr};
use axum::Json;
use colored::*;
use client::data_store::DataStore;
use formnet_server::{db::CrdtMap, DatabasePeer};
use shared::{interface_config::InterfaceConfig, wg, IoErrorContext, NetworkOpts, PeerContents};
use wireguard_control::{DeviceUpdate, InterfaceName, Key};
use alloy_core::primitives::Address;
use k256::ecdsa::SigningKey;
use reqwest::Client;
use serde::{Serialize, Deserialize};
use crate::{CONFIG_DIR, DATA_DIR, NETWORK_NAME};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum LeaveRequest {
    Operator(OperatorLeaveRequest),
    User(UserLeaveRequest),
    Instance(VmLeaveRequest),
}

impl LeaveRequest {
    pub fn id(&self) -> String {
        match self {
            LeaveRequest::Operator(req) => req.operator_id.clone(),
            LeaveRequest::User(req) => req.user_id.clone(),
            LeaveRequest::Instance(req) => req.vm_id.clone(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OperatorLeaveRequest {
    operator_id: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UserLeaveRequest {
    user_id: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VmLeaveRequest {
    vm_id: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum LeaveResponse {
    Success,
    Failure
}

pub async fn leave(bootstraps: Vec<String>, key: String) -> Result<(), Box<dyn std::error::Error>> {
    let mut bootstraps = bootstraps.clone();
    let mut bootstrap_iter = bootstraps.iter_mut();
    let address = hex::encode(Address::from_private_key(&SigningKey::from_slice(&hex::decode(key)?)?));
    let request = LeaveRequest::Operator(OperatorLeaveRequest { operator_id: address });
    let client = Client::new();
    while let Some(dial) = bootstrap_iter.next() {
        match client.post(&format!("http://{dial}/51820/leave"))
            .json(&request)
            .send()
            .await {
                Ok(resp) => match resp.json::<LeaveResponse>().await {
                    Ok(LeaveResponse::Success) => return Ok(()),
                    Ok(LeaveResponse::Failure) => continue,
                    Err(e) => { 
                        log::error!("Error trying to leave while dialing {dial}: {e}");
                        continue;
                    }
                }
                Err(e) => {
                    log::error!("Error trying to leave while dialing {dial}: {e}");
                    continue;
                }
        }
    }

    Ok(())
}

pub fn uninstall() -> Result<(), Box<dyn std::error::Error>> {
    let interface = InterfaceName::from_str("formnet")?;
    let config = InterfaceConfig::get_path(&PathBuf::from(CONFIG_DIR), &interface);
    let data = DataStore::<String>::get_path(&PathBuf::from(DATA_DIR), &interface);

    if !config.exists() && !data.exists() {
        return Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!(
            "No network named \"{}\" exists.",
            interface.as_str_lossy().yellow()
            ))
        ));
    }

    log::info!("bringing down interface (if up).");
    let network = NetworkOpts::default();
    wg::down(&interface, network.backend).ok();
    std::fs::remove_file(&config)
            .with_path(&config)
            .map_err(|e| log::warn!("{}", e.to_string().yellow()))
            .ok();
        std::fs::remove_file(&data)
            .with_path(&data)
            .map_err(|e| log::warn!("{}", e.to_string().yellow()))
            .ok();
        log::info!(
            "network {} is uninstalled.",
            interface.as_str_lossy().yellow()
        );
    Ok(())
}

pub async fn handle_leave_request(
    Json(leave_request): Json<LeaveRequest>,
) -> axum::Json<LeaveResponse> {
    match disable_peer(leave_request.id()).await {
        Ok(()) => {
            log::info!("SUCCESS! Sending Response");
            return Json(LeaveResponse::Success)
        },
        Err(_) => {
            Json(LeaveResponse::Failure)
        }
    }
}

async fn disable_peer(id: String) -> Result<(), Box<dyn std::error::Error>> {
    let mut peer = DatabasePeer::<String, CrdtMap>::get(id).await?;
    peer.update(PeerContents {
        is_disabled: true,
        ..peer.inner.contents.clone()
    }).await?;
    let public_key = Key::from_base64(&peer.public_key)?;

    DeviceUpdate::new()
        .remove_peer_by_key(&public_key)
        .apply(
            &InterfaceName::from_str(NETWORK_NAME)?,
            NetworkOpts::default().backend
        )?;

    Ok(())
}
