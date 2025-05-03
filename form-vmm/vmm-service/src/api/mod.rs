use alloy_primitives::Address;
use axum::{
    extract::{State, Path, Request, Extension}, 
    routing::{get, post}, 
    Json, Router,
    body::Body,
    response::{Response, IntoResponse}
};
use form_p2p::queue::{QueueRequest, QueueResponse, QUEUE_PORT};
use reqwest::Client;
use serde::{de::DeserializeOwned, Serialize, Deserialize}; 
use tiny_keccak::{Hasher, Sha3};
use tokio::sync::mpsc;
use tokio::sync::Mutex;
use vmm::api::{VmInfo, VmmPingResponse};
use std::{sync::Arc, time::Duration};
use std::net::SocketAddr;
use k256::ecdsa::VerifyingKey;
use http::{HeaderMap, StatusCode};
use axum::middleware::{self, Next};
use std::ops::Deref;
use bytes::Bytes;
use sha2;

use crate::VmmError;
use form_types::{BootCompleteRequest, CreateVmRequest, DeleteVmRequest, GetVmRequest, PingVmmRequest, StartVmRequest, StopVmRequest, VmResponse, VmmEvent, VmmResponse};
use crate::api::helpers::{
    create::create,
    start::start,
    stop::stop,
    delete::delete,
    complete::complete,
    reboot::reboot,
    power_button::power_button,
    commit::commit,
    snapshot::snapshot,
    coredump::coredump,
    restore::restore,
    resize_vcpu::resize_vcpu,
    resize_memory::resize_memory,
    add_device::add_device,
    add_disk::add_disk,
    add_fs::add_fs,
    remove_device::remove_device,
    migrate_to::migrate_to,
    migrate_from::migrate_from,
    get::get_vm,
    list::list
};
use crate::queue::read::read_from_queue;
use crate::queue::helpers::{
    create::handle_create_vm_message,
    boot::handle_boot_vm_message,
    delete::handle_delete_vm_message,
    stop::handle_stop_vm_message,
    reboot::handle_reboot_vm_message,
};

pub mod auth;
pub mod helpers;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum HealthStatus {
    Healthy,
    Degraded { reason: String },
    Unhealthy { reason: String }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthResponse {
    status: HealthStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    uptime: Option<u64>
}
pub struct VmmApiChannel {
    event_sender: mpsc::Sender<VmmEvent>,
    response_receiver: mpsc::Receiver<String>,
}

impl VmmApiChannel {
    pub fn new(
        tx: mpsc::Sender<VmmEvent>,
        rx: mpsc::Receiver<String>,
    ) -> Self {
        Self{
            event_sender: tx,
            response_receiver: rx,
        }
    }

    pub async fn send(
        &self,
        event: VmmEvent
    ) -> Result<(), mpsc::error::SendError<VmmEvent>> {
        self.event_sender.send(event).await
    }

    pub async fn recv<T: DeserializeOwned>(
        &mut self
    ) -> Option<T> {
        match self.response_receiver.recv().await {
            Some(value) => {
                match serde_json::from_str::<T>(&value) {
                    Ok(resp) => return Some(resp),
                    Err(e) => {
                        log::error!("{e}");
                        return None
                    }
                }
            }
            None => return None
        }
    }
}

/// API server that allows direct interaction with the VMM service
pub struct VmmApi {
    /// Channels to send events to the service and receive responses
    channel: Arc<Mutex<VmmApiChannel>>,
    /// Server address
    addr: SocketAddr,
}

impl VmmApi {
    pub fn new(
        api_channel: Arc<Mutex<VmmApiChannel>>,
        addr: SocketAddr
    ) -> Self {
        Self {
            channel: api_channel, addr
        }
    }

    pub fn extract_owner_from_create_request(request: CreateVmRequest) -> Result<String, VmmError> {
        // Get signature from request
        let signature = request.signature.ok_or(VmmError::Config("Signature is required".to_string()))?;
        
        // Create message for verification (name + formfile hash)
        let message = format!("CreateVmRequest:{}:{}", request.name, hex::encode(&request.formfile.as_bytes()[0..32]));
        
        // Use our SignatureVerifier utility to verify the signature and get the owner's address
        auth::SignatureVerifier::verify_signature(message, &signature, request.recovery_id)
    }

    pub fn extract_build_id(name: String, owner: String) -> Result<String, VmmError> {
        let mut hasher = Sha3::v256();
        let mut hash = [0u8; 32];
        let owner = Address::from_slice(&hex::decode(owner).map_err(|e| {
            VmmError::Config(e.to_string())
        })?);
        hasher.update(owner.as_ref());
        hasher.update(name.as_bytes());
        hasher.finalize(&mut hash);
        Ok(hex::encode(hash))
    }

