use crate::auth::AuthenticationContext;
use crate::datastore::DataStore;
use crate::instances::*;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::{
    extract::{Path, State},
    Json,
};
use form_types::state::{Response, Success};
use form_vm_metrics::system::SystemMetrics;
use reqwest::Client;
use serde_json::json;
use std::net::IpAddr;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::Mutex;

/// Create instance with admin bypass logic
///
/// Sub-sub-task 3.5.3.1: Add admin bypass logic to instance management handlers
/// Sub-sub-task 3.5.3.2: Implement admin context extraction for instance operations
pub async fn create_instance(
    State(state): State<Arc<Mutex<DataStore>>>,
    auth_context: AuthenticationContext,
    Json(payload): Json<Instance>,
) -> impl IntoResponse {
    log::info!(
        "Received instance create request from {}",
        auth_context.description()
    );

    let mut datastore = state.lock().await;

    // Admin bypass: Admins can create instances for any owner
    if auth_context.is_admin() {
        log::info!(
            "create_instance: Admin {} creating instance for owner {}",
            auth_context.description(),
            payload.instance_owner
        );

        let mut instance = payload.clone();
        instance.instance_owner = instance.instance_owner.to_lowercase();
        instance.created_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        instance.updated_at = instance.created_at;

        let op = datastore
            .instance_state
            .update_instance_local(instance.clone());
        if let Err(e) = datastore.handle_instance_op(op).await {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "success": false,
                    "error": format!("Failed to create instance: {}", e),
                    "access_level": "admin"
                })),
            );
        }

        return (
            StatusCode::CREATED,
            Json(json!({
                "success": true,
                "message": format!("Instance created successfully by admin {}", auth_context.description()),
                "instance": instance,
                "access_level": "admin",
                "created_by": auth_context.description()
            })),
        );
    }

    // Normal user access: can only create instances with their own address as owner
    match auth_context {
        AuthenticationContext::User { address, .. } => {
            let authenticated_address = address.as_hex();

            // Ensure users can only create instances with their own address as owner
            let normalized_authenticated_address = authenticated_address.to_lowercase();
            let normalized_payload_owner = payload
                .instance_owner
                .strip_prefix("0x")
                .unwrap_or(&payload.instance_owner)
                .to_lowercase();

            if normalized_authenticated_address != normalized_payload_owner {
                log::warn!(
                    "Unauthorized: Address {} attempted to create instance for owner {}",
                    authenticated_address,
                    payload.instance_owner
                );
                return (
                    StatusCode::FORBIDDEN,
                    Json(json!({
                        "success": false,
                        "error": "You can only create instances with your own authenticated address as owner",
                        "authenticated_as": authenticated_address,
                        "requested_owner": payload.instance_owner,
                        "access_level": "user"
                    })),
                );
            }

            let mut instance = payload.clone();
            instance.instance_owner = authenticated_address.to_lowercase();
            instance.created_at = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64;
            instance.updated_at = instance.created_at;

            let op = datastore
                .instance_state
                .update_instance_local(instance.clone());
            if let Err(e) = datastore.handle_instance_op(op).await {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({
                        "success": false,
                        "error": format!("Failed to create instance: {}", e),
                        "access_level": "user"
                    })),
                );
            }

            (
                StatusCode::CREATED,
                Json(json!({
                    "success": true,
                    "message": "Instance created successfully",
                    "instance": instance,
                    "access_level": "user"
                })),
            )
        }
        _ => {
            // This shouldn't happen since middleware should handle authentication
            log::error!(
                "create_instance: Unexpected authentication context: {:?}",
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

/// Update instance with admin bypass logic
///
/// Sub-sub-task 3.5.3.1: Add admin bypass logic to instance management handlers
/// Sub-sub-task 3.5.3.2: Implement admin context extraction for instance operations
pub async fn update_instance(
    State(state): State<Arc<Mutex<DataStore>>>,
    auth_context: AuthenticationContext,
    Json(payload): Json<Instance>,
) -> impl IntoResponse {
    log::info!(
        "update_instance: Request from {} for instance_id: {}",
        auth_context.description(),
        payload.instance_id
    );
    let mut datastore = state.lock().await;

    let existing_instance = match datastore
        .instance_state
        .get_instance(payload.instance_id.clone())
    {
        Some(inst) => inst,
        None => {
            log::warn!(
                "update_instance: Instance_id {} not found for update.",
                payload.instance_id
            );
            return (
                StatusCode::NOT_FOUND,
                Json(json!({
                    "success": false,
                    "error": "Instance not found"
                })),
            );
        }
    };

    // Admin bypass: Admins can update any instance
    if auth_context.is_admin() {
        log::info!(
            "update_instance: Admin {} updating instance {}",
            auth_context.description(),
            payload.instance_id
        );

        let mut instance_to_update = payload;
        instance_to_update.updated_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        let op = datastore
            .instance_state
            .update_instance_local(instance_to_update.clone());
        if let Err(e) = datastore.handle_instance_op(op).await {
            log::error!(
                "update_instance: Failed to update instance {}: {}",
                instance_to_update.instance_id,
                e
            );
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "success": false,
                    "error": format!("Failed to update instance: {}", e),
                    "access_level": "admin"
                })),
            );
        }

        log::info!(
            "update_instance: Instance {} updated successfully by admin.",
            instance_to_update.instance_id
        );
        return (
            StatusCode::OK,
            Json(json!({
                "success": true,
                "message": format!("Instance updated successfully by admin {}", auth_context.description()),
                "instance": instance_to_update,
                "access_level": "admin",
                "updated_by": auth_context.description()
            })),
        );
    }

    // Normal user access: can only update instances they own
    match auth_context {
        AuthenticationContext::User { address, .. } => {
            let authenticated_address = address.as_hex();
            let normalized_auth_address = authenticated_address.to_lowercase();
            let normalized_instance_owner = existing_instance
                .instance_owner
                .strip_prefix("0x")
                .unwrap_or(&existing_instance.instance_owner)
                .to_lowercase();

            if normalized_auth_address != normalized_instance_owner {
                log::warn!("update_instance: User {} does not own instance {} and is not admin. Update denied.", authenticated_address, payload.instance_id);
                return (
                    StatusCode::FORBIDDEN,
                    Json(json!({
                        "success": false,
                        "error": "You don't have permission to update this instance",
                        "authenticated_as": authenticated_address,
                        "instance_owner": existing_instance.instance_owner,
                        "access_level": "user"
                    })),
                );
            }

            // Prevent changing instance_owner for normal users
            let normalized_payload_owner = payload
                .instance_owner
                .strip_prefix("0x")
                .unwrap_or(&payload.instance_owner)
                .to_lowercase();
            if normalized_payload_owner != normalized_instance_owner {
                log::warn!(
                    "update_instance: Attempt to change instance_owner for {} from {} to {} by non-admin user {}. Denied.", 
                    payload.instance_id, existing_instance.instance_owner, payload.instance_owner, authenticated_address
                );
                return (
                    StatusCode::FORBIDDEN,
                    Json(json!({
                        "success": false,
                        "error": "Cannot change instance owner via this update. Use dedicated ownership transfer.",
                        "access_level": "user"
                    })),
                );
            }

            log::info!(
                "update_instance: User {} owns instance {}. Access granted.",
                authenticated_address,
                payload.instance_id
            );

            let mut instance_to_update = payload;
            instance_to_update.updated_at = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64;

            let op = datastore
                .instance_state
                .update_instance_local(instance_to_update.clone());
            if let Err(e) = datastore.handle_instance_op(op).await {
                log::error!(
                    "update_instance: Failed to update instance {}: {}",
                    instance_to_update.instance_id,
                    e
                );
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({
                        "success": false,
                        "error": format!("Failed to update instance: {}", e),
                        "access_level": "user"
                    })),
                );
            }

            log::info!(
                "update_instance: Instance {} updated successfully.",
                instance_to_update.instance_id
            );
            (
                StatusCode::OK,
                Json(json!({
                    "success": true,
                    "message": "Instance updated successfully",
                    "instance": instance_to_update,
                    "access_level": "user"
                })),
            )
        }
        _ => {
            // This shouldn't happen since middleware should handle authentication
            log::error!(
                "update_instance: Unexpected authentication context: {:?}",
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

/// Get instance with admin bypass logic
///
/// Sub-sub-task 3.5.3.1: Add admin bypass logic to instance management handlers
/// Sub-sub-task 3.5.3.2: Implement admin context extraction for instance operations
pub async fn get_instance(
    State(state): State<Arc<Mutex<DataStore>>>,
    auth_context: AuthenticationContext,
    Path(id): Path<String>,
) -> impl IntoResponse {
    log::info!(
        "get_instance: {} requesting instance {}",
        auth_context.description(),
        id
    );

    let datastore = state.lock().await;

    if let Some(instance) = datastore.instance_state.get_instance(id.clone()) {
        // Admin bypass: Admins can access any instance
        if auth_context.is_admin() {
            log::info!(
                "get_instance: Admin access by {} for instance {}",
                auth_context.description(),
                id
            );
            return (
                StatusCode::OK,
                Json(json!({
                    "success": true,
                    "instance": instance,
                    "access_level": "admin",
                    "accessed_by": auth_context.description()
                })),
            );
        }

        // Normal user access: check ownership and authorization
        match auth_context {
            AuthenticationContext::User { address, .. } => {
                let authenticated_address = address.as_hex();

                // Check if user has access to this instance
                let account = datastore.account_state.get_account(&authenticated_address);

                let is_authorized = match account {
                    Some(account) => {
                        account.owned_instances.contains(&id)
                            || account.get_authorization_level(&id).is_some()
                    }
                    None => false,
                };

                if !is_authorized {
                    log::warn!(
                        "get_instance: User {} does not have permission to access instance {}",
                        authenticated_address,
                        id
                    );
                    return (
                        StatusCode::FORBIDDEN,
                        Json(json!({
                            "success": false,
                            "error": "You don't have permission to access this instance",
                            "authenticated_as": authenticated_address,
                            "instance_id": id,
                            "access_level": "user"
                        })),
                    );
                }

                log::info!(
                    "get_instance: User {} has access to instance {}",
                    authenticated_address,
                    id
                );
                return (
                    StatusCode::OK,
                    Json(json!({
                        "success": true,
                        "instance": instance,
                        "access_level": "user"
                    })),
                );
            }
            _ => {
                // This shouldn't happen since middleware should handle authentication
                log::error!(
                    "get_instance: Unexpected authentication context: {:?}",
                    auth_context
                );
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({
                        "success": false,
                        "error": "Internal authentication error"
                    })),
                );
            }
        }
    }

    (
        StatusCode::NOT_FOUND,
        Json(json!({
            "success": false,
            "error": format!("Unable to find instance with id: {}", id)
        })),
    )
}

