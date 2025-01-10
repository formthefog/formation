use form_traits::Node;
use form_types::{Event, QuorumEvent, QuorumTopic};
use crate::formation_rpc::{MessageHeader, QuorumGossipRequest, QuorumGossipResponse};
use tonic::{Response, Status};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use alloy::primitives::{Address, hex::FromHex};
use sha3::{Digest, Sha3_256};

pub async fn publish_quorum_gossip<N>(
    node: &N,
    request: &QuorumGossipRequest
) -> Result<Response<QuorumGossipResponse>, Status> 
where
    N: Node + Send + Sync
{
    let (original_message_id, request_sender_id, request_sender_address) = {
        let original_header = request.header.clone().ok_or(
            Status::failed_precondition(
                "QuorumGossipRequest missing MessageHeader"
            )
        )?;

        (original_header.message_id, original_header.peer_id, original_header.peer_address)
    };

    node.publish(
        Box::new(QuorumTopic),
        Box::new(Event::QuorumEvent(QuorumEvent::QuorumGossip {
            node_id: Address::from_hex(&request_sender_id).map_err(|e| {
                Status::invalid_argument(
                    format!("Node address is not a valid Ethereum Compatible Address: {e}")
                )
            })?,
            node_address: request_sender_address.parse().map_err(|e| {
                Status::invalid_argument(
                    format!("Request sender address is not a valid SocketAddr: {e}")
                )
            })?,
            timestamp: request.timestamp.parse().map_err(|e| {
                Status::invalid_argument(
                    format!("Timestamp is not a valid i64: {e}")
                )
            })?,
            node_recovery_id: request.recovery_id.to_be_bytes()[3],
            node_signature: request.sig.clone(),
            request_type: request.request_type,
            payload: request.payload.clone(),

        }))
    ).await.map_err(|e| {
        Status::failed_precondition(e.to_string())
    })?;


    let header = MessageHeader {
        message_id: uuid::Uuid::new_v4().to_string(),
        peer_id: node.id().to_string(),
        peer_address: node.ip_address().to_string()
    };

    let mut hasher = Sha3_256::new(); 

    hasher.update(URL_SAFE_NO_PAD.encode(
        serde_json::json!({
            "response_id": header.message_id,
            "responder_id": header.peer_id,
            "responder_address": header.peer_address,
            "request_id": original_message_id,
            "request_type": request.request_type,
            "payload": request.payload,
            "node_id": request_sender_id,
            "node_address": request_sender_address,
            "node_signature": request.sig,
            "node_recovery_id": request.recovery_id,
        }).to_string()
    ).as_bytes());

    let payload = hasher.finalize().to_vec();

    let (signature, recovery_id) = node.sign_quorum_gossip_response(payload).map_err(|e| {
        Status::invalid_argument(e.to_string())
    })?;

    let quorum_gossip_response = QuorumGossipResponse {
        header: Some(header),
        original_message_id,
        ack: true,
        sig: signature.to_string(),
        recovery_id: recovery_id.to_byte() as u32
    };

    Ok(Response::new(quorum_gossip_response))
}
