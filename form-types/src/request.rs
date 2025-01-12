use serde::{Serialize, Deserialize};
use clap::Args;

#[derive(Debug, Clone, Serialize, Deserialize, Args)]
pub struct PingVmmRequest {
    #[clap(long, short)]
    pub name: String,
}

/// Request to create a new VM instance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateVmRequest {
    pub distro: String,
    pub version: String,
    pub memory_mb: u64,
    pub vcpu_count: u8,
    pub name: String,
    pub user_data: Option<String>,
    pub meta_data: Option<String>,
    pub signature: Option<String>,
    pub recovery_id: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StartVmRequest {
    pub id: String,
    pub name: String,
    pub signature: Option<String>,
    pub recovery_id: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StopVmRequest {
    pub id: String,
    pub name: String,
    pub signature: Option<String>,
    pub recovery_id: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteVmRequest {
    pub id: String,
    pub name: String,
    pub signature: Option<String>,
    pub recovery_id: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetVmRequest {
    pub id: String,
    pub name: String,
    pub signature: Option<String>,
    pub recovery_id: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListRequest {
    pub requestor: String,
    pub recovery_id: u32,
}

/// Response containing VM information
#[derive(Debug, Serialize, Deserialize)]
pub struct VmResponse {
    pub id: String,
    pub name: String,
    pub state: String,
}
