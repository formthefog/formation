use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use form_pack::manager::FormPackManager;
use tokio::sync::broadcast::channel;
use clap::{Parser, ValueEnum};

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
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let parser = Cli::parse();
    let addr = parser.interface.into_socketaddr(parser.port);
    let manager = FormPackManager::new(None, addr);
    let (tx, rx) = channel(1);
    tokio::task::spawn(async move {
        if let Err(e) = manager.run(rx).await {
            eprintln!("Error running FormPackManager: {e}");
        };
    });

    tokio::signal::ctrl_c().await?;

    tx.send(())?;

    Ok(())
}
