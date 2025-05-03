use crate::api::{VmmApiChannel, VmmEvent, VmmPingResponse, PingVmmRequest};
use crate::api::helpers::request_receive;

pub async fn ping(
    State(channel): State<Arc<Mutex<VmmApiChannel>>>,
    Json(request): Json<PingVmmRequest>
) -> Result<Json<VmmPingResponse>, String> {
    let event = VmmEvent::Ping { name: request.name.to_string() };
    request_receive(channel, event).await
}