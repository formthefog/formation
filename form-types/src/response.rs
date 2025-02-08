use serde::{Serialize, Deserialize};

/// Response containing VM information
#[derive(Debug, Serialize, Deserialize)]
pub struct VmResponse {
    pub id: String,
    pub name: String,
    pub state: String,
}
