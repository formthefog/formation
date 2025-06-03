use crate::auth::RecoveredAddress;
use crate::billing::UsageTracker;
use crate::datastore::DataStore;
use crate::model::{AIModel, ModelId};
use crate::nodes::NodeId;
use axum::{
    extract::{ConnectInfo, Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::net::SocketAddr;
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::Mutex;
use axum::{extract::{State, Path}, Json};
use serde::{Serialize, Deserialize};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use serde_json::json;
use crate::auth::{RecoveredAddress, AuthenticationContext};
use crate::model::AIModel;

// Helper function to determine if an address belongs to an admin
// This would ideally be replaced with a proper role-based system
fn is_admin_address(address: &str) -> bool {
    // For now, we'll use a simple check for a specific address pattern
    // In a real system, this would query against a database or use JWT claims
    address.to_lowercase() == "0xadmin" || address.starts_with("0x000admin")
}

fn is_localhost_address(address: &str) -> bool {
    address.starts_with("127.0.0.1") || address.starts_with("::1")
}

/// Create model with admin bypass logic
/// 
/// Sub-sub-task 3.5.4.1: Add admin bypass logic to agent/model handlers
/// Sub-sub-task 3.5.4.2: Implement admin context extraction for agent/model operations
pub async fn create_model(
    State(state): State<Arc<Mutex<DataStore>>>,
    auth_context: AuthenticationContext,
    Json(payload): Json<serde_json::Value>,
) -> impl IntoResponse {
    log::info!("create_model: Request from {}", auth_context.description());
    let mut datastore = state.lock().await;
    

    // Get the address of the authenticated user
    let user_address = match recovered {
        Some(recovered) => recovered.as_hex(),
        None => payload
            .get("owner_id")
            .and_then(|v| v.as_str())
            .unwrap_or("0x0000000000000000000000000000000000000000")
            .to_string(),
    };

    // Check if this is a request from an admin node with an original user address
    let effective_address = if datastore.network_state.is_admin_address(&user_address) {
        // If it's an admin node, extract the original user address from the payload
        crate::auth::extract_original_user_address(&payload).unwrap_or_else(|| user_address.clone())
    } else {
        // If it's a regular user, use their address
        user_address
    };

    // Parse the model data from the payload
    let model_data: Result<AIModel, serde_json::Error> = serde_json::from_value(payload.clone());

    match model_data {
        Ok(mut model) => {
            // Admin bypass: Admins can create models for any owner
            if auth_context.is_admin() {
                log::info!("create_model: Admin {} creating model for owner {}", 
                           auth_context.description(), model.owner_id);
                
                // Use the owner_id from payload for admin requests
                model.owner_id = model.owner_id.clone();
                
                let op = datastore.model_state.update_model_local(model.clone());
                if let Err(e) = datastore.handle_model_op(op).await {
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(json!({
                            "error": format!("Failed to create model: {}", e),
                            "access_level": "admin"
                        })),
                    );
                }
                
                return (
                    StatusCode::CREATED,
                    Json(json!({
                        "status": "success",
                        "model": model,
                        "access_level": "admin",
                        "created_by": auth_context.description()
                    })),
                );
            }

            // Normal user access: can only create models with their own address as owner
            match auth_context {
                AuthenticationContext::User { address, .. } => {
                    let authenticated_address = address.as_hex();
                    
                    // Force owner to be the authenticated user
                    model.owner_id = authenticated_address.to_lowercase();
                    
                    log::info!("create_model: Creating model with owner ID: {}", authenticated_address);
                    
                    let op = datastore.model_state.update_model_local(model.clone());
                    if let Err(e) = datastore.handle_model_op(op).await {
                        return (
                            StatusCode::INTERNAL_SERVER_ERROR,
                            Json(json!({
                                "error": format!("Failed to create model: {}", e),
                                "access_level": "user"
                            })),
                        );
                    }
                    
                    return (
                        StatusCode::CREATED,
                        Json(json!({
                            "status": "success",
                            "model": model,
                            "access_level": "user"
                        })),
                    );
                }
                _ => {
                    // This shouldn't happen since middleware should handle authentication
                    log::error!("create_model: Unexpected authentication context: {:?}", auth_context);
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(json!({
                            "error": "Internal authentication error"
                        })),
                    );
                }
            }
        },
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(json!({
                "error": format!("Invalid model data: {}", e)
            })),
        ),
    }
}

