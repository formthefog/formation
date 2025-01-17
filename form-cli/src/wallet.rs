use clap::{Args, Subcommand};
use std::{fs::OpenOptions, io::Read, path::Path};
use serde::{Serialize, Deserialize};

#[derive(Debug, Subcommand)]
pub enum WalletCommand {
    New(NewWalletCommand),
}

#[derive(Debug, Args)]
pub struct NewWalletCommand {}

pub struct Wallet;


#[derive(Debug, Serialize, Deserialize)]
pub struct Keypair {
    signing_key: String,
    verifying_key: String,
}

impl Keypair {
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self, Box<dyn std::error::Error>> {
        let mut buf = Vec::new();
        let mut file = OpenOptions::new()
            .read(true)
            .write(false)
            .open(path.as_ref())?;

        file.read_to_end(&mut buf)?;

        Ok(serde_json::from_slice(&buf)?)
    }
}
