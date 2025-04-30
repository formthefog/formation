use serde::{Serialize, Deserialize};
use crdts::bft_reg::RecoverableSignature;
use crate::formfile::Formfile;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PackBuildRequest {
    pub sig: RecoverableSignature,
    pub hash: [u8; 32],
    pub request: PackRequest,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PackRequest {
    pub name: String,
    pub formfile: Formfile,
    pub artifacts: Vec<u8>,
}
