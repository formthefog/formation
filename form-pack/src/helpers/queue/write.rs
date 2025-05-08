
use k256::ecdsa::{RecoveryId, Signature, VerifyingKey};
use alloy_primitives::Address;
use std::collections::BTreeMap;
use std::time::{UNIX_EPOCH, SystemTime};
use uuid::Uuid;
use reqwest::Client;
use tiny_keccak::{Sha3, Hasher};
use serde::Serialize;
use form_state::datastore::{AgentRequest, InstanceRequest, AccountRequest};
use form_state::agent::AIAgent;
use form_state::instances::{InstanceResources, InstanceStatus};
use form_state::instances::Instance;
use form_types::state::{Success, Response as StateResponse};
use form_p2p::queue::{QueueResponse, QueueRequest};
use form_p2p::queue::QUEUE_PORT;
use crate::formfile::Formfile;
use crate::types::status::PackBuildStatus;
use crate::types::response::PackBuildResponse;
use crate::types::request::PackBuildRequest;
use crate::helpers::utils::{
    build_instance_id,
    create_new_agent_entry,
    create_new_instance_entry
};

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
            QueueResponse::Failure { reason } => return Err(
                Box::new(
                    std::io::Error::new(
                        std::io::ErrorKind::Other,
                        format!("{reason:?}")
                    )
                )
            ),
            _ => return Err(
                Box::new(
                    std::io::Error::new(
                        std::io::ErrorKind::Other,
                        "Invalid response variant for write_local endpoint"
                    )
                )
            )
    }
}

