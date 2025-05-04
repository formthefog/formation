use crate::datastore::{DataStore, DB_HANDLE, InstanceRequest};
use crate::db::write_datastore;
use crate::instances::*;
use crate::auth::RecoveredAddress;
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
    
    // Parse the instance data from the payload
    let instance_data: Result<Instance, serde_json::Error> = serde_json::from_value(payload.clone());
    
    match instance_data {
        Ok(mut instance) => {
            // Ensure the instance has the correct owner set to the authenticated user
            instance.instance_owner = effective_address.to_lowercase();
            
            // Create and apply the instance update
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
        },
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(json!({
                "error": format!("Invalid instance data: {}", e)
            })),
        ),
    }
}

pub async fn update_instance(
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
    
    // Parse the instance data from the payload
    let instance_data: Result<Instance, serde_json::Error> = serde_json::from_value(payload.clone());
    
    match instance_data {
        Ok(instance) => {
            // Check if the instance exists
            let existing_instance = datastore.instance_state.get_instance(instance.instance_id.clone());
            if let Some(existing_instance) = existing_instance {
                // Verify ownership unless the request is from an admin
                if existing_instance.instance_owner.to_lowercase() != effective_address.to_lowercase() && 
                   !datastore.network_state.is_admin_address(&user_address) {
                    return (
                        StatusCode::FORBIDDEN,
                        Json(json!({
                            "error": "You don't have permission to update this instance"
                        })),
                    );
                }
                
                // Create and apply the instance update
                let op = datastore.instance_state.update_instance_local(instance.clone());
                if let Err(e) = datastore.handle_instance_op(op).await {
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(json!({
                            "error": format!("Failed to update instance: {}", e)
                        })),
                    );
                }
                
                (
                    StatusCode::OK,
                    Json(json!({
                        "status": "success",
                        "instance": instance
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
        },
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(json!({
                "error": format!("Invalid instance data: {}", e)
            })),
        ),
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
