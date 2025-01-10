pub mod formation_rpc {
    tonic::include_proto!("formation_rpc");
    pub const FILE_DESCRIPTOR_SET: &[u8] =
        tonic::include_file_descriptor_set!("formation_descriptor");
}

pub mod heartbeat;
pub mod join;
pub mod server;
pub mod client;
pub mod helpers;

pub use helpers::*;
