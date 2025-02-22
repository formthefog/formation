use std::{sync::Arc, time::Duration};

use form_p2p::queue::{QueueRequest, QueueResponse, QUEUE_PORT};
use reqwest::Client;
use serde::Serialize;
use tiny_keccak::{Hasher, Sha3};
use tokio::{sync::Mutex, time::interval};

use crate::{capabilities::NodeCapabilities, capacity::NodeCapacity, metrics::NodeMetrics, NodeMetricsRequest};

pub async fn write_to_queue(
    message: impl Serialize + Clone,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut hasher = Sha3::v256();
    let mut topic_hash = [0u8; 32];
    hasher.update(b"state");
    hasher.finalize(&mut topic_hash);
    let mut message_code = vec![6];
    message_code.extend(serde_json::to_vec(&message)?);
    let request = QueueRequest::Write { 
        content: message_code, 
        topic: hex::encode(topic_hash) 
    };

    match Client::new()
        .post(format!("http://127.0.0.1:{}/queue/write_local", QUEUE_PORT))
        .json(&request)
        .send().await?
        .json::<QueueResponse>().await? {
            QueueResponse::OpSuccess => return Ok(()),
            QueueResponse::Failure { reason } => return Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, format!("{reason:?}")))),
            _ => return Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, "Invalid response variant for write_local endpoint")))
    }
}

pub async fn report_metrics(
    capacity: Arc<Mutex<NodeCapacity>>,
    metrics: Arc<Mutex<NodeMetrics>>,
    refresh: Duration,
    node_id: String,
) {
    let mut interval = interval(refresh);
    loop {
        interval.tick().await;
        let capacity_report = capacity.lock().await.clone();
        let metrics_report = metrics.lock().await.clone();
        let request = NodeMetricsRequest::UpdateMetrics { 
            node_id: node_id.clone(), 
            node_capacity: capacity_report, 
            node_metrics: metrics_report 
        };

        if let Err(e) = write_to_queue(request).await {
            log::error!("Error writing to queue: {e}");
        }
    }
}

pub async fn report_initial_metrics(
    capabilities: NodeCapabilities,
    capacity: Arc<Mutex<NodeCapacity>>,
    node_id: String,
) {
    let report_capacity = capacity.lock().await.clone();
    let request = NodeMetricsRequest::SetInitialMetrics { 
        node_id,
        node_capabilities: capabilities, 
        node_capacity: report_capacity, 
    };

    if let Err(e) = write_to_queue(request).await {
        log::error!("Error writing to queue: {e}");
    }
}