pub async fn get_instance_by_build_id(
    State(state): State<Arc<Mutex<DataStore>>>,
    Path(id): Path<String>,
) -> Json<Response<Instance>> {
    let datastore = state.lock().await;
    log::info!("Attempting to get instance {id}");
    let instances: Vec<Instance> = datastore
        .instance_state
        .map
        .iter()
        .filter_map(|ctx| {
            let (_, reg) = ctx.val;
            if let Some(val) = reg.val() {
                let instance = val.value();
                if instance.build_id == id {
                    Some(instance)
                } else {
                    None
                }
            } else {
                None
            }
        })
        .collect();

    Json(Response::Success(Success::List(instances)))
}

pub async fn get_instance_ips(
    State(state): State<Arc<Mutex<DataStore>>>,
    Path(id): Path<String>,
) -> Json<Response<IpAddr>> {
    let datastore = state.lock().await;
    log::info!("Attempting to get instance {id}");
    let instances: Vec<Instance> = datastore
        .instance_state
        .map
        .iter()
        .filter_map(|ctx| {
            let (_, reg) = ctx.val;
            if let Some(val) = reg.val() {
                let instance = val.value();
                if instance.build_id == *id {
                    Some(instance)
                } else {
                    None
                }
            } else {
                None
            }
        })
        .collect();

    let ips = instances
        .iter()
        .filter_map(|inst| inst.formnet_ip)
        .collect();

    Json(Response::Success(Success::List(ips)))
}

