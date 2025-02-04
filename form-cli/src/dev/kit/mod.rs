use serde::{Serialize, Deserialize};
use clap::Subcommand;

pub mod init;
pub mod util;
pub mod operator;
pub use operator::*;
pub use init::*;
pub use util::*;

#[derive(Clone, Debug, Serialize, Deserialize, Subcommand)]
pub enum KitCommand {
    Init(Init),
    #[clap(subcommand)]
    Operator(Operator)
}
