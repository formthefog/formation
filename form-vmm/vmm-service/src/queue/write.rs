use sha3::{Sha3, Sha3v256};
use serde::Serialize;
use reqwest::Client;
use crate::queue::QueueRequest;
use crate::queue::QueueResponse;

pub async fn write_to_queue(
    message: impl Serialize + Clone,
    sub_topic: u8,
    topic: &str
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut hasher = Sha3::v256();
    let mut topic_hash = [0u8; 32];
    hasher.update(topic.as_bytes());
    hasher.finalize(&mut topic_hash);
    let mut message_code = vec![sub_topic];
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