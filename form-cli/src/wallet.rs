use clap::{Args, Subcommand};

#[derive(Debug, Subcommand)]
pub enum WalletCommand {
    New(NewWalletCommand),
}

#[derive(Debug, Args)]
pub struct NewWalletCommand {}

pub struct Wallet;
