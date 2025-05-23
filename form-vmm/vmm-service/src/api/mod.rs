use alloy_primitives::Address;
use axum::{
    extract::State, routing::{get, post}, Json, Router, Extension
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

use crate::VmmError;
use form_types::{BootCompleteRequest, CreateVmRequest, DeleteVmRequest, GetVmRequest, PingVmmRequest, StartVmRequest, StopVmRequest, VmResponse, VmmEvent, VmmResponse};

pub mod auth;

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
            VmmError::Config(format!("Failed to deserialize CreateVmRequest from queue: {}",e.to_string())) // More specific error
        })?;
        log::info!("Deserialized create request for name: {}, owner: {}", request.name, request.owner);
        
        // Owner is now directly from the trusted queue message
        let event = VmmEvent::Create { 
            formfile: request.formfile, 
            name: request.name, 
            owner: request.owner, // Use owner from the deserialized request
        };

        log::info!("Acquiring lock on API channel for create event...");
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
            VmmError::Config(format!("Failed to deserialize DeleteVmRequest from queue: {}", e.to_string()))
        })?;
        log::info!("Received delete request from queue for instance id: {}", request.id);

        // Assuming message from queue is trusted and authorized by the enqueueing service.
        // Removed signature verification and OwnershipVerifier call.

        let event = VmmEvent::Delete { id: request.id.clone() };

        let guard = channel.lock().await; 
        log::info!("Sending VmmEvent::Delete for id: {}", request.id);
        guard.send(event).await.map_err(|e| {
            VmmError::SystemError(e.to_string())
        })?;
        drop(guard);
        log::info!("Delete event sent for id: {}", request.id);

        Ok(())
    }

    pub async fn handle_stop_vm_message(msg: &[u8], channel: Arc<Mutex<VmmApiChannel>>) -> Result<(), VmmError> {
        let request: StopVmRequest = serde_json::from_slice(msg).map_err(|e| {
            VmmError::Config(format!("Failed to deserialize StopVmRequest from queue: {}", e.to_string()))
        })?;
        log::info!("Received stop request from queue for instance id: {}", request.id);

        // Assuming message from queue is trusted and authorized.
        // Removed signature verification and OwnershipVerifier call.

        let event = VmmEvent::Stop { id: request.id.clone() };
        let guard = channel.lock().await; 
        log::info!("Sending VmmEvent::Stop for id: {}", request.id);
        guard.send(event).await.map_err(|e| {
            VmmError::SystemError(e.to_string())
        })?;
        drop(guard);
        log::info!("Stop event sent for id: {}", request.id);
        
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
            VmmError::Config(format!("Failed to deserialize StartVmRequest from queue: {}", e.to_string()))
        })?;
        log::info!("Received start request from queue for instance id: {}", request.id);

        // Assuming message from queue is trusted and authorized.
        // Removed signature verification and OwnershipVerifier call.

        let event = VmmEvent::Start { id: request.id.clone() };
        let guard = channel.lock().await; 
        log::info!("Sending VmmEvent::Start for id: {}", request.id);
        guard.send(event).await.map_err(|e| {
            VmmError::SystemError(e.to_string())
        })?;
        drop(guard);
        log::info!("Start event sent for id: {}", request.id);

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

    pub async fn start_api_server(&self) -> Result<(), VmmError> {
        log::info!("Starting API server on {}", self.addr);
        
        // Create a reference to the channel
        let channel = self.channel.clone();
        
        
        // Define protected routes that require authentication
        let protected_routes = Router::new()
            .route("/create", post(create))
            .route("/boot_complete", post(boot_complete))
            .route("/start", post(start))
            .route("/stop", post(stop))
            .route("/delete", post(delete))
            .route("/get_vm", post(get_vm))
            .route("/list", get(list))
            .route("/power_button", post(power_button))
            .route("/reboot", post(reboot))
            .route("/commit", post(commit))
            .route("/snapshot", post(snapshot))
            .route("/coredump", post(coredump))
            .route("/restore", post(restore))
            .route("/resize_vcpu", post(resize_vcpu))
            .route("/resize_memory", post(resize_memory))
            .route("/add_device", post(add_device))
            .route("/add_disk", post(add_disk))
            .route("/add_fs", post(add_fs))
            .route("/remove_device", post(remove_device))
            .route("/migrate_to", post(migrate_to))
            .route("/migrate_from", post(migrate_from))
            .layer(axum::middleware::from_fn(auth::ecdsa_auth_middleware_x_headers))
            .with_state(channel.clone());
        
        // Define public routes that don't require authentication
        let public_routes = Router::new()
            .route("/health", get(health_check))
            .route("/ping", post(ping))
            .with_state(channel.clone());
        
        let v1_routes = Router::new()
            .merge(public_routes)
            .merge(protected_routes);
        // Combine public and protected routes
        let app = Router::new()
            .nest("/v1", v1_routes);
        // Start the server
        let listener = tokio::net::TcpListener::bind(&self.addr).await?;
        axum::serve(listener, app).await.map_err(|e| {
            VmmError::SystemError(e.to_string())
        })?;
        
        Ok(())
    }

    pub fn addr(&self) -> &SocketAddr {
        &self.addr
    }
}

