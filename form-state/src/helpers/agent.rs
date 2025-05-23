use crate::datastore::DataStore;
use crate::agent::*;
use crate::auth::RecoveredAddress;
use std::sync::Arc;
use tokio::sync::Mutex;
use axum::{extract::{State, Path, ConnectInfo}, Json};
use axum::http::StatusCode;
use serde_json::json;
use axum::response::IntoResponse;
use std::net::SocketAddr;

pub async fn create_agent(
    State(state): State<Arc<Mutex<DataStore>>>,
    recovered: Option<RecoveredAddress>,
    ConnectInfo(connection_info): ConnectInfo<SocketAddr>,
    Json(payload): Json<AIAgent>,
) -> impl IntoResponse {
    log::info!("Received create agent request from {}", connection_info.to_string());
    let mut datastore = state.lock().await;
    
    let remote_addr = connection_info.to_string();
    let is_localhost = remote_addr.starts_with("127.0.0.1") || remote_addr.starts_with("::1");
    log::info!("Is localhost: {}", is_localhost);
    
    // Check if this is a request from an admin node with an original user address
    let effective_address = if is_localhost {
        // If it's an admin node, extract the original user address from the payload
        log::info!("Extracting original user address from payload");
        payload.owner_id.clone()
    } else {
        // If it's a regular user, use their address
        match recovered {
            Some(address) => {
                log::info!("Using recovered address: {}", address.as_hex());
                address.as_hex()
            },
            None => {
                log::error!("No recovered address found in create agent request");
                return (
                    StatusCode::UNAUTHORIZED,
                    Json(
                        json!({
                            "error": format!("Failed to create agent: requests from remote address must included a valid recovered address")
                        })
                    )
                )
            }
        }
    };

    log::info!("Effective address: {}", effective_address);

    let mut agent = payload.clone();
    agent.owner_id = effective_address.clone();

    log::info!("Creating agent with owner ID: {}", effective_address);
    
    // Create and apply the agent update
    let op = datastore.agent_state.update_agent_local(agent.clone());
    if let Err(e) = datastore.handle_agent_op(op).await {
        log::error!("Failed to create agent: {}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({
                "error": format!("Failed to create agent: {}", e)
            })),
        );
    }

    log::info!("Agent created successfully");

    (
        StatusCode::CREATED,
        Json(json!({
            "status": "success",
            "agent": payload 
        })),
    )
}

