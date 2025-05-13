use crate::datastore::DataStore;
use crate::instances::*;
use crate::auth::RecoveredAddress;
use reqwest::Client;
use std::sync::Arc;
use tokio::sync::Mutex;
use form_types::state::{Response, Success};
use axum::{extract::{State, Path, ConnectInfo}, Json};
use form_vm_metrics::system::SystemMetrics;
use std::net::{IpAddr, SocketAddr};
use serde_json::json;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use std::time::{SystemTime, UNIX_EPOCH};

pub async fn create_instance(
    State(state): State<Arc<Mutex<DataStore>>>,
    recovered: Option<RecoveredAddress>,
    ConnectInfo(connection_info): ConnectInfo<SocketAddr>,
    Json(payload): Json<Instance>,
) -> impl IntoResponse {
    let mut datastore = state.lock().await;
    
    let remote_addr = connection_info.to_string();
    let is_localhost = remote_addr.starts_with("127.0.0.1") || remote_addr.starts_with("::1");
    
    // Check if this is a request from an admin node with an original user address
    let effective_address = if is_localhost {
        payload.instance_owner.clone()
    } else {
        // If it's a regular user, use their address
        match recovered {
            Some(address) => address.as_hex(),
            None => {
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
    
    let mut instance = payload.clone();
    instance.instance_owner = effective_address.to_lowercase();
            
    let op = datastore.instance_state.update_instance_local(instance.clone());
    if let Err(e) = datastore.handle_instance_op(op).await {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({
                "error": format!("Failed to create instance: {}", e)
            })),
        );
    }
            
    (
        StatusCode::CREATED,
        Json(json!({
            "status": "success",
            "instance": instance
        })),
    )
}

pub async fn update_instance(
    State(state): State<Arc<Mutex<DataStore>>>,
    recovered: Option<RecoveredAddress>,
    ConnectInfo(connection_info): ConnectInfo<SocketAddr>,
    Json(payload): Json<Instance>,
) -> impl IntoResponse {
    log::info!("update_instance: Request from: {} for instance_id: {}", connection_info.to_string(), payload.instance_id);
    let mut datastore = state.lock().await;
    
    let remote_addr = connection_info.to_string();
    let is_localhost = remote_addr.starts_with("127.0.0.1") || remote_addr.starts_with("::1");

    let existing_instance = match datastore.instance_state.get_instance(payload.instance_id.clone()) {
        Some(inst) => inst,
        None => {
            log::warn!("update_instance: Instance_id {} not found for update.", payload.instance_id);
            return (
                StatusCode::NOT_FOUND,
                Json(json!({
                    "success": false,
                    "error": "Instance not found"
                })),
            );
        }
    };

    let mut can_update = false;
    let mut is_caller_admin = false;
    let mut authenticated_address_hex = String::new(); // For logging if needed

    if is_localhost {
        log::info!("update_instance: Request from localhost for instance_id: {}. Access granted to update.", payload.instance_id);
        can_update = true;
        // For localhost, effective_owner for permission check could be considered the one in the payload, assuming it's an internal trusted update.
    } else if let Some(auth_data) = recovered.as_ref() { // Borrow to use auth_data later if needed
        authenticated_address_hex = auth_data.as_hex();
        let normalized_auth_address = authenticated_address_hex.to_lowercase();
        let normalized_instance_owner = existing_instance.instance_owner.strip_prefix("0x").unwrap_or(&existing_instance.instance_owner).to_lowercase();
        
        is_caller_admin = datastore.network_state.is_admin_address(&authenticated_address_hex);

        if normalized_auth_address == normalized_instance_owner {
            log::info!("update_instance: User {} owns instance {}. Access granted.", authenticated_address_hex, payload.instance_id);
            can_update = true;
        } else if is_caller_admin {
            log::info!("update_instance: Admin user {} updating instance {}. Access granted.", authenticated_address_hex, payload.instance_id);
            can_update = true;
        } else {
            log::warn!("update_instance: User {} does not own instance {} and is not admin. Update denied.", authenticated_address_hex, payload.instance_id);
        }
    } else {
        // Not localhost and no recovered address
        log::warn!("update_instance: Unauthenticated non-localhost request for instance_id: {}. Update denied.", payload.instance_id);
    }

    if !can_update {
        return (
            StatusCode::FORBIDDEN,
            Json(json!({
                "success": false,
                "error": "You don't have permission to update this instance"
            })),
        );
    }

    // Prevent changing instance_owner unless localhost or admin
    let normalized_payload_owner = payload.instance_owner.strip_prefix("0x").unwrap_or(&payload.instance_owner).to_lowercase();
    let normalized_existing_owner = existing_instance.instance_owner.strip_prefix("0x").unwrap_or(&existing_instance.instance_owner).to_lowercase();

    if normalized_payload_owner != normalized_existing_owner {
        if !is_localhost && !is_caller_admin { // only admins or localhost can change owner via update
            log::warn!(
                "update_instance: Attempt to change instance_owner for {} from {} to {} by non-admin/non-localhost user {}. Denied.", 
                payload.instance_id, existing_instance.instance_owner, payload.instance_owner, authenticated_address_hex
            );
            return (
                StatusCode::FORBIDDEN,
                Json(json!({
                    "success": false,
                    "error": "Cannot change instance owner via this update. Use dedicated ownership transfer."
                })),
            );
        }
        log::info!("update_instance: Authorized change of instance_owner for {} to {} by {} (is_localhost: {}, is_admin: {}).", 
            payload.instance_id, payload.instance_owner, authenticated_address_hex, is_localhost, is_caller_admin);
    }
    
    // If localhost, the payload.instance_owner is trusted as the new owner if it's different.
    // If not localhost, owner change is only allowed if current authenticated user is admin.
    // The actual update of instance_owner happens when `payload` is used in `update_instance_local`.
    let mut instance_to_update = payload; // payload is already the full Instance data
    instance_to_update.updated_at = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64; // Ensure updated_at is fresh

    let op = datastore.instance_state.update_instance_local(instance_to_update.clone());
    if let Err(e) = datastore.handle_instance_op(op).await {
        log::error!("update_instance: Failed to update instance {}: {}", instance_to_update.instance_id, e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({
                "success": false,
                "error": format!("Failed to update instance: {}", e)
            })),
        );
    }
    
    log::info!("update_instance: Instance {} updated successfully.", instance_to_update.instance_id);
    (StatusCode::OK, Json(json!({ "success": true, "instance": instance_to_update })))
}

