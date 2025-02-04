use std::{collections::BTreeSet, fmt::Debug, net::IpAddr};
use alloy::signers::k256::ecdsa::SigningKey;
use crdts::{bft_queue::Message, bft_topic_queue::TopicQueue, map::Op, merkle_reg::Sha3Hash, BFTQueue, CmRDT, VClock};
use form_state::datastore::{Response, Success};
use shared::Peer;
use serde::{Deserialize, Serialize};
use reqwest::Client;

pub const QUEUE_PORT: u16 = 53333;
pub type QueueOp<T> = Op<[u8; 32], BFTQueue<T>, String>; 

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum QueueRequest {
    Op(QueueOp<Vec<u8>>),
    Write(Message<Vec<u8>>),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum QueueResponse {
    OpSuccess,
    Op(QueueOp<Vec<u8>>),
    Some(Vec<u8>),
    List(Vec<Vec<u8>>)
}


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

    pub fn read(&self, topic: [u8; 32]) -> Option<Vec<Message<Vec<u8>>>> {
        if let Some(ref queue) = &self.queue.read_topic(&topic) {
            return Some(queue.read().iter().map(|m| m.to_owned().clone()).collect())
        }     

        None
    }

    pub fn read_n(&self, topic: [u8; 32], after: &VClock<String>, n: usize) -> Option<Vec<Message<Vec<u8>>>> {
        if let Some(ref list) = &self.read_after(topic, after) {
            if list.len() > n {
                return Some(list[..n].to_vec())
            } else {
                return Some(list.to_vec())
            }
        }
        None
    }

    pub fn read_after(&self, topic: [u8; 32], after: &VClock<String>)-> Option<Vec<Message<Vec<u8>>>> {
        if let Some(ref queue) = &self.queue.read_topic(&topic) {
            return Some(queue.read_after(after).iter().map(|m| m.to_owned().clone()).collect())
        }

        None
    }

    pub fn write_local(
        &mut self,
        topic: [u8; 32],
        content: Vec<u8>,
        deps: BTreeSet<[u8; 32]>
    ) -> Result<(), Box<dyn std::error::Error>> {
        let signing_key = SigningKey::from_slice(&hex::decode(self.pk.clone())?)?;
        let op = self.queue.enqueue(
            topic,
            content,
            deps,
            self.node_id.clone(),
            signing_key
        )?;

        self.apply(op.clone());
        Ok(())
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
                    return q.contains(op.hash)
                }
            }
            _ => {}
        }

        false
    }

    pub async fn broadcast_op(&self, op: QueueOp<Vec<u8>>) -> Result<(), Box<dyn std::error::Error>> {
        let peers = self.get_peers().await?;
        if self.op_success(op.clone()) {
            let request = QueueRequest::Op(op.clone()); 
            for peer in peers {
                match self.client.post(format!("http://{}:{}/queue/write_op", peer, QUEUE_PORT))
                    .json(&request)
                    .send()
                    .await?
                    .json::<QueueResponse>()
                    .await {
                        Ok(_resp) => {},
                        Err(_e) => {}
                }
            }
        }

        Ok(())
    }

    pub async fn get_peers(&self) -> Result<Vec<IpAddr>, Box<dyn std::error::Error>> {
        let uri = format!("http://{}/user/list_admin", self.state_uri);
        let resp = self.client.get(&uri).send().await?.json::<Response<Peer<String>>>().await?;
        match resp {
            Response::Success(Success::List(admins)) => return Ok(admins.iter().map(|peer| peer.ip.clone()).collect()), 
            Response::Success(Success::None) => return Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, "Returned Success::None, instead of Success::List"))),
            Response::Success(Success::Some(_)) => return Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, "Returned Success::Some(peer) instead of Success::List"))),
            Response::Success(Success::Relationships(_)) => return Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, "Returned Success::Relationship((cidr1, cidr2)) instead of Success::List"))),
            Response::Failure { reason } => return Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, format!("Returned Failure: {reason:?}"))))
        }
    }
}