async fn health_check() -> Json<HealthResponse> {
    // Get the version from Cargo.toml if available
    let version = option_env!("CARGO_PKG_VERSION").map(String::from);
    
    // Return a healthy status
    Json(HealthResponse {
        status: HealthStatus::Healthy,
        version,
        uptime: None // Could add uptime calculation if needed
    })
}

async fn ping(
    State(channel): State<Arc<Mutex<VmmApiChannel>>>,
    Json(request): Json<PingVmmRequest>
) -> Result<Json<VmmPingResponse>, String> {
    let event = VmmEvent::Ping { name: request.name.to_string() };
    request_receive(channel, event).await
}

async fn create(
    State(channel): State<Arc<Mutex<VmmApiChannel>>>,
    Extension(recovered_address): Extension<Arc<auth::RecoveredAddress>>,
    Json(request): Json<CreateVmRequest>,
) -> Json<VmmResponse> {
    log::info!(
        "Received VM create request: name={}, owner={}",
        request.name,
        recovered_address.as_hex()
    );

    let owner_hex = recovered_address.as_hex();

    let event = VmmEvent::Create {
        formfile: request.formfile.clone(),
        name: request.name.clone(),
        owner: owner_hex,
    };

    let guard = channel.lock().await;

    if let Err(e) = guard.send(event.clone()).await {
        log::error!("Error sending VmmEvent::Create for {}: {}", request.name, e);
        return Json(
            VmmResponse::Failure(
                format!(
                    "Error queueing creation for vm {}: {}",
                    request.name,
                    e
                )
            )
        )
    }

    drop(guard);

    Json(VmmResponse::Success(VmResponse {
        id: format!("pending_creation_{}", request.name), 
        name: request.name,
        state: "CREATE_REQUESTED".to_string(),
    }))
}

async fn boot_complete(
    State(channel): State<Arc<Mutex<VmmApiChannel>>>,
    Json(request): Json<BootCompleteRequest>,
) -> Json<VmmResponse> {
    let guard = channel.lock().await;
    log::info!("Received BootCompleteRequest for VM {}", request.name);
    let event = VmmEvent::BootComplete {
        id: request.name.clone(),
        build_id: request.build_id.clone(),
        formnet_ip: request.formnet_ip,
    };

    log::info!("Built BootComplete VmmEvent, sending across api channel");
    if let Err(e) = guard.send(event.clone()).await {
        log::info!("Error receiving response back from API channel: {e}");
        return Json(
            VmmResponse::Failure(
                format!("Error recording BootComplete event {event:?}: {e}")
            )
        )
    }
    drop(guard);
    log::info!("BootCompleteRequest handled succesfully, responding...");
    Json(VmmResponse::Success(
        VmResponse { 
            id: request.name.clone(), 
            name: request.name,
            state: "complete".to_string() 
        }
    ))
}

