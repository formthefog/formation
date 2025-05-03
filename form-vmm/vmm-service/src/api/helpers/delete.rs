use crate::api::{VmmApiChannel, VmmEvent, VmmResponse, AuthenticatedUser};
use crate::api::helpers::{request_receive, auth};
use axum::{extract::{Extension, State}, Json};
use std::sync::Arc;
use tokio::sync::Mutex;
use form_types::{BootCompleteRequest, CreateVmRequest, DeleteVmRequest, GetVmRequest, PingVmmRequest, StartVmRequest, StopVmRequest, VmResponse, VmmEvent, VmmResponse};

pub async fn delete(
    State(channel): State<Arc<Mutex<VmmApiChannel>>>,
    Json(request): Json<DeleteVmRequest>,
) -> Json<VmmResponse> {
    // Verify signature if provided
    if let Some(signature) = &request.signature {
        // Create message for verification
        let message = auth::SignatureVerifier::create_operation_message("DeleteVmRequest", &request.id);
        
        // Verify signature
        match auth::SignatureVerifier::verify_signature(message, signature, request.recovery_id) {
            Ok(signer_address) => {
                // Check if signer is authorized
                match auth::OwnershipVerifier::verify_authorization(
                    &request.id, 
                    &signer_address, 
                    auth::Permission::Owner // Deletion requires owner permission
                ).await {
                    Ok(true) => {
                        // Authorized - proceed with operation
                    },
                    Ok(false) => {
                        return Json(VmmResponse::Failure(
                            format!("Unauthorized: Address {} is not the owner of instance {}", 
                                   signer_address, request.id)
                        ));
                    },
                    Err(e) => {
                        return Json(VmmResponse::Failure(
                            format!("Error checking authorization: {}", e)
                        ));
                    }
                }
            },
            Err(e) => {
                return Json(VmmResponse::Failure(
                    format!("Signature verification failed: {}", e)
                ));
            }
        }
    } else {
        return Json(VmmResponse::Failure("Signature is required".to_string()));
    }
    
    // Proceed with deleting the VM
    let event = VmmEvent::Delete {
        id: request.id.clone(),
    };

    if let Err(e) = request_receive::<()>(channel, event).await {
        return Json(VmmResponse::Failure(e.to_string()))
    }

    Json(VmmResponse::Success(
        VmResponse {
            id: request.id, 
            name: request.name,
            state: "pending".to_string()
    }))
}
