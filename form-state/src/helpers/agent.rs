use crate::agent::*;
use crate::auth::{AuthenticationContext, RecoveredAddress};
use crate::datastore::DataStore;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::{
    extract::{ConnectInfo, Path, State},
    Json,
};
use serde_json::json;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Create agent with admin bypass logic
///
/// Sub-sub-task 3.5.4.1: Add admin bypass logic to agent/model handlers
/// Sub-sub-task 3.5.4.2: Implement admin context extraction for agent/model operations
pub async fn create_agent(
    State(state): State<Arc<Mutex<DataStore>>>,
    auth_context: AuthenticationContext,
    Json(payload): Json<AIAgent>,
) -> impl IntoResponse {
    log::info!("create_agent: Request from {}", auth_context.description());
    let mut datastore = state.lock().await;

    // Admin bypass: Admins can create agents for any owner
    if auth_context.is_admin() {
        log::info!(
            "create_agent: Admin {} creating agent for owner {}",
            auth_context.description(),
            payload.owner_id
        );

        let mut agent = payload.clone();
        agent.owner_id = payload.owner_id.clone(); // Use the owner_id from payload for admin requests

        let op = datastore.agent_state.update_agent_local(agent.clone());
        if let Err(e) = datastore.handle_agent_op(op).await {
            log::error!("create_agent: Failed to create agent: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "error": format!("Failed to create agent: {}", e),
                    "access_level": "admin"
                })),
            );
        }

        log::info!("create_agent: Agent created successfully by admin");
        return (
            StatusCode::CREATED,
            Json(json!({
                "status": "success",
                "agent": agent,
                "access_level": "admin",
                "created_by": auth_context.description()
            })),
        );
    }

    // Normal user access: can only create agents with their own address as owner
    match auth_context {
        AuthenticationContext::User { address, .. } => {
            let authenticated_address = address.as_hex();

            let mut agent = payload.clone();
            agent.owner_id = authenticated_address.clone(); // Force owner to be the authenticated user

            log::info!(
                "create_agent: Creating agent with owner ID: {}",
                authenticated_address
            );

            let op = datastore.agent_state.update_agent_local(agent.clone());
            if let Err(e) = datastore.handle_agent_op(op).await {
                log::error!("create_agent: Failed to create agent: {}", e);
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({
                        "error": format!("Failed to create agent: {}", e),
                        "access_level": "user"
                    })),
                );
            }

            log::info!("create_agent: Agent created successfully");
            (
                StatusCode::CREATED,
                Json(json!({
                    "status": "success",
                    "agent": agent,
                    "access_level": "user"
                })),
            )
        }
        _ => {
            // This shouldn't happen since middleware should handle authentication
            log::error!(
                "create_agent: Unexpected authentication context: {:?}",
                auth_context
            );
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "error": "Internal authentication error"
                })),
            )
        }
    }
}