pub async fn update_agent(
    State(state): State<Arc<Mutex<DataStore>>>,
    recovered: Option<RecoveredAddress>,
    ConnectInfo(connection_info): ConnectInfo<SocketAddr>,
    Json(payload): Json<AIAgent>,
) -> impl IntoResponse {
    log::info!("Received update agent request for agent_id: {}", payload.agent_id);
    let mut datastore = state.lock().await;
    
    let remote_addr = connection_info.to_string();
    let is_localhost = remote_addr.starts_with("127.0.0.1") || remote_addr.starts_with("::1");

    let existing_agent = match datastore.agent_state.get_agent(&payload.agent_id) {
        Some(agent) => agent,
        None => {
            log::warn!("Update attempt for non-existent agent_id: {}", payload.agent_id);
            return (
                StatusCode::NOT_FOUND,
                Json(json!({
                    "success": false,
                    "error": "Agent not found"
                })),
            );
        }
    };

    let can_update = if is_localhost {
        log::info!("Agent update from localhost for agent_id: {} - access granted.", payload.agent_id);
        true
    } else {
        match recovered.as_ref() {
            Some(auth_data) => {
                let authenticated_address_hex = auth_data.as_hex();
                let normalized_authenticated_address = authenticated_address_hex.to_lowercase();
                let owner_id_normalized = existing_agent.owner_id.strip_prefix("0x").unwrap_or(&existing_agent.owner_id).to_lowercase();

                if normalized_authenticated_address == owner_id_normalized {
                    true
                } else if datastore.network_state.is_admin_address(&authenticated_address_hex) {
                    log::info!("Admin {} updating agent {}", authenticated_address_hex, payload.agent_id);
                    true
                } else {
                    log::warn!("Unauthorized agent update: Auth as {} for agent {} owned by {}.", authenticated_address_hex, payload.agent_id, existing_agent.owner_id);
                    false
                }
            }
            None => {
                log::warn!("Unauthorized agent update: No authentication data provided for non-localhost request for agent_id: {}.", payload.agent_id);
                false
            }
        }
    };

    if !can_update {
        return (
            StatusCode::FORBIDDEN,
            Json(json!({
                "success": false,
                "error": "You don't have permission to update this agent"
            })),
        );
    }
    
    // Check if trying to change owner_id without proper rights
    let mut attempting_owner_change = false;
    if payload.owner_id.strip_prefix("0x").unwrap_or(&payload.owner_id).to_lowercase() != existing_agent.owner_id.strip_prefix("0x").unwrap_or(&existing_agent.owner_id).to_lowercase() {
        attempting_owner_change = true;
    }

    if attempting_owner_change {
        let mut can_change_owner = is_localhost;
        if !can_change_owner {
            if let Some(auth_data) = recovered.as_ref() {
                if datastore.network_state.is_admin_address(&auth_data.as_hex()) {
                    can_change_owner = true;
                }
            }
        }
        if !can_change_owner {
            log::warn!("Attempt to change owner_id during agent update denied for agent_id: {}. Original owner: {}, Attempted: {}", payload.agent_id, existing_agent.owner_id, payload.owner_id);
            return (
                StatusCode::FORBIDDEN,
                Json(json!({
                    "success": false,
                    "error": "Cannot change agent owner_id via update. Use transfer operation."
                })),
            );
        }
        log::info!("Authorized change of owner_id for agent {} by {} (is_localhost: {}).", payload.agent_id, recovered.as_ref().map_or_else(|| "localhost".to_string(), |r|r.as_hex()), is_localhost);
    }

    // Create and apply the agent update
    let mut agent_to_update = payload; // payload is already AIAgent
    // Ensure updated_at is set if your AIAgent struct has it and it's not auto-managed by BFTReg or similar
    // agent_to_update.updated_at = SystemTime::now().duration_since(UNIX_EPOCH).map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?.as_secs() as i64; 

    let op = datastore.agent_state.update_agent_local(agent_to_update.clone());
    if let Err(e) = datastore.handle_agent_op(op).await {
        log::error!("Failed to update agent {}: {}", agent_to_update.agent_id, e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({
                "success": false,
                "error": format!("Failed to update agent: {}", e)
            })),
        );
    }
            
    (StatusCode::OK, Json(json!({ "success": true, "agent": agent_to_update })))
}

/// Delete an agent
pub async fn delete_agent(
    State(state): State<Arc<Mutex<DataStore>>>,
    recovered: RecoveredAddress,
    Json(payload): Json<serde_json::Value>,
) -> impl IntoResponse {
    // Extract the agent ID from the payload
    let agent_id = match payload.get("id") {
        Some(id) => match id.as_str() {
            Some(s) => s.to_string(),
            None => {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(json!({
                        "error": "Invalid agent ID format"
                    })),
                );
            }
        },
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({
                    "error": "Agent ID is required"
                })),
            );
        }
    };
    
    let mut datastore = state.lock().await;
    
    // Get the address of the authenticated user
    let user_address = recovered.as_hex();
    
    // Check if this is a request from an admin node with an original user address
    let effective_address = if datastore.network_state.is_admin_address(&user_address.clone()) {
        // If it's an admin node, extract the original user address from the payload
        crate::auth::extract_original_user_address(&payload)
            .unwrap_or_else(|| user_address.clone())
    } else {
        // If it's a regular user, use their address
        user_address.clone()
    };
    
    // Check if the agent exists
    let existing_agent = datastore.agent_state.get_agent(&agent_id);
    if let Some(existing_agent) = existing_agent {
        // Verify ownership unless the request is from an admin
        if existing_agent.owner_id.to_lowercase() != effective_address.to_lowercase() && 
           !datastore.network_state.is_admin_address(&user_address) {
            return (
                StatusCode::FORBIDDEN,
                Json(json!({
                    "error": "You don't have permission to delete this agent"
                })),
            );
        }
        
        // Delete the agent
        let op = datastore.agent_state.remove_agent_local(agent_id.clone());
        if let Err(e) = datastore.handle_agent_op(op).await {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "error": format!("Failed to delete agent: {}", e)
                })),
            );
        }
        
        (
            StatusCode::OK,
            Json(json!({
                "status": "success",
                "message": "Agent deleted successfully"
            })),
        )
    } else {
        (
            StatusCode::NOT_FOUND,
            Json(json!({
                "error": "Agent not found"
            })),
        )
    }
}

