
use k256::ecdsa::{RecoveryId, Signature, VerifyingKey};
use alloy_primitives::Address;
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
use crate::types::status::PackBuildStatus;
use crate::types::response::PackBuildResponse;
use crate::types::request::PackBuildRequest;
use crate::helpers::utils::build_instance_id;

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
            QueueResponse::Failure { reason } => return Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, format!("{reason:?}")))),
            _ => return Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, "Invalid response variant for write_local endpoint")))
    }
}

pub async fn write_pack_status_started(message: &PackBuildRequest, node_id: String) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let signer_address = {
        let pk = VerifyingKey::recover_from_msg(
            &message.hash,
            &Signature::from_slice(&hex::decode(message.sig.sig.clone())?)?,
            RecoveryId::from_byte(message.sig.rec).ok_or(Box::new(std::io::Error::new(std::io::ErrorKind::Other, "invalid recovery id")))?
        )?;
        Address::from_public_key(&pk)
    };
    let mut hasher = Sha3::v256();
    let mut hash = [0u8; 32];
    hasher.update(signer_address.as_ref());
    hasher.update(message.request.formfile.name.as_bytes());
    hasher.finalize(&mut hash);
    let instance_id = build_instance_id(node_id.clone(), hex::encode(hash))?;
    let signer_address_hex = hex::encode(signer_address);

    let instance = Instance {
        instance_id: instance_id.clone(),
        node_id: node_id.clone(),
        build_id: hex::encode(hash),
        instance_owner: signer_address_hex.clone(),
        updated_at: SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() as i64,
        status: InstanceStatus::Building,
        formfile: serde_json::to_string(&message.request.formfile)?,
        resources: InstanceResources {
            vcpus: message.request.formfile.get_vcpus(),
            memory_mb: message.request.formfile.get_memory() as u32,
            bandwidth_mbps: 1000,
            gpu: None,
        },
        ..Default::default()
    };
    
    // Create and register the AIAgent
    let mut agent = AIAgent::default();
    agent.agent_id = Uuid::new_v4().to_string();
    agent.name = message.request.formfile.name.clone();
    agent.owner_id = signer_address_hex.clone();
    agent.description = message.request.formfile.get_description().unwrap_or("").to_string();
    agent.requires_specific_model = message.request.formfile.is_model_required();
    agent.required_model_id = message.request.formfile.get_model_id().map(|s| s.to_string());
    
    // Set formfile template
    if let Ok(formfile_json) = serde_json::to_string(&message.request.formfile) {
        agent.formfile_template = base64::encode(formfile_json);
    }
    
    // Set resource requirements based on Formfile
    agent.resource_requirements.min_vcpus = message.request.formfile.get_vcpus();
    agent.resource_requirements.recommended_vcpus = message.request.formfile.get_vcpus();
    agent.resource_requirements.min_memory_mb = message.request.formfile.get_memory() as u64;
    agent.resource_requirements.recommended_memory_mb = message.request.formfile.get_memory() as u64;
    agent.resource_requirements.min_disk_gb = message.request.formfile.get_storage().unwrap_or(5) as u64;
    agent.resource_requirements.recommended_disk_gb = message.request.formfile.get_storage().unwrap_or(5) as u64;
    agent.resource_requirements.requires_gpu = message.request.formfile.get_gpu_devices().is_some();
    agent.has_filesystem_access = true; // VM-based agents have filesystem access
    
    // Add metadata for build ID
    agent.metadata.insert("build_id".to_string(), hex::encode(hash));
    
    // Create account update to link instance to owner
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

    #[cfg(feature = "devnet")]
    {
        reqwest::Client::new().post("http://127.0.0.1:3004/instance/create")
            .json(&instance_request)
            .send()
            .await?
            .json()
            .await?;
            
        reqwest::Client::new().post("http://127.0.0.1:3004/agent/create")
            .json(&agent_request)
            .send()
            .await?
            .json()
            .await?;
            
        reqwest::Client::new().post("http://127.0.0.1:3004/account/update")
            .json(&account_request)
            .send()
            .await?
            .json()
            .await?;
    }

    Ok(())
}