/// Update model with admin bypass logic
/// 
/// Sub-sub-task 3.5.4.1: Add admin bypass logic to agent/model handlers
/// Sub-sub-task 3.5.4.2: Implement admin context extraction for agent/model operations
pub async fn update_model(
    State(state): State<Arc<Mutex<DataStore>>>,
    auth_context: AuthenticationContext,
    Json(payload): Json<serde_json::Value>,
) -> impl IntoResponse {
    log::info!("update_model: Request from {}", auth_context.description());
    let mut datastore = state.lock().await;

    // Get the address of the authenticated user
    let user_address = match recovered {
        Some(recovered) => recovered.as_hex(),
        None => payload
            .get("owner_id")
            .and_then(|v| v.as_str())
            .unwrap_or("0x0000000000000000000000000000000000000000")
            .to_string(),
    };

    // Check if this is a request from an admin node with an original user address
    let effective_address = if datastore.network_state.is_admin_address(&user_address) {
        // If it's an admin node, extract the original user address from the payload
        crate::auth::extract_original_user_address(&payload).unwrap_or_else(|| user_address.clone())
    } else {
        // If it's a regular user, use their address
        user_address
    };

    // Parse the model data from the payload
    let model_data: Result<AIModel, serde_json::Error> = serde_json::from_value(payload.clone());

    match model_data {
        Ok(model) => {
            // Check if the model exists
            let existing_model = datastore.model_state.get_model(&model.model_id);
            if let Some(existing_model) = existing_model {
                // Admin bypass: Admins can update any model
                if auth_context.is_admin() {
                    log::info!("update_model: Admin {} updating model {}", 
                               auth_context.description(), model.model_id);
                    
                    let op = datastore.model_state.update_model_local(model.clone());
                    if let Err(e) = datastore.handle_model_op(op).await {
                        return (
                            StatusCode::INTERNAL_SERVER_ERROR,
                            Json(json!({
                                "error": format!("Failed to update model: {}", e),
                                "access_level": "admin"
                            })),
                        );
                    }
                    
                    return (
                        StatusCode::OK,
                        Json(json!({
                            "status": "success",
                            "model": model,
                            "access_level": "admin",
                            "updated_by": auth_context.description()
                        })),
                    );
                }

                // Normal user access: can only update models they own
                match auth_context {
                    AuthenticationContext::User { address, .. } => {
                        let authenticated_address = address.as_hex();
                        
                        // Verify ownership
                        if existing_model.owner_id.to_lowercase() != authenticated_address.to_lowercase() {
                            log::warn!("update_model: User {} does not own model {} and is not admin. Update denied.", 
                                     authenticated_address, model.model_id);
                            return (
                                StatusCode::FORBIDDEN,
                                Json(json!({
                                    "error": "You don't have permission to update this model",
                                    "authenticated_as": authenticated_address,
                                    "model_owner": existing_model.owner_id,
                                    "access_level": "user"
                                })),
                            );
                        }
                        
                        log::info!("update_model: User {} owns model {}. Update granted.", authenticated_address, model.model_id);
                        
                        let op = datastore.model_state.update_model_local(model.clone());
                        if let Err(e) = datastore.handle_model_op(op).await {
                            return (
                                StatusCode::INTERNAL_SERVER_ERROR,
                                Json(json!({
                                    "error": format!("Failed to update model: {}", e),
                                    "access_level": "user"
                                })),
                            );
                        }
                        
                        return (
                            StatusCode::OK,
                            Json(json!({
                                "status": "success",
                                "model": model,
                                "access_level": "user"
                            })),
                        );
                    }
                    _ => {
                        // This shouldn't happen since middleware should handle authentication
                        log::error!("update_model: Unexpected authentication context: {:?}", auth_context);
                        return (
                            StatusCode::INTERNAL_SERVER_ERROR,
                            Json(json!({
                                "error": "Internal authentication error"
                            })),
                        );
                    }
                }
            } else {
                (
                    StatusCode::NOT_FOUND,
                    Json(json!({
                        "error": "Model not found"
                    })),
                )
            }
        }
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(json!({
                "error": format!("Invalid model data: {}", e)
            })),
        ),
    }
}

