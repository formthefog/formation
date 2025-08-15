use crate::auth::AuthenticationContext;
use crate::datastore::{DataStore, ModelRequest, DB_HANDLE};
use crate::db::write_datastore;
use crate::model::{AIModel, ModelId};
use crate::nodes::NodeId;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::str::FromStr;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::sync::Mutex;

// Round robin counter for load balancing
static ROUND_ROBIN_COUNTER: AtomicUsize = AtomicUsize::new(0);

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

    // Parse the model data from the payload
    let model_data: Result<AIModel, serde_json::Error> = serde_json::from_value(payload.clone());

    match model_data {
        Ok(mut model) => {
            // Admin bypass: Admins can create models for any owner
            if auth_context.is_admin() {
                log::info!(
                    "create_model: Admin {} creating model for owner {}",
                    auth_context.description(),
                    model.owner_id
                );

                // Use the owner_id from payload for admin requests
                model.owner_id = model.owner_id.clone();

                let op = datastore.model_state.update_model_local(model.clone());
                if let Err(e) = datastore.handle_model_op(op.clone()).await {
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(json!({
                            "error": format!("Failed to create model: {}", e),
                            "access_level": "admin"
                        })),
                    );
                }

                // Validate that we're only forwarding Op::Up operations to update endpoints
                match &op {
                    crdts::map::Op::Rm { .. } => {
                        return (
                            StatusCode::INTERNAL_SERVER_ERROR,
                            Json(json!({
                                "error": "Model create operation generated Op::Rm instead of Op::Up",
                                "access_level": "admin"
                            })),
                        );
                    }
                    crdts::map::Op::Up { .. } => {
                        // This is the expected case - continue with broadcasting
                    }
                }

                // Broadcast the model operation
                let request = ModelRequest::Op(op);
                match datastore
                    .broadcast::<serde_json::Value>(request, "v1/model/create")
                    .await
                {
                    Ok(()) => {
                        let _ = write_datastore(&DB_HANDLE, &datastore.clone());
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
                    Err(e) => eprintln!("Error broadcasting Model Create Request: {e}"),
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

                    log::info!(
                        "create_model: Creating model with owner ID: {}",
                        authenticated_address
                    );

                    let op = datastore.model_state.update_model_local(model.clone());
                    if let Err(e) = datastore.handle_model_op(op.clone()).await {
                        return (
                            StatusCode::INTERNAL_SERVER_ERROR,
                            Json(json!({
                                "error": format!("Failed to create model: {}", e),
                                "access_level": "user"
                            })),
                        );
                    }

                    // Validate that we're only forwarding Op::Up operations to update endpoints
                    match &op {
                        crdts::map::Op::Rm { .. } => {
                            return (
                                StatusCode::INTERNAL_SERVER_ERROR,
                                Json(json!({
                                    "error": "Model create operation generated Op::Rm instead of Op::Up",
                                    "access_level": "user"
                                })),
                            );
                        }
                        crdts::map::Op::Up { .. } => {
                            // This is the expected case - continue with broadcasting
                        }
                    }

                    // Broadcast the model operation
                    let request = ModelRequest::Op(op);
                    match datastore
                        .broadcast::<serde_json::Value>(request, "v1/model/create")
                        .await
                    {
                        Ok(()) => {
                            let _ = write_datastore(&DB_HANDLE, &datastore.clone());
                            return (
                                StatusCode::CREATED,
                                Json(json!({
                                    "status": "success",
                                    "model": model,
                                    "access_level": "user"
                                })),
                            );
                        }
                        Err(e) => eprintln!("Error broadcasting Model Create Request: {e}"),
                    }

                    (
                        StatusCode::CREATED,
                        Json(json!({
                            "status": "success",
                            "model": model,
                            "access_level": "user"
                        })),
                    )
                }
                _ => {
                    // This shouldn't happen since middleware should handle authentication
                    log::error!(
                        "create_model: Unexpected authentication context: {:?}",
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

    // Parse the model data from the payload
    let model_data: Result<AIModel, serde_json::Error> = serde_json::from_value(payload.clone());

    match model_data {
        Ok(model) => {
            // Check if the model exists
            let existing_model = datastore.model_state.get_model(&model.model_id);
            if let Some(existing_model) = existing_model {
                // Admin bypass: Admins can update any model
                if auth_context.is_admin() {
                    log::info!(
                        "update_model: Admin {} updating model {}",
                        auth_context.description(),
                        model.model_id
                    );

                    let op = datastore.model_state.update_model_local(model.clone());
                    if let Err(e) = datastore.handle_model_op(op.clone()).await {
                        return (
                            StatusCode::INTERNAL_SERVER_ERROR,
                            Json(json!({
                                "error": format!("Failed to update model: {}", e),
                                "access_level": "admin"
                            })),
                        );
                    }

                    // Validate that we're only forwarding Op::Up operations to update endpoints
                    match &op {
                        crdts::map::Op::Rm { .. } => {
                            return (
                                StatusCode::INTERNAL_SERVER_ERROR,
                                Json(json!({
                                    "error": "Model update operation generated Op::Rm instead of Op::Up",
                                    "access_level": "admin"
                                })),
                            );
                        }
                        crdts::map::Op::Up { .. } => {
                            // This is the expected case - continue with broadcasting
                        }
                    }

                    // Broadcast the model operation
                    let request = ModelRequest::Op(op);
                    match datastore
                        .broadcast::<serde_json::Value>(request, "v1/model/update")
                        .await
                    {
                        Ok(()) => {
                            let _ = write_datastore(&DB_HANDLE, &datastore.clone());
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
                        Err(e) => eprintln!("Error broadcasting Model Update Request: {e}"),
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
                        if existing_model.owner_id.to_lowercase()
                            != authenticated_address.to_lowercase()
                        {
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

                        log::info!(
                            "update_model: User {} owns model {}. Update granted.",
                            authenticated_address,
                            model.model_id
                        );

                        let op = datastore.model_state.update_model_local(model.clone());
                        if let Err(e) = datastore.handle_model_op(op.clone()).await {
                            return (
                                StatusCode::INTERNAL_SERVER_ERROR,
                                Json(json!({
                                    "error": format!("Failed to update model: {}", e),
                                    "access_level": "user"
                                })),
                            );
                        }

                        // Validate that we're only forwarding Op::Up operations to update endpoints
                        match &op {
                            crdts::map::Op::Rm { .. } => {
                                return (
                                    StatusCode::INTERNAL_SERVER_ERROR,
                                    Json(json!({
                                        "error": "Model update operation generated Op::Rm instead of Op::Up",
                                        "access_level": "user"
                                    })),
                                );
                            }
                            crdts::map::Op::Up { .. } => {
                                // This is the expected case - continue with broadcasting
                            }
                        }

                        // Broadcast the model operation
                        let request = ModelRequest::Op(op);
                        match datastore
                            .broadcast::<serde_json::Value>(request, "v1/model/update")
                            .await
                        {
                            Ok(()) => {
                                let _ = write_datastore(&DB_HANDLE, &datastore.clone());
                                return (
                                    StatusCode::OK,
                                    Json(json!({
                                        "status": "success",
                                        "model": model,
                                        "access_level": "user"
                                    })),
                                );
                            }
                            Err(e) => eprintln!("Error broadcasting Model Update Request: {e}"),
                        }

                        (
                            StatusCode::OK,
                            Json(json!({
                                "status": "success",
                                "model": model,
                                "access_level": "user"
                            })),
                        )
                    }
                    _ => {
                        // This shouldn't happen since middleware should handle authentication
                        log::error!(
                            "update_model: Unexpected authentication context: {:?}",
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
    auth_context: AuthenticationContext,
    Path(model_id): Path<String>,
    Json(payload): Json<serde_json::Value>,
) -> impl IntoResponse {
    let mut datastore = state.lock().await;

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
        if auth_context.is_admin() {
            log::info!(
                "add_node_to_model: Admin {} adding node {} to model {}",
                auth_context.description(),
                node_id,
                model_id
            );

            // Create and apply the model update
            let op = datastore
                .model_state
                .add_node_to_model(model_id.clone(), node_id, port);
            match op {
                Some(op) => {
                    if let Err(e) = datastore.handle_model_op(op.clone()).await {
                        return (
                            StatusCode::INTERNAL_SERVER_ERROR,
                            Json(json!({
                                "error": format!("Failed to add node to model: {}", e)
                            })),
                        );
                    }

                    // Validate that we're only forwarding Op::Up operations to update endpoints
                    match &op {
                        crdts::map::Op::Rm { .. } => {
                            return (
                                StatusCode::INTERNAL_SERVER_ERROR,
                                Json(json!({
                                    "error": "Add node operation generated Op::Rm instead of Op::Up"
                                })),
                            );
                        }
                        crdts::map::Op::Up { .. } => {
                            // This is the expected case - continue with broadcasting
                        }
                    }

                    // Broadcast the model operation
                    let request = ModelRequest::Op(op);
                    match datastore
                        .broadcast::<serde_json::Value>(request, &format!("v1/model/{}/add-node", model_id))
                        .await
                    {
                        Ok(()) => {
                            let _ = write_datastore(&DB_HANDLE, &datastore.clone());
                            return (
                                StatusCode::OK,
                                Json(json!({
                                    "status": "success",
                                    "message": "Node added to model successfully"
                                })),
                            );
                        }
                        Err(e) => eprintln!("Error broadcasting Add Node to Model Request: {e}"),
                    }

                    return (
                        StatusCode::OK,
                        Json(json!({
                            "status": "success",
                            "message": "Node added to model successfully"
                        })),
                    );
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
        }

        if let Some(existing_node) = datastore.node_state.get_node(node_id.to_string()) {
            match auth_context {
                AuthenticationContext::User { address, .. } => {
                    let authenticated_address = address.as_hex();

                    // Verify ownership
                    if existing_node.node_owner.to_lowercase()
                        != authenticated_address.to_lowercase()
                    {
                        log::warn!("add_node_to_model: User {} does not own model {} and is not admin. Add denied.",
                                 authenticated_address, model_id);
                        return (
                            StatusCode::FORBIDDEN,
                            Json(json!({
                                "error": "You don't have permission to modify this model",
                                "authenticated_as": authenticated_address,
                                "model_owner": existing_model.owner_id,
                                "access_level": "user"
                            })),
                        );
                    }

                    log::info!(
                        "add_node_to_model: User {} owns model {}. Add granted.",
                        authenticated_address,
                        model_id
                    );

                    // Create and apply the model update
                    let op =
                        datastore
                            .model_state
                            .add_node_to_model(model_id.clone(), node_id, port);
                    match op {
                        Some(op) => {
                            if let Err(e) = datastore.handle_model_op(op.clone()).await {
                                return (
                                    StatusCode::INTERNAL_SERVER_ERROR,
                                    Json(json!({
                                        "error": format!("Failed to add node to model: {}", e)
                                    })),
                                );
                            }

                            // Validate that we're only forwarding Op::Up operations to update endpoints
                            match &op {
                                crdts::map::Op::Rm { .. } => {
                                    return (
                                        StatusCode::INTERNAL_SERVER_ERROR,
                                        Json(json!({
                                            "error": "Add node operation generated Op::Rm instead of Op::Up"
                                        })),
                                    );
                                }
                                crdts::map::Op::Up { .. } => {
                                    // This is the expected case - continue with broadcasting
                                }
                            }

                            // Broadcast the model operation
                            let request = ModelRequest::Op(op);
                            match datastore
                                .broadcast::<serde_json::Value>(request, &format!("v1/model/{}/add-node", model_id))
                                .await
                            {
                                Ok(()) => {
                                    let _ = write_datastore(&DB_HANDLE, &datastore.clone());
                                    return (
                                        StatusCode::OK,
                                        Json(json!({
                                            "status": "success",
                                            "message": "Node added to model successfully"
                                        })),
                                    );
                                }
                                Err(e) => {
                                    eprintln!("Error broadcasting Add Node to Model Request: {e}")
                                }
                            }

                            (
                                StatusCode::OK,
                                Json(json!({
                                    "status": "success",
                                    "message": "Node added to model successfully"
                                })),
                            )
                        }
                        None => (
                            StatusCode::BAD_REQUEST,
                            Json(json!({
                                "error": "Failed to add node to model"
                            })),
                        ),
                    }
                }
                _ => {
                    // This shouldn't happen since middleware should handle authentication
                    log::error!(
                        "add_node_to_model: Unexpected authentication context: {:?}",
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
                    "error": "Node not found"
                })),
            )
        }
    } else {
        // Model not found
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
    auth_context: AuthenticationContext,
    Path(model_id): Path<String>,
    Json(payload): Json<serde_json::Value>,
) -> impl IntoResponse {
    let mut datastore = state.lock().await;

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
    let node_id: NodeId = match payload
        .get("node_id")
        .and_then(|v| v.as_str())
        .and_then(|node_str| NodeId::from_str(node_str).ok())
    {
        Some(node_id) => node_id,
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
        if auth_context.is_admin() {
            log::info!(
                "add_node_to_model: Admin {} adding node {} to model {}",
                auth_context.description(),
                node_id,
                model_id
            );

            // remove node from model
            let op = datastore
                .model_state
                .remove_node_from_model(&model_id, &node_id);
            match op {
                Some(op) => {
                    if let Err(e) = datastore.handle_model_op(op.clone()).await {
                        return (
                            StatusCode::INTERNAL_SERVER_ERROR,
                            Json(json!({
                                "error": format!("Failed to remove node from model: {}", e)
                            })),
                        );
                    }

                    // Validate that we're only forwarding Op::Up operations to update endpoints
                    match &op {
                        crdts::map::Op::Rm { .. } => {
                            return (
                                StatusCode::INTERNAL_SERVER_ERROR,
                                Json(json!({
                                    "error": "Remove node operation generated Op::Rm instead of Op::Up"
                                })),
                            );
                        }
                        crdts::map::Op::Up { .. } => {
                            // This is the expected case - continue with broadcasting
                        }
                    }

                    // Broadcast the model operation
                    let request = ModelRequest::Op(op);
                    match datastore
                        .broadcast::<serde_json::Value>(request, &format!("v1/model/{}/remove-node", model_id))
                        .await
                    {
                        Ok(()) => {
                            let _ = write_datastore(&DB_HANDLE, &datastore.clone());
                            return (
                                StatusCode::OK,
                                Json(json!({
                                    "status": "success",
                                    "message": "Node removed from model successfully"
                                })),
                            );
                        }
                        Err(e) => {
                            eprintln!("Error broadcasting Remove Node from Model Request: {e}")
                        }
                    }

                    return (
                        StatusCode::OK,
                        Json(json!({
                            "status": "success",
                            "message": "Node removed from model successfully"
                        })),
                    );
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
        }

        if let Some(existing_node) = datastore.node_state.get_node(node_id.to_string()) {
            match auth_context {
                AuthenticationContext::User { address, .. } => {
                    let authenticated_address = address.as_hex();

                    // Verify ownership
                    if existing_node.node_owner.to_lowercase()
                        != authenticated_address.to_lowercase()
                    {
                        log::warn!("remove_node_from_model: User {} does not own model {} and is not admin. Add denied.",
                                 authenticated_address, model_id);
                        return (
                            StatusCode::FORBIDDEN,
                            Json(json!({
                                "error": "You don't have permission to modify this model",
                                "authenticated_as": authenticated_address,
                                "model_owner": existing_model.owner_id,
                                "access_level": "user"
                            })),
                        );
                    }

                    log::info!(
                        "remove_node_from_model: User {} owns model {}. Add granted.",
                        authenticated_address,
                        model_id
                    );

                    // remove node from model
                    let op = datastore
                        .model_state
                        .remove_node_from_model(&model_id, &node_id);
                    match op {
                        Some(op) => {
                            if let Err(e) = datastore.handle_model_op(op.clone()).await {
                                return (
                                    StatusCode::INTERNAL_SERVER_ERROR,
                                    Json(json!({
                                        "error": format!("Failed to remove node from model: {}", e)
                                    })),
                                );
                            }

                            // Validate that we're only forwarding Op::Up operations to update endpoints
                            match &op {
                                crdts::map::Op::Rm { .. } => {
                                    return (
                                        StatusCode::INTERNAL_SERVER_ERROR,
                                        Json(json!({
                                            "error": "Remove node operation generated Op::Rm instead of Op::Up"
                                        })),
                                    );
                                }
                                crdts::map::Op::Up { .. } => {
                                    // This is the expected case - continue with broadcasting
                                }
                            }

                            // Broadcast the model operation
                            let request = ModelRequest::Op(op);
                            match datastore
                                .broadcast::<serde_json::Value>(request, &format!("v1/model/{}/remove-node", model_id))
                                .await
                            {
                                Ok(()) => {
                                    let _ = write_datastore(&DB_HANDLE, &datastore.clone());
                                    return (
                                        StatusCode::OK,
                                        Json(json!({
                                            "status": "success",
                                            "message": "Node removed from model successfully"
                                        })),
                                    );
                                }
                                Err(e) => eprintln!(
                                    "Error broadcasting Remove Node from Model Request: {e}"
                                ),
                            }

                            (
                                StatusCode::OK,
                                Json(json!({
                                    "status": "success",
                                    "message": "Node removed from model successfully"
                                })),
                            )
                        }
                        None => (
                            StatusCode::BAD_REQUEST,
                            Json(json!({
                                "error": "Failed to remove node from model"
                            })),
                        ),
                    }
                }
                _ => {
                    // This shouldn't happen since middleware should handle authentication
                    log::error!(
                        "remove_node_from_model: Unexpected authentication context: {:?}",
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
                    "error": "Node not found"
                })),
            )
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

    log::info!(
        "delete_model: {} requesting deletion of model {}",
        auth_context.description(),
        model_id
    );
    let mut datastore = state.lock().await;

    // Check if the model exists
    let existing_model = datastore.model_state.get_model(&model_id.clone());
    if let Some(existing_model) = existing_model {
        // Admin bypass: Admins can delete any model
        if auth_context.is_admin() {
            log::info!(
                "delete_model: Admin {} deleting model {}",
                auth_context.description(),
                model_id
            );

            // Create a copy for the response
            let model_copy = existing_model.clone();

            // Delete the model
            let op = datastore.model_state.remove_model_local(model_id.clone());
            if let Err(e) = datastore.handle_model_op(op.clone()).await {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({
                        "error": format!("Failed to delete model: {}", e),
                        "access_level": "admin"
                    })),
                );
            }

            // Broadcast the model operation
            let request = ModelRequest::Op(op);
            match datastore
                .broadcast::<serde_json::Value>(
                    request,
                    &format!("v1/model/{}/delete", model_id.clone()),
                )
                .await
            {
                Ok(()) => {
                    let _ = write_datastore(&DB_HANDLE, &datastore.clone());
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
                Err(e) => eprintln!("Error broadcasting Model Delete Request: {e}"),
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

                log::info!(
                    "delete_model: User {} owns model {}. Delete granted.",
                    authenticated_address,
                    model_id
                );

                // Create a copy for the response
                let model_copy = existing_model.clone();

                // Delete the model
                let op = datastore.model_state.remove_model_local(model_id.clone());
                if let Err(e) = datastore.handle_model_op(op.clone()).await {
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(json!({
                            "error": format!("Failed to delete model: {}", e),
                            "access_level": "user"
                        })),
                    );
                }

                // Broadcast the model operation
                let request = ModelRequest::Op(op);
                match datastore
                    .broadcast::<serde_json::Value>(
                        request,
                        &format!("v1/model/{}/delete", model_id.clone()),
                    )
                    .await
                {
                    Ok(()) => {
                        let _ = write_datastore(&DB_HANDLE, &datastore.clone());
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
                    Err(e) => eprintln!("Error broadcasting Model Delete Request: {e}"),
                }

                (
                    StatusCode::OK,
                    Json(json!({
                        "status": "success",
                        "message": "Model deleted successfully",
                        "model": model_copy,
                        "access_level": "user"
                    })),
                )
            }
            _ => {
                // This shouldn't happen since middleware should handle authentication
                log::error!(
                    "delete_model: Unexpected authentication context: {:?}",
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
                "error": "Model not found"
            })),
        )
    }
}

/// Get information about a specific AI model
pub async fn get_model(
    State(state): State<Arc<Mutex<DataStore>>>,
    auth_context: AuthenticationContext,
    Path(model_id): Path<String>,
) -> impl IntoResponse {
    log::info!(
        "User {} is requesting model {}",
        auth_context.description(),
        model_id
    );

    // Get the model from datastore
    let datastore = state.lock().await;

    match datastore.model_state.get_model(&model_id) {
        Some(model) => {
            // Check authorization if the model is private
            if model.is_private {
                let is_authorized = match &auth_context {
                    AuthenticationContext::Admin { .. } => true,
                    AuthenticationContext::Localhost { .. } => true,
                    AuthenticationContext::User { address, .. } => {
                        model.owner_id == address.as_hex()
                    }
                    _ => false,
                };

                if !is_authorized {
                    log::warn!(
                        "Unauthorized attempt to access private model: {} by {}",
                        model_id,
                        auth_context.description()
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
                auth_context.description()
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
    auth_context: AuthenticationContext,
) -> impl IntoResponse {
    // Get the address of the authenticated user
    log::info!(
        "Account {} is requesting list of all models",
        auth_context.description()
    );

    let is_allowed_all = match auth_context {
        AuthenticationContext::Admin { .. } => true,
        AuthenticationContext::Localhost { .. } => true,
        _ => false,
    };

    // Get all models from datastore
    let datastore = state.lock().await;

    // Get the account
    let account = if let AuthenticationContext::User { address, .. } = &auth_context {
        datastore.account_state.get_account(&address.as_hex())
    } else {
        None
    };

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

            // Admins and localhost can see all models
            if is_allowed_all {
                return true;
            }

            // Owner can see their own models
            if let AuthenticationContext::User { address, .. } = &auth_context {
                if model.owner_id == address.as_hex() {
                    return true;
                }
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
pub async fn find_node_for_model(
    State(state): State<Arc<Mutex<DataStore>>>,
    auth_context: AuthenticationContext,
    Path(model_id): Path<String>,
) -> impl IntoResponse {
    log::info!(
        "Account {} is requesting inference from model {}",
        auth_context.description(),
        model_id
    );

    let datastore = state.lock().await;

    let model = match datastore.model_state.get_model(&model_id) {
        Some(model) => model,
        None => {
            return (
                StatusCode::NOT_FOUND,
                Json(json!({
                    "success": false,
                    "error": format!("Model {} not found", model_id)
                })),
            );
        }
    };

    if model.is_private {
        match &auth_context {
            AuthenticationContext::Admin { .. } => {
                log::info!(
                    "model_inference: Admin {} using private model {} for inference",
                    auth_context.description(),
                    model_id
                );
            }
            AuthenticationContext::Localhost { .. } => {
                log::info!(
                    "model_inference: Localhost using private model {} for inference",
                    model_id
                );
            }
            AuthenticationContext::User { address, .. } => {
                // Check if user is the owner, has authorization, or is an admin
                let account = datastore.account_state.get_account(&address.as_hex());

                // Determine if the user has access to this model
                let is_authorized = match &account {
                    Some(account) => {
                        // User is authorized if they own the model or are an admin
                        account.owned_models.contains(&model_id)
                            || model.owner_id == account.address
                    }
                    None => false,
                };

                if !is_authorized {
                    log::warn!(
                        "Unauthorized attempt to use private model for inference: {} by {}",
                        model_id,
                        auth_context.description()
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
        }
    }

    // Use round robin to select the next node from all available nodes
    if !model.nodes.is_empty() {
        // Get the current counter value and increment it atomically
        let current_index = ROUND_ROBIN_COUNTER.fetch_add(1, Ordering::Relaxed);
        let node_index = current_index % model.nodes.len();

        // Get the node at the calculated index
        if let Some((round_robin_node_id, round_robin_port)) = model.nodes.get(node_index) {
            let node_id = &datastore.node_state.node_id;

            // If the selected node is the current node, return localhost
            if node_id == round_robin_node_id {
                log::info!(
                    "Round robin load balancing: Selected current node {} (index {}/{}) for model {}",
                    round_robin_node_id,
                    node_index,
                    model.nodes.len(),
                    model_id
                );

                return (
                    StatusCode::OK,
                    Json(json!({
                        "success": true,
                        "model": model_id,
                        "node_id": round_robin_node_id,
                        "port": round_robin_port,
                        "address": "localhost"
                    })),
                );
            }

            // Otherwise, return the selected remote node
            if let Some(round_robin_node) =
                datastore.node_state.get_node(round_robin_node_id.clone())
            {
                log::info!(
                    "Round robin load balancing: Selected remote node {} (index {}/{}) for model {}",
                    round_robin_node_id,
                    node_index,
                    model.nodes.len(),
                    model_id
                );

                return (
                    StatusCode::OK,
                    Json(json!({
                        "success": true,
                        "model": model_id,
                        "node_id": round_robin_node_id,
                        "port": round_robin_port,
                        "address": round_robin_node.host
                    })),
                );
            }
        }
    }

    // No nodes available for this model
    (
        StatusCode::SERVICE_UNAVAILABLE,
        Json(json!({
            "success": false,
            "error": "No nodes available for this model"
        })),
    )
}

// pub async fn model_inference_accounting(
//     State(state): State<Arc<Mutex<DataStore>>>,
//     auth_context: AuthenticationContext,
//     Path(model_id): Path<String>,
//     Json(payload): Json<ModelInferenceRequest>,
// ) -> impl IntoResponse {
//     log::info!(
//         "Account {} is requesting inference from model {}",
//         auth_context.description(),
//         model_id
//     );

//     let mut datastore = state.lock().await;

//     let model = match datastore.model_state.get_model(&model_id) {
//         Some(model) => model,
//         None => {
//             return (
//                 StatusCode::NOT_FOUND,
//                 Json(json!({
//                     "success": false,
//                     "error": format!("Model {} not found", model_id)
//                 })),
//             );
//         }
//     };

//     // Calculate token usage
//     let input_tokens = payload.input_tokens.unwrap_or(0);
//     let output_tokens = payload.output_tokens.unwrap_or(0);
//     let total_tokens = input_tokens + output_tokens;

//     if model.is_private {
//         match &auth_context {
//             AuthenticationContext::Admin { .. } => {
//                 log::info!(
//                     "model_inference: Admin {} using private model {} for inference",
//                     auth_context.description(),
//                     model_id
//                 );
//             },
//             AuthenticationContext::Localhost { .. } => {
//                 log::info!(
//                     "model_inference: Localhost using private model {} for inference",
//                     model_id
//                 );
//             },
//             AuthenticationContext::User { address, .. } => {
//                 // Check if user is the owner, has authorization, or is an admin
//                 let account = datastore.account_state.get_account(&address.as_hex());

//                 // Determine if the user has access to this model
//                 let is_authorized = match &account {
//                     Some(account) => {
//                         // User is authorized if they own the model or are an admin
//                         account.owned_models.contains(&model_id)
//                             || model.owner_id == account.address
//                     }
//                     None => false,
//                 };

//                 if !is_authorized {
//                     log::warn!(
//                         "Unauthorized attempt to use private model for inference: {} by {}",
//                         model_id,
//                         auth_context.description()
//                     );
//                     return (
//                         StatusCode::FORBIDDEN,
//                         Json(json!({
//                             "success": false,
//                             "error": "You don't have permission to use this private model for inference"
//                         })),
//                     );
//                 }
//             }
//         }
//     }

//     let node_id = &datastore.node_state.node_id;

//     match auth_context {
//         AuthenticationContext::Admin {..} | AuthenticationContext::Localhost {..} => {
//             // Return successful response
//             return (
//                 StatusCode::OK,
//                 Json(json!({
//                     "success": true,
//                     "model": model_id,
//                     "node_id": model.node_id,
//                     "address": model.address,

//                 })),
//             );
//         },
//         AuthenticationContext::User { address, .. } => {
//             // Get or create the user's account
//             let mut account = match datastore.account_state.get_account(&account_address) {
//                 Some(acc) => acc,
//                 None => {
//                     // Create a new account if it doesn't exist
//                     let new_account = crate::accounts::Account::new(account_address.clone());
//                     let op = datastore
//                         .account_state
//                         .update_account_local(new_account.clone());
//                     if let Err(_) = datastore.handle_account_op(op).await {
//                         return (
//                             StatusCode::INTERNAL_SERVER_ERROR,
//                             Json(json!({
//                                 "success": false,
//                                 "error": "Failed to create new account"
//                             })),
//                         );
//                     }
//                     new_account
//                 }
//             };

//             // Validate eligibility
//             if let Err(err) = check_operation_credits(&account, operation) {
//                 let error_message = format!("{}", err);
//                 log::warn!("Inference rejected: {}", error_message);

//                 return (
//                     StatusCode::PAYMENT_REQUIRED,
//                     Json(json!({
//                         "success": false,
//                         "error": error_message,
//                         "available_credits": account.available_credits()
//                     })),
//                 );
//             }

//             // Initialize usage tracker if needed
//             if account.usage.is_none() {
//                 account.usage = Some(UsageTracker::new());
//             }

//             // Now we know usage exists, we can use expect safely
//             let cost = account
//                 .usage
//                 .as_mut()
//                 .expect("Usage tracker exists")
//                 .record_token_usage(&model_id, input_tokens, output_tokens);
//             // Check if the account has enough credits for this operation
//             use crate::billing::middleware::{check_operation_credits, OperationType};
//             let operation = OperationType::TokenConsumption {
//                 model_id: model_id.clone(),
//                 input_tokens,
//                 output_tokens,
//             };

//             log::info!(
//                 "Recorded {} tokens ({}+{}) for model {}, cost: {} credits",
//                 total_tokens,
//                 input_tokens,
//                 output_tokens,
//                 model_id,
//                 cost
//             );

//             // Update the account
//             let op = datastore
//                 .account_state
//                 .update_account_local(account.clone());
//             if let Err(err) = datastore.handle_account_op(op).await {
//                 log::error!("Failed to update account usage tracking: {}", err);
//                 return (
//                     StatusCode::INTERNAL_SERVER_ERROR,
//                     Json(json!({
//                         "success": false,
//                         "error": "Failed to update usage tracking"
//                     })),
//                 );
//             }

//             // Return successful response
//             return (
//                 StatusCode::OK,
//                 Json(json!({
//                     "success": true,
//                     "model": model_id,
//                     "tokens_used": total_tokens,
//                     "result": payload.prompt.unwrap_or_else(|| "No prompt provided".to_string()),
//                     "remaining_credits": account.available_credits()
//                 })),
//             );
//         }
//     }
// }

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

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicUsize;

    #[test]
    fn test_round_robin_counter() {
        // Reset the counter for testing
        ROUND_ROBIN_COUNTER.store(0, Ordering::Relaxed);

        // Test with 3 nodes
        let node_count = 3;

        // Simulate multiple requests
        for i in 0..10 {
            let current_index = ROUND_ROBIN_COUNTER.fetch_add(1, Ordering::Relaxed);
            let node_index = current_index % node_count;

            // Verify round robin pattern: 0, 1, 2, 0, 1, 2, ...
            let expected_index = i % node_count;
            assert_eq!(
                node_index, expected_index,
                "Round robin failed at iteration {}: expected {}, got {}",
                i, expected_index, node_index
            );
        }
    }
}
