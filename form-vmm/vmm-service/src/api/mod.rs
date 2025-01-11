use axum::{
    routing::{get, post},
    Router,
    Json,
    extract::State,
};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use tokio::sync::mpsc;
use tokio::sync::Mutex;
use vmm::api::VmmPingResponse;
use std::{sync::Arc, time::Duration};
use std::net::SocketAddr;

use crate::VmmError;
use form_types::VmmEvent;

pub struct VmmApiChannel {
    event_sender: mpsc::Sender<VmmEvent>,
    response_receiver: mpsc::Receiver<String>
}

impl VmmApiChannel {
    pub fn new(tx: mpsc::Sender<VmmEvent>, rx: mpsc::Receiver<String>) -> Self {
        Self{
            event_sender: tx,
            response_receiver: rx
        }
    }

    pub async fn send(&self, event: VmmEvent) -> Result<(), mpsc::error::SendError<VmmEvent>> {
        self.event_sender.send(event).await
    }

    pub async fn recv<T: DeserializeOwned>(&mut self) -> Option<T> {
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PingVmmRequest {
    name: String,
}

/// Request to create a new VM instance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateVmRequest {
    pub distro: String,
    pub version: String,
    pub memory_mb: u64,
    pub vcpu_count: u8,
    pub name: String,
    pub user_data: Option<String>,
    pub meta_data: Option<String>,
    pub signature: Option<String>,
    pub recovery_id: u32,
}

/// Response containing VM information
#[derive(Debug, Serialize, Deserialize)]
pub struct VmResponse {
    pub id: String,
    pub name: String,
    pub state: String,
}

impl VmmApi {
    pub fn new(
        event_sender: mpsc::Sender<VmmEvent>,
        response_receiver: mpsc::Receiver<String>,
        addr: SocketAddr
    ) -> Self {
        let api_channel = Arc::new(Mutex::new(VmmApiChannel::new(
            event_sender,
            response_receiver
        )));
        Self {
            channel: api_channel, addr
        }
    }

    pub async fn start(&self) -> Result<(), VmmError> {
        log::info!("Attempting to start API server");
        let app_state = self.channel.clone();

        log::info!("Establishing Routes");
        let app = Router::new()
            .route("/vm/:id/ping", post(ping))
            .route("/health", get(health_check))
            .route("/vm", post(create))
            .route("/vm/:id/start", post(start_vm))
            .route("/vm/:id/stop", post(stop_vm))
            .route("/vm/:id/delete", post(delete_vm))
            .route("/vm/:id", get(get_vm))
            .route("/vms", get(list_vms))
            .with_state(app_state);

        log::info!("Established route, binding to {}", &self.addr);
        let listener = tokio::net::TcpListener::bind(self.addr.clone()).await
            .map_err(|e| VmmError::SystemError(format!("Failed to bind listener to address {}: {e}", self.addr.clone())))?;
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

async fn health_check() -> &'static str {
    "OK"
}

async fn create(
    State(channel): State<Arc<Mutex<VmmApiChannel>>>,
    Json(request): Json<CreateVmRequest>,
) -> Result<Json<VmResponse>, String> {
    log::info!(
        "Received VM create request: name={}, distro={}, version={}",
        request.name, request.distro, request.version
    );
    // Convert request into a VmmEvent::Create
    let event = VmmEvent::Create {
        owner: "test".to_string(),
        recovery_id: 0,
        requestor: "test-api".to_string(),
        distro: request.distro,
        version: request.version,
        user_data: request.user_data,
        meta_data: request.meta_data,
        memory_mb: request.memory_mb,
        vcpu_count: request.vcpu_count,
        name: request.name.clone(),
        custom_cmdline: None,
        rng_source: None,
        console_type: None,
    };

    log::info!("Sending create event to VMM service: {:?}", event);

    channel.lock().await.send(event).await.map_err(|e| e.to_string())?;

    log::info!("VM Creation request processed for {}", request.name);

    Ok(Json(VmResponse {
        id: "pending".to_string(),
        name: request.name,
        state: "creating".to_string()
    }))

}

async fn ping(
    State(channel): State<Arc<Mutex<VmmApiChannel>>>,
    Json(request): Json<PingVmmRequest>
) -> Result<Json<VmmPingResponse>, String> {
    let event = VmmEvent::Ping { name: request.name.to_string() };
    let mut channel = channel.lock().await; 
    channel.send(event).await.map_err(|e| e.to_string())?;
    tokio::select! {
        Some(resp) = channel.recv() => {
            Ok(Json(resp))
        }
        _ = tokio::time::sleep(Duration::from_secs(5)) => {
            Err("Ping request timed out awaiting response".to_string())
        }
    }
}

async fn start_vm() {}
async fn stop_vm() {}
async fn delete_vm() {}
async fn get_vm() {}
async fn list_vms() {}
