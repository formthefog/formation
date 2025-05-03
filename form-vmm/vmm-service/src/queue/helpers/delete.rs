use crate::api::VmmApiChannel;
use crate::error::VmmError;
use std::sync::Arc;
use tokio::sync::Mutex;


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