pub async fn add_node_to_model(
    State(state): State<Arc<Mutex<DataStore>>>,
    recovered: Option<RecoveredAddress>,
    Path(model_id): Path<String>,
    ConnectInfo(connection_info): ConnectInfo<SocketAddr>,
    Json(payload): Json<serde_json::Value>,
) -> impl IntoResponse {
    let mut datastore = state.lock().await;

    let remote_addr = connection_info.to_string();
    let is_localhost = remote_addr.starts_with("127.0.0.1") || remote_addr.starts_with("::1");
    log::info!("Is localhost: {}", is_localhost);

    // Get the address of the authenticated user
    let user_address = match recovered {
        Some(recovered) => recovered.as_hex(),
        None => payload
            .get("owner_id")
            .and_then(|v| v.as_str())
            .unwrap_or("0x0000000000000000000000000000000000000000")
            .to_string(),
    };

    // Check if this is a request from an admin node with an original user address
    let effective_address = if datastore.network_state.is_admin_address(&user_address) {
        // If it's an admin node, extract the original user address from the payload
        crate::auth::extract_original_user_address(&payload).unwrap_or_else(|| user_address.clone())
    } else {
        // If it's a regular user, use their address
        user_address
    };

    // Parse the model ID
    let model_id: ModelId = match model_id.try_into() {
        Ok(id) => id,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({
                    "error": "Invalid model ID format"
                })),
            );
        }
    };

    // Parse node ID and port from payload
    let (node_id, port) = match (
        payload.get("node_id").and_then(|v| v.as_str()),
        payload.get("port").and_then(|v| v.as_u64()),
    ) {
        (Some(node_id_str), Some(port_num)) => {
            match (node_id_str.try_into(), port_num.try_into()) {
                (Ok(node_id), Ok(port)) => (node_id, port),
                _ => {
                    return (
                        StatusCode::BAD_REQUEST,
                        Json(json!({
                            "error": "Invalid node ID or port format"
                        })),
                    );
                }
            }
        }
        _ => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({
                    "error": "Missing required fields: node_id and port"
                })),
            );
        }
    };

    // Check if the model exists and verify ownership
    if let Some(existing_model) = datastore.model_state.get_model(&model_id.to_string()) {
        // Verify ownership unless the request is from an admin
        if existing_model.owner_id.to_lowercase() != effective_address.to_lowercase()
            && !datastore.network_state.is_admin_address(&effective_address)
            && !is_localhost
        {
            return (
                StatusCode::FORBIDDEN,
                Json(json!({
                    "error": "You don't have permission to modify this model"
                })),
            );
        }

        // Create and apply the model update
        let op = datastore
            .model_state
            .add_node_to_model(model_id, node_id, port);
        match op {
            Some(op) => {
                if let Err(e) = datastore.handle_model_op(op).await {
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(json!({
                            "error": format!("Failed to add node to model: {}", e)
                        })),
                    );
                }

                (
                    StatusCode::OK,
                    Json(json!({
                        "status": "success",
                        "message": "Node added to model successfully"
                    })),
                )
            }
            None => {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(json!({
                        "error": "Failed to add node to model"
                    })),
                );
            }
        }
    } else {
        (
            StatusCode::NOT_FOUND,
            Json(json!({
                "error": "Model not found"
            })),
        )
    }
}