pub async fn get_agent(
    State(state): State<Arc<Mutex<DataStore>>>,
    recovered: Option<RecoveredAddress>,
    ConnectInfo(connection_info): ConnectInfo<SocketAddr>,
    Path(agent_id): Path<String>
) -> impl IntoResponse {
    log::info!("get_agent: Request from: {} for agent_id: {}", connection_info.to_string(), agent_id);

    let remote_addr = connection_info.to_string();
    let is_localhost = remote_addr.starts_with("127.0.0.1") || remote_addr.starts_with("::1");

    let datastore = state.lock().await;

    // Check if the agent exists first
    let agent = match datastore.agent_state.get_agent(&agent_id) {
        Some(agent_data) => agent_data,
        None => {
            log::warn!("get_agent: Agent with id {} not found.", agent_id);
            return (
                StatusCode::NOT_FOUND,
                Json(json!({
                    "success": false,
                    "error": format!("Agent with id {} does not exist", agent_id)
                }))
            );
        }
    };

    // Authorization logic
    let mut authorized = false;

    if is_localhost {
        log::info!("get_agent: Request from localhost. Access granted for agent_id: {}.", agent_id);
        authorized = true;
    } else if let Some(auth_data) = recovered {
        let authenticated_address = auth_data.as_hex();
        log::info!("get_agent: Authenticated as {}. Checking ownership/admin for agent_id: {}.", authenticated_address, agent_id);
        if agent.owner_id.strip_prefix("0x").unwrap_or(&agent.owner_id).to_lowercase() == authenticated_address.to_lowercase() {
            log::info!("get_agent: User {} owns agent {}. Access granted.", authenticated_address, agent_id);
            authorized = true;
        } else if datastore.network_state.is_admin_address(&authenticated_address) {
            log::info!("get_agent: Admin user {} accessing agent {}. Access granted.", authenticated_address, agent_id);
            authorized = true;
        } else if !agent.is_private {
            log::info!("get_agent: Agent {} is public. Access granted to {}.", agent_id, authenticated_address);
            authorized = true;
        }
    } else {
        // Not localhost and no recovered address. Only allow if agent is public.
        if !agent.is_private {
            log::info!("get_agent: Agent {} is public. Access granted to unauthenticated non-localhost.", agent_id);
            authorized = true;
        } else {
            log::warn!("get_agent: Unauthenticated non-localhost attempt to access private agent: {}. Denying access.", agent_id);
        }
    }

    if authorized {
        log::info!("get_agent: Access to agent {} authorized.", agent_id);
        return (
            StatusCode::OK,
            Json(json!({
                "success": true,
                "agent": agent
            }))
        );
    } else {
        log::warn!("get_agent: Access to agent {} denied.", agent_id);
        return (
            StatusCode::FORBIDDEN,
            Json(json!({
                "success": false,
                "error": "You don't have permission to access this agent"
            }))
        );
    }
}

// Helper function to determine if an address belongs to an admin
// This would ideally be replaced with a proper role-based system
fn is_admin_address(address: &str) -> bool {
    // For now, we'll use a simple check for a specific address pattern
    // In a real system, this would query against a database or use JWT claims
    address.to_lowercase() == "0xadmin" || address.starts_with("0x000admin")
}

