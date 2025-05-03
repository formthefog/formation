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
    get_vm,
    list
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

    pub async fn start_queue_reader(
        channel: Arc<Mutex<VmmApiChannel>>,
        mut shutdown: tokio::sync::broadcast::Receiver<()>
    ) -> Result<(), VmmError> { 
        let mut n = 0;
        #[cfg(not(feature = "devnet"))]
        loop {
            tokio::select! {
                Ok(messages) = Self::read_from_queue(Some(n), None) => {
                    for message in &messages {
                        if let Err(e) = Self::handle_message(message.to_vec(), channel.clone()).await {
                            eprintln!("Error handling message in queue reader: {e}");
                        }
                    }
                    n += messages.len();
                }
                _ = tokio::time::sleep(Duration::from_millis(100)) => {}
                _ = shutdown.recv() => {
                    break;
                }
            }
        }
        Ok(())
    }

    pub async fn handle_message(message: Vec<u8>, channel: Arc<Mutex<VmmApiChannel>>) -> Result<(), VmmError> {
        let subtopic = message[0];
        log::info!("Received subtopic: {subtopic}");
        let msg = &message[1..];
        match subtopic {
            0 => Self::handle_create_vm_message(msg, channel.clone()).await?,
            1 => Self::handle_boot_vm_message(msg, channel.clone()).await?, 
            2 => Self::handle_delete_vm_message(msg, channel.clone()).await?,
            3 => Self::handle_stop_vm_message(msg, channel.clone()).await?,
            4 => Self::handle_reboot_vm_message(msg, channel.clone()).await?,
            5 => Self::handle_start_vm_message(msg, channel.clone()).await?,
            _ => unreachable!()
        }
        Ok(())
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

    pub async fn handle_create_vm_message(msg: &[u8], channel: Arc<Mutex<VmmApiChannel>>) -> Result<(), VmmError> {
        log::info!("Received create request from queue..");
        let request: CreateVmRequest = serde_json::from_slice(msg).map_err(|e| {
            VmmError::Config(e.to_string())
        })?;
        log::info!("Deserialized create request..");
        let owner = Self::extract_owner_from_create_request(request.clone())?;
        log::info!("built create event...");
        let event = VmmEvent::Create { 
            formfile: request.formfile, 
            name: request.name, 
            owner,
        };

        log::info!("Acquiring lock on API channel...");
        let guard = channel.lock().await; 
        log::info!("Sending event...");
        guard.send(event).await.map_err(|e| {
            VmmError::SystemError(e.to_string())
        })?;

        log::info!("dropping guard");
        drop(guard);
        log::info!("guard dropped, returning...");

        Ok(())
    }

    pub async fn handle_boot_vm_message(msg: &[u8], channel: Arc<Mutex<VmmApiChannel>>) -> Result<(), VmmError> {
        log::info!("Recevied boot request from queue...");
        let request: StartVmRequest = serde_json::from_slice(msg).map_err(|e| {
            VmmError::Config(e.to_string())
        })?;

        log::info!("Building start event..");
        let event = VmmEvent::Start { id: request.id };
        log::info!("Acquiring lock on API channel..");
        let guard = channel.lock().await; 
        log::info!("Sending event...");
        guard.send(event).await.map_err(|e| {
            VmmError::SystemError(e.to_string())
        })?;

        log::info!("dropping guard...");
        drop(guard);
        log::info!("guard dropped, returning...");
        Ok(())
    }

    pub async fn handle_delete_vm_message(msg: &[u8], channel: Arc<Mutex<VmmApiChannel>>) -> Result<(), VmmError> {
        let request: DeleteVmRequest = serde_json::from_slice(msg).map_err(|e| {
            VmmError::Config(e.to_string())
        })?;

        // Verify signature if provided
        if let Some(signature) = &request.signature {
            // Create message for verification
            let message = auth::SignatureVerifier::create_operation_message("DeleteVmRequest", &request.id);
            
            // Verify signature
            let signer_address = auth::SignatureVerifier::verify_signature(message, signature, request.recovery_id)?;
            
            // Check if signer is authorized
            let is_authorized = auth::OwnershipVerifier::verify_authorization(
                &request.id, 
                &signer_address, 
                auth::Permission::Owner // Deletion requires owner permission
            ).await?;
            
            if !is_authorized {
                return Err(VmmError::Config(format!(
                    "Unauthorized: Address {} is not the owner of instance {}", 
                    signer_address, request.id
                )));
            }
        } else {
            return Err(VmmError::Config("Signature is required".to_string()));
        }

        // Proceed with deleting the VM
        let event = VmmEvent::Delete { id: request.id };

        let guard = channel.lock().await; 
        guard.send(event).await.map_err(|e| {
            VmmError::SystemError(e.to_string())
        })?;
        drop(guard);

        Ok(())
    }

    pub async fn handle_stop_vm_message(msg: &[u8], channel: Arc<Mutex<VmmApiChannel>>) -> Result<(), VmmError> {
        let request: StopVmRequest = serde_json::from_slice(msg).map_err(|e| {
            VmmError::Config(e.to_string())
        })?;

        // Verify signature if provided
        if let Some(signature) = &request.signature {
            // Create message for verification
            let message = auth::SignatureVerifier::create_operation_message("StopVmRequest", &request.id);
            
            // Verify signature
            let signer_address = auth::SignatureVerifier::verify_signature(message, signature, request.recovery_id)?;
            
            // Check if signer is authorized
            let is_authorized = auth::OwnershipVerifier::verify_authorization(
                &request.id, 
                &signer_address, 
                auth::Permission::Operator
            ).await?;
            
            if !is_authorized {
                return Err(VmmError::Config(format!(
                    "Unauthorized: Address {} is not the owner or authorized user for instance {}", 
                    signer_address, request.id
                )));
            }
        } else {
            return Err(VmmError::Config("Signature is required".to_string()));
        }

        // Proceed with stopping the VM
        let event = VmmEvent::Stop { id: request.id };
        let guard = channel.lock().await; 
        guard.send(event).await.map_err(|e| {
            VmmError::SystemError(e.to_string())
        })?;

        drop(guard);
        
        Ok(())
    }

    pub async fn handle_reboot_vm_message(msg: &[u8], channel: Arc<Mutex<VmmApiChannel>>) -> Result<(), VmmError> {
        let request: StopVmRequest = serde_json::from_slice(msg).map_err(|e| {
            VmmError::Config(e.to_string())
        })?;

        let event = VmmEvent::Stop { id: request.id.clone() };

        let guard = channel.lock().await; 
        guard.send(event).await.map_err(|e| {
            VmmError::SystemError(e.to_string())
        })?;

        let event = VmmEvent::Start { id: request.id };
        let guard = channel.lock().await; 
        guard.send(event).await.map_err(|e| {
            VmmError::SystemError(e.to_string())
        })?;

        drop(guard);

        Ok(())
    }

    pub async fn handle_start_vm_message(msg: &[u8], channel: Arc<Mutex<VmmApiChannel>>) -> Result<(), VmmError> {
        let request: StartVmRequest = serde_json::from_slice(msg).map_err(|e| {
            VmmError::Config(e.to_string())
        })?;

        // Verify signature if provided
        if let Some(signature) = &request.signature {
            // Create message for verification
            let message = auth::SignatureVerifier::create_operation_message("StartVmRequest", &request.id);
            
            // Verify signature
            let signer_address = auth::SignatureVerifier::verify_signature(message, signature, request.recovery_id)?;
            
            // Check if signer is authorized
            let is_authorized = auth::OwnershipVerifier::verify_authorization(
                &request.id, 
                &signer_address, 
                auth::Permission::Operator
            ).await?;
            
            if !is_authorized {
                return Err(VmmError::Config(format!(
                    "Unauthorized: Address {} is not the owner or authorized user for instance {}", 
                    signer_address, request.id
                )));
            }
        } else {
            return Err(VmmError::Config("Signature is required".to_string()));
        }

        // Proceed with starting the VM
        let event = VmmEvent::Start { id: request.id };
        let guard = channel.lock().await; 
        guard.send(event).await.map_err(|e| {
            VmmError::SystemError(e.to_string())
        })?;

        drop(guard);

        Ok(())
    }

    pub async fn write_to_queue(
        message: impl Serialize + Clone,
        sub_topic: u8,
        topic: &str
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut hasher = Sha3::v256();
        let mut topic_hash = [0u8; 32];
        hasher.update(topic.as_bytes());
        hasher.finalize(&mut topic_hash);
        let mut message_code = vec![sub_topic];
        message_code.extend(serde_json::to_vec(&message)?);
        let request = QueueRequest::Write { 
            content: message_code, 
            topic: hex::encode(topic_hash) 
        };

        match Client::new()
            .post(format!("http://127.0.0.1:{}/queue/write_local", QUEUE_PORT))
            .json(&request)
            .send().await?
            .json::<QueueResponse>().await? {
                QueueResponse::OpSuccess => return Ok(()),
                QueueResponse::Failure { reason } => return Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, format!("{reason:?}")))),
                _ => return Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, "Invalid response variant for write_local endpoint")))
        }
    }

    pub async fn read_from_queue(
        last: Option<usize>,
        n: Option<usize>,
    ) -> Result<Vec<Vec<u8>>, Box<dyn std::error::Error + Send + Sync>> {
        let mut endpoint = format!("http://127.0.0.1:{}/queue/vmm", QUEUE_PORT);
        if let Some(idx) = last {
            endpoint.push_str(&format!("/{idx}"));
            if let Some(n) = n {
                endpoint.push_str(&format!("/{n}/get_n_after"));
            } else {
                endpoint.push_str("/get_after");
            }
        } else {
            if let Some(n) = n {
                endpoint.push_str(&format!("/{n}/get_n"))
            } else {
                endpoint.push_str("/get")
            }
        }

        match Client::new()
            .get(endpoint.clone())
            .send().await?
            .json::<QueueResponse>().await? {
                QueueResponse::List(list) => Ok(list),
                QueueResponse::Failure { reason } => Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, format!("{reason:?}")))),
                _ => Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, format!("Invalid response variant for {endpoint}")))) 
        }
    }

    pub async fn start_api_server(&self, config: &crate::config::Config) -> Result<(), VmmError> {
        log::info!("Attempting to start API server");
        let app_state = self.channel.clone();

        // Create the router with public routes (no authentication)
        let public_routes = Router::new()
            .route("/health", get(health_check))
            .route("/vm/boot_complete", post(boot_complete));
        
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
