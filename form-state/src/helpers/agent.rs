use crate::datastore::{AccountRequest, AgentRequest, DataStore, DB_HANDLE};
use crate::db::write_datastore;
use crate::agent::*;
use crate::auth::RecoveredAddress;
use std::sync::Arc;
use tokio::sync::Mutex;
use axum::{extract::{State, Path}, Json};
use form_types::state::{Response, Success};
use axum::http::StatusCode;
use serde_json::json;
use axum::response::IntoResponse;

pub async fn create_agent(
    State(state): State<Arc<Mutex<DataStore>>>,
    recovered: RecoveredAddress,
    Json(payload): Json<serde_json::Value>,
) -> impl IntoResponse {
    let mut datastore = state.lock().await;
    
    // Get the address of the authenticated user
    let user_address = recovered.as_hex();
    
    // Check if this is a request from an admin node with an original user address
    let effective_address = if datastore.network_state.is_admin_address(&user_address) {
        // If it's an admin node, extract the original user address from the payload
        crate::auth::extract_original_user_address(&payload)
            .unwrap_or_else(|| user_address.clone())
    } else {
        // If it's a regular user, use their address
        user_address
    };
    
    // Parse the agent data from the payload
    let agent_data: Result<AIAgent, serde_json::Error> = serde_json::from_value(payload.clone());
    
    match agent_data {
        Ok(mut agent) => {
            // Ensure the agent has the correct owner set to the authenticated user
            agent.owner_id = effective_address.to_lowercase();
            
            // Create and apply the agent update
            let op = datastore.agent_state.update_agent_local(agent.clone());
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
                    "agent": agent
                })),
            )
        },
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(json!({
                "error": format!("Invalid agent data: {}", e)
            })),
        ),
    }
}

pub async fn update_agent(
    State(state): State<Arc<Mutex<DataStore>>>,
    recovered: RecoveredAddress,
    Json(payload): Json<serde_json::Value>,
) -> impl IntoResponse {
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
    
    // Parse the agent data from the payload
    let agent_data: Result<AIAgent, serde_json::Error> = serde_json::from_value(payload.clone());
    
    match agent_data {
        Ok(agent) => {
            // Check if the agent exists
            let existing_agent = datastore.agent_state.get_agent(&agent.agent_id);
            if let Some(existing_agent) = existing_agent {
                // Verify ownership unless the request is from an admin
                if existing_agent.owner_id.to_lowercase() != effective_address.to_lowercase() && 
                   !datastore.network_state.is_admin_address(&user_address) {
                    return (
                        StatusCode::FORBIDDEN,
                        Json(json!({
                            "error": "You don't have permission to update this agent"
                        })),
                    );
                }
                
                // Create and apply the agent update
                let op = datastore.agent_state.update_agent_local(agent.clone());
                if let Err(e) = datastore.handle_agent_op(op).await {
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(json!({
                            "error": format!("Failed to update agent: {}", e)
                        })),
                    );
                }
                
                (
                    StatusCode::OK,
                    Json(json!({
                        "status": "success",
                        "agent": agent
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
        },
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(json!({
                "error": format!("Invalid agent data: {}", e)
            })),
        ),
    }
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
    recovered: RecoveredAddress,
    Path(agent_id): Path<String>
) -> impl IntoResponse {
    log::info!("Received get agent request for {}", agent_id);
    
    // Get the authenticated user's address
    let authenticated_address = recovered.as_hex();
    
    let datastore = state.lock().await;
    
    // Check if the agent exists
    if let Some(agent) = datastore.agent_state.get_agent(&agent_id) {
        // If the agent is private, check authorization
        if agent.is_private {
            // Check if the user is the owner of the agent
            let account = datastore.account_state.get_account(&authenticated_address);
            
            match account {
                Some(account) => {
                    // Allow access if the user is the owner
                    let is_owner = account.owned_agents.contains(&agent_id);
                    
                    // Determine if user is admin by checking a special admin address list
                    // This is a temporary solution until proper role-based auth is implemented
                    let is_admin = is_admin_address(&authenticated_address);
                    
                    if !is_owner && !is_admin {
                        log::warn!("Unauthorized attempt to access private agent: {} by {}", agent_id, authenticated_address);
                        return (
                            StatusCode::FORBIDDEN,
                            Json(json!({
                                "success": false,
                                "error": "You don't have permission to access this private agent"
                            }))
                        );
                    }
                },
                None => {
                    return (
                        StatusCode::UNAUTHORIZED,
                        Json(json!({
                            "success": false,
                            "error": "Account not found"
                        }))
                    );
                }
            }
        }
        
        // Return the agent data
        return (
            StatusCode::OK,
            Json(json!({
                "success": true,
                "agent": agent
            }))
        );
    } else {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({
                "success": false,
                "error": format!("Agent with id {} does not exist", agent_id)
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

pub async fn list_agent(
    State(state): State<Arc<Mutex<DataStore>>>,
    recovered: RecoveredAddress
) -> impl IntoResponse {
    log::info!("Received list agents request");
    
    // Get the authenticated user's address
    let authenticated_address = recovered.as_hex();
    
    let datastore = state.lock().await;
    
    // Check if the user is an admin
    let is_admin = is_admin_address(&authenticated_address);
    
    // Get the account
    let account = datastore.account_state.get_account(&authenticated_address);
    
    // Get all agents from the datastore
    let all_agents = datastore.agent_state.list_agents();
    
    // Filter the agents based on authorization
    let filtered_agents: Vec<AIAgent> = all_agents
        .into_iter()
        .filter(|(_, agent)| {
            // Admins can see all agents
            if is_admin {
                return true;
            }
            
            // Public agents are visible to all authenticated users
            if !agent.is_private {
                return true;
            }
            
            // For private agents, check if the user is the owner
            if let Some(acc) = &account {
                if acc.owned_agents.contains(&agent.agent_id) {
                    return true;
                }
            }
            
            // Otherwise, the user can't see this agent
            false
        })
        .map(|(_, agent)| agent)
        .collect();
    
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