pub async fn list_agents(
    State(state): State<Arc<Mutex<DataStore>>>,
    recovered: Option<RecoveredAddress>,
    ConnectInfo(connection_info): ConnectInfo<SocketAddr>,
) -> impl IntoResponse {
    log::info!("list_agents: Request from: {}", connection_info.to_string());

    let remote_addr = connection_info.to_string();
    let is_localhost = remote_addr.starts_with("127.0.0.1") || remote_addr.starts_with("::1");

    let datastore = state.lock().await;
    let all_agents = datastore.agent_state.list_agents();

    let filtered_agents: Vec<AIAgent> = all_agents
        .into_iter()
        .filter_map(|(_agent_id, agent)| {
            if is_localhost {
                log::debug!("list_agents: Localhost access, allowing agent: {}", agent.agent_id);
                return Some(agent);
            }
            if let Some(auth_data) = &recovered {
                let authenticated_address = auth_data.as_hex();
                if datastore.network_state.is_admin_address(&authenticated_address) {
                    log::debug!("list_agents: Admin access ({}), allowing agent: {}", authenticated_address, agent.agent_id);
                    return Some(agent);
                }
                if agent.is_private && agent.owner_id.strip_prefix("0x").unwrap_or(&agent.owner_id).to_lowercase() == authenticated_address.to_lowercase() {
                    log::debug!("list_agents: Owner access ({}), allowing private agent: {}", authenticated_address, agent.agent_id);
                    return Some(agent);
                }
            }
            if !agent.is_private {
                log::debug!("list_agents: Agent {} is public, allowing access.", agent.agent_id);
                return Some(agent);
            }
            log::debug!("list_agents: Agent {} is private, access denied or no matching auth criteria.", agent.agent_id);
            None
        })
        .collect();
    
    log::info!("list_agents: Returning {} agents after filtering.", filtered_agents.len());
    return (
        StatusCode::OK,
        Json(json!({
            "success": true,
            "count": filtered_agents.len(),
            "agents": filtered_agents
        }))
    );
}

/// Handler for hiring an agent
pub async fn agent_hire(
    State(state): State<Arc<Mutex<DataStore>>>,
    recovered: RecoveredAddress,  // Use RecoveredAddress from ECDSA auth
    Path(agent_id): Path<String>,
    Json(payload): Json<serde_json::Value>,
) -> impl IntoResponse {
    log::info!("User {} is attempting to hire agent {}", recovered.as_hex(), agent_id);
    
    let mut datastore = state.lock().await;
    
    // Check if the agent exists
    if let Some(agent) = datastore.agent_state.get_agent(&agent_id) {
        // Get or create an account using the recovered address
        let account_address = recovered.as_hex();
        let mut account = match datastore.account_state.get_account(&account_address) {
            Some(acc) => acc,
            None => {
                // Create a new account if it doesn't exist
                let new_account = crate::accounts::Account::new(account_address);
                let op = datastore.account_state.update_account_local(new_account.clone());
                if let Err(_) = datastore.handle_account_op(op).await {
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(json!({
                            "success": false,
                            "error": "Failed to create new account"
                        }))
                    );
                }
                new_account
            }
        };
        
        // Add the agent to the hired agents
        account.hire_agent(agent_id.clone());
        
        // Update the account
        let op = datastore.account_state.update_account_local(account.clone());
        if let Err(err) = datastore.handle_account_op(op).await {
            log::error!("Failed to update account after hiring agent: {}", err);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "success": false,
                    "error": "Failed to update account after hiring agent"
                }))
            );
        }
        
        // Return success
        return (
            StatusCode::OK,
            Json(json!({
                "success": true,
                "message": format!("Successfully hired agent {}", agent_id),
                "agent": {
                    "id": agent_id,
                    "name": agent.name,
                    "description": agent.description
                },
                "credits_remaining": account.available_credits(),
                "hired_agent_count": account.hired_agent_count()
            }))
        );
    } else {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({
                "success": false,
                "error": format!("Agent {} not found", agent_id)
            }))
        );
    }
}

pub async fn create_agent_without_connect_info(
    State(state): State<Arc<Mutex<DataStore>>>,
    recovered: Option<RecoveredAddress>,
    Json(payload): Json<AIAgent>,
) -> impl IntoResponse {
    let mut datastore = state.lock().await;
    
    // Determine the effective user address
    let effective_address = if let Some(recovered_addr) = recovered {
        // For normal requests, use the recovered address
        recovered_addr.as_hex()
    } else {
        // If no recovered address (which should not happen unless auth was bypassed),
        // use the owner_id from the payload (assuming it's coming from localhost)
        payload.owner_id.clone()
    };
    
    // Create and apply the agent update
    let op = datastore.agent_state.update_agent_local(payload.clone());
    if let Err(e) = datastore.handle_agent_op(op).await {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({
                "error": format!("Failed to create agent: {}", e)
            })),
        );
    }
            
    (
        StatusCode::CREATED,
        Json(json!({
            "status": "success",
            "agent": payload 
        })),
    )
}
