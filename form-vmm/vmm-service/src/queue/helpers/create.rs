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