pub async fn get_instance(
    State(state): State<Arc<Mutex<DataStore>>>,
    recovered: RecoveredAddress,
    Path(id): Path<String>,
) -> impl IntoResponse {
    log::info!("Attempting to get instance {id}");
    
    // Get the authenticated user's address
    let authenticated_address = recovered.as_hex();
    
    let datastore = state.lock().await;
    
    if let Some(instance) = datastore.instance_state.get_instance(id.clone()) {
        // Check if user has access to this instance
        let account = datastore.account_state.get_account(&authenticated_address);
        
        // For simplicity, we'll check if:
        // 1. User is the owner
        // 2. User has any authorization level for this instance
        // 3. User is an admin (using the same helper we used for agents)
        let is_authorized = match account {
            Some(account) => {
                account.owned_instances.contains(&id) || 
                account.get_authorization_level(&id).is_some() ||
                datastore.network_state.is_admin_address(&authenticated_address)
            },
            None => false
        };
        
        if !is_authorized {
            return (
                StatusCode::FORBIDDEN,
                Json(json!({
                    "success": false,
                    "error": "You don't have permission to access this instance"
                }))
            );
        }
        
        return (
            StatusCode::OK,
            Json(json!({
                "success": true,
                "instance": instance
            }))
        );
    }

    return (
        StatusCode::NOT_FOUND,
        Json(json!({
            "success": false,
            "error": format!("Unable to find instance with id: {}", id)
        }))
    );
}

pub async fn get_instance_by_build_id(
    State(state): State<Arc<Mutex<DataStore>>>,
    Path(id): Path<String>,
) -> Json<Response<Instance>> {
    let datastore = state.lock().await;
    log::info!("Attempting to get instance {id}");
    let instances: Vec<Instance> = datastore.instance_state.map.iter().filter_map(|ctx| {
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
    }).collect();

    return Json(Response::Success(Success::List(instances)));
}

pub async fn get_instance_ips(
    State(state): State<Arc<Mutex<DataStore>>>,
    Path(id): Path<String>,
) -> Json<Response<IpAddr>> {
    let datastore = state.lock().await;
    log::info!("Attempting to get instance {id}");
    let instances: Vec<Instance> = datastore.instance_state.map.iter().filter_map(|ctx| {
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
    }).collect();

    let ips = instances.iter().filter_map(|inst| {
        inst.formnet_ip
    }).collect();

    return Json(Response::Success(Success::List(ips)));
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
                    match Client::new()
                        .get(endpoint)
                        .send().await {
                            Ok(resp) => {
                                match resp.json::<SystemMetrics>().await {
                                    Ok(r) => {
                                        return Json(Response::Success(Success::Some(r)))
                                    }
                                    Err(e) => {
                                        return Json(Response::Failure { reason: Some(format!("Unable to deserialize response from VM: {e}")) })
                                    }
                                }
                            }
                            Err(e) => {
                                return Json(Response::Failure { reason: Some(e.to_string()) })
                            }
                    }
                } else {
                    return Json(Response::Failure { reason: Some("Instance has no formnet ip".to_string()) });
                }
            } else {
                return Json(Response::Failure { reason: Some("Instance has not started".to_string()) });
            }
        } else {
            return Json(Response::Failure { reason: Some("No record of instance in datastore".to_string()) });
        }
    } else {
        return Json(Response::Failure { reason: Some("No record of instance in datastore".to_string()) });
    }
}

