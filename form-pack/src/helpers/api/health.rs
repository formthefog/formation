use serde::{Serialize, Deserialize};
use axum::Json;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum HealthStatus {
    Healthy,
    Degraded { reason: String },
    Unhealthy { reason: String }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthResponse {
    status: HealthStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    uptime: Option<u64>
}

pub(crate) async fn health_check() -> Json<HealthResponse> {
    let version = option_env!("CARGO_PKG_VERSION").map(String::from);
    
    // Return a healthy status
    Json(HealthResponse {
        status: HealthStatus::Healthy,
        version,
        uptime: None // Could add uptime calculation if needed
    })
}
