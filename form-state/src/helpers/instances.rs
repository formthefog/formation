use crate::datastore::{DataStore, DB_HANDLE, InstanceRequest};
use crate::db::write_datastore;
use crate::instances::*;
use crate::auth::RecoveredAddress;
use crate::accounts::AuthorizationLevel;
use reqwest::Client;
use std::sync::Arc;
use tokio::sync::Mutex;
use form_types::state::{Response, Success};
use axum::{extract::{State, Path}, Json};
use form_vm_metrics::system::SystemMetrics;
use std::net::IpAddr;
use serde_json::json;
use axum::http::StatusCode;
use axum::response::IntoResponse;

// Helper function to determine if an address belongs to an admin
// This would ideally be replaced with a proper role-based system
fn is_admin_address(address: &str) -> bool {
    // For now, we'll use a simple check for a specific address pattern
    // In a real system, this would query against a database or use JWT claims
    address.to_lowercase() == "0xadmin" || address.starts_with("0x000admin")
}

pub async fn create_instance(
    State(state): State<Arc<Mutex<DataStore>>>,
    recovered: RecoveredAddress,
    Json(request): Json<InstanceRequest>
) -> impl IntoResponse {
    log::info!("Received instance create request");
    
    // Get the authenticated user's address
    let authenticated_address = recovered.as_hex();
    
    let mut datastore = state.lock().await;
    
    // Check if the user account exists
    let mut account = match datastore.account_state.get_account(&authenticated_address) {
        Some(acc) => acc.clone(),
        None => {
            // Create a new account if it doesn't exist
            let new_account = crate::accounts::Account::new(authenticated_address.clone());
            let op = datastore.account_state.update_account_local(new_account.clone());
            if let Err(e) = datastore.handle_account_op(op).await {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({
                        "success": false,
                        "error": format!("Failed to create account: {}", e)
                    }))
                );
            }
            new_account
        }
    };
    
    match request {
        InstanceRequest::Op(map_op) => {
            // For InstanceRequest::Op, we only allow this from internal network operations
            // External users should not be able to directly apply operations
            return (
                StatusCode::FORBIDDEN,
                Json(json!({
                    "success": false,
                    "error": "Direct operation application is not allowed for external users"
                }))
            );
        },
        InstanceRequest::Create(mut instance) => {
            log::info!("Create Instance request was a direct request");
            
            // Set the owner to the authenticated user
            instance.instance_owner = authenticated_address.clone();
            
            // Create the instance
            let map_op = datastore.instance_state.update_instance_local(instance.clone());
            datastore.instance_state.instance_op(map_op.clone());
            
            match &map_op {
                crdts::map::Op::Rm { .. } => {
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(json!({
                            "success": false,
                            "error": "Map generated RM context instead of Add context on Create request"
                        }))
                    );
                },
                crdts::map::Op::Up { ref key, ref op, .. } => {
                    if let (true, v) = datastore.instance_state.instance_op_success(key.clone(), op.clone()) {
                        // Add the instance to the user's owned instances
                        account.add_owned_instance(instance.instance_id.clone());
                        let account_op = datastore.account_state.update_account_local(account);
                        if let Err(e) = datastore.handle_account_op(account_op).await {
                            log::error!("Failed to update account after instance creation: {}", e);
                            // Continue anyway since the instance was created
                        }
                        
                        log::info!("Map Op was successful, broadcasting...");
                        let broadcast_request = InstanceRequest::Op(map_op);
                        if let Err(e) = datastore.broadcast::<Response<Instance>>(broadcast_request, "/instance/create").await {
                            log::error!("Error broadcasting Instance Create Request: {}", e);
                        }
                        
                        // Write to persistent storage
                        let _ = write_datastore(&DB_HANDLE, &datastore.clone());
                        
                        return (
                            StatusCode::CREATED,
                            Json(json!({
                                "success": true,
                                "instance": v
                            }))
                        );
                    } else {
                        return (
                            StatusCode::INTERNAL_SERVER_ERROR,
                            Json(json!({
                                "success": false,
                                "error": "Update was rejected"
                            }))
                        );
                    }
                }
            }
        },
        _ => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({
                    "success": false,
                    "error": "Invalid request for create instance"
                }))
            );
        }
    }
}

