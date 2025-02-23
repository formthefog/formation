use std::{sync::Arc, time::Duration};

use axum::{extract::State, routing::get, Json, Router};
use form_vm_metrics::system::{collect_system_metrics, SystemMetrics};
use tokio::{net::TcpListener, sync::{broadcast::channel, Mutex}, time::interval};


#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {

    let default_metrics = Arc::new(Mutex::new(SystemMetrics::default()));
    let system_metrics = collect_system_metrics(default_metrics).await;

    let (tx, mut rx) = channel(2);


    let mut inner_receiver = tx.subscribe();
    let collector_metrics = system_metrics.clone();
    let metrics_collection_handle = tokio::spawn(async move {
        let mut interval = interval(Duration::from_secs(30));
        loop {
            interval.tick().await;
            tokio::select! {
                _ = inner_receiver.recv() => { break }
                _ = collect_system_metrics(collector_metrics.clone())  => {}
            }
        }
    });

    let metrics_reporting_handle = tokio::spawn(async move {
        tokio::select! {
            res = serve(system_metrics) => {
                if let Err(e) = res {
                    eprintln!("Error serving metrics endpoint: {e}");
                }
            }
            _ = rx.recv() => {} 
        }
    });

    tokio::signal::ctrl_c().await?;

    tx.send(())?;

    metrics_collection_handle.await?;
    metrics_reporting_handle.await?;

    Ok(())
}

pub async fn serve(metrics: Arc<Mutex<SystemMetrics>>) -> Result<(), Box<dyn std::error::Error>> {
    let routes = Router::new()
        .route("/get", get(get_metrics))
        .with_state(metrics);

    let listener = TcpListener::bind("0.0.0.0:63210").await?;

    axum::serve(listener, routes).await?;

    Ok(())
}

async fn get_metrics(
    State(state): State<Arc<Mutex<SystemMetrics>>>
) -> Json<SystemMetrics> {
    let metrics = state.lock().await.clone();
    Json(metrics)
}
