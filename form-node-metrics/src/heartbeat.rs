use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::time::interval;
use crate::{util::write_to_queue, NodeMetricsRequest};

pub async fn heartbeat(refresh: Duration, node_id: String) {
    let mut interval = interval(refresh);
    loop {
        interval.tick().await;
        if let Ok(timestamp) = SystemTime::now().duration_since(UNIX_EPOCH) {
            let heartbeat_request = NodeMetricsRequest::Heartbeat { node_id: node_id.clone(), timestamp: timestamp.as_secs() as i64 };
            if let Err(e) = write_to_queue(heartbeat_request).await {
                log::error!("Error writing to queue: {e}");
            }
        }
    }
}