pub async fn update_instance(
    State(state): State<Arc<Mutex<DataStore>>>,
    recovered: RecoveredAddress,
    Json(request): Json<InstanceRequest>
) -> impl IntoResponse {
    log::info!("Received instance update request");
    
    // Get the authenticated user's address
    let authenticated_address = recovered.as_hex();
    
    let mut datastore = state.lock().await;
    
    match request {
        InstanceRequest::Op(map_op) => {
            // For InstanceRequest::Op, we only allow this from internal network operations
            // External users should not be able to directly apply operations
            return (
                StatusCode::FORBIDDEN,
                Json(json!({
                    "success": false,
                    "error": "Direct operation application is not allowed for external users"
                }))
            );
        },
        InstanceRequest::Update(mut instance) => {
            log::info!("Update Instance request was a direct request");
            
            // Check if the instance exists
            if let Some(existing_instance) = datastore.instance_state.get_instance(instance.instance_id.clone()) {
                // Check authorization - user must be owner or have Manager permissions
                let account = datastore.account_state.get_account(&authenticated_address);
                
                match account {
                    Some(account) => {
                        // Check if the user is the owner or has Manager authorization
                        let is_owner = account.owned_instances.contains(&instance.instance_id);
                        let has_manager_auth = match account.get_authorization_level(&instance.instance_id) {
                            Some(AuthorizationLevel::Owner) | Some(AuthorizationLevel::Manager) => true,
                            _ => false
                        };
                        
                        if !is_owner && !has_manager_auth {
                            log::warn!("Unauthorized attempt to update instance: {} by {}", instance.instance_id, authenticated_address);
                            return (
                                StatusCode::FORBIDDEN,
                                Json(json!({
                                    "success": false,
                                    "error": "You don't have permission to update this instance"
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
                
                // Preserve the owner field - users cannot change ownership via update
                instance.instance_owner = existing_instance.instance_owner;
                
                // Update the instance
                let map_op = datastore.instance_state.update_instance_local(instance);
                datastore.instance_state.instance_op(map_op.clone());
                
                match &map_op {
                    crdts::map::Op::Rm { .. } => {
                        return (
                            StatusCode::INTERNAL_SERVER_ERROR,
                            Json(json!({
                                "success": false,
                                "error": "Map generated RM context instead of Add context on Update request"
                            }))
                        );
                    },
                    crdts::map::Op::Up { ref key, ref op, .. } => {
                        if let (true, v) = datastore.instance_state.instance_op_success(key.clone(), op.clone()) {
                            log::info!("Map Op was successful, broadcasting...");
                            let broadcast_request = InstanceRequest::Op(map_op);
                            if let Err(e) = datastore.broadcast::<Response<Instance>>(broadcast_request, "/instance/update").await {
                                log::error!("Error broadcasting Instance Update Request: {}", e);
                            }
                            
                            // Write to persistent storage
                            let _ = write_datastore(&DB_HANDLE, &datastore.clone());
                            
                            return (
                                StatusCode::OK,
                                Json(json!({
                                    "success": true,
                                    "instance": v
                                }))
                            );
                        } else {
                            return (
                                StatusCode::INTERNAL_SERVER_ERROR,
                                Json(json!({
                                    "success": false,
                                    "error": "Update was rejected"
                                }))
                            );
                        }
                    }
                }
            } else {
                return (
                    StatusCode::NOT_FOUND,
                    Json(json!({
                        "success": false,
                        "error": format!("Instance with id {} does not exist", instance.instance_id)
                    }))
                );
            }
        },
        _ => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({
                    "success": false,
                    "error": "Invalid request for update instance"
                }))
            );
        }
    }
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
                is_admin_address(&authenticated_address)
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
    Path(instance_id): Path<String>
) -> impl IntoResponse {
    log::info!("Received instance delete request for {}", instance_id);
    
    // Get the authenticated user's address
    let authenticated_address = recovered.as_hex();
    
    let mut datastore = state.lock().await;
    
    // Check if the instance exists
    if let Some(instance) = datastore.instance_state.get_instance(instance_id.clone()) {
        // Check authorization - user must be the owner
        let account = datastore.account_state.get_account(&authenticated_address);
        
        match account {
            Some(mut account) => {
                // Only the owner can delete an instance
                let is_owner = account.owned_instances.contains(&instance_id);
                let auth_level = account.get_authorization_level(&instance_id);
                
                if !is_owner && auth_level != Some(&AuthorizationLevel::Owner) {
                    log::warn!("Unauthorized attempt to delete instance: {} by {}", instance_id, authenticated_address);
                    return (
                        StatusCode::FORBIDDEN,
                        Json(json!({
                            "success": false,
                            "error": "You don't have permission to delete this instance"
                        }))
                    );
                }
                
                // Create and apply the delete operation
                let map_op = datastore.instance_state.remove_instance_local(instance_id.clone());
                datastore.instance_state.instance_op(map_op.clone());
                
                // Remove the instance from the user's owned instances
                account.remove_owned_instance(&instance_id);
                let account_op = datastore.account_state.update_account_local(account);
                if let Err(e) = datastore.handle_account_op(account_op).await {
                    log::error!("Failed to update account after instance deletion: {}", e);
                    // Continue anyway since the instance is deleted
                }
                
                // Broadcast the delete operation
                let broadcast_request = InstanceRequest::Op(map_op);
                if let Err(e) = datastore.broadcast::<Response<Instance>>(broadcast_request, &format!("/instance/{}/delete", instance_id)).await {
                    log::error!("Error broadcasting Delete Instance request: {}", e);
                }
                
                // Write to persistent storage
                let _ = write_datastore(&DB_HANDLE, &datastore.clone());
                
                return (
                    StatusCode::OK,
                    Json(json!({
                        "success": true,
                        "message": format!("Successfully deleted instance {}", instance_id)
                    }))
                );
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
    } else {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({
                "success": false,
                "error": format!("Instance with id {} does not exist", instance_id)
            }))
        );
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
    let is_admin = is_admin_address(&authenticated_address);
    
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
