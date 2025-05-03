use crate::api::VmmApiChannel;
use crate::error::VmmError;
use std::sync::Arc;
use tokio::sync::Mutex;

pub async fn handle_reboot_vm_message(msg: &[u8], channel: Arc<Mutex<VmmApiChannel>>) -> Result<(), VmmError> {
    let request: StopVmRequest = serde_json::from_slice(msg).map_err(|e| {
        VmmError::Config(e.to_string())
    })?;

    let event = VmmEvent::Stop { id: request.id.clone() };

    let guard = channel.lock().await; 
    guard.send(event).await.map_err(|e| {
        VmmError::SystemError(e.to_string())
    })?;

    let event = VmmEvent::Start { id: request.id };
    let guard = channel.lock().await; 
    guard.send(event).await.map_err(|e| {
        VmmError::SystemError(e.to_string())
    })?;

    drop(guard);

    Ok(())
}