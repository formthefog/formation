use alloy::{primitives::Address, signers::k256::ecdsa::SigningKey};
use clap::{Parser, Subcommand};
use form_p2p::queue::{FormMQ, QUEUE_PORT};
use tokio::sync::RwLock;
use std::{path::PathBuf, sync::Arc};
use form_config::OperatorConfig;

#[derive(Parser, Debug)]
#[command(name = "form-mq", about = "Formation Message Queue")]
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


#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Setup the logger
    simple_logger::init_with_level(log::Level::Info)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;

    // Parse command line args
    let args = CliArgs::parse();
    let config = OperatorConfig::from_file(args.config, args.encrypted, args.password.as_deref()).ok();
    match args.command {
        CliCommand::Run { signing_key, sub_addr: _, pub_addr: _ } => {
            log::info!("Acquiring signing key");
            let signing_key = if signing_key.is_none() {
                let config = config.unwrap();
                config.secret_key.unwrap()
            } else {
                signing_key.unwrap()
            };
            log::info!("Deriving address from signing key");
            let address = hex::encode(
                Address::from_private_key(
                    &SigningKey::from_slice(
                        &hex::decode(&signing_key)?
                    )?
                )
            );
            log::info!("Building shared queue");
            let queue = Arc::new(RwLock::new(FormMQ::new(address, signing_key, String::new())));
            let (shutdown_tx, shutdown_rx) = tokio::sync::broadcast::channel(1024);
            let inner_queue = queue.clone();
            let handle = tokio::spawn(async move {
                log::info!("Serving queue api on 0.0.0.0:{QUEUE_PORT}");
                if let Err(e) = form_p2p::api::serve(inner_queue, QUEUE_PORT).await {
                    eprintln!("Error serving queue api: {e}");
                } 
            });
            log::info!("Awaiting shutdown signal");
            let _ = tokio::signal::ctrl_c().await?;
            shutdown_tx.send(())?;
            handle.abort();
        }
        _ => {}
    }

    Ok(())
}