pub async fn list_instance_metrics(
    State(state): State<Arc<Mutex<DataStore>>>
) -> Json<Response<SystemMetrics>> {
    let datastore = state.lock().await.instance_state.clone();
    let instances: Vec<Instance> = datastore.map().iter().filter_map(|ctx| {
        let (_, value) = ctx.val;
        match value.val() {
            Some(node) => {
                Some(node.value())
            }
            None => return None
        }
    }).collect(); 

    let mut metrics = vec![];
    for instance in instances {
        if let Some(ip) = instance.formnet_ip {
            let endpoint = format!("http://{ip}:63210/get");
            match Client::new()
                .get(endpoint)
                .send().await {
                    Ok(resp) => {
                        match resp.json::<SystemMetrics>().await {
                            Ok(r) => {
                                metrics.push(r);
                            }
                            Err(e) => {
                                log::error!("Unable to deserialize response from VM: {e}");
                            }
                        }
                    }
                    Err(e) => { 
                        log::error!("{e}");
                    }
            }
        }
    }

    if !metrics.is_empty() {
        return Json(Response::Success(Success::List(metrics)))
    } else {
        return Json(Response::Failure { reason: Some("Unable to collect any metrics from any instances".to_string()) });
    }
}

pub async fn get_cluster_metrics(
    State(state): State<Arc<Mutex<DataStore>>>,
    Path(id): Path<String>,
) -> Json<Response<SystemMetrics>> {
    let datastore = state.lock().await;
    log::info!("Attempting to get instance {id}");
    let endpoints: Vec<String> = datastore.instance_state.map.iter().filter_map(|ctx| {
        let (_, reg) = ctx.val;
        if let Some(val) = reg.val() {
            let instance = val.value();
            if instance.build_id == *id {
                match instance.formnet_ip {
                    Some(ip) => Some(format!("http://{ip}:63210/get")),
                    None => None
                }
            } else {
                None
            }
        } else {
            None
        }
    }).collect();

    let mut results = vec![]; 
    for endpoint in endpoints {
        if let Ok(resp) = Client::new()
            .get(endpoint)
            .send().await {
                if let Ok(r) = resp.json::<SystemMetrics>().await {
                    results.push(r);
                }
            }
    };

    if !results.is_empty() {
        Json(Response::Success(Success::List(results)))
    } else {
        Json(Response::Failure { reason: Some(format!("Unable to acquire any metrics foor instances with build_id: {id}")) }) 
    }
}

pub async fn delete_instance(
    State(state): State<Arc<Mutex<DataStore>>>,
    recovered: RecoveredAddress,
    Path(instance_id): Path<String>,
    payload: Option<Json<serde_json::Value>>,
) -> impl IntoResponse {
    let mut datastore = state.lock().await;
    
    // Get the address of the authenticated user
    let user_address = recovered.as_hex();
    
    // Check if this is a request from an admin node with an original user address
    let effective_address = if datastore.network_state.is_admin_address(&user_address.clone()) {
        // If it's an admin node, extract the original user address from the payload
        if let Some(Json(p)) = &payload {
            crate::auth::extract_original_user_address(p)
                .unwrap_or_else(|| user_address.clone())
        } else {
            user_address.clone()
        }
    } else {
        user_address.clone()
    };
    
    // Check if the instance exists
    let existing_instance = datastore.instance_state.get_instance(instance_id.clone());
    if let Some(existing_instance) = existing_instance {
        // Verify ownership unless the request is from an admin
        if existing_instance.instance_owner.to_lowercase() != effective_address.to_lowercase() && 
           !datastore.network_state.is_admin_address(&user_address) {
            return (
                StatusCode::FORBIDDEN,
                Json(json!({
                    "error": "You don't have permission to delete this instance"
                })),
            );
        }
        
        // Delete the instance
        let op = datastore.instance_state.remove_instance_local(instance_id.clone());
        if let Err(e) = datastore.handle_instance_op(op).await {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "error": format!("Failed to delete instance: {}", e)
                })),
            );
        }
        
        (
            StatusCode::OK,
            Json(json!({
                "status": "success",
                "message": "Instance deleted successfully"
            })),
        )
    } else {
        (
            StatusCode::NOT_FOUND,
            Json(json!({
                "error": "Instance not found"
            })),
        )
    }
}

pub async fn list_instances(
    State(state): State<Arc<Mutex<DataStore>>>,
    recovered: RecoveredAddress,
) -> impl IntoResponse {
    log::info!("Received list instances request");
    
    // Get the authenticated user's address
    let authenticated_address = recovered.as_hex();
    
    let datastore = state.lock().await;
    
    // Check if the user is an admin
    let is_admin = datastore.network_state.is_admin_address(&authenticated_address);
    
    // Get the account
    let account = datastore.account_state.get_account(&authenticated_address);
    
    // Get all instances from the datastore
    let all_instances: Vec<Instance> = datastore.instance_state.map().iter().filter_map(|ctx| {
        let (_, value) = ctx.val;
        match value.val() {
            Some(node) => Some(node.value()),
            None => None
        }
    }).collect();
    
    // Filter the instances based on authorization
    let filtered_instances: Vec<Instance> = all_instances
        .into_iter()
        .filter(|instance| {
            // Admins can see all instances
            if is_admin {
                return true;
            }
            
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
    
    return (
        StatusCode::OK,
        Json(json!({
            "success": true,
            "count": filtered_instances.len(),
            "instances": filtered_instances
        }))
    );
}