pub async fn get_instance_metrics(
    State(state): State<Arc<Mutex<DataStore>>>,
    Path(id): Path<String>,
) -> Json<Response<SystemMetrics>> {
    let datastore = state.lock().await.instance_state.clone();
    if let Some(reg) = datastore.map.get(&id).val {
        if let Some(node) = reg.val() {
            let instance = node.value();
            if instance.status == InstanceStatus::Started {
                if let Some(ip) = instance.formnet_ip {
                    let endpoint = format!("http://{ip}:63210/get");
                    match Client::new().get(endpoint).send().await {
                        Ok(resp) => match resp.json::<SystemMetrics>().await {
                            Ok(r) => Json(Response::Success(Success::Some(r))),
                            Err(e) => Json(Response::Failure {
                                reason: Some(format!(
                                    "Unable to deserialize response from VM: {e}"
                                )),
                            }),
                        },
                        Err(e) => Json(Response::Failure {
                            reason: Some(e.to_string()),
                        }),
                    }
                } else {
                    Json(Response::Failure {
                        reason: Some("Instance has no formnet ip".to_string()),
                    })
                }
            } else {
                Json(Response::Failure {
                    reason: Some("Instance has not started".to_string()),
                })
            }
        } else {
            Json(Response::Failure {
                reason: Some("No record of instance in datastore".to_string()),
            })
        }
    } else {
        Json(Response::Failure {
            reason: Some("No record of instance in datastore".to_string()),
        })
    }
}