pub async fn remove_node_from_model(
    State(state): State<Arc<Mutex<DataStore>>>,
    recovered: Option<RecoveredAddress>,
    Path(model_id): Path<String>,
    ConnectInfo(connection_info): ConnectInfo<SocketAddr>,
    Json(payload): Json<serde_json::Value>,
) -> impl IntoResponse {
    let mut datastore = state.lock().await;

    let remote_addr = connection_info.to_string();
    let is_localhost = remote_addr.starts_with("127.0.0.1") || remote_addr.starts_with("::1");
    log::info!("Is localhost: {}", is_localhost);

    // Check if this is a request from an admin node with an original user address
    let effective_address = if is_localhost {
        // If it's an admin node, extract the original user address from the payload
        log::info!("Extracting original user address from payload");
        payload
        .get("owner_id")
        .and_then(|v| v.as_str())
        .unwrap_or("0x0000000000000000000000000000000000000000")
        .to_string()
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

    // Parse the model ID
    let model_id: ModelId = match model_id.try_into() {
        Ok(id) => id,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({
                    "error": "Invalid model ID format"
                })),
            );
        }
    };

    // get node from payload
    let node_id = match payload.get("node_id").and_then(|v| v.as_str()) {
        Some(node_id_str) => node_id_str.to_string(),
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({
                    "error": "Node ID is required"
                })),
            );
        }
    };

    // Check if the model exists and verify ownership
    if let Some(existing_model) = datastore.model_state.get_model(&model_id.to_string()) {
        // Verify ownership unless the request is from an admin
        if existing_model.owner_id.to_lowercase() != effective_address.to_lowercase()
            && !datastore.network_state.is_admin_address(&effective_address)
            && !is_localhost
        {
            return (
                StatusCode::FORBIDDEN,
                Json(json!({
                    "error": "You don't have permission to modify this model"
                })),
            );
        }


        // remove node from model
        let op = datastore
            .model_state
            .remove_node_from_model(model_id, &node_id);
        match op {
            Some(op) => {
                if let Err(e) = datastore.handle_model_op(op).await {
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(json!({
                            "error": format!("Failed to remove node from model: {}", e)
                        })),
                    );
                }

                (
                    StatusCode::OK,
                    Json(json!({
                        "status": "success",
                        "message": "Node removed from model successfully"
                    })),
                )
            }
            None => {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(json!({
                        "error": "Failed to remove node from model"
                    })),
                );
            }
        }
    } else {
        (
            StatusCode::NOT_FOUND,
            Json(json!({
                "error": "Model not found"
            })),
        )
    }

}

pub async fn get_nodes_for_model(
    State(state): State<Arc<Mutex<DataStore>>>,
    Path(model_id): Path<String>,
) -> impl IntoResponse {
    let datastore = state.lock().await;

    // get nodes from model
    let nodes = datastore.model_state.get_model_nodes(model_id);
    if let Some(nodes) = nodes {
        (
            StatusCode::OK,
            Json(json!({
                "nodes": nodes
            })),
        )
    } else {
        (
            StatusCode::NOT_FOUND,
            Json(json!({
                "error": "No nodes available for this model"
            })),
        )
    }
}

/// Delete model with admin bypass logic
/// 
/// Sub-sub-task 3.5.4.1: Add admin bypass logic to agent/model handlers
/// Sub-sub-task 3.5.4.2: Implement admin context extraction for agent/model operations
pub async fn delete_model(
    State(state): State<Arc<Mutex<DataStore>>>,
    auth_context: AuthenticationContext,
    Json(payload): Json<serde_json::Value>,
) -> impl IntoResponse {
    // Extract the model ID from the payload
    let model_id = match payload.get("id") {
        Some(id) => match id.as_str() {
            Some(s) => s.to_string(),
            None => {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(json!({
                        "error": "Invalid model ID format"
                    })),
                );
            }
        },
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({
                    "error": "Model ID is required"
                })),
            );
        }
    };
    
    log::info!("delete_model: {} requesting deletion of model {}", auth_context.description(), model_id);
    let mut datastore = state.lock().await;
    
    // Check if the model exists
    let existing_model = datastore.model_state.get_model(&model_id.clone());
    if let Some(existing_model) = existing_model {
        // Admin bypass: Admins can delete any model
        if auth_context.is_admin() {
            log::info!("delete_model: Admin {} deleting model {}", 
                       auth_context.description(), model_id);
            
            // Create a copy for the response
            let model_copy = existing_model.clone();
            
            // Delete the model
            let op = datastore.model_state.remove_model_local(model_id.clone());
            if let Err(e) = datastore.handle_model_op(op).await {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({
                        "error": format!("Failed to delete model: {}", e),
                        "access_level": "admin"
                    })),
                );
            }
            
            return (
                StatusCode::OK,
                Json(json!({
                    "status": "success",
                    "message": format!("Model deleted successfully by admin {}", auth_context.description()),
                    "model": model_copy,
                    "access_level": "admin",
                    "deleted_by": auth_context.description()
                })),
            );
        }

        // Normal user access: can only delete models they own
        match auth_context {
            AuthenticationContext::User { address, .. } => {
                let authenticated_address = address.as_hex();
                
                // Verify ownership
                if existing_model.owner_id.to_lowercase() != authenticated_address.to_lowercase() {
                    log::warn!("delete_model: User {} does not own model {} and is not admin. Delete denied.", 
                             authenticated_address, model_id);
                    return (
                        StatusCode::FORBIDDEN,
                        Json(json!({
                            "error": "You don't have permission to delete this model",
                            "authenticated_as": authenticated_address,
                            "model_owner": existing_model.owner_id,
                            "access_level": "user"
                        })),
                    );
                }
                
                log::info!("delete_model: User {} owns model {}. Delete granted.", authenticated_address, model_id);
                
                // Create a copy for the response
                let model_copy = existing_model.clone();
                
                // Delete the model
                let op = datastore.model_state.remove_model_local(model_id.clone());
                if let Err(e) = datastore.handle_model_op(op).await {
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(json!({
                            "error": format!("Failed to delete model: {}", e),
                            "access_level": "user"
                        })),
                    );
                }
                
                return (
                    StatusCode::OK,
                    Json(json!({
                        "status": "success",
                        "message": "Model deleted successfully",
                        "model": model_copy,
                        "access_level": "user"
                    })),
                );
            }
            _ => {
                // This shouldn't happen since middleware should handle authentication
                log::error!("delete_model: Unexpected authentication context: {:?}", auth_context);
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({
                        "error": "Internal authentication error"
                    })),
                );
            }
        }
    } else {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({
                "error": "Model not found"
            })),
        );
    }
}