    pub async fn start_api_server(&self, config: &crate::config::Config) -> Result<(), VmmError> {
        log::info!("Attempting to start API server");
        let app_state = self.channel.clone();

        // Create the router with public routes (no authentication)
        let public_routes = Router::new()
            .route("/health", get(health_check))
            .route("/vm/boot_complete", post(complete));
        
        // Create the router with protected routes (requires authentication)
        let protected_routes = Router::new()
            .route("/vm/create", post(create))
            .route("/vm/:id/boot", post(start))
            .route("/vm/:id/delete", post(delete))
            .route("/vm/:id/pause", post(stop))
            .route("/vm/:id/stop", post(stop))
            .route("/vm/:id/reboot", post(reboot))
            .route("/vm/:id/resume", post(start))
            .route("/vm/:id/start", post(start))
            .route("/vm/:id/on", post(start))
            .route("/vm/:id/power_button", post(power_button))
            .route("/vm/:id/commit", post(commit))
            .route("/vm/:id/update", post(commit))
            .route("/vm/:id/snapshot", post(snapshot))
            .route("/vm/:id/coredump", post(coredump))
            .route("/vm/:id/restore", post(restore))
            .route("/vm/:id/resize_vcpu", post(resize_vcpu))
            .route("/vm/:id/resize_memory", post(resize_memory))
            .route("/vm/:id/add_device", post(add_device))
            .route("/vm/:id/add_disk", post(add_disk))
            .route("/vm/:id/add_fs", post(add_fs))
            .route("/vm/:id/remove_device", post(remove_device))
            .route("/vm/:id/migrate_to", post(migrate_to))
            .route("/vm/:id/migrate_from", post(migrate_from))
            .route("/vm/:id/ping", post(ping))
            .route("/vm/:id/info", get(get_vm))
            .route("/vm/:id", get(get_vm))
            .route("/vms/list", get(list))
            .layer(middleware::from_fn(signature_verify_middleware));

        // Merge the routers
        let app = Router::new()
            .merge(public_routes)
            .merge(protected_routes)
            .with_state(app_state);

        log::info!("Established routes, binding to {}", &self.addr);
        let listener = tokio::net::TcpListener::bind(
            self.addr.clone()).await
            .map_err(|e| {
                VmmError::SystemError(
                    format!(
                        "Failed to bind listener to address {}: {e}",
                        self.addr.clone()
                    )
                )
            })?;
            
        // Start the API server
        log::info!("Starting server");
        axum::serve(listener, app).await
            .map_err(|e| VmmError::SystemError(format!("Failed to serve API server {e}")))?;

        Ok(())
    }

    pub fn addr(&self) -> &SocketAddr {
        &self.addr
    }
}

// Helper function to extract signature data from request headers
fn extract_signature_data(headers: &HeaderMap) -> Result<(String, String, i64), VmmError> {
    let signature = headers.get("X-Signature")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| VmmError::Config("Missing X-Signature header".to_string()))?
        .to_string();
    
    let recovery_id = headers.get("X-Recovery-ID")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| VmmError::Config("Missing X-Recovery-ID header".to_string()))?
        .to_string();
    
    let timestamp = headers.get("X-Timestamp")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.parse::<i64>().ok())
        .ok_or_else(|| VmmError::Config("Missing or invalid X-Timestamp header".to_string()))?;
    
    Ok((signature, recovery_id, timestamp))
}

// Define a new type to hold authentication details for request extensions
#[derive(Debug, Clone)]
pub struct AuthenticatedUser {
    pub address: String,
}

// Fix the signature verification middleware to properly handle bodies
async fn signature_verify_middleware(
    req: Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    // Skip authentication for public paths
    let path = req.uri().path();
    if path == "/health" || path == "/vm/boot_complete" {
        return Ok(next.run(req).await);
    }
    
    // Extract headers from the request
    let headers = req.headers().clone();
    
    // Extract signature data
    let (signature, recovery_id, timestamp) = match extract_signature_data(&headers) {
        Ok(data) => data,
        Err(_) => return Err(StatusCode::UNAUTHORIZED),
    };
    
    // We need to handle the body properly for security
    // First, split the request into parts and body
    let (parts, body) = req.into_parts();
    
    // Read the entire body
    let bytes = match hyper::body::to_bytes(body).await {
        Ok(b) => b,
        Err(_) => return Err(StatusCode::BAD_REQUEST),
    };
    
    // Create the message by combining path, timestamp, and body hash (if it's not empty)
    let mut message = format!("{}", path);
    
    // For non-empty bodies, include a hash of the body in the message
    if !bytes.is_empty() {
        // Create a secure hash of the body content
        let mut hasher = sha2::Sha256::new();
        sha2::Digest::update(&mut hasher, &bytes);
        let body_hash = hasher.finalize();
        
        // Add body hash to the message
        message = format!("{}:{}", message, hex::encode(body_hash));
    }
    
    // Always include timestamp
    message = format!("{}:{}", message, timestamp);
    
    log::debug!("Verifying signature for message: {}", message);
    
    // Verify the signature and get the signer's address
    let address = match auth::SignatureVerifier::verify_signature(
        message,
        &signature,
        u8::from_str_radix(&recovery_id, 16).unwrap_or(0) as u32
    ) {
        Ok(address) => address,
        Err(e) => {
            log::warn!("Signature verification failed: {}", e);
            return Err(StatusCode::UNAUTHORIZED);
        }
    };
    
    // Log the authenticated user
    log::info!("Request authenticated: path={}, user={}", path, address);
    
    // Create a new request with the authenticated user
    let mut req = Request::from_parts(parts, Body::from(bytes));
    req.extensions_mut().insert(AuthenticatedUser { address });
    
    // Continue to the handler
    Ok(next.run(req).await)
}