/// Update agent with admin bypass logic
///
/// Sub-sub-task 3.5.4.1: Add admin bypass logic to agent/model handlers
/// Sub-sub-task 3.5.4.2: Implement admin context extraction for agent/model operations
pub async fn update_agent(
    State(state): State<Arc<Mutex<DataStore>>>,
    auth_context: AuthenticationContext,
    Json(payload): Json<AIAgent>,
) -> impl IntoResponse {
    log::info!(
        "update_agent: Request from {} for agent_id: {}",
        auth_context.description(),
        payload.agent_id
    );
    let mut datastore = state.lock().await;

    let existing_agent = match datastore.agent_state.get_agent(&payload.agent_id) {
        Some(agent) => agent,
        None => {
            log::warn!(
                "update_agent: Agent_id {} not found for update.",
                payload.agent_id
            );
            return (
                StatusCode::NOT_FOUND,
                Json(json!({
                    "success": false,
                    "error": "Agent not found"
                })),
            );
        }
    };

    // Admin bypass: Admins can update any agent
    if auth_context.is_admin() {
        log::info!(
            "update_agent: Admin {} updating agent {}",
            auth_context.description(),
            payload.agent_id
        );

        let agent_to_update = payload;
        let op = datastore
            .agent_state
            .update_agent_local(agent_to_update.clone());
        if let Err(e) = datastore.handle_agent_op(op).await {
            log::error!(
                "update_agent: Failed to update agent {}: {}",
                agent_to_update.agent_id,
                e
            );
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "success": false,
                    "error": format!("Failed to update agent: {}", e),
                    "access_level": "admin"
                })),
            );
        }

        log::info!(
            "update_agent: Agent {} updated successfully by admin.",
            agent_to_update.agent_id
        );
        return (
            StatusCode::OK,
            Json(json!({
                "success": true,
                "agent": agent_to_update,
                "access_level": "admin",
                "updated_by": auth_context.description()
            })),
        );
    }

    // Normal user access: can only update agents they own
    match auth_context {
        AuthenticationContext::User { address, .. } => {
            let authenticated_address = address.as_hex();
            let normalized_auth_address = authenticated_address.to_lowercase();
            let normalized_agent_owner = existing_agent
                .owner_id
                .strip_prefix("0x")
                .unwrap_or(&existing_agent.owner_id)
                .to_lowercase();

            if normalized_auth_address != normalized_agent_owner {
                log::warn!(
                    "update_agent: User {} does not own agent {} and is not admin. Update denied.",
                    authenticated_address,
                    payload.agent_id
                );
                return (
                    StatusCode::FORBIDDEN,
                    Json(json!({
                        "success": false,
                        "error": "You don't have permission to update this agent",
                        "authenticated_as": authenticated_address,
                        "agent_owner": existing_agent.owner_id,
                        "access_level": "user"
                    })),
                );
            }

            // Prevent changing agent owner for normal users
            let normalized_payload_owner = payload
                .owner_id
                .strip_prefix("0x")
                .unwrap_or(&payload.owner_id)
                .to_lowercase();
            if normalized_payload_owner != normalized_agent_owner {
                log::warn!(
                    "update_agent: Attempt to change agent owner for {} from {} to {} by non-admin user {}. Denied.", 
                    payload.agent_id, existing_agent.owner_id, payload.owner_id, authenticated_address
                );
                return (
                    StatusCode::FORBIDDEN,
                    Json(json!({
                        "success": false,
                        "error": "Cannot change agent owner via this update. Use dedicated ownership transfer.",
                        "access_level": "user"
                    })),
                );
            }

            log::info!(
                "update_agent: User {} owns agent {}. Access granted.",
                authenticated_address,
                payload.agent_id
            );

            let agent_to_update = payload;
            let op = datastore
                .agent_state
                .update_agent_local(agent_to_update.clone());
            if let Err(e) = datastore.handle_agent_op(op).await {
                log::error!(
                    "update_agent: Failed to update agent {}: {}",
                    agent_to_update.agent_id,
                    e
                );
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({
                        "success": false,
                        "error": format!("Failed to update agent: {}", e),
                        "access_level": "user"
                    })),
                );
            }

            log::info!(
                "update_agent: Agent {} updated successfully.",
                agent_to_update.agent_id
            );
            (
                StatusCode::OK,
                Json(json!({
                    "success": true,
                    "agent": agent_to_update,
                    "access_level": "user"
                })),
            )
        }
        _ => {
            // This shouldn't happen since middleware should handle authentication
            log::error!(
                "update_agent: Unexpected authentication context: {:?}",
                auth_context
            );
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "success": false,
                    "error": "Internal authentication error"
                })),
            )
        }
    }
}