/// Get information about a specific AI model
pub async fn get_model(
    State(state): State<Arc<Mutex<DataStore>>>,
    recovered: RecoveredAddress,
    Path(model_id): Path<String>,
) -> impl IntoResponse {
    log::info!(
        "User {} is requesting model {}",
        recovered.as_hex(),
        model_id
    );

    // Get the authenticated user's address
    let authenticated_address = recovered.as_hex();

    // Get the model from datastore
    let datastore = state.lock().await;

    match datastore.model_state.get_model(&model_id) {
        Some(model) => {
            // Check authorization if the model is private
            if model.is_private {
                // Check if user is the owner, has authorization, or is an admin
                let account = datastore.account_state.get_account(&authenticated_address);

                // Determine if the user has access to this model
                let is_authorized = match account {
                    Some(account) => {
                        // User is authorized if they own the model or are an admin
                        account.owned_models.contains(&model_id)
                            || is_admin_address(&authenticated_address)
                            || model.owner_id == authenticated_address
                    }
                    None => false,
                };

                if !is_authorized {
                    log::warn!(
                        "Unauthorized attempt to access private model: {} by {}",
                        model_id,
                        authenticated_address
                    );
                    return (
                        StatusCode::FORBIDDEN,
                        Json(json!({
                            "success": false,
                            "error": "You don't have permission to access this private model"
                        })),
                    );
                }
            }

            // Log access for auditing
            log::info!(
                "Model access: {} by account {}",
                model_id,
                recovered.as_hex()
            );

            // Return the model with 200 OK
            (
                StatusCode::OK,
                Json(json!({
                    "success": true,
                    "model": model
                })),
            )
        }
        None => {
            // Model not found
            (
                StatusCode::NOT_FOUND,
                Json(json!({
                    "success": false,
                    "error": format!("Model {} not found", model_id)
                })),
            )
        }
    }
}

/// List all available models
pub async fn list_model(
    State(state): State<Arc<Mutex<DataStore>>>,
    recovered: Option<RecoveredAddress>,
) -> impl IntoResponse {
    // Get the address of the authenticated user
    let authenticated_address = match recovered {
        Some(recovered) => recovered.as_hex(),
        None => "127.0.0.1".to_string(),
    };
    log::info!(
        "Account {} is requesting list of all models",
        authenticated_address
    );

    // Get all models from datastore
    let datastore = state.lock().await;

    // Check if the user is an admin
    let is_admin =
        is_localhost_address(&authenticated_address) || is_admin_address(&authenticated_address);

    // Get the account
    let account = datastore.account_state.get_account(&authenticated_address);

    // Get all models
    let all_models = datastore.model_state.list_models();

    // Filter the models based on authorization
    let filtered_models: Vec<_> = all_models
        .into_iter()
        .filter(|(model_id, model)| {
            // If model is not private, everyone can see it
            if !model.is_private {
                return true;
            }

            // Admins can see all models
            if is_admin {
                return true;
            }

            // Owner can see their own models
            if model.owner_id == authenticated_address {
                return true;
            }

            // Check if user has ownership in their account records
            if let Some(acc) = &account {
                if acc.owned_models.contains(model_id) {
                    return true;
                }
            }

            // Otherwise, the user can't see this model
            false
        })
        .map(|(_, model)| model)
        .collect();

    // Return the filtered models with 200 OK
    (
        StatusCode::OK,
        Json(json!({
            "success": true,
            "models": filtered_models,
            "total": filtered_models.len()
        })),
    )
}

