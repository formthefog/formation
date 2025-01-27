use std::path::PathBuf;

use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(name = "vmm-service", about = "Formation VMM Service")]
pub struct CliArgs {
    /// Enable debug logging
    #[arg(short, long, default_value="false")]
    pub debug: bool,
    #[arg(short='C', long, default_value_os_t=PathBuf::from("/etc/formation/.operator-config.json"))]
    pub config: PathBuf,
    #[arg(short, long, default_value="true")]
    pub encrypted: bool,
    #[arg(short, long)]
    pub password: Option<String>,
    /// Command to execute
    #[command(subcommand)]
    pub command: CliCommand,
}

#[derive(Subcommand, Debug)]
pub enum CliCommand {
    /// Run the VMM service
    #[command(name = "run")]
    Run {
        #[clap(aliases=["secret-key", "private-key"])]
        signing_key: Option<String>,
        /// Message broker subscriber address
        #[arg(long, short)]
        sub_addr: Option<String>,
        /// Message broker Publish Address
        #[arg(long, short)]
        pub_addr: Option<String>,
    },
    /// Show service status
    #[command(name = "status")]
    Status,
}