pub async fn write_pack_status_completed(message: &PackBuildRequest, node_id: String) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let signer_address = {
        let pk = VerifyingKey::recover_from_msg(
            &message.hash,
            &Signature::from_slice(&hex::decode(message.sig.sig.clone())?)?,
            RecoveryId::from_byte(message.sig.rec).ok_or(Box::new(std::io::Error::new(std::io::ErrorKind::Other, "invalid recovery id")))?
        )?;
        Address::from_public_key(&pk)
    };
    println!("signer address: {signer_address:x}");
    let mut hasher = Sha3::v256();
    let mut build_id = [0u8; 32];
    hasher.update(signer_address.as_ref());
    hasher.update(message.request.formfile.name.as_bytes());
    hasher.finalize(&mut build_id);
    let instance_id = build_instance_id(node_id.clone(), hex::encode(build_id))?;
    let signer_address_hex = hex::encode(signer_address);

    let mut instance = match Client::new() 
        .get(format!("http://127.0.0.1:3004/instance/{instance_id}/get"))
        .send().await?.json::<StateResponse<Instance>>().await {
            Ok(StateResponse::Success(Success::Some(instance))) => instance,
            _ => {
                Instance {
                    instance_id: instance_id.clone(),
                    node_id: node_id.clone(),
                    build_id: hex::encode(build_id),
                    instance_owner: signer_address_hex.clone(),
                    updated_at: SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() as i64,
                    formfile: serde_json::to_string(&message.request.formfile)?,
                    snapshots: None,
                    resources: InstanceResources {
                        vcpus: message.request.formfile.get_vcpus(),
                        memory_mb: message.request.formfile.get_memory() as u32,
                        bandwidth_mbps: 1000,
                        gpu: None,
                    },
                    ..Default::default()
                }
            }
    };

    // Get the existing agent to update it
    let agent_response = Client::new()
        .get(format!("http://127.0.0.1:3004/agent/by_build_id/{}", hex::encode(build_id)))
        .send().await?.json::<StateResponse<AIAgent>>().await;
    
    let mut agent = match agent_response {
        Ok(StateResponse::Success(Success::Some(agent))) => agent,
        _ => {
            // If agent not found, create a new one
            let mut agent = AIAgent::default();
            agent.agent_id = Uuid::new_v4().to_string();
            agent.name = message.request.formfile.name.clone();
            agent.owner_id = signer_address_hex.clone();
            agent.description = message.request.formfile.get_description().unwrap_or("").to_string();
            agent.requires_specific_model = message.request.formfile.is_model_required();
            agent.required_model_id = message.request.formfile.get_model_id().map(|s| s.to_string());
            
            // Set formfile template
            if let Ok(formfile_json) = serde_json::to_string(&message.request.formfile) {
                agent.formfile_template = base64::encode(formfile_json);
            }
            
            // Set resource requirements based on Formfile
            agent.resource_requirements.min_vcpus = message.request.formfile.get_vcpus();
            agent.resource_requirements.recommended_vcpus = message.request.formfile.get_vcpus();
            agent.resource_requirements.min_memory_mb = message.request.formfile.get_memory() as u64;
            agent.resource_requirements.recommended_memory_mb = message.request.formfile.get_memory() as u64;
            agent.resource_requirements.min_disk_gb = message.request.formfile.get_storage().unwrap_or(5) as u64;
            agent.resource_requirements.recommended_disk_gb = message.request.formfile.get_storage().unwrap_or(5) as u64;
            agent.resource_requirements.requires_gpu = message.request.formfile.get_gpu_devices().is_some();
            agent.has_filesystem_access = true; // VM-based agents have filesystem access
            
            // Add metadata for build ID
            agent.metadata.insert("build_id".to_string(), hex::encode(build_id));
            
            agent
        }
    };
    
    // Update agent with instance ID and status
    agent.metadata.insert("instance_id".to_string(), instance_id.clone());
    agent.updated_at = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() as i64;
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

    #[cfg(feature = "devnet")]
    {
        reqwest::Client::new().post("http://127.0.0.1:3004/instance/update")
            .json(&instance_request)
            .send()
            .await?
            .json()
            .await?;
            
        reqwest::Client::new().post("http://127.0.0.1:3004/agent/update")
            .json(&agent_request)
            .send()
            .await?
            .json()
            .await?;
            
        reqwest::Client::new().post("http://127.0.0.1:3004/account/update")
            .json(&account_request)
            .send()
            .await?
            .json()
            .await?;
    }

    Ok(())
}

pub async fn write_pack_status_failed(message: &PackBuildRequest, reason: String) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let signer_address = {
        let pk = VerifyingKey::recover_from_msg(
            &message.hash,
            &Signature::from_slice(&hex::decode(message.sig.sig.clone())?)?,
            RecoveryId::from_byte(message.sig.rec).ok_or(Box::new(std::io::Error::new(std::io::ErrorKind::Other, "invalid recovery id")))?
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