/// Handler for model inference
pub async fn model_inference(
    State(state): State<Arc<Mutex<DataStore>>>,
    recovered: RecoveredAddress,
    Path(model_id): Path<String>,
    Json(payload): Json<ModelInferenceRequest>,
) -> impl IntoResponse {
    let account_address = recovered.as_hex();
    log::info!(
        "Account {} is requesting inference from model {}",
        account_address,
        model_id
    );

    let mut datastore = state.lock().await;

    // Check if the model exists
    if let Some(model) = datastore.model_state.get_model(&model_id) {
        // Check authorization if the model is private
        if model.is_private {
            // Check if user is the owner, has authorization, or is an admin
            let account = datastore.account_state.get_account(&account_address);

            // Determine if the user has access to this model
            let is_authorized = match &account {
                Some(account) => {
                    // User is authorized if they own the model or are an admin
                    account.owned_models.contains(&model_id)
                        || is_admin_address(&account_address)
                        || model.owner_id == account_address
                }
                None => false,
            };

            if !is_authorized {
                log::warn!(
                    "Unauthorized attempt to use private model for inference: {} by {}",
                    model_id,
                    account_address
                );
                return (
                    StatusCode::FORBIDDEN,
                    Json(json!({
                        "success": false,
                        "error": "You don't have permission to use this private model for inference"
                    })),
                );
            }
        }

        // Get or create the user's account
        let mut account = match datastore.account_state.get_account(&account_address) {
            Some(acc) => acc,
            None => {
                // Create a new account if it doesn't exist
                let new_account = crate::accounts::Account::new(account_address.clone());
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

        // Calculate token usage
        let input_tokens = payload.input_tokens.unwrap_or(0);
        let output_tokens = payload.output_tokens.unwrap_or(0);
        let total_tokens = input_tokens + output_tokens;

        // Check if the account has enough credits for this operation
        use crate::billing::middleware::{check_operation_credits, OperationType};
        let operation = OperationType::TokenConsumption {
            model_id: model_id.clone(),
            input_tokens,
            output_tokens,
        };

        // Validate eligibility
        if let Err(err) = check_operation_credits(&account, operation) {
            let error_message = format!("{}", err);
            log::warn!("Inference rejected: {}", error_message);

            return (
                StatusCode::PAYMENT_REQUIRED,
                Json(json!({
                    "success": false,
                    "error": error_message,
                    "available_credits": account.available_credits()
                })),
            );
        }

        // Initialize usage tracker if needed
        if account.usage.is_none() {
            account.usage = Some(UsageTracker::new());
        }

        // Now we know usage exists, we can use expect safely
        let cost = account
            .usage
            .as_mut()
            .expect("Usage tracker exists")
            .record_token_usage(&model_id, input_tokens, output_tokens);

        log::info!(
            "Recorded {} tokens ({}+{}) for model {}, cost: {} credits",
            total_tokens,
            input_tokens,
            output_tokens,
            model_id,
            cost
        );

        // Update the account
        let op = datastore
            .account_state
            .update_account_local(account.clone());
        if let Err(err) = datastore.handle_account_op(op).await {
            log::error!("Failed to update account usage tracking: {}", err);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "success": false,
                    "error": "Failed to update usage tracking"
                })),
            );
        }

        // Return successful response
        return (
            StatusCode::OK,
            Json(json!({
                "success": true,
                "model": model_id,
                "tokens_used": total_tokens,
                "result": payload.prompt.unwrap_or_else(|| "No prompt provided".to_string()),
                "remaining_credits": account.available_credits()
            })),
        );
    } else {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({
                "success": false,
                "error": format!("Model {} not found", model_id)
            })),
        );
    }
}

/// Request for model inference
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInferenceRequest {
    /// The prompt to generate from
    pub prompt: Option<String>,

    /// Number of input tokens
    pub input_tokens: Option<u64>,

    /// Number of output tokens
    pub output_tokens: Option<u64>,

    /// Additional model parameters
    pub parameters: Option<serde_json::Value>,
}
