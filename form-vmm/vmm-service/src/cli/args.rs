use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(name = "vmm-service", about = "Formation VMM Service")]
pub struct CliArgs {
    /// Enable debug logging
    #[arg(short, long)]
    pub debug: bool,

    /// Command to execute
    #[command(subcommand)]
    pub command: CliCommand,
}

#[derive(Subcommand, Debug)]
pub enum CliCommand {
    /// Run the VMM service
    #[command(name = "run")]
    Run {
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
