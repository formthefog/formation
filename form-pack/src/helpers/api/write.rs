use axum::response::Response;
use form_state::accounts::Account;
use crate::formfile::Formfile;
use reqwest::Client;
use std::time::{SystemTime, UNIX_EPOCH};
use k256::ecdsa::{RecoveryId, Signature, VerifyingKey};
use alloy_primitives::Address;
use tiny_keccak::{Sha3, Hasher};
use form_types::state::{Success, Response as StateResponse};
use form_state::datastore::{AgentRequest, InstanceRequest, AccountRequest};
use form_state::agent::AIAgent;
use form_state::instances::{Instance, InstanceStatus};
use crate::types::request::PackBuildRequest;
use crate::types::response::PackBuildResponse;
use crate::types::status::PackBuildStatus;
use crate::helpers::utils::{build_instance_id, create_new_instance_entry, create_new_agent_entry};
use crate::helpers::queue::write::write_to_queue;
use serde_json;


pub async fn write_pack_status_started(
    formfile: Formfile,
    build_id: String,
    node_id: String,
    signer_address: String,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {

    let mut hasher = Sha3::v256();
    let mut hash = [0u8; 32];
    hasher.update(signer_address.as_ref());
    hasher.update(formfile.name.as_bytes());
    hasher.finalize(&mut hash);

    let instance_id = build_instance_id(node_id.clone(), hex::encode(hash))?;
    let signer_address_hex = hex::encode(signer_address);

    let instance = create_new_instance_entry(
        instance_id, 
        node_id, 
        build_id.clone(), 
        signer_address_hex.clone(), 
        formfile.clone()
    )?; 

    // Create and register the AIAgent
    let agent = create_new_agent_entry(
        formfile.clone(),
        build_id.clone(),
        signer_address_hex.clone()
    )?;     

    
    let _ = Client::new()
        .post("http://127.0.0.1:3004/instance/create")
        .json(&instance)
        .send().await?;

    let _ = Client::new()
        .post("http://127.0.0.1:3004/agent/create")
        .json(&agent)
        .send().await?; 

    let endpoint = format!("http://127.0.0.1:3004/account/{}/get", signer_address_hex);
    let account_response = Client::new()
        .get(endpoint)
        .send().await?;

    if !account_response.status().is_success() {
        let error_text = account_response.text().await?;
        return Err(format!("API Error: {}", error_text).into());
    }

    let response_json: serde_json::Value = account_response.json().await?;
    if let Some(account_value) = response_json.get("account") {
        if let Some(mut account) = serde_json::from_value::<Account>(account_value.clone()).ok() {
            account.owned_instances.insert(instance.instance_id);
            Client::new()
                .post("http://127.0.0.1:3004/account/update")
                .json(&account.clone())
                .send().await?;
        }
    }


    Ok(())
}

pub async fn write_pack_status_completed(
    formfile: Formfile,
    build_id: String,
    node_id: String,
    signer_address: String,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let instance_id = build_instance_id(node_id.clone(), build_id.clone())?;

    // Get instance if it exists or create a new one
    let instance_response = Client::new()
        .get(format!("http://127.0.0.1:3004/instance/{instance_id}/get"))
        .send().await?;
    
    let mut instance = if instance_response.status().is_success() {
        let response_json: serde_json::Value = instance_response.json().await?;
        if let Some(instance_value) = response_json.get("instance") {
            if let Some(instance) = serde_json::from_value::<Instance>(instance_value.clone()).ok() {
                instance
            } else {
                create_new_instance_entry(
                    instance_id.clone(),
                    node_id.clone(),
                    build_id.clone(),
                    signer_address.clone(),
                    formfile.clone()
                )?
            }
        } else {
            create_new_instance_entry(
                instance_id.clone(),
                node_id.clone(),
                build_id.clone(),
                signer_address.clone(),
                formfile.clone()
            )?
        }
    } else {
        create_new_instance_entry(
            instance_id.clone(),
            node_id.clone(),
            build_id.clone(),
            signer_address.clone(),
            formfile.clone()
        )?
    };

    // Create new agent if none exists
    let agent_response = Client::new()
        .get(format!("http://127.0.0.1:3004/agents/{}", build_id))
        .send().await?;
    
    let mut agent = if agent_response.status().is_success() {
        let response_json: serde_json::Value = agent_response.json().await?;
        if let Some(agent_value) = response_json.get("agent") {
            if let Some(mut agent) = serde_json::from_value::<AIAgent>(agent_value.clone()).ok() {
                agent.metadata.insert("status".to_string(), "built".to_string());
                agent
            } else {
                create_new_agent_entry(
                    formfile.clone(),
                    build_id.clone(),
                    signer_address.clone()
                )?
            }
        } else {
            create_new_agent_entry(
                formfile.clone(),
                build_id.clone(),
                signer_address.clone()
            )?
        }
    } else {
        create_new_agent_entry(
            formfile.clone(),
            build_id.clone(),
            signer_address.clone()
        )?
    };
    
    // Update instance status and agent information
    instance.status = InstanceStatus::Built;
    
    // Update the instance via API
    let _ = Client::new()
        .post("http://127.0.0.1:3004/instance/update")
        .json(&instance)
        .send().await?;
    
    // Update the agent via API
    let _ = Client::new()
        .post("http://127.0.0.1:3004/agents/update")
        .json(&agent)
        .send().await?;
    
    // Add instance to account if not already added
    let account_endpoint = format!("http://127.0.0.1:3004/account/{}/get", signer_address);
    let account_response = Client::new()
        .get(account_endpoint)
        .send().await?;
    
    if account_response.status().is_success() {
        let response_json: serde_json::Value = account_response.json().await?;
        if let Some(account_value) = response_json.get("account") {
            if let Some(mut account) = serde_json::from_value::<Account>(account_value.clone()).ok() {
                if !account.owned_instances.contains(&instance_id) {
                    account.owned_instances.insert(instance_id.clone());
                    let _ = Client::new()
                        .post("http://127.0.0.1:3004/account/update")
                        .json(&account)
                        .send().await?;
                }
            }
        }
    }
    
    Ok(())
}

pub async fn write_pack_status_failed(
    formfile: &Formfile,
    signer_address: String,
    build_id: String,
    node_id: String,
    reason: String
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Calculate the instance ID from node and build ID
    let instance_id = build_instance_id(node_id.clone(), build_id.clone())?;
    
    // Check if instance exists
    let instance_response = Client::new()
        .get(format!("http://127.0.0.1:3004/instance/{instance_id}/get"))
        .send().await?;
    
    if instance_response.status().is_success() {
        let response_json: serde_json::Value = instance_response.json().await?;
        if let Some(instance_value) = response_json.get("instance") {
            if let Some(mut instance) = serde_json::from_value::<Instance>(instance_value.clone()).ok() {
                // Update instance status to CriticalError
                instance.status = InstanceStatus::CriticalError;
                
                // Update the instance via API
                let _ = Client::new()
                    .post("http://127.0.0.1:3004/instance/update")
                    .json(&instance)
                    .send().await?;
                
                log::info!("Updated instance {} status to CriticalError", instance_id);
            }
        }
    } else {
        // Create a new instance with CriticalError status
        let mut instance = create_new_instance_entry(
            instance_id.clone(),
            node_id.clone(),
            build_id.clone(),
            signer_address.clone(),
            formfile.clone()
        )?;
        instance.status = InstanceStatus::CriticalError;
        
        // Create the instance via API
        let _ = Client::new()
            .post("http://127.0.0.1:3004/instance/create")
            .json(&instance)
            .send().await?;
        
        log::info!("Created new failed instance {} for build {}", instance_id, build_id);
    }
    
    // Instead of using /agent/by_build_id, use the instance to get the agent ID
    // and then update or create an agent with the failure information
    let instance_response = Client::new()
        .get(format!("http://127.0.0.1:3004/instance/{instance_id}/get"))
        .send().await?;
    
    if instance_response.status().is_success() {
        let response_json: serde_json::Value = instance_response.json().await?;
        if let Some(instance) = response_json.get("instance") {
            // If we have an agent_id in the instance, use it to update the agent
            if let Some(agent_id) = instance.get("agent_id").and_then(|v| v.as_str()) {
                let agent_response = Client::new()
                    .get(format!("http://127.0.0.1:3004/agents/{}", agent_id))
                    .send().await?;
                
                if agent_response.status().is_success() {
                    let agent_json: serde_json::Value = agent_response.json().await?;
                    if let Some(agent_value) = agent_json.get("agent") {
                        if let Some(mut agent) = serde_json::from_value::<AIAgent>(agent_value.clone()).ok() {
                            // Update agent metadata to indicate failure
                            agent.metadata.insert("build_status".to_string(), "failed".to_string());
                            agent.metadata.insert("failure_reason".to_string(), reason.clone());
                            agent.updated_at = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() as i64;
                            
                            // Update the agent via API
                            let _ = Client::new()
                                .post("http://127.0.0.1:3004/agents/update")
                                .json(&agent)
                                .send().await?;
                            
                            log::info!("Updated agent {} to reflect build failure", agent_id);
                        }
                    }
                }
            } else {
                // No agent ID found, create a new agent if necessary
                let agent = create_new_agent_entry(
                    formfile.clone(),
                    build_id.clone(),
                    signer_address.clone()
                )?;
                
                // Update agent with failure metadata
                let mut agent = agent;
                agent.metadata.insert("build_status".to_string(), "failed".to_string());
                agent.metadata.insert("failure_reason".to_string(), reason.clone());
                
                // Create the agent via API
                let _ = Client::new()
                    .post("http://127.0.0.1:3004/agents/create")
                    .json(&agent)
                    .send().await?;
                
                log::info!("Created new agent with failed status for build {}", build_id);
            }
        }
    }
    
    Ok(())
}
