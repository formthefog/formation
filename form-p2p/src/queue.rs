use std::{collections::BTreeMap, fmt::Debug, net::IpAddr};
use alloy::signers::k256::ecdsa::SigningKey;
use crdts::{bft_queue::Message, bft_topic_queue::TopicQueue, map::Op, merkle_reg::Sha3Hash, BFTQueue, CmRDT, VClock};
use form_types::state::{Response, Success};
use shared::Peer;
use serde::{Deserialize, Serialize};
use reqwest::Client;

pub const QUEUE_PORT: u16 = 53333;
pub type QueueOp<T> = Op<String, BFTQueue<T>, String>; 

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum QueueRequest {
    Op(QueueOp<Vec<u8>>),
    Write {
        content: Vec<u8>,
        topic: String,
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum QueueResponse {
    OpSuccess,
    Op(QueueOp<Vec<u8>>),
    Some(Vec<u8>),
    List(Vec<Vec<u8>>),
    Failure { reason: Option<String> },
    Full(BTreeMap<String, Vec<Vec<u8>>>)
}

#[allow(unused)]
pub struct FormMQ<T: Sha3Hash + Default + Debug + Clone + Ord> {
    queue: TopicQueue<T>,
    node_id: String,
    pk: String,
    state_uri: String,
    client: Client
}

impl FormMQ<Vec<u8>> {
    pub fn new(node_id: String, pk: String, state_uri: String) -> Self {
        Self {
            queue: TopicQueue::new(),
            node_id,
            pk,
            state_uri,
            client: Client::new()
        }
    }

    pub fn queue(&self) -> &TopicQueue<Vec<u8>> {
        &self.queue
    }

    pub fn read(&self, topic: String) -> Option<Vec<Message<Vec<u8>>>> {
        if let Some(ref queue) = &self.queue.read_topic(&topic) {
            return Some(queue.read().iter().map(|m| m.to_owned().clone()).collect())
        }     

        None
    }

    pub fn read_n(&self, topic: String, after: &VClock<String>, n: usize) -> Option<Vec<Message<Vec<u8>>>> {
        if let Some(ref list) = &self.read_after(topic, after) {
            if list.len() > n {
                return Some(list[..n].to_vec())
            } else {
                return Some(list.to_vec())
            }
        }
        None
    }

    pub fn read_after(&self, topic: String, after: &VClock<String>)-> Option<Vec<Message<Vec<u8>>>> {
        if let Some(ref queue) = &self.queue.read_topic(&topic) {
            return Some(queue.read_after(after).iter().map(|m| m.to_owned().clone()).collect())
        }

        None
    }

    pub fn write_local(
        &mut self,
        topic: String,
        content: Vec<u8>,
    ) -> Result<QueueOp<Vec<u8>>, Box<dyn std::error::Error>> {
        log::info!("Received write_local request");
        let signing_key = SigningKey::from_slice(&hex::decode(self.pk.clone())?)?;
        let op = self.queue.enqueue(
            topic,
            content,
            self.node_id.clone(),
            signing_key
        )?;

        log::info!("Built enqueue Op, applying...");
        self.apply(op.clone());
        log::info!("Op applied...");
        Ok(op)
    }

    pub fn apply(
        &mut self,
        op: QueueOp<Vec<u8>> 
    ) {
        self.queue.apply(op);
    }

    pub fn op_success(&self, op: QueueOp<Vec<u8>>) -> bool {
        match op {
            Op::Up { dot: _, key, op } => {
                if let Some(ref q) = self.queue.read_topic(&key) {
                    log::info!("Update Op applied successfully...");
                    return q.contains(op.hash)
                }
            }
            _ => {}
        }

        false
    }

    pub async fn send_op(op: QueueOp<Vec<u8>>, addr: IpAddr, port: u16) -> Result<(), Box<dyn std::error::Error>> {
        log::info!("Attempting to send op to peers");
        let request = QueueRequest::Op(op.clone()); 
        match Client::new().post(format!("http://{}:{}/queue/write_op", addr, port))
            .json(&request)
            .send()
            .await?
            .json::<QueueResponse>()
            .await {
                Ok(_resp) => Ok(()), 
                Err(e) => Err(Box::new(e)) 
        }
    }

    pub async fn broadcast_op(op: QueueOp<Vec<u8>>) -> Result<(), Box<dyn std::error::Error>> {
        let peers = Self::get_peers().await?;
        for peer in peers {
            if let Err(e) = Self::send_op(op.clone(), peer, QUEUE_PORT).await {
                eprintln!("Error attempting to send op to {peer}: {e}");
            }
        }

        Ok(())
    }

    pub async fn get_peers() -> Result<Vec<IpAddr>, Box<dyn std::error::Error>> {
        let uri = "http://127.0.0.1:3004/user/list_admin";
        let resp = Client::new().get(uri).send().await?.json::<Response<Peer<String>>>().await?;
        match resp {
            Response::Success(Success::List(admins)) => return Ok(admins.iter().map(|peer| peer.ip.clone()).collect()), 
            Response::Success(Success::None) => return Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, "Returned Success::None, instead of Success::List"))),
            Response::Success(Success::Some(_)) => return Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, "Returned Success::Some(peer) instead of Success::List"))),
            Response::Success(Success::Relationships(_)) => return Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, "Returned Success::Relationship((cidr1, cidr2)) instead of Success::List"))),
            Response::Failure { reason } => return Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, format!("Returned Failure: {reason:?}"))))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy::{primitives::Address, signers::k256::ecdsa::SigningKey};
    use rand::thread_rng;
    use tiny_keccak::{Hasher, Sha3};

    // Test message type that implements necessary traits
    #[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
    struct TestMessage {
        id: String,
        data: Vec<u8>,
    }

    impl AsRef<[u8]> for TestMessage {
        fn as_ref(&self) -> &[u8] {
            &self.data
        }
    }

    impl Default for TestMessage {
        fn default() -> Self {
            Self {
                id: String::new(),
                data: Vec::new(),
            }
        }
    }

    // Helper function to create a test message
    fn create_test_message(id: &str, data: &[u8]) -> TestMessage {
        TestMessage {
            id: id.to_string(),
            data: data.to_vec(),
        }
    }

    // Helper function to create a topic hash
    fn create_topic_hash(name: &str) -> [u8; 32] {
        let mut hasher = Sha3::v256();
        hasher.update(name.as_bytes());
        let mut hash = [0u8; 32];
        hasher.finalize(&mut hash);
        hash
    }

    #[tokio::test]
    async fn test_topic_queue_serialization() -> Result<(), Box<dyn std::error::Error>> {
        let topics = ["something", "or", "another", "howso"];
        let test_messages = (0..100).into_iter().map(|n| {
            let mut msgs = vec![];
            for topic in &topics {
                let id = format!("test-{n}");
                let data = format!("This is test data {n}");
                let topic_hash = create_topic_hash(topic);
                let tm = create_test_message(&id, &data.as_bytes());
                msgs.push((topic_hash, tm))
            };
            msgs
        }).flatten().collect::<Vec<([u8; 32], TestMessage)>>();

        let mut queue = TopicQueue::new();
        let sk = SigningKey::random(&mut thread_rng());
        let actor = hex::encode(Address::from_private_key(&sk));

        for (topic, msg) in test_messages {
            let op = queue.enqueue(hex::encode(topic), msg, actor.clone(), sk.clone())?;
            queue.apply(op);
        }

        serde_json::to_vec(&queue).unwrap();

        Ok(())

    }
}
