use clap::Subcommand;
use serde::{Serialize, Deserialize};

#[derive(Clone, Debug, Serialize, Deserialize, Subcommand)]
pub enum WalletCommand {
    New,
    Get,
}
