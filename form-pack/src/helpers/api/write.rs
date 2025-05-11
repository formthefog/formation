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
// DO NOT REMOVE P2P import
use crate::helpers::queue::write::write_to_queue;
use serde_json;
use log::{info, error, warn};
use serde_json::json;

// Define the URL for form-state
const FORM_STATE_URL: &str = "http://127.0.0.1:3004"; // Or use std::env::var if preferred


pub async fn write_pack_status_started(
    formfile: Formfile,
    build_id: String,
    node_id: String,
    signer_address: String,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    info!("[write_pack_status_started] For build_id: {}, node_id: {}, signer_address: {}", build_id, node_id, signer_address);

    let mut hasher = Sha3::v256();
    let mut hash_output_for_instance_id = [0u8; 32]; // Renamed to avoid confusion with `hash` if used elsewhere
    // It seems signer_address is already hex from recovered_address.as_hex(). No further hex encoding needed here.
    hasher.update(signer_address.as_bytes()); // Assuming signer_address is raw bytes or a string that represents them correctly for hashing
    hasher.update(formfile.name.as_bytes());
    hasher.finalize(&mut hash_output_for_instance_id);

    let instance_id = build_instance_id(node_id.clone(), build_id.clone())?;
    // signer_address is already the hex string from recovered_address.as_hex()
    // let signer_address_hex = signer_address; // No longer needed, use signer_address directly

    let instance_data = create_new_instance_entry(
        instance_id.clone(), // Pass cloned instance_id
        node_id.clone(), 
        build_id.clone(), 
        signer_address.clone(), // Pass signer_address directly 
        formfile.clone()
    )?; 

    let agent_data = create_new_agent_entry(
        formfile.clone(),
        build_id.clone(),
        signer_address.clone() // Pass signer_address directly
    )?;     

    let client = Client::new(); // Create client once

    // Call to create instance
    let instance_create_url = format!("{}/instance/create", FORM_STATE_URL);
    info!("[write_pack_status_started] Attempting to POST to {}: Payload: {:#?}", instance_create_url, instance_data);
    match client.post(&instance_create_url).json(&instance_data).send().await {
        Ok(resp) => {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_else(|e| format!("Error reading text from instance/create response: {}", e));
            info!("[write_pack_status_started] Response from {}/instance/create: Status: {}, Body: {}", FORM_STATE_URL, status, text);
            if !status.is_success() {
                error!("[write_pack_status_started] Call to {}/instance/create failed with status {} and body {}", FORM_STATE_URL, status, text);
            }
        }
        Err(e) => {
            error!("[write_pack_status_started] Error sending request to {}/instance/create: {}", FORM_STATE_URL, e);
            return Err(Box::new(e)); // Propagate error
        }
    }

    // Call to create agent
    let agent_create_url = format!("{}/agents/create", FORM_STATE_URL);
    info!("[write_pack_status_started] Attempting to POST to {}: Payload: {:#?}", agent_create_url, agent_data);
    match client.post(&agent_create_url).json(&agent_data).send().await {
        Ok(resp) => {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_else(|e| format!("Error reading text from agents/create response: {}", e));
            info!("[write_pack_status_started] Response from {}/agents/create: Status: {}, Body: {}", FORM_STATE_URL, status, text);
            if !status.is_success() {
                error!("[write_pack_status_started] Call to {}/agents/create failed with status {} and body {}.", FORM_STATE_URL, status, text);
            }
        }
        Err(e) => {
            error!("[write_pack_status_started] Error sending request to {}/agents/create: {}", FORM_STATE_URL, e);
            return Err(Box::new(e)); // Propagate error
        }
    }

    // Call to get account
    let prefixed_signer_address = if signer_address.starts_with("0x") {
        signer_address.clone()
    } else {
        format!("0x{}", signer_address)
    };
    let account_get_url = format!("{}/account/{}/get", FORM_STATE_URL, prefixed_signer_address);
    info!("[write_pack_status_started] Attempting to GET from {}", account_get_url);
    match client.get(&account_get_url).send().await {
        Ok(account_response) => {
            let status = account_response.status();
            let acc_text = account_response.text().await.unwrap_or_else(|e| format!("Error reading text from account/get response: {}",e));
            info!("[write_pack_status_started] Response from {}/account/{}/get: Status: {}, Body: {}", FORM_STATE_URL, prefixed_signer_address, status, acc_text);
            
            if status.is_success() {
                match serde_json::from_str::<serde_json::Value>(&acc_text) {
                    Ok(response_json) => {
                        if let Some(account_value) = response_json.get("account") {
                            if let Some(mut account) = serde_json::from_value::<Account>(account_value.clone()).ok() {
                                let mut updated = false;
                                if !account.owned_instances.contains(&instance_id) {
                                    account.owned_instances.insert(instance_id.clone());
                                    updated = true;
                                }
                                // Assuming agent_id for owned_agents is the same as build_id
                                if !account.owned_agents.contains(&build_id) {
                                    account.owned_agents.insert(build_id.clone());
                                    updated = true;
                                }

                                if updated {
                                    account.updated_at = SystemTime::now().duration_since(UNIX_EPOCH).map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?.as_secs() as i64;
                                    let account_update_url = format!("{}/account/update", FORM_STATE_URL);
                                    let account_update_payload = AccountRequest::Update(account); // Use enum variant
                                    info!("[write_pack_status_started] Attempting to POST to {}: Payload: {:#?}", account_update_url, account_update_payload);
                                    match client.post(&account_update_url).json(&account_update_payload).send().await {
                                        Ok(up_resp) => {
                                            let up_status = up_resp.status();
                                            let up_text = up_resp.text().await.unwrap_or_else(|e| format!("Error reading text from account/update response: {}", e));
                                            info!("[write_pack_status_started] Response from {}/account/update: Status: {}, Body: {}", FORM_STATE_URL, up_status, up_text);
                                            if !up_status.is_success() {
                                                error!("[write_pack_status_started] Call to {}/account/update failed with status {} and body {}.", FORM_STATE_URL, up_status, up_text);
                                            }
                                        }
                                        Err(e) => {
                                            error!("[write_pack_status_started] Error sending request to {}/account/update: {}", FORM_STATE_URL, e);
                                            // Decide if this should also return Err
                                        }
                                    }
                                }
                            } else { error!("[write_pack_status_started] Failed to deserialize 'account' object from JSON value: {:?}", account_value); }
                        } else { error!("[write_pack_status_started] 'account' field not found in JSON response for GET account: {:?}", response_json); }
                    }
                    Err(e) => { error!("[write_pack_status_started] Failed to parse GET account response as JSON: {}. Raw text: {}",e, acc_text); }
                }
            } else {
                error!("[write_pack_status_started] Call to {}/account/{}/get failed with status {}.", FORM_STATE_URL, prefixed_signer_address, status);
            }
        }
        Err(e) => {
            error!("[write_pack_status_started] Error sending request to {}/account/{}/get: {}", FORM_STATE_URL, prefixed_signer_address, e);
            return Err(Box::new(e)); // Propagate error
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
    info!("[write_pack_status_completed] For build_id: {}, node_id: {}, signer_address: {}", build_id, node_id, signer_address);

    let instance_id = build_instance_id(node_id.clone(), build_id.clone())?;
    info!("[write_pack_status_completed] Derived instance_id: {}", instance_id);

    let client = Client::new();
    let mut instance_to_process: Instance;

    // Get existing instance or prepare a new one
    let instance_get_url = format!("{}/instance/{}/get", FORM_STATE_URL, instance_id);
    info!("[write_pack_status_completed] Attempting to GET instance from: {}", instance_get_url);
    match client.get(&instance_get_url).send().await {
        Ok(resp) => {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_else(|e| format!("Error reading text from instance/get: {}", e));
            info!("[write_pack_status_completed] Response from instance/get: Status: {}, Body: {}", status, text);
            if status.is_success() {
                match serde_json::from_str::<serde_json::Value>(&text) {
                    Ok(json_val) => {
                        if let Some(inst_val) = json_val.get("instance") {
                            match serde_json::from_value::<Instance>(inst_val.clone()) {
                                Ok(inst) => instance_to_process = inst,
                                Err(e) => {
                                    error!("[write_pack_status_completed] Failed to deserialize existing instance: {}. Creating new.", e);
                                    instance_to_process = create_new_instance_entry(instance_id.clone(), node_id.clone(), build_id.clone(), signer_address.clone(), formfile.clone())?;
                                }
                            }
                        } else {
                            error!("[write_pack_status_completed] 'instance' field not found in GET instance response. Creating new.");
                            instance_to_process = create_new_instance_entry(instance_id.clone(), node_id.clone(), build_id.clone(), signer_address.clone(), formfile.clone())?;
                        }
                    }
                    Err(e) => {
                        error!("[write_pack_status_completed] Failed to parse instance/get response as JSON: {}. Creating new. Raw: {}",e, text);
                        instance_to_process = create_new_instance_entry(instance_id.clone(), node_id.clone(), build_id.clone(), signer_address.clone(), formfile.clone())?;
                    }
                }
            } else {
                info!("[write_pack_status_completed] Instance {} not found or GET failed. Creating new.", instance_id);
                instance_to_process = create_new_instance_entry(instance_id.clone(), node_id.clone(), build_id.clone(), signer_address.clone(), formfile.clone())?;
            }
        }
        Err(e) => {
            error!("[write_pack_status_completed] Error on GET instance request: {}. Creating new.", e);
            instance_to_process = create_new_instance_entry(instance_id.clone(), node_id.clone(), build_id.clone(), signer_address.clone(), formfile.clone())?;
        }
    }

    instance_to_process.status = InstanceStatus::Built;
    instance_to_process.updated_at = SystemTime::now().duration_since(UNIX_EPOCH).map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?.as_secs() as i64;
    
    // Update the instance
    let instance_update_url = format!("{}/instance/update", FORM_STATE_URL);
    info!("[write_pack_status_completed] Attempting to POST to {}: Payload: {:#?}", instance_update_url, instance_to_process);
    match client.post(&instance_update_url).json(&instance_to_process).send().await {
        Ok(resp) => {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_else(|e| format!("Error reading text from instance/update: {}", e));
            info!("[write_pack_status_completed] Response from instance/update: Status: {}, Body: {}", status, text);
            if !status.is_success() { error!("[write_pack_status_completed] Call to instance/update failed: {} - {}", status, text); }
        }
        Err(e) => { error!("[write_pack_status_completed] Error sending instance/update request: {}", e); /* Consider returning Err(e) */ }
    }

    // Get or create agent
    let agent_get_url = format!("{}/agents/{}", FORM_STATE_URL, build_id);
    info!("[write_pack_status_completed] Attempting to GET agent from: {}", agent_get_url);
    let mut agent_to_process: AIAgent;
    match client.get(&agent_get_url).send().await {
        Ok(resp) => {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_else(|e| format!("Error reading text from agent/get: {}",e));
            info!("[write_pack_status_completed] Response from agent/get: Status: {}, Body: {}", status, text);
            if status.is_success() {
                match serde_json::from_str::<serde_json::Value>(&text) {
                    Ok(json_val) => {
                         if let Some(agent_val) = json_val.get("agent") {
                            match serde_json::from_value::<AIAgent>(agent_val.clone()) {
                                Ok(mut agent) => { 
                                    agent.metadata.insert("status".to_string(), "built".to_string());
                                    agent.updated_at = SystemTime::now().duration_since(UNIX_EPOCH).map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?.as_secs() as i64;
                                    agent_to_process = agent; 
                                }
                                Err(e) => { 
                                    error!("[write_pack_status_completed] Failed to deserialize existing agent: {}. Creating new.", e);
                                    agent_to_process = create_new_agent_entry(formfile.clone(), build_id.clone(), signer_address.clone())?;
                                    agent_to_process.metadata.insert("status".to_string(), "built".to_string());
                                }
                            }
                         } else {
                            error!("[write_pack_status_completed] 'agent' field not found in GET agent. Creating new.");
                            agent_to_process = create_new_agent_entry(formfile.clone(), build_id.clone(), signer_address.clone())?;
                            agent_to_process.metadata.insert("status".to_string(), "built".to_string());
                         }
                    }
                    Err(e) => {
                        error!("[write_pack_status_completed] Failed to parse agent/get as JSON: {}. Creating new. Raw: {}", e, text);
                        agent_to_process = create_new_agent_entry(formfile.clone(), build_id.clone(), signer_address.clone())?;
                        agent_to_process.metadata.insert("status".to_string(), "built".to_string());
                    }
                }
            } else {
                info!("[write_pack_status_completed] Agent {} not found or GET failed. Creating new.", build_id);
                agent_to_process = create_new_agent_entry(formfile.clone(), build_id.clone(), signer_address.clone())?;
                agent_to_process.metadata.insert("status".to_string(), "built".to_string());
            }
        }
        Err(e) => {
            error!("[write_pack_status_completed] Error on GET agent request: {}. Creating new.", e);
            agent_to_process = create_new_agent_entry(formfile.clone(), build_id.clone(), signer_address.clone())?;
            agent_to_process.metadata.insert("status".to_string(), "built".to_string());
        }
    }

    // Update or Create the agent (prefer update if exists)
    let agent_update_url = format!("{}/agents/update", FORM_STATE_URL);
    info!("[write_pack_status_completed] Attempting to POST to {}: Payload: {:#?}", agent_update_url, agent_to_process);
    match client.post(&agent_update_url).json(&agent_to_process).send().await {
        Ok(resp) => {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_else(|e| format!("Error reading text from agent/update: {}", e));
            info!("[write_pack_status_completed] Response from agent/update: Status: {}, Body: {}", status, text);
            if !status.is_success() { error!("[write_pack_status_completed] Call to agent/update failed: {} - {}", status, text); }
        }
        Err(e) => { error!("[write_pack_status_completed] Error sending agent/update request: {}", e); }
    }
    
    // Get account for final update
    let prefixed_signer_address_completed = if signer_address.starts_with("0x") {
        signer_address.clone()
    } else {
        format!("0x{}", signer_address)
    };
    let account_get_url = format!("{}/account/{}/get", FORM_STATE_URL, prefixed_signer_address_completed);
    info!("[write_pack_status_completed] Attempting to GET account for final update from: {}", account_get_url);
    match client.get(&account_get_url).send().await {
        Ok(account_response) => {
            let status = account_response.status();
            let acc_text = account_response.text().await.unwrap_or_else(|e| format!("Error reading text from account/get for update: {}", e));
            info!("[write_pack_status_completed] Response from account/get for update: Status: {}, Body: {}", status, acc_text);
            if status.is_success() {
                match serde_json::from_str::<serde_json::Value>(&acc_text) {
                    Ok(response_json) => {
                        if let Some(account_value) = response_json.get("account") {
                            if let Some(mut account) = serde_json::from_value::<Account>(account_value.clone()).ok() {
                                let mut updated = false;
                                if !account.owned_instances.contains(&instance_id) {
                                    account.owned_instances.insert(instance_id.clone());
                                    updated = true;
                                }
                                if !account.owned_agents.contains(&build_id) { 
                                    account.owned_agents.insert(build_id.clone());
                                    updated = true;
                                }
                                if updated {
                                    account.updated_at = SystemTime::now().duration_since(UNIX_EPOCH).map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?.as_secs() as i64;
                                    let account_update_url = format!("{}/account/update", FORM_STATE_URL);
                                    let account_update_payload = AccountRequest::Update(account);
                                    info!("[write_pack_status_completed] Attempting final POST to {}: Payload: {:#?}", account_update_url, account_update_payload);
                                    match client.post(&account_update_url).json(&account_update_payload).send().await {
                                        Ok(up_resp) => {
                                            let up_status = up_resp.status();
                                            let up_text = up_resp.text().await.unwrap_or_else(|e| format!("Error reading text from final account/update: {}",e));
                                            info!("[write_pack_status_completed] Response from final account/update: Status: {}, Body: {}", up_status, up_text);
                                            if !up_status.is_success() { error!("[write_pack_status_completed] Final call to account/update failed: {} - {}", up_status, up_text); }
                                        }
                                        Err(e) => { error!("[write_pack_status_completed] Error sending final account/update request: {}", e); }
                                    }
                                }
                            } else { error!("[write_pack_status_completed] Failed to deserialize account for final update: {:?}", account_value); }
                        } else { error!("[write_pack_status_completed] 'account' field not found in GET account for final update: {:?}", response_json); }
                    }
                    Err(e) => { error!("[write_pack_status_completed] Failed to parse GET account for final update as JSON: {}. Raw: {}",e, acc_text); }
                }
            } else { error!("[write_pack_status_completed] Final GET account call failed: Status {}", status); }
        }
        Err(e) => { error!("[write_pack_status_completed] Error sending final GET account request: {}", e); }
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
    info!("[write_pack_status_failed] For build_id: {}, node_id: {}, signer_address: {}, reason: {}", build_id, node_id, signer_address, reason);
    let client = Client::new();

    let instance_id = build_instance_id(node_id.clone(), build_id.clone())?;
    info!("[write_pack_status_failed] Derived instance_id: {}", instance_id);
    
    let mut instance_to_process: Instance;
    let mut instance_needs_creation = false;

    // Get existing instance or prepare a new one
    let instance_get_url = format!("{}/instance/{}/get", FORM_STATE_URL, instance_id);
    info!("[write_pack_status_failed] Attempting to GET instance from: {}", instance_get_url);
    match client.get(&instance_get_url).send().await {
        Ok(resp) => {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_else(|e| format!("Error reading text from instance/get for failed status: {}", e));
            info!("[write_pack_status_failed] Response from instance/get: Status: {}, Body: {}", status, text);

            if status.is_success() {
                match serde_json::from_str::<serde_json::Value>(&text) {
                    Ok(json_val) => {
                        if let Some(inst_val) = json_val.get("instance") {
                            match serde_json::from_value::<Instance>(inst_val.clone()) {
                                Ok(inst) => instance_to_process = inst,
                                Err(e) => {
                                    error!("[write_pack_status_failed] Failed to deserialize existing instance: {}. Will create new.", e);
                                    instance_to_process = create_new_instance_entry(instance_id.clone(), node_id.clone(), build_id.clone(), signer_address.clone(), formfile.clone())?;
                                    instance_needs_creation = true;
                                }
                            }
                        } else {
                            error!("[write_pack_status_failed] 'instance' field not found in GET instance response. Will create new.");
                            instance_to_process = create_new_instance_entry(instance_id.clone(), node_id.clone(), build_id.clone(), signer_address.clone(), formfile.clone())?;
                            instance_needs_creation = true;
                        }
                    }
                    Err(e) => {
                        error!("[write_pack_status_failed] Failed to parse instance/get response as JSON: {}. Will create new. Raw: {}",e, text);
                        instance_to_process = create_new_instance_entry(instance_id.clone(), node_id.clone(), build_id.clone(), signer_address.clone(), formfile.clone())?;
                        instance_needs_creation = true;
                    }
                }
            } else {
                info!("[write_pack_status_failed] Instance {} not found or GET failed. Will create new.", instance_id);
                instance_to_process = create_new_instance_entry(instance_id.clone(), node_id.clone(), build_id.clone(), signer_address.clone(), formfile.clone())?;
                instance_needs_creation = true;
            }
        }
        Err(e) => {
            error!("[write_pack_status_failed] Error on GET instance request: {}. Will create new.", e);
            instance_to_process = create_new_instance_entry(instance_id.clone(), node_id.clone(), build_id.clone(), signer_address.clone(), formfile.clone())?;
            instance_needs_creation = true;
        }
    }

    instance_to_process.status = InstanceStatus::CriticalError; // Set to CriticalError
    instance_to_process.updated_at = SystemTime::now().duration_since(UNIX_EPOCH).map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?.as_secs() as i64;
    // Potentially add reason to instance metadata if the struct supports it
    // instance_to_process.metadata.insert("failure_reason".to_string(), reason.clone()); 

    // Update or Create the instance with CriticalError status
    let (instance_call_url, use_create_for_instance) = if instance_needs_creation {
        (format!("{}/instance/create", FORM_STATE_URL), true)
    } else {
        (format!("{}/instance/update", FORM_STATE_URL), false)
    };
    info!("[write_pack_status_failed] Attempting to POST to {}: Payload: {:#?}", instance_call_url, instance_to_process);
    match client.post(&instance_call_url).json(&instance_to_process).send().await {
        Ok(resp) => {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_else(|e| format!("Error reading text from instance update/create: {}", e));
            info!("[write_pack_status_failed] Response from {}: Status: {}, Body: {}", instance_call_url, status, text);
            if !status.is_success() { error!("[write_pack_status_failed] Call to {} failed: {} - {}", instance_call_url, status, text); }
        }
        Err(e) => { error!("[write_pack_status_failed] Error sending request to {}: {}", instance_call_url, e); }
    }

    // Update or create an agent with failure information
    let agent_get_url = format!("{}/agents/{}", FORM_STATE_URL, build_id);
    info!("[write_pack_status_failed] Attempting to GET agent from: {}", agent_get_url);
    let mut agent_to_process: AIAgent;
    let mut agent_needs_creation = false;

    match client.get(&agent_get_url).send().await {
        Ok(resp) => {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_else(|e| format!("Error reading text from agent/get for failure: {}",e));
            info!("[write_pack_status_failed] Response from agent/get for failure: Status: {}, Body: {}", status, text);
            if status.is_success() {
                 match serde_json::from_str::<serde_json::Value>(&text) {
                    Ok(json_value) => {
                        if let Some(agent_val) = json_value.get("agent") {
                            match serde_json::from_value::<AIAgent>(agent_val.clone()) {
                                Ok(mut agent) => agent_to_process = agent,
                                Err(e) => {
                                    error!("[write_pack_status_failed] Failed to deserialize existing agent: {}. Creating new.", e);
                                    agent_to_process = create_new_agent_entry(formfile.clone(), build_id.clone(), signer_address.clone())?;
                                    agent_needs_creation = true;
                                }
                            }
                        } else {
                            error!("[write_pack_status_failed] 'agent' field not found in GET agent response. Creating new.");
                            agent_to_process = create_new_agent_entry(formfile.clone(), build_id.clone(), signer_address.clone())?;
                            agent_needs_creation = true;
                        }
                    }
                    Err(e) => {
                        error!("[write_pack_status_failed] Failed to parse agent/get as JSON: {}. Creating new. Raw: {}", e, text);
                        agent_to_process = create_new_agent_entry(formfile.clone(), build_id.clone(), signer_address.clone())?;
                        agent_needs_creation = true;
                    }
                 }
            } else {
                info!("[write_pack_status_failed] Agent {} not found or GET failed. Creating new.", build_id);
                agent_to_process = create_new_agent_entry(formfile.clone(), build_id.clone(), signer_address.clone())?;
                agent_needs_creation = true;
            }
        }
        Err(e) => {
            error!("[write_pack_status_failed] Error on GET agent request: {}. Creating new.", e);
            agent_to_process = create_new_agent_entry(formfile.clone(), build_id.clone(), signer_address.clone())?;
            agent_needs_creation = true;
        }
    }
    
    agent_to_process.metadata.insert("build_status".to_string(), "failed".to_string());
    agent_to_process.metadata.insert("failure_reason".to_string(), reason.clone());
    agent_to_process.updated_at = SystemTime::now().duration_since(UNIX_EPOCH).map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?.as_secs() as i64;

    // Update or Create the agent with failure status
    let (agent_call_url, use_create_for_agent) = if agent_needs_creation {
        (format!("{}/agents/create", FORM_STATE_URL), true)
    } else {
        (format!("{}/agents/update", FORM_STATE_URL), false)
    };
    info!("[write_pack_status_failed] Attempting to POST to {}: Payload: {:#?}", agent_call_url, agent_to_process);
    match client.post(&agent_call_url).json(&agent_to_process).send().await {
        Ok(resp) => {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_else(|e| format!("Error reading text from agent update/create: {}", e));
            info!("[write_pack_status_failed] Response from {}: Status: {}, Body: {}", agent_call_url, status, text);
            if !status.is_success() { error!("[write_pack_status_failed] Call to {} failed: {} - {}", agent_call_url, status, text); }
        }
        Err(e) => { error!("[write_pack_status_failed] Error sending request to {}: {}", agent_call_url, e); }
    }

    Ok(())
}
