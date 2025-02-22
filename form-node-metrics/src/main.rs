use std::{path::PathBuf, time::Duration};
use alloy_primitives::Address;
use clap::Parser;
use form_config::OperatorConfig;
use form_node_metrics::{capabilities::NodeCapabilities, capacity::start_capacity_monitor, heartbeat::heartbeat, metrics::start_metrics_monitor, util::{report_initial_metrics, report_metrics}};
use k256::ecdsa::SigningKey;
use tokio::sync::broadcast::channel;

#[derive(Clone, Debug, Parser)]
pub struct Cli {
    #[clap(long, short)]
    config: PathBuf,
    #[clap(long, short, default_value_t=true)]
    encrypted: bool,
    #[clap(long, short='P')]
    password: String
    
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let parser = Cli::parse();
    let config = OperatorConfig::from_file(parser.config, parser.encrypted, Some(&parser.password))?;
    let config = config;
    let signing_key = config.secret_key.unwrap();
    let node_id = hex::encode(Address::from_private_key(&SigningKey::from_slice(
        &hex::decode(&signing_key)?
    )?));

    let (tx, mut rx) = channel(2);

    let capabilities = NodeCapabilities::collect();
    let capacity = start_capacity_monitor(Duration::from_secs(30)).await;
    let metrics = start_metrics_monitor(Duration::from_secs(30)).await;

    report_initial_metrics(capabilities, capacity.clone(), node_id.clone()).await;

    let inner_node_id = node_id.clone();
    tokio::spawn(async move {
        tokio::select! {
            _ = heartbeat(Duration::from_secs(30), inner_node_id.clone()) => {}
            _ = rx.recv() => {}
        }
    });

    let mut metrics_rx = tx.subscribe();
    tokio::spawn(async move {
        tokio::select! {
            _ = report_metrics(capacity.clone(), metrics.clone(), Duration::from_secs(30), node_id.clone()) => {}
            _ = metrics_rx.recv() => {}
        }
    });

    tokio::signal::ctrl_c().await?;
    tx.send(())?;

    Ok(())
}