/// Delete agent with admin bypass logic
///
/// Sub-sub-task 3.5.4.1: Add admin bypass logic to agent/model handlers
/// Sub-sub-task 3.5.4.2: Implement admin context extraction for agent/model operations
pub async fn delete_agent(
    State(state): State<Arc<Mutex<DataStore>>>,
    auth_context: AuthenticationContext,
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

    log::info!(
        "delete_agent: {} requesting deletion of agent {}",
        auth_context.description(),
        agent_id
    );
    let mut datastore = state.lock().await;

    // Check if the agent exists
    let existing_agent = datastore.agent_state.get_agent(&agent_id);
    if let Some(existing_agent) = existing_agent {
        // Admin bypass: Admins can delete any agent
        if auth_context.is_admin() {
            log::info!(
                "delete_agent: Admin {} deleting agent {}",
                auth_context.description(),
                agent_id
            );

            // Create a copy for the response
            let agent_copy = existing_agent.clone();

            // Delete the agent
            let op = datastore.agent_state.remove_agent_local(agent_id.clone());
            if let Err(e) = datastore.handle_agent_op(op).await {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({
                        "error": format!("Failed to delete agent: {}", e),
                        "access_level": "admin"
                    })),
                );
            }

            return (
                StatusCode::OK,
                Json(json!({
                    "status": "success",
                    "message": format!("Agent deleted successfully by admin {}", auth_context.description()),
                    "agent": agent_copy,
                    "access_level": "admin",
                    "deleted_by": auth_context.description()
                })),
            );
        }

        // Normal user access: can only delete agents they own
        match auth_context {
            AuthenticationContext::User { address, .. } => {
                let authenticated_address = address.as_hex();

                // Verify ownership
                if existing_agent.owner_id.to_lowercase() != authenticated_address.to_lowercase() {
                    log::warn!("delete_agent: User {} does not own agent {} and is not admin. Delete denied.", 
                             authenticated_address, agent_id);
                    return (
                        StatusCode::FORBIDDEN,
                        Json(json!({
                            "error": "You don't have permission to delete this agent",
                            "authenticated_as": authenticated_address,
                            "agent_owner": existing_agent.owner_id,
                            "access_level": "user"
                        })),
                    );
                }

                log::info!(
                    "delete_agent: User {} owns agent {}. Delete granted.",
                    authenticated_address,
                    agent_id
                );

                // Create a copy for the response
                let agent_copy = existing_agent.clone();

                // Delete the agent
                let op = datastore.agent_state.remove_agent_local(agent_id.clone());
                if let Err(e) = datastore.handle_agent_op(op).await {
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(json!({
                            "error": format!("Failed to delete agent: {}", e),
                            "access_level": "user"
                        })),
                    );
                }

                (
                    StatusCode::OK,
                    Json(json!({
                        "status": "success",
                        "message": "Agent deleted successfully",
                        "agent": agent_copy,
                        "access_level": "user"
                    })),
                )
            }
            _ => {
                // This shouldn't happen since middleware should handle authentication
                log::error!(
                    "delete_agent: Unexpected authentication context: {:?}",
                    auth_context
                );
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({
                        "error": "Internal authentication error"
                    })),
                )
            }
        }
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
    Path(agent_id): Path<String>,
) -> impl IntoResponse {
    log::info!(
        "get_agent: Request from: {} for agent_id: {}",
        connection_info.to_string(),
        agent_id
    );

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
                })),
            );
        }
    };

    // Authorization logic
    let mut authorized = false;

    if is_localhost {
        log::info!(
            "get_agent: Request from localhost. Access granted for agent_id: {}.",
            agent_id
        );
        authorized = true;
    } else if let Some(auth_data) = recovered {
        let authenticated_address = auth_data.as_hex();
        log::info!(
            "get_agent: Authenticated as {}. Checking ownership/admin for agent_id: {}.",
            authenticated_address,
            agent_id
        );
        if agent
            .owner_id
            .strip_prefix("0x")
            .unwrap_or(&agent.owner_id)
            .to_lowercase()
            == authenticated_address.to_lowercase()
        {
            log::info!(
                "get_agent: User {} owns agent {}. Access granted.",
                authenticated_address,
                agent_id
            );
            authorized = true;
        } else if datastore
            .network_state
            .is_admin_address(&authenticated_address)
        {
            log::info!(
                "get_agent: Admin user {} accessing agent {}. Access granted.",
                authenticated_address,
                agent_id
            );
            authorized = true;
        } else if !agent.is_private {
            log::info!(
                "get_agent: Agent {} is public. Access granted to {}.",
                agent_id,
                authenticated_address
            );
            authorized = true;
        }
    } else {
        // Not localhost and no recovered address. Only allow if agent is public.
        if !agent.is_private {
            log::info!(
                "get_agent: Agent {} is public. Access granted to unauthenticated non-localhost.",
                agent_id
            );
            authorized = true;
        } else {
            log::warn!("get_agent: Unauthenticated non-localhost attempt to access private agent: {}. Denying access.", agent_id);
        }
    }

    if authorized {
        log::info!("get_agent: Access to agent {} authorized.", agent_id);
        (
            StatusCode::OK,
            Json(json!({
                "success": true,
                "agent": agent
            })),
        )
    } else {
        log::warn!("get_agent: Access to agent {} denied.", agent_id);
        (
            StatusCode::FORBIDDEN,
            Json(json!({
                "success": false,
                "error": "You don't have permission to access this agent"
            })),
        )
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
                log::debug!(
                    "list_agents: Localhost access, allowing agent: {}",
                    agent.agent_id
                );
                return Some(agent);
            }
            if let Some(auth_data) = &recovered {
                let authenticated_address = auth_data.as_hex();
                if datastore
                    .network_state
                    .is_admin_address(&authenticated_address)
                {
                    log::debug!(
                        "list_agents: Admin access ({}), allowing agent: {}",
                        authenticated_address,
                        agent.agent_id
                    );
                    return Some(agent);
                }
                if agent.is_private
                    && agent
                        .owner_id
                        .strip_prefix("0x")
                        .unwrap_or(&agent.owner_id)
                        .to_lowercase()
                        == authenticated_address.to_lowercase()
                {
                    log::debug!(
                        "list_agents: Owner access ({}), allowing private agent: {}",
                        authenticated_address,
                        agent.agent_id
                    );
                    return Some(agent);
                }
            }
            if !agent.is_private {
                log::debug!(
                    "list_agents: Agent {} is public, allowing access.",
                    agent.agent_id
                );
                return Some(agent);
            }
            log::debug!(
                "list_agents: Agent {} is private, access denied or no matching auth criteria.",
                agent.agent_id
            );
            None
        })
        .collect();

    log::info!(
        "list_agents: Returning {} agents after filtering.",
        filtered_agents.len()
    );
    (
        StatusCode::OK,
        Json(json!({
            "success": true,
            "count": filtered_agents.len(),
            "agents": filtered_agents
        })),
    )
}

