pub mod create;
pub mod complete;
pub mod start;
pub mod stop;
pub mod delete;


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

pub async fn list(
    State(channel): State<Arc<Mutex<VmmApiChannel>>>,
) -> Result<Json<Vec<VmInfo>>, String> {

    let event = VmmEvent::GetList {
        requestor: "test".to_string(),
    };

    request_receive(channel, event).await
}

async fn ping(
    State(channel): State<Arc<Mutex<VmmApiChannel>>>,
    Json(request): Json<PingVmmRequest>
) -> Result<Json<VmmPingResponse>, String> {
    let event = VmmEvent::Ping { name: request.name.to_string() };
    request_receive(channel, event).await
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

pub async fn request_receive(channel: Arc<Mutex<VmmApiChannel>>, event: VmmEvent) -> Result<Json<VmInfo>, String> {
    let guard = channel.lock().await;
    let response = guard.send(event).await;
    drop(guard);
    response
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
