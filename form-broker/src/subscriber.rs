use tokio::net::TcpStream;
use tokio::io::{AsyncWriteExt, AsyncReadExt};
use crate::{util::{parse_next_message, try_get_message_len, try_get_topic_len}, HEADER_SIZE, TOPIC_SIZE_OFFSET};

#[async_trait::async_trait]
pub trait SubStream {
    type Message;
    async fn receive(&mut self) -> std::io::Result<Self::Message>;
    async fn parse_messages(msg: &mut Vec<u8>) -> std::io::Result<Self::Message>; 
}

pub struct Subscriber {
    stream: TcpStream
}

impl Subscriber {
    pub async fn new(uri: &str, topics: Vec<String>) -> std::io::Result<Self> {
        let mut stream = TcpStream::connect(uri).await?;
        let topic_str = topics.join(",");
        stream.write_all(topic_str.as_bytes()).await?;
        Ok(Self { stream })
    }

}

#[async_trait::async_trait]
impl SubStream for Subscriber {
    type Message = Vec<Vec<u8>>;
    async fn receive(&mut self) -> std::io::Result<Vec<Vec<u8>>> {
        let mut buffer = Vec::new();
        loop {
            let mut read_buffer = [0; 1024]; 
            let n = self.stream.read(&mut read_buffer).await.expect("unable to read stream to buffer");
            if n == 0 {
                break;
            }

            buffer.extend_from_slice(&read_buffer[..n]);
            let results = Self::parse_messages(&mut buffer).await?;
            if !results.is_empty() {
                return Ok(results)
            }
        }
        Err(
            std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "No complete messages received"
            )
        )
    }

    async fn parse_messages(msg: &mut Vec<u8>) -> std::io::Result<Vec<Vec<u8>>> {
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

        Ok(results)
    }
}
