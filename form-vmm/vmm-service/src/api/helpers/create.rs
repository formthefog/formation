use axum::{extract::{Extension, State}, Json};
use std::sync::Arc;
use tokio::sync::Mutex;
use crate::api::{VmmApiChannel, AuthenticatedUser, VmmResponse};
use form_types::{BootCompleteRequest, CreateVmRequest, DeleteVmRequest, GetVmRequest, PingVmmRequest, StartVmRequest, StopVmRequest, VmResponse, VmmEvent, VmmResponse};

pub async fn create(
    State(channel): State<Arc<Mutex<VmmApiChannel>>>,
    Extension(auth_user): Extension<AuthenticatedUser>,
    Json(request): Json<CreateVmRequest>,
) -> Json<VmmResponse> {
    log::info!("Received VM create request: name={} from {}", request.name, auth_user.address);
    
    // Use the authenticated address as the owner
    let owner = auth_user.address.clone();
    
    let event = VmmEvent::Create {
        formfile: request.formfile.clone(),
        name: request.name.clone(),
        owner,
    };

    let guard = channel.lock().await;

    if let Err(e) = guard.send(event.clone())
        .await {
            log::info!("Error sending {event:?}: {e}");
            return Json(
                VmmResponse::Failure(
                    format!(
                        "Error sending event {event:?} across VmmApiChannel to request creation of vm {}",
                        request.name
                    )
                )
            )
    }

    drop(guard);

    Json(VmmResponse::Success(VmResponse {
        id: "pending".to_string(),
        name: request.name,
        state: "PENDING".to_string()
    }))
}

