use serde::{Serialize, Deserialize};
use clap::Args;

#[derive(Debug, Clone, Serialize, Deserialize, Args)]
pub struct PingVmmRequest {
    #[clap(long, short)]
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BootCompleteRequest {
    pub build_id: String,
    pub name: String,
    pub formnet_ip: String,
}

/// Request to create a new VM instance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateVmRequest {
    pub name: String,
    pub formfile: String,
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

#[derive(Debug, Serialize, Deserialize)]
pub enum VmmResponse {
    Success(VmResponse),
    Failure(String),
}
