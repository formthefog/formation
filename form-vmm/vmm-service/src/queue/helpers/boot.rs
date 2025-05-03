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