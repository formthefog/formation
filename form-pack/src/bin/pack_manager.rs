use std::{net::{IpAddr, Ipv4Addr, SocketAddr}, path::PathBuf};
use alloy_primitives::Address;
use form_pack::manager::FormPackManager;
use k256::ecdsa::SigningKey;
use tokio::sync::broadcast::channel;
use clap::{Parser, ValueEnum};
use form_config::OperatorConfig;
use log::LevelFilter;

#[derive(Clone, Debug, ValueEnum)]
enum Interface {
    All,
    Localhost,
}

impl Interface {
    pub fn into_socketaddr(&self, port: u16) -> SocketAddr {
        match self { 
            Self::All => SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0,0,0,0)), port),
            Self::Localhost => SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), port)
        }
    }
}

#[derive(Clone, Debug, Parser)]
pub struct Cli {
    #[clap(long, short)]
    interface: Interface,
    #[clap(long, short)]
    port: u16,
    #[clap(long, short)]
    config: PathBuf,
    #[clap(long, short, default_value_t=true)]
    encrypted: bool,
    #[clap(long, short='P')]
    password: String
    
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    simple_logger::SimpleLogger::new().with_level(LevelFilter::Info).init().unwrap_or_else(|e| {
        eprintln!("Failed to initialize logger: {}", e);
    });

    log::info!("form-pack-manager starting...");

    let parser = Cli::parse();
    let addr = parser.interface.into_socketaddr(parser.port);
    log::info!("Attempting to read config from: {:?}", parser.config);
    let config = OperatorConfig::from_file(parser.config, parser.encrypted, Some(&parser.password))?;
    let signing_key = config.secret_key.unwrap();
    let pk = SigningKey::from_slice(
        &hex::decode(&signing_key)?
    )?;

    let node_id = hex::encode(Address::from_private_key(&pk));
    log::info!("form-pack-manager Node ID: {}", node_id);
    log::info!("form-pack-manager listening on: {}", addr);

    let manager = FormPackManager::new(addr, node_id);
    let (tx, rx) = channel(1);
    tokio::task::spawn(async move {
        if let Err(e) = manager.run(rx).await {
            log::error!("Error running FormPackManager: {}", e);
        };
    });

    tokio::signal::ctrl_c().await?;
    log::info!("Ctrl-C received, shutting down form-pack-manager...");

    tx.send(())?;

    Ok(())
}
