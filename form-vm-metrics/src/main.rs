use std::time::Duration;

use form_vm_metrics::system::collect_system_metrics;
use tokio::{sync::broadcast::channel, time::interval};


#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {

    let mut system_metrics = collect_system_metrics(None, 30).await;
    let (tx, mut rx) = channel(1);
    let mut interval = interval(Duration::from_secs(30));
    let metrics_collection_handle = tokio::spawn(async move {
        loop {
            interval.tick().await;

            let disk_stats = system_metrics.disk_stats.clone();
            tokio::select! {
                _ = rx.recv() => { break }
                metrics = collect_system_metrics(Some(disk_stats), 30)  => {
                    system_metrics = metrics;
                }
            }
        }
    });

    tokio::signal::ctrl_c().await?;
    tx.send(())?;
    metrics_collection_handle.await?;

    Ok(())
}