/// Handler for hiring an agent
pub async fn agent_hire(
    State(state): State<Arc<Mutex<DataStore>>>,
    recovered: RecoveredAddress, // Use RecoveredAddress from ECDSA auth
    Path(agent_id): Path<String>,
    Json(payload): Json<serde_json::Value>,
) -> impl IntoResponse {
    log::info!(
        "User {} is attempting to hire agent {}",
        recovered.as_hex(),
        agent_id
    );

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
                let op = datastore
                    .account_state
                    .update_account_local(new_account.clone());
                if let Err(_) = datastore.handle_account_op(op).await {
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(json!({
                            "success": false,
                            "error": "Failed to create new account"
                        })),
                    );
                }
                new_account
            }
        };

        // Add the agent to the hired agents
        account.hire_agent(agent_id.clone());

        // Update the account
        let op = datastore
            .account_state
            .update_account_local(account.clone());
        if let Err(err) = datastore.handle_account_op(op).await {
            log::error!("Failed to update account after hiring agent: {}", err);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "success": false,
                    "error": "Failed to update account after hiring agent"
                })),
            );
        }

        // Return success
        (
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
            })),
        )
    } else {
        (
            StatusCode::NOT_FOUND,
            Json(json!({
                "success": false,
                "error": format!("Agent {} not found", agent_id)
            })),
        )
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
