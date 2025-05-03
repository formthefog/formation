use crate::api::{VmmApiChannel, VmmEvent};

pub async fn request_receive(channel: Arc<Mutex<VmmApiChannel>>, event: VmmEvent) -> Result<Json<VmInfo>, String> {
    let guard = channel.lock().await;
    let response = guard.send(event).await;
    drop(guard);
    response
}