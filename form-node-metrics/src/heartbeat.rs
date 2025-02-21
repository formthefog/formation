use std::time::Duration;
use tokio::time::interval;

pub async fn heartbeat(refresh: Duration) {
    let mut interval = interval(refresh);
    loop {
        interval.tick().await;
        //TODO: Send heartbeat to queue
    }
}
