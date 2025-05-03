use axum::{extract::{Extension, State}, Json};
use std::sync::Arc;
use tokio::sync::Mutex;
use crate::api::{VmmApiChannel, AuthenticatedUser, VmmResponse};
use form_types::{BootCompleteRequest, CreateVmRequest, DeleteVmRequest, GetVmRequest, PingVmmRequest, StartVmRequest, StopVmRequest, VmResponse, VmmEvent, VmmResponse};

pub async fn complete(
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

