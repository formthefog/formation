use std::collections::HashMap;
use std::sync::Arc;

use crate::formation_rpc::{
    formation_rpc_server::FormationRpc, 
    HeartbeatRequest, 
    HeartbeatResponse, 
    JoinRequest, 
    JoinResponse, 
    QuorumGossipRequest, 
    QuorumGossipResponse, 
    NetworkGossipRequest, 
    NetworkGossipResponse, 
    DirectedMessageRequest, 
    DirectedMessageResponse, 
    UserRequestMessage, 
    UserResponse,
};
use form_traits::Node;
use getset::Getters;
use tonic::{Request, Response, Status};
use tokio::{sync::Mutex, io::AsyncWriteExt};
use serde::{Serialize, Deserialize};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _ };
use crate::{
    publish_join_request,
    publish_heartbeat_request,
    publish_quorum_gossip,
    publish_network_gossip, 
    publish_direct_message, 
    publish_user_request
};

pub const DUMP_LOG_LENGTH: usize = 5000;

/// An enum representing the different Protobuf defined message types
/// such that the messages can be stored in an ergonomic enum as a string
/// which can later be deserialized if needed.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Message {
    HeartbeatRequest(String),
    JoinRequest(String),
    QuorumGossipRequest(String),
    NetworkGossipRequest(String),
    DirectedMessageRequest(String),
    UserRequest(String),
}

/// A GRPC server that implements the service created by the protobuf.
/// contains the node's ID, address and a publisher as well as a signer.
/// This struct can be converted into a simple wrapper around a `Node` struct.
#[derive(Clone, Getters)]
#[getset(get="pub")]
pub struct NodeServer<N> 
where
    N: Node + Send + Sync
{
    // Takes a node, which has node id (an Ethereum compatible address), ip address
    // (a SocketAddr), and a message log to track messages received.
    // Message log can be treated as an in-memory cache, with a dump to a file
    // the file can be compacted and cleaned as to not create bloat.
    node: N,
    log: Arc<Mutex<HashMap<String, Message>>>,
    log_file: String,
}

