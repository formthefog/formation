pub async fn get_vm(
    State(channel): State<Arc<Mutex<VmmApiChannel>>>,
    Json(request): Json<GetVmRequest>,
) -> Result<Json<VmInfo>, String>  {
    // Verify signature if provided
    if let Some(signature) = &request.signature {
        // Create message for verification
        let message = auth::SignatureVerifier::create_operation_message("GetVmRequest", &request.id);
        
        // Verify signature
        match auth::SignatureVerifier::verify_signature(message, signature, request.recovery_id) {
            Ok(signer_address) => {
                // Check if signer is authorized - Read access requires at least ReadOnly permission
                match auth::OwnershipVerifier::verify_authorization(
                    &request.id, 
                    &signer_address, 
                    auth::Permission::ReadOnly
                ).await {
                    Ok(true) => {
                        // Authorized - proceed with operation
                    },
                    Ok(false) => {
                        return Err(format!("Unauthorized: Address {} is not authorized to view instance {}", 
                                 signer_address, request.id));
                    },
                    Err(e) => {
                        return Err(format!("Error checking authorization: {}", e));
                    }
                }
            },
            Err(e) => {
                return Err(format!("Signature verification failed: {}", e));
            }
        }
    } else {
        return Err("Signature is required".to_string());
    }
    
    // Proceed with getting the VM information
    let event = VmmEvent::Get {
        id: request.id.clone(),
    };

    request_receive(channel, event).await
}