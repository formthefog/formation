use axum::Json;
use serde_json::Value;

pub(crate) async fn handle_ping() -> Json<Value> {
    println!("Received ping request, responding");
    Json(serde_json::json!({"ping": "pong"}))
}