impl<N> NodeServer<N> 
where
    N: Node + Send + Sync
{

    /// Creates and returns (if no error) a new `NodeServer` with the 
    /// arguments provided.
    pub async fn new(
        node: N,
        log_file: Option<String>,
    ) -> std::io::Result<Self> {
        Ok(Self {
            node,
            log: Arc::new(Mutex::new(HashMap::new())),
            log_file: log_file.unwrap_or_else(|| "/var/log/form-server.log".to_string()) 
        })
    }

    async fn is_new_message(&self, message_id: &String) -> std::io::Result<()> {
        let guard = self.log.lock().await;

        if guard.contains_key(message_id) {
            return Err(
                std::io::Error::new(
                    std::io::ErrorKind::AlreadyExists,
                    format!("Message {message_id} already exists in log")
                )
            )
        } 

        drop(guard);

        Ok(())
    }

    /// Takes a `JoinRequest` and writes it to the in-memory log
    async fn join_request_to_log(&self, req: &JoinRequest) -> std::io::Result<()> {
        let header = req.header.clone().ok_or(
            std::io::Error::new(
                std::io::ErrorKind::Other,
                "JoinRequest missing MessageHeader"
            )
        )?;

        self.is_new_message(&header.message_id).await?;

        let sender_id = if req.forwarded { 
            header.peer_id 
        } else { 
            req.new_peer_id.clone() 
        };

        let sender_address = if req.forwarded { 
            header.peer_address
        } else { 
            req.new_peer_address.clone() 
        };

        let sender_signature = if req.forwarded {
            Some(req.sender_signature.clone().ok_or(
                    std::io::Error::new(
                        std::io::ErrorKind::Other,
                        "JoinRequest was forwarded but did not contain a sender signature"
                    )
                )?
            )
        } else {
            None
        };

        let sender_recovery_id = if req.forwarded {
            Some(req.sender_recovery_id.ok_or(
                std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "JoinRequest was forwarded but did not contain a sender recovery id"
                )
            )?.to_be_bytes()[3])
        } else {
            None
        };

        let message_string = URL_SAFE_NO_PAD.encode(serde_json::json!({
            "sender_id": sender_id,
            "sender_address": sender_address,
            "message_id": header.message_id.clone(),
            "new_peer_id": req.new_peer_id,
            "new_peer_address": req.new_peer_address,
            "new_peer_signature": req.new_peer_signature,
            "new_peer_recovery_id": req.new_peer_recovery_id.to_be_bytes()[3],
            "sender_signature": sender_signature,
            "sender_recovery_id": sender_recovery_id,
            "forwarded": req.forwarded
        }).to_string());


        self.log_message(
            header.message_id.clone(), 
            Message::JoinRequest(message_string)
        ).await?;

        Ok(())
    }

    /// Takes a `HeartbeatRequest` and writes it to the in-memory log
    async fn heartbeat_request_to_log(&self, req: &HeartbeatRequest) -> std::io::Result<()> {
        let header = req.header.clone().ok_or(
            std::io::Error::new(
                std::io::ErrorKind::Other,
                "HeartbeatRequest missing MessageHeader"
            )
        )?;

        self.is_new_message(&header.message_id).await?;

        let message_string = URL_SAFE_NO_PAD.encode(serde_json::json!({
            "sender_id": header.peer_id,
            "sender_address": header.peer_address,
            "timestamp": req.timestamp,
            "signature": req.sig,
            "recovery_id": req.recovery_id.to_be_bytes()[3]
        }).to_string());

        self.log_message(
            header.message_id.clone(),
            Message::HeartbeatRequest(message_string)
        ).await?;

        Ok(())
    }

    /// Takes a `QuorumGossipRequest` and writes it to the in-memory log
    async fn quorum_gossip_to_log(&self, req: &QuorumGossipRequest) -> std::io::Result<()> {
        let header = req.header.clone().ok_or(
            std::io::Error::new(
                std::io::ErrorKind::Other,
                "QuorumGossipRequest missing MessageHeader"
            )
        )?;

        self.is_new_message(&header.message_id).await?;

        let message_string = URL_SAFE_NO_PAD.encode(serde_json::json!({
            "sender_id": header.peer_id,
            "sender_address": header.peer_address,
            "timestamp": req.timestamp,
            "recovery_id": req.recovery_id.to_be_bytes()[3],
            "request_type": format!("{:?}", req.request_type()),
            "payload": req.payload
        }).to_string());

        self.log_message(
            header.message_id,
            Message::QuorumGossipRequest(message_string)
        ).await?;

        Ok(())
    }

    /// Takes a `NetworkGossipRequest` and writes it to the in-memory log
    async fn network_gossip_to_log(&self, req: &NetworkGossipRequest) -> std::io::Result<()> {
        let header = req.header.clone().ok_or(
            std::io::Error::new(
                std::io::ErrorKind::Other,
                "NetworkGossipRequest missing MessageHeader"
            )
        )?;

        self.is_new_message(&header.message_id).await?;

        let message_string = URL_SAFE_NO_PAD.encode(serde_json::json!({
            "sender_id": header.peer_id,
            "sender_address": header.peer_address,
            "timestamp": req.timestamp,
            "recovery_id": req.recovery_id.to_be_bytes()[3],
            "request_type": format!("{:?}", req.request_type()),
            "payload": req.payload
        }).to_string());

        self.log_message(
            header.message_id,
            Message::NetworkGossipRequest(message_string)
        ).await?;

        Ok(())
    }

    /// Takes a `DirectedMessageRequest` and writes it to the in-memory log
    async fn direct_message_to_log(&self, req: &DirectedMessageRequest) -> std::io::Result<()> {
        let header = req.header.clone().ok_or(
            std::io::Error::new(
                std::io::ErrorKind::Other,
                "DirectedMessageRequest missing MessageHeader"
            )
        )?;

        self.is_new_message(&header.message_id).await?;

        let message_string = URL_SAFE_NO_PAD.encode(serde_json::json!({
            "sender_id": header.peer_id,
            "sender_address": header.peer_address,
            "timestamp": req.timestamp,
            "recovery_id": req.recovery_id.to_be_bytes()[3],
            "message_type": format!("{:?}", req.message_type()),
            "payload": req.payload
        }).to_string());

        self.log_message(
            header.message_id,
            Message::NetworkGossipRequest(message_string)
        ).await?;

        Ok(())
    }

    /// Takes a `UserRequestMessage` and writes it to the in-memory log
    async fn user_request_to_log(&self, req: &UserRequestMessage) -> std::io::Result<()> {
        self.is_new_message(&req.message_id).await?;

        let message_string = URL_SAFE_NO_PAD.encode(serde_json::json!({
            "signature": req.sig,
            "recovery_id": req.recovery_id.to_be_bytes()[3],
            "timestamp": req.timestamp,
            "request_type": format!("{:?}", req.request_type()),
            "payload": req.payload
        }).to_string());

        self.log_message(
            req.message_id.clone(),
            Message::UserRequest(message_string)
        ).await?;

        Ok(())
    }

    /// Writes the `Message` to the in-memory log with the message_id `String`
    /// as the key and the `Message` itself as the value. Checks whether the
    /// in memory length is equal to or greater than the `DUMP_LOG_LENGTH` and
    /// if so, dumps the log to the log file and clears the in-memory log.
    async fn log_message(&self, message_id: String, message: Message) -> std::io::Result<()> {
        let mut guard = self.log.lock().await;
        guard.insert(message_id, message);

        let mut dump_log = false;

        if guard.len() >= DUMP_LOG_LENGTH { 
            dump_log = true;
        }

        drop(guard);

        if dump_log {
            self.dump_log().await?;
        }

        Ok(())
    }

    /// Appends the in memory log to the log file and clears the in-memory
    /// log cache.
    async fn dump_log(&self) -> std::io::Result<()> {
        let mut file = tokio::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .write(true)
            .open(&self.log_file).await?;

        let mut guard = self.log.lock().await;

        let contents = serde_json::to_string(&guard.clone())?;

        file.write_all(&contents.as_bytes()).await?;

        guard.clear();

        drop(guard);

        Ok(())
    }

    #[allow(dead_code)]
    async fn compact_log(&self) -> std::io::Result<()> {
        todo!()
    }


}