pub async fn list_instance_metrics(
    State(state): State<Arc<Mutex<DataStore>>>,
) -> Json<Response<SystemMetrics>> {
    let datastore = state.lock().await.instance_state.clone();
    let instances: Vec<Instance> = datastore
        .map()
        .iter()
        .filter_map(|ctx| {
            let (_, value) = ctx.val;
            value.val().map(|node| node.value())
        })
        .collect();

    let mut metrics = vec![];
    for instance in instances {
        if let Some(ip) = instance.formnet_ip {
            let endpoint = format!("http://{ip}:63210/get");
            match Client::new().get(endpoint).send().await {
                Ok(resp) => match resp.json::<SystemMetrics>().await {
                    Ok(r) => {
                        metrics.push(r);
                    }
                    Err(e) => {
                        log::error!("Unable to deserialize response from VM: {e}");
                    }
                },
                Err(e) => {
                    log::error!("{e}");
                }
            }
        }
    }

    if !metrics.is_empty() {
        Json(Response::Success(Success::List(metrics)))
    } else {
        Json(Response::Failure {
            reason: Some("Unable to collect any metrics from any instances".to_string()),
        })
    }
}

pub async fn get_cluster_metrics(
    State(state): State<Arc<Mutex<DataStore>>>,
    Path(id): Path<String>,
) -> Json<Response<SystemMetrics>> {
    let datastore = state.lock().await;
    log::info!("Attempting to get instance {id}");
    let endpoints: Vec<String> = datastore
        .instance_state
        .map
        .iter()
        .filter_map(|ctx| {
            let (_, reg) = ctx.val;
            if let Some(val) = reg.val() {
                let instance = val.value();
                if instance.build_id == *id {
                    instance
                        .formnet_ip
                        .map(|ip| format!("http://{ip}:63210/get"))
                } else {
                    None
                }
            } else {
                None
            }
        })
        .collect();

    let mut results = vec![];
    for endpoint in endpoints {
        if let Ok(resp) = Client::new().get(endpoint).send().await {
            if let Ok(r) = resp.json::<SystemMetrics>().await {
                results.push(r);
            }
        }
    }

    if !results.is_empty() {
        Json(Response::Success(Success::List(results)))
    } else {
        Json(Response::Failure {
            reason: Some(format!(
                "Unable to acquire any metrics foor instances with build_id: {id}"
            )),
        })
    }
}