pub async fn write_pack_status_started(
    message: &PackBuildRequest,
    node_id: String
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {

    let signer_address = {
        let pk = VerifyingKey::recover_from_msg(
            &message.hash,
            &Signature::from_slice(&hex::decode(message.sig.sig.clone())?)?,
            RecoveryId::from_byte(message.sig.rec).ok_or(
                Box::new(
                    std::io::Error::new(
                        std::io::ErrorKind::Other, 
                        "invalid recovery id"
                    )
                )
            )?
        )?;
        Address::from_public_key(&pk)
    };

    let mut hasher = Sha3::v256();
    let mut hash = [0u8; 32];
    hasher.update(signer_address.as_ref());
    hasher.update(message.request.formfile.name.as_bytes());
    hasher.finalize(&mut hash);

    let build_id = hex::encode(hash);

    let instance_id = build_instance_id(node_id.clone(), build_id.clone())?;
    let signer_address_hex = hex::encode(signer_address);

    let instance = create_new_instance_entry(
        instance_id.clone(), 
        node_id.clone(), 
        build_id.clone(), 
        signer_address_hex.clone(), 
        message.request.formfile.clone()
    )?; 

    // Create and register the AIAgent
    let mut agent = create_new_agent_entry(
        message.request.formfile.clone(),
        build_id.clone(),
        signer_address_hex.clone()
    )?;    // Create account update to link instance to owner
           //
    let account_request = AccountRequest::AddOwnedInstance {
        address: signer_address_hex.clone(),
        instance_id: instance_id.clone(),
    };
    
    let status_message = PackBuildResponse {
        status: PackBuildStatus::Started(hex::encode(hash)),
        request: message.clone()
    };

    #[cfg(not(feature = "devnet"))]
    write_to_queue(status_message, 1, "pack").await?;

    let instance_request = InstanceRequest::Create(instance);
    let agent_request = AgentRequest::Create(agent);
    
    #[cfg(not(feature = "devnet"))]
    {
        write_to_queue(instance_request.clone(), 4, "state").await?;
        write_to_queue(agent_request.clone(), 8, "state").await?;
        write_to_queue(account_request.clone(), 2, "state").await?;
    }

    Ok(())
}

pub async fn write_pack_status_completed(
    message: &PackBuildRequest,
    node_id: String
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {

    let signer_address = {
        let pk = VerifyingKey::recover_from_msg(
            &message.hash,
            &Signature::from_slice(&hex::decode(message.sig.sig.clone())?)?,
            RecoveryId::from_byte(message.sig.rec).ok_or(
                Box::new(
                    std::io::Error::new(
                        std::io::ErrorKind::Other,
                        "invalid recovery id"
                    )
                )
            )?
        )?;
        Address::from_public_key(&pk)
    };
    println!("signer address: {signer_address:x}");
    let mut hasher = Sha3::v256();
    let mut build_id = [0u8; 32];
    hasher.update(signer_address.as_ref());
    hasher.update(message.request.formfile.name.as_bytes());
    hasher.finalize(&mut build_id);
    let build_id = hex::encode(build_id);
    let instance_id = build_instance_id(
        node_id.clone(),
        build_id.clone()
    )?;
    let signer_address_hex = hex::encode(signer_address);

    let mut instance = match Client::new() 
        .get(format!("http://127.0.0.1:3004/instance/{instance_id}/get"))
        .send().await?.json::<StateResponse<Instance>>().await {
            Ok(StateResponse::Success(Success::Some(instance))) => instance,
            _ => create_new_instance_entry(
                instance_id.clone(),
                node_id.clone(),
                build_id.clone(),
                signer_address_hex.clone(),
                message.request.formfile.clone()
            )? 
    };

    // Get the existing agent to update it
    let agent_response = Client::new()
        .get(
            format!(
                "http://127.0.0.1:3004/agent/by_build_id/{}",
                build_id.clone()
            )
        )
        .send().await?.json::<StateResponse<AIAgent>>().await;
    
    let mut agent = match agent_response {
        Ok(StateResponse::Success(Success::Some(agent))) => agent,
        _ => create_new_agent_entry(
            message.request.formfile.clone(),
            build_id.clone(),
            signer_address_hex.clone()
        )?, 
    };
    
    // Update agent with instance ID and status
    agent.metadata.insert("instance_id".to_string(), instance_id.clone());
    agent.updated_at = SystemTime::now()
        .duration_since(UNIX_EPOCH)?
        .as_secs() as i64;
    agent.deployment_count += 1;
    
    // Update instance status
    instance.status = InstanceStatus::Built;
    
    // Create necessary requests
    let status_message = PackBuildResponse {
        status: PackBuildStatus::Completed{
            instance: instance.clone(),
            agent: Some(agent.clone()),
            model: None,
        },
        request: message.clone()
    };

    // Create account update to link instance and agent to owner
    let account_request = AccountRequest::AddOwnedInstance {
        address: signer_address_hex.clone(),
        instance_id: instance_id.clone(),
    };
    
    #[cfg(not(feature = "devnet"))]
    write_to_queue(status_message, 1, "pack").await?;

    let instance_request = InstanceRequest::Update(instance);
    let agent_request = AgentRequest::Update(agent);

    #[cfg(not(feature = "devnet"))]
    {
        write_to_queue(instance_request, 4, "state").await?;
        write_to_queue(agent_request, 8, "state").await?;
        write_to_queue(account_request, 2, "state").await?;
    }


    Ok(())
}

pub async fn write_pack_status_failed(
    message: &PackBuildRequest,
    reason: String
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {

    let signer_address = {
        let pk = VerifyingKey::recover_from_msg(
            &message.hash,
            &Signature::from_slice(&hex::decode(message.sig.sig.clone())?)?,
            RecoveryId::from_byte(message.sig.rec)
                .ok_or(
                    Box::new(
                        std::io::Error::new(
                            std::io::ErrorKind::Other,
                            "invalid recovery id"
                        )
                    )
                )?
        )?;
        Address::from_public_key(&pk)
    };
    let mut hasher = Sha3::v256();
    let mut hash = [0u8; 32];
    hasher.update(signer_address.as_ref());
    hasher.update(message.request.formfile.name.as_bytes());
    hasher.finalize(&mut hash);

    let status_message = PackBuildResponse {
        status: PackBuildStatus::Failed { build_id: hex::encode(hash), reason },
        request: message.clone()
    };

    #[cfg(not(feature = "devnet"))]
    write_to_queue(status_message, 1, "pack").await?;

    Ok(())
}