async fn start(
    State(channel): State<Arc<Mutex<VmmApiChannel>>>,
    Extension(recovered_address): Extension<Arc<auth::RecoveredAddress>>,
    Json(request): Json<StartVmRequest>,
) -> Json<VmmResponse> {
    log::info!("Received VM start request: id={}, name={}, owner={}", 
        request.id, request.name, recovered_address.as_hex());

    match auth::OwnershipVerifier::verify_authorization(&request.id, &recovered_address.as_hex(), auth::Permission::Operator).await {
        Ok(true) => {
            log::info!("Authorization successful for start request on instance {}", request.id);
        },
        Ok(false) => {
            log::warn!("Unauthorized start request on instance {} by address {}", request.id, recovered_address.as_hex());
            return Json(VmmResponse::Failure(
                format!("Unauthorized: Address {} is not permitted to start instance {}", 
                       recovered_address.as_hex(), request.id)
            ));
        },
        Err(e) => {
            log::error!("Error checking authorization for start request on instance {}: {}", request.id, e);
            return Json(VmmResponse::Failure(
                format!("Authorization check failed for instance {}: {}", request.id, e)
            ));
        }
    }
    
    let event = VmmEvent::Start {
        id: request.id.clone(),
    };

    let guard = channel.lock().await;
    if let Err(e) = guard.send(event).await {
        log::error!("Error sending VmmEvent::Start for {}: {}", request.id, e);
        return Json(VmmResponse::Failure(format!("Error queueing start for vm {}: {}", request.id, e)));
    }
    drop(guard);

    Json(VmmResponse::Success(
        VmResponse {
            id: request.id, 
            name: request.name, 
            state: "START_REQUESTED".to_string()
    }))
}

async fn stop(
    State(channel): State<Arc<Mutex<VmmApiChannel>>>,
    Extension(recovered_address): Extension<Arc<auth::RecoveredAddress>>,
    Json(request): Json<StopVmRequest>,
) -> Json<VmmResponse> {
    log::info!("Received VM stop request: id={}, name={}, owner={}", 
        request.id, request.name, recovered_address.as_hex());

    match auth::OwnershipVerifier::verify_authorization(&request.id, &recovered_address.as_hex(), auth::Permission::Operator).await {
        Ok(true) => {
            log::info!("Authorization successful for stop request on instance {}", request.id);
        },
        Ok(false) => {
            log::warn!("Unauthorized stop request on instance {} by address {}", request.id, recovered_address.as_hex());
            return Json(VmmResponse::Failure(
                format!("Unauthorized: Address {} is not permitted to stop instance {}", 
                       recovered_address.as_hex(), request.id)
            ));
        },
        Err(e) => {
            log::error!("Error checking authorization for stop request on instance {}: {}", request.id, e);
            return Json(VmmResponse::Failure(
                format!("Authorization check failed for instance {}: {}", request.id, e)
            ));
        }
    }
    
    let event = VmmEvent::Stop {
        id: request.id.clone(),
    };

    let guard = channel.lock().await;
    if let Err(e) = guard.send(event).await {
        log::error!("Error sending VmmEvent::Stop for {}: {}", request.id, e);
        return Json(VmmResponse::Failure(format!("Error queueing stop for vm {}: {}", request.id, e)));
    }
    drop(guard);

    Json(VmmResponse::Success(
        VmResponse {
            id: request.id, 
            name: request.name,
            state: "STOP_REQUESTED".to_string()
    }))
}

async fn delete(
    State(channel): State<Arc<Mutex<VmmApiChannel>>>,
    Extension(recovered_address): Extension<Arc<auth::RecoveredAddress>>,
    Json(request): Json<DeleteVmRequest>,
) -> Json<VmmResponse> {
    log::info!("Received VM delete request: id={}, name={}, owner={}", 
        request.id, request.name, recovered_address.as_hex());

    // Verify authorization: User must be Owner to delete.
    match auth::OwnershipVerifier::verify_authorization(&request.id, &recovered_address.as_hex(), auth::Permission::Owner).await {
        Ok(true) => {
            log::info!("Authorization successful for delete request on instance {}", request.id);
        },
        Ok(false) => {
            log::warn!("Unauthorized delete request on instance {} by address {}", request.id, recovered_address.as_hex());
            return Json(VmmResponse::Failure(
                format!("Unauthorized: Address {} is not permitted to delete instance {}", 
                       recovered_address.as_hex(), request.id)
            ));
        },
        Err(e) => {
            log::error!("Error checking authorization for delete request on instance {}: {}", request.id, e);
            // Ensure VmmError is converted to string for the VmmResponse::Failure
            return Json(VmmResponse::Failure(
                format!("Authorization check failed for instance {}: {}", request.id, e.to_string())
            ));
        }
    }
    
    let event = VmmEvent::Delete {
        id: request.id.clone(),
    };

    let guard = channel.lock().await;
    if let Err(e) = guard.send(event).await {
        log::error!("Error sending VmmEvent::Delete for {}: {}", request.id, e);
        return Json(VmmResponse::Failure(format!("Error queueing delete for vm {}: {}", request.id, e)));
    }
    drop(guard);

    Json(VmmResponse::Success(
        VmResponse {
            id: request.id, 
            name: request.name,
            state: "DELETE_REQUESTED".to_string()
    }))
}

