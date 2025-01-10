use form_types::{Event, QuorumTopic, QuorumEvent};
use form_traits::Node;
use crate::formation_rpc::{JoinRequest, JoinResponse, MessageHeader};
use tonic::{Response, Status};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use alloy::primitives::{Address, hex::FromHex};
use sha3::{Digest, Sha3_256};

pub async fn publish_join_request<N>(node: &N, request: &JoinRequest) -> Result<Response<JoinResponse>, Status> 
where
    N: Node + Send + Sync
{
    node.publish(
        Box::new(QuorumTopic),
        Box::new(Event::QuorumEvent(QuorumEvent::NewPeer {
            node_id: Address::from_hex(request.new_peer_id.clone()).map_err(|e| {
                Status::invalid_argument(
                    format!("new_peer_id is not a valid Ethereum compatible address: {e}")
                )
            })?,
            node_address: request.new_peer_address.parse().map_err(|e| {
                Status::invalid_argument(
                    format!("new_peer_address is not a valid SocketAddr: {e}")
                )
            })?,
            new_peer_signature: request.new_peer_signature.clone(),
            new_peer_recovery_id: request.new_peer_recovery_id.to_be_bytes()[3],
            sender_signature: request.sender_signature.clone(),
            sender_recovery_id: request.sender_recovery_id
        }))
    ).await.map_err(|e| {
        Status::failed_precondition(e.to_string())
    })?;

    let (original_message_id, request_sender_id, request_sender_address) = {
        let original_header = request.header.clone().ok_or(
            Status::failed_precondition(
                "JoinRequest missing MessageHeader"
            )
        )?;

        (original_header.message_id, original_header.peer_id, original_header.peer_address)
    };

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
            "request_sender_id": request_sender_id,
            "request_sender_address": request_sender_address,
            "request_sender_signature": request.sender_signature,
            "request_sender_recovery_id": request.sender_recovery_id,
            "new_peer_id": request.new_peer_id,
            "new_peer_address": request.new_peer_address,
            "new_peer_signature": request.new_peer_signature,
            "new_peer_recovery_id": request.new_peer_recovery_id
        }).to_string()
    ).as_bytes());

    let payload = hasher.finalize().to_vec();

    let (signature, recovery_id) = node.sign_join_response(payload).map_err(|e| {
        Status::invalid_argument(e.to_string())
    })?;

    let join_response = JoinResponse {
        header: Some(header),
        original_message_id,
        ack: true,
        sig: signature.to_string(),
        recovery_id: recovery_id.to_byte() as u32
    };

    Ok(Response::new(join_response))
}
