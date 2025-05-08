use serde::{Serialize, Deserialize};
use crate::types::request::PackBuildRequest;
use crate::types::status::PackBuildStatus;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum PackResponse {
    Success,
    Failure,
    Status(PackBuildStatus)
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PackBuildResponse {
    pub(crate) status: PackBuildStatus,
    pub(crate) request: PackBuildRequest,
}