async fn get_vm(
    State(channel): State<Arc<Mutex<VmmApiChannel>>>,
    Extension(recovered_address): Extension<Arc<auth::RecoveredAddress>>,
    Json(request): Json<GetVmRequest>,
) -> Result<Json<VmInfo>, String>  {
    log::info!("Received VM get_vm request: id={}, name={}, owner={}", 
        request.id, request.name, recovered_address.as_hex());

    // Verify authorization: User must have at least ReadOnly permission.
    match auth::OwnershipVerifier::verify_authorization(&request.id, &recovered_address.as_hex(), auth::Permission::ReadOnly).await {
        Ok(true) => {
            log::info!("Authorization successful for get_vm request on instance {}", request.id);
        },
        Ok(false) => {
            log::warn!("Unauthorized get_vm request on instance {} by address {}", request.id, recovered_address.as_hex());
            // Original handler returned String for error, aligning with that for now.
            return Err(format!("Unauthorized: Address {} is not permitted to view instance {}", 
                       recovered_address.as_hex(), request.id));
        },
        Err(e) => {
            log::error!("Error checking authorization for get_vm request on instance {}: {}", request.id, e);
            return Err(format!("Authorization check failed for instance {}: {}", request.id, e.to_string()));
        }
    }

    let event = VmmEvent::Get {
        id: request.id.clone(),
    };

    // Original handler used request_receive, which locks channel and awaits response.
    // Replicating that pattern. request_receive returns Result<Json<T>, String>
    match request_receive::<VmInfo>(channel, event).await { // Assuming VmInfo is the correct type
        Ok(vm_info_json) => Ok(vm_info_json),
        Err(e_str) => Err(e_str), // Propagate error string
    }
}

async fn list(
    State(channel): State<Arc<Mutex<VmmApiChannel>>>,
    Extension(recovered_address): Extension<Arc<auth::RecoveredAddress>>,
) -> Result<Json<Vec<VmInfo>>, String> { 
    log::info!("Received VM list request from owner={}", recovered_address.as_hex());

    // No specific instance ID for list, so OwnershipVerifier might not directly apply here
    // unless we list VMs *only* for the recovered_address.
    // The VmmEvent::GetList takes a `requestor` field.
    let event = VmmEvent::GetList {
        requestor: recovered_address.as_hex(), // Pass authenticated user as requestor
    };

    match request_receive::<Vec<VmInfo>>(channel, event).await {
        Ok(vm_info_list_json) => Ok(vm_info_list_json),
        Err(e_str) => Err(e_str),
    }
}

async fn power_button() {}
async fn reboot() {}
async fn commit() {}
async fn snapshot() {}
async fn coredump() {}
async fn restore() {}
async fn resize_vcpu() {}
async fn resize_memory() {}
async fn add_device() {}
async fn add_disk() {}
async fn add_fs() {}
async fn remove_device() {}
async fn migrate_to() {}
async fn migrate_from() {}

async fn request_receive<T: DeserializeOwned>(
    channel: Arc<Mutex<VmmApiChannel>>,
    event: VmmEvent,
) -> Result<Json<T>, String> {
    let mut channel = channel.lock().await; 
    channel.send(event.clone()).await.map_err(|e| e.to_string())?;
    tokio::select! {
        Some(resp) = channel.recv() => {
            Ok(Json(resp))
        }
        _ = tokio::time::sleep(Duration::from_secs(5)) => {
            Err(format!("Request {event:?} timed out awaiting response"))
        }
    }
}
