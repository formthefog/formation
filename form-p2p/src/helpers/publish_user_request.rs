use form_traits::Node;
use form_types::{Event, QuorumEvent, QuorumTopic};
use crate::formation_rpc::{MessageHeader, UserRequestMessage, UserResponse};
use tonic::{Response, Status};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use sha3::{Digest, Sha3_256};

pub async fn publish_user_request<N>(
    node: &N,
    request: &UserRequestMessage
) -> Result<Response<UserResponse>, Status> 
where
    N: Node + Send + Sync
{
    node.publish(
        Box::new(QuorumTopic),
        Box::new(Event::QuorumEvent(QuorumEvent::UserRequest {
            message_id: request.message_id.clone(),
            timestamp: request.timestamp.parse().map_err(|e| {
                Status::invalid_argument(
                    format!("Timestamp is not a valid i64: {e}")
                )
            })?,
            user_recovery_id: request.recovery_id.to_be_bytes()[3],
            user_signature: request.sig.clone(),
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
            "request_id": request.message_id.clone(),
            "request_type": request.request_type,
            "payload": request.payload,
            "user_signature": request.sig,
            "user_recovery_id": request.recovery_id,
        }).to_string()
    ).as_bytes());

    let payload = hasher.finalize().to_vec();

    let (signature, recovery_id) = node.sign_network_gossip_response(payload).map_err(|e| {
        Status::invalid_argument(e.to_string())
    })?;

    let network_gossip_response = UserResponse {
        header: Some(header),
        original_message_id: request.message_id.clone(),
        ack: true,
        sig: signature.to_string(),
        recovery_id: recovery_id.to_byte() as u32,
        status: 0,
        failure_reason: None,
    };

    Ok(Response::new(network_gossip_response))
}
