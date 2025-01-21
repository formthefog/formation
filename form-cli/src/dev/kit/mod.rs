use serde::{Serialize, Deserialize};
use clap::Subcommand;

pub mod init;
pub mod util;
pub use init::*;
pub use util::*;

#[derive(Clone, Debug, Serialize, Deserialize, Subcommand)]
pub enum KitCommand {
    Init(Init),
    Load,
}