/// Delete instance with admin bypass logic
///
/// Sub-sub-task 3.5.3.1: Add admin bypass logic to instance management handlers
/// Sub-sub-task 3.5.3.2: Implement admin context extraction for instance operations
pub async fn delete_instance(
    State(state): State<Arc<Mutex<DataStore>>>,
    auth_context: AuthenticationContext,
    Path(instance_id): Path<String>,
) -> impl IntoResponse {
    log::info!(
        "delete_instance: {} requesting deletion of instance {}",
        auth_context.description(),
        instance_id
    );

    let mut datastore = state.lock().await;

    // Check if the instance exists
    let existing_instance = datastore.instance_state.get_instance(instance_id.clone());
    if let Some(existing_instance) = existing_instance {
        // Admin bypass: Admins can delete any instance
        if auth_context.is_admin() {
            log::info!(
                "delete_instance: Admin {} deleting instance {}",
                auth_context.description(),
                instance_id
            );

            // Create a copy for the response
            let instance_copy = existing_instance.clone();

            // Delete the instance
            let op = datastore
                .instance_state
                .remove_instance_local(instance_id.clone());
            if let Err(e) = datastore.handle_instance_op(op).await {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({
                        "success": false,
                        "error": format!("Failed to delete instance: {}", e),
                        "access_level": "admin"
                    })),
                );
            }

            return (
                StatusCode::OK,
                Json(json!({
                    "success": true,
                    "message": format!("Instance deleted successfully by admin {}", auth_context.description()),
                    "instance": instance_copy,
                    "access_level": "admin",
                    "deleted_by": auth_context.description()
                })),
            );
        }

        // Normal user access: can only delete instances they own
        match auth_context {
            AuthenticationContext::User { address, .. } => {
                let authenticated_address = address.as_hex();

                // Verify ownership
                if existing_instance.instance_owner.to_lowercase()
                    != authenticated_address.to_lowercase()
                {
                    log::warn!("delete_instance: User {} does not own instance {} and is not admin. Delete denied.", 
                             authenticated_address, instance_id);
                    return (
                        StatusCode::FORBIDDEN,
                        Json(json!({
                            "success": false,
                            "error": "You don't have permission to delete this instance",
                            "authenticated_as": authenticated_address,
                            "instance_owner": existing_instance.instance_owner,
                            "access_level": "user"
                        })),
                    );
                }

                log::info!(
                    "delete_instance: User {} owns instance {}. Delete granted.",
                    authenticated_address,
                    instance_id
                );

                // Create a copy for the response
                let instance_copy = existing_instance.clone();

                // Delete the instance
                let op = datastore
                    .instance_state
                    .remove_instance_local(instance_id.clone());
                if let Err(e) = datastore.handle_instance_op(op).await {
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(json!({
                            "success": false,
                            "error": format!("Failed to delete instance: {}", e),
                            "access_level": "user"
                        })),
                    );
                }

                (
                    StatusCode::OK,
                    Json(json!({
                        "success": true,
                        "message": "Instance deleted successfully",
                        "instance": instance_copy,
                        "access_level": "user"
                    })),
                )
            }
            _ => {
                // This shouldn't happen since middleware should handle authentication
                log::error!(
                    "delete_instance: Unexpected authentication context: {:?}",
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
    } else {
        (
            StatusCode::NOT_FOUND,
            Json(json!({
                "success": false,
                "error": "Instance not found"
            })),
        )
    }
}

/// List instances with admin bypass logic
///
/// Sub-sub-task 3.5.3.1: Add admin bypass logic to instance management handlers
/// Sub-sub-task 3.5.3.2: Implement admin context extraction for instance operations
pub async fn list_instances(
    State(state): State<Arc<Mutex<DataStore>>>,
    auth_context: AuthenticationContext,
) -> impl IntoResponse {
    log::info!(
        "list_instances: Request from {}",
        auth_context.description()
    );

    let datastore = state.lock().await;

    // Get all instances from the datastore
    let all_instances: Vec<Instance> = datastore
        .instance_state
        .map()
        .iter()
        .filter_map(|ctx| {
            let (_, value) = ctx.val;
            value.val().map(|node| node.value())
        })
        .collect();

    // Admin bypass: Admins can see all instances
    if auth_context.is_admin() {
        log::info!(
            "list_instances: Admin access by {}. Listing all instances.",
            auth_context.description()
        );
        return (
            StatusCode::OK,
            Json(json!({
                "success": true,
                "message": format!("Retrieved all instances (admin access by {})", auth_context.description()),
                "count": all_instances.len(),
                "instances": all_instances,
                "access_level": "admin"
            })),
        );
    }

    // Normal user access: filter instances based on authorization
    match auth_context {
        AuthenticationContext::User { address, .. } => {
            let authenticated_address = address.as_hex();

            // Get the account
            let account = datastore.account_state.get_account(&authenticated_address);

            // Filter the instances based on authorization
            let filtered_instances: Vec<Instance> = all_instances
                .into_iter()
                .filter(|instance| {
                    // For regular users, check if they have access
                    if let Some(acc) = &account {
                        // Include instances the user owns
                        if acc.owned_instances.contains(&instance.instance_id) {
                            return true;
                        }

                        // Include instances the user has authorization for
                        if acc.get_authorization_level(&instance.instance_id).is_some() {
                            return true;
                        }
                    }

                    // Otherwise, the user can't see this instance
                    false
                })
                .collect();

            log::info!(
                "list_instances: User {} can access {} instances",
                authenticated_address,
                filtered_instances.len()
            );
            (
                StatusCode::OK,
                Json(json!({
                    "success": true,
                    "message": "Retrieved your accessible instances",
                    "count": filtered_instances.len(),
                    "instances": filtered_instances,
                    "access_level": "user"
                })),
            )
        }
        _ => {
            // This shouldn't happen since middleware should handle authentication
            log::error!(
                "list_instances: Unexpected authentication context: {:?}",
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