#[tonic::async_trait]
impl<N> FormationRpc for NodeServer<N> 
where 
    N: Node + Send + Sync + 'static
{
    /// A protobuf generated gRPC method that peers call to join the network.
    async fn join(&self, request: Request<JoinRequest>) -> Result<Response<JoinResponse>, Status> {
        let req = request.into_inner();
        // Forward message as QuorumEvent::Join to QuorumTopic
        self.join_request_to_log(&req).await?;
        publish_join_request(self.node(), &req).await
    }

    /// A protobuf generated gRPC method that peers call to inform the network
    /// they are still up and alive.
    async fn heartbeat(&self, request: Request<HeartbeatRequest>) -> Result<Response<HeartbeatResponse>, Status> {
        let req = request.into_inner();

        self.heartbeat_request_to_log(&req).await?;
        publish_heartbeat_request(self.node(), &req).await
    }

    /// A protobuf generated gRPC method that peers call to spread a gossip
    /// message within their quorum.
    async fn quorum_gossip(&self, request: Request<QuorumGossipRequest>) -> Result<Response<QuorumGossipResponse>, Status> {
        let req = request.into_inner();

        self.quorum_gossip_to_log(&req).await?;
        publish_quorum_gossip(self.node(), &req).await
    }

    /// A protobuf generated gRPC method that peers call to spread a gossip
    /// message across the entire network.
    async fn network_gossip(&self, request: Request<NetworkGossipRequest>) -> Result<Response<NetworkGossipResponse>, Status> {
        let req = request.into_inner();

        self.network_gossip_to_log(&req).await?;
        publish_network_gossip(self.node(), &req).await
    }

    /// A protobuf generated gRPC method that peers call to send a direct message
    /// to a specific node
    async fn direct_message(&self, request: Request<DirectedMessageRequest>) -> Result<Response<DirectedMessageResponse>, Status> {
        let req = request.into_inner();

        self.direct_message_to_log(&req).await?;
        publish_direct_message(self.node(), &req).await
    }

    /// A protobuf generated gRPC method that users call (via CLI or UI) to
    /// make a request to the network
    async fn user_request(&self, request: Request<UserRequestMessage>) -> Result<Response<UserResponse>, Status> {
        let req = request.into_inner();

        self.user_request_to_log(&req).await?;
        publish_user_request(self.node(), &req).await
    }
}
