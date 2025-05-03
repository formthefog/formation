use crate::api::{VmmApiChannel, AuthenticatedUser, VmmResponse, VmmEvent, VmResponse};
use crate::api::helpers::{request_receive, auth};
use axum::{extract::{Extension, State}, Json};
use std::sync::Arc;
use tokio::sync::Mutex;

pub async fn start(
    State(channel): State<Arc<Mutex<VmmApiChannel>>>,
    Extension(auth_user): Extension<AuthenticatedUser>,
    Path(id): Path<String>,
) -> Json<VmmResponse> {
    // Check if user is authorized for this operation
    match auth::OwnershipVerifier::verify_authorization(
        &id,
        &auth_user.address,
        auth::Permission::Operator
    ).await {
        Ok(true) => {
            // Authorized, proceed with starting the VM
            let event = VmmEvent::Start { id: id.clone() };
            
            let result = request_receive::<()>(channel, event).await;
            match result {
                Ok(_) => Json(VmmResponse::Success(
                    VmResponse {
                        id: id.clone(),
                        name: id.clone(),  // Use ID as the name
                        state: "pending".to_string()
                    }
                )),
                Err(e) => Json(VmmResponse::Failure(e.to_string()))
            }
        },
        Ok(false) => Json(VmmResponse::Failure(
            format!("Unauthorized: Address {} is not authorized to start instance {}", 
                   auth_user.address, id)
        )),
        Err(e) => Json(VmmResponse::Failure(
            format!("Error checking authorization: {}", e)
        )),
    }
}

