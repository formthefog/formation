use crate::event::Event;
use crate::topic::{NetworkTopic, VmmTopic};
use form_traits::topic::Topic;
use form_traits::IntoEvent;
use conductor::publisher::PubStream;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;
use crate::event::{NetworkEvent, VmmEvent};
use conductor::subscriber::SubStream;
use conductor::util::{parse_next_message, try_get_message_len, try_get_topic_len};
use conductor::{HEADER_SIZE, TOPIC_SIZE_OFFSET};
use tokio::io::AsyncReadExt;

pub struct NetworkSubscriber {
    stream: TcpStream,
}

impl NetworkSubscriber {
    pub async fn new(uri: &str) -> std::io::Result<Self> {
        let mut stream = TcpStream::connect(uri).await?;
        let topics_str = NetworkTopic.to_string();
        stream.write_all(topics_str.as_bytes()).await?;
        Ok(Self { stream })
    }
}

pub struct VmmSubscriber {
    stream: TcpStream,
}

impl VmmSubscriber {
    pub async fn new(uri: &str) -> std::io::Result<Self> {
        let mut stream = TcpStream::connect(uri).await?;
        let topics_str = VmmTopic.to_string();
        stream.write_all(topics_str.as_bytes()).await?;
        Ok(Self { stream })
    }
}

#[async_trait::async_trait]
impl SubStream for NetworkSubscriber {
    type Message = Vec<NetworkEvent>;

    async fn receive(&mut self) -> std::io::Result<Self::Message> {
        let mut buffer = Vec::new();
        loop {
            let mut read_buffer = [0; 4096];
            match self.stream.read(&mut read_buffer).await {
                Err(e) => log::error!("Error reading stream to buffer: {e}..."),
                Ok(n) => {
                    if n == 0 {
                        break;
                    }

                    buffer.extend_from_slice(&read_buffer[..n]);
                    let results = Self::parse_messages(&mut buffer).await?;
                    if !results.is_empty() {
                        return Ok(results);
                    }
                }
            }
        }
        Err(std::io::Error::new(
            std::io::ErrorKind::UnexpectedEof,
            "No complete messages received",
        ))
    }

    async fn parse_messages(msg: &mut Vec<u8>) -> std::io::Result<Self::Message> {
        let mut results = Vec::new();
        while msg.len() >= HEADER_SIZE {
            let total_len = try_get_message_len(msg)?;
            if msg.len() >= total_len {
                let topic_len = try_get_topic_len(msg)?;
                let (_, message) = parse_next_message(total_len, topic_len, msg).await;
                let message_offset = TOPIC_SIZE_OFFSET + topic_len;
                let msg = &message[message_offset..message_offset + total_len];
                results.push(msg.to_vec());
            }
        }

        let msg_results = results
            .iter()
            .filter_map(|m| serde_json::from_slice(&m).ok())
            .collect();

        Ok(msg_results)
    }
}

#[async_trait::async_trait]
impl SubStream for VmmSubscriber {
    type Message = Vec<VmmEvent>;

    async fn receive(&mut self) -> std::io::Result<Self::Message> {
        let mut buffer = Vec::new();
        loop {
            let mut read_buffer = [0; 4096];
            match self.stream.read(&mut read_buffer).await {
                Err(e) => log::error!("Error reading stream to buffer: {e}..."),
                Ok(n) => {
                    if n == 0 {
                        break;
                    }

                    buffer.extend_from_slice(&read_buffer[..n]);
                    let results = Self::parse_messages(&mut buffer).await?;
                    if !results.is_empty() {
                        return Ok(results);
                    }
                }
            }
        }
        Err(std::io::Error::new(
            std::io::ErrorKind::UnexpectedEof,
            "No complete messages received",
        ))
    }

    async fn parse_messages(msg: &mut Vec<u8>) -> std::io::Result<Self::Message> {
        let mut results = Vec::new();
        while msg.len() >= HEADER_SIZE {
            let total_len = try_get_message_len(msg)?;
            if msg.len() >= total_len {
                let topic_len = try_get_topic_len(msg)?;
                let (_, message) = parse_next_message(total_len, topic_len, msg).await;
                let message_offset = TOPIC_SIZE_OFFSET + topic_len;
                let msg = &message[message_offset..message_offset + total_len];
                results.push(msg.to_vec());
            }
        }

        let msg_results = results
            .iter()
            .filter_map(|m| serde_json::from_slice(&m).ok())
            .collect();

        Ok(msg_results)
    }
}

pub struct GenericPublisher {
    stream: TcpStream,
}

impl GenericPublisher {
    pub async fn new(uri: &str) -> std::io::Result<Self> {
        Ok(Self {
            stream: TcpStream::connect(uri).await?,
        })
    }

    pub fn peer_addr(&self) -> std::io::Result<String> {
        let socket_addr = self.stream.peer_addr()?;
        Ok(socket_addr.to_string())
    }
}

#[async_trait::async_trait]
impl PubStream for GenericPublisher {
    type Topic = Box<dyn Topic + Send>;
    type Message<'async_trait> = Box<dyn IntoEvent<Event = Event> + Send>;

    async fn publish(
        &mut self,
        topic: Self::Topic,
        msg: Self::Message<'async_trait>,
    ) -> std::io::Result<()> {
        let topic_len = topic.to_string().len();
        let topic_len_bytes = topic_len.to_be_bytes();
        let event: Event = msg.to_inner().into();
        let message_str = event.inner_to_string()?;
        let message_len = message_str.len();
        let message_len_bytes = message_len.to_be_bytes();
        let total_len =
            conductor::HEADER_SIZE + conductor::TOPIC_SIZE_OFFSET + topic_len + message_len;
        let mut full_message = Vec::with_capacity(total_len);
        full_message.extend_from_slice(&message_len_bytes);
        full_message.extend_from_slice(&topic_len_bytes);
        full_message.extend_from_slice(&topic.to_string().as_bytes());
        full_message.extend_from_slice(message_str.as_bytes());
        self.stream.write_all(&full_message).await?;

        Ok(())
    }
}
