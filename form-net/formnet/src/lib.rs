pub mod init;
pub mod add_peer;
pub mod add_cidr;
pub mod serve;
pub mod join;
pub mod up;
pub mod fetch;
pub mod redeem;
pub mod add_assoc;
pub mod leave;
pub mod resolve;
pub mod api;
pub mod relay;
pub mod nat_relay;
pub mod bootstrap;

pub use init::*;
pub use add_peer::*;
pub use serve::*;
pub use join::*;
pub use leave::*;
pub use up::*;
pub use fetch::*;
pub use redeem::*;
pub use add_cidr::*;
pub use add_assoc::*;
pub use resolve::*;
// Don't use relay::* to avoid polluting the namespace,
// users should access relay functionality through the relay module

pub const CONFIG_DIR: &'static str = "/etc/formnet";
pub const DATA_DIR: &'static str = "/var/lib/formnet";
pub const SERVER_CONFIG_DIR: &'static str = "/etc/formnet";
pub const SERVER_DATA_DIR: &'static str = "/var/lib/formnet";
pub const NETWORK_NAME: &str = "formnet";

pub async fn api_shutdown_handler(
    mut rx: tokio::sync::broadcast::Receiver<()>
) {
    tokio::select! {
        res = rx.recv() => {
            log::info!("Received shutdown signal for api server: {res:?}");
        }
    }
}
