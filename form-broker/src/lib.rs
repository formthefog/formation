pub const HEADER_SIZE: usize = 8;
pub const TOPIC_SIZE_OFFSET: usize = HEADER_SIZE + 8;

pub mod broker;
pub mod publisher;
pub mod subscriber;

pub mod util {
    use crate::{HEADER_SIZE, TOPIC_SIZE_OFFSET};
    use std::{collections::HashSet, sync::Arc};
    use tokio::sync::Mutex;
    use tokio::net::TcpStream;
    use std::collections::HashMap;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    pub fn try_get_message_len(msg: &mut Vec<u8>) -> std::io::Result<usize> {
        Ok(usize::from_be_bytes(msg[..HEADER_SIZE].try_into().map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("unable to convert first 8 bytes to usize: {e}")
            )
        })?))
    }

    pub fn try_get_topic_len(msg: &mut Vec<u8>) -> std::io::Result<usize> {
        Ok(usize::from_be_bytes(msg[HEADER_SIZE..TOPIC_SIZE_OFFSET].try_into().map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("unable to convert second 8 bytes into u64: {e}")
            )
        })?))
    }

    pub async fn parse_next_message(msg_len: usize, topic_len: usize, msg: &mut Vec<u8>) -> (String, Vec<u8>) {
        let topic = String::from_utf8_lossy(&msg[TOPIC_SIZE_OFFSET..TOPIC_SIZE_OFFSET + topic_len]).to_string();
        let message_offset = TOPIC_SIZE_OFFSET + topic_len;
        let message = msg.drain(..message_offset + msg_len).collect::<Vec<_>>();

        (topic, message)
    }

    pub async fn handle_message_parsing(
        msg: &mut Vec<u8>,
        subscriptions: Arc<Mutex<HashMap<String, Vec<Arc<Mutex<TcpStream>>>>>>
    ) -> std::io::Result<()> {
        while msg.len() >= HEADER_SIZE {
            let total_len = try_get_message_len(msg)?;
            if msg.len() >= total_len {
                log::info!("Parsing message of length {}", msg.len());
                let topic_len = try_get_topic_len(msg)?;
                let (topic, message) = parse_next_message(total_len, topic_len, msg).await;
                log::info!("Message Topic: {topic}");
                log::info!("Message {message:?}");
                let subs = subscriptions.clone();
                let mut guard = subs.lock().await;
                if let Some(subscribers) = guard.get_mut(&topic) {
                    log::info!("Found {} subscribers to topic {topic}", subscribers.len());
                    let mut dead_streams = HashSet::new();
                    for (i, subscriber) in subscribers.iter_mut().enumerate() {
                        let mut stream = subscriber.lock().await;
                        log::info!("attemping to write to topic {} for subscribers {:?} stream...", topic, stream.peer_addr());
                        if let Err(e) = stream.write_all(&message).await {
                            if let Ok(addr) = stream.peer_addr() {
                                log::info!("subscriber {} is dead, adding to dead_streams", addr.to_string());
                            }
                            dead_streams.insert(i);
                            log::error!("Failed to write to socket: {e}");
                        }

                        log::info!("Dropping lock on stream...");
                        drop(stream);
                    }
                    log::info!("Cleaning dead streams...");
                    dead_streams.iter().for_each(|i| {
                        subscribers.remove(*i);
                    });
                }

                log::info!("Dropping lock on subs...");
                drop(guard);
            }
        }
        Ok(())
    }

    pub async fn handle_new_subscription(
        subscriptions: Arc<Mutex<HashMap<String, Vec<Arc<Mutex<TcpStream>>>>>>,
        stream: TcpStream,
    ) -> std::io::Result<()> {
        log::info!("New subscription received");
        let subs = subscriptions.clone();
        tokio::spawn(async move {
            let stream = Arc::new(Mutex::new(stream));
            let mut buffer = [0; 1024];
            let n = stream.lock().await.read(&mut buffer).await.map_err(|e| {
                std::io::Error::new(
                    std::io::ErrorKind::Other,
                    e
                )
            })?;
            let topics = String::from_utf8_lossy(&buffer[..n]).to_string();
            let topic_list: Vec<String> = topics.split(',').map(|s| s.trim().to_string()).collect();
            log::info!("Topics subscribed to: {:?}", topic_list);
            let mut guard = subs.lock().await;
            for topic in topic_list {
                guard.entry(topic).or_insert_with(Vec::new).push(stream.clone());
            }

            Ok::<(), std::io::Error>(())
        });

        Ok(())
    }
}
