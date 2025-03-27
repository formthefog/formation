use crate::datastore::{DataStore, DB_HANDLE, InstanceRequest};
use crate::db::write_datastore;
use crate::instances::*;
use reqwest::Client;
use std::sync::Arc;
use tokio::sync::Mutex;
use form_types::state::{Response, Success};
use serde::{Serialize, Deserialize};
use axum::{extract::{State, Path}, Json, Router};
use form_vm_metrics::system::SystemMetrics;
use std::net::IpAddr;

pub async fn create_instance(
    State(state): State<Arc<Mutex<DataStore>>>,
    Json(request): Json<InstanceRequest>
) -> Json<Response<Instance>> {
    let mut datastore = state.lock().await;
    match request {
        InstanceRequest::Op(map_op) => {
            log::info!("Create Instance request is an Op from another peer");
            match &map_op {
                crdts::map::Op::Up { ref key, ref op, .. } => {
                    datastore.instance_state.instance_op(map_op.clone());
                    if let (true, v) = datastore.instance_state.instance_op_success(key.clone(), op.clone()) {
                        let _ = write_datastore(&DB_HANDLE, &datastore.clone());
                        log::info!("Instance Op succesffully applied...");
                        return Json(Response::Success(Success::Some(v.into())))
                    } else {
                        log::info!("Instance Op rejected...");
                        return Json(Response::Failure { reason: Some("update was rejected".to_string()) })
                    }
                }
                crdts::map::Op::Rm { .. } => {
                    return Json(Response::Failure { reason: Some("Invalid Op type for Create Instance".into()) });
                }
            }
        }
        InstanceRequest::Create(contents) => {
            log::info!("Create Instance request was a direct request...");
            log::info!("Building Map Op...");
            let map_op = datastore.instance_state.update_instance_local(contents);
            log::info!("Map op created... Applying...");
            datastore.instance_state.instance_op(map_op.clone());
            match &map_op {
                crdts::map::Op::Rm { .. } => {
                    return Json(Response::Failure { reason: Some("Map generated RM context instead of Add context on Create request".to_string()) });
                }
                crdts::map::Op::Up { ref key, ref op, .. } => {
                    if let (true, v) = datastore.instance_state.instance_op_success(key.clone(), op.clone()) {
                        log::info!("Map Op was successful, broadcasting...");
                        let request = InstanceRequest::Op(map_op);
                        match datastore.broadcast::<Response<Instance>>(request, "/instance/create").await {
                            Ok(()) => {
                                let _ = write_datastore(&DB_HANDLE, &datastore.clone());
                                return Json(Response::Success(Success::Some(v.into())))
                            }
                            Err(e) => eprintln!("Error broadcasting Instance Create Request: {e}")
                        }
                        return Json(Response::Success(Success::Some(v.into())))
                    } else {
                        return Json(Response::Failure { reason: Some("update was rejected".to_string()) })
                    }
                }
            }
        }
        _ => {
            return Json(Response::Failure { reason: Some("Invalid request for create instance".into()) });
        }
    }
}

pub async fn update_instance(
    State(state): State<Arc<Mutex<DataStore>>>,
    Json(request): Json<InstanceRequest>
) -> Json<Response<Instance>> {
    let mut datastore = state.lock().await;
    match request {
        InstanceRequest::Op(map_op) => {
            log::info!("Update Instance request is an Op from another peer");
            match &map_op {
                crdts::map::Op::Up { ref key, ref op, .. } => {
                    datastore.instance_state.instance_op(map_op.clone());
                    if let (true, v) = datastore.instance_state.instance_op_success(key.clone(), op.clone()) {
                        log::info!("Instance Op succesffully applied...");
                        let _ = write_datastore(&DB_HANDLE, &datastore.clone());
                        return Json(Response::Success(Success::Some(v.into())))
                    } else {
                        log::info!("Instance Op rejected...");
                        return Json(Response::Failure { reason: Some("update was rejected".to_string()) })
                    }
                }
                crdts::map::Op::Rm { .. } => {
                    return Json(Response::Failure { reason: Some("Invalid Op type for Update Instance".into()) });
                }
            }
        }
        InstanceRequest::Update(contents) => {
            log::info!("Update Instance request was a direct request...");
            log::info!("Building Map Op...");
            let map_op = datastore.instance_state.update_instance_local(contents);
            log::info!("Map op created... Applying...");
            datastore.instance_state.instance_op(map_op.clone());
            match &map_op {
                crdts::map::Op::Rm { .. } => {
                    return Json(Response::Failure { reason: Some("Map generated RM context instead of Add context on Update request".to_string()) });
                }
                crdts::map::Op::Up { ref key, ref op, .. } => {
                    if let (true, v) = datastore.instance_state.instance_op_success(key.clone(), op.clone()) {
                        log::info!("Map Op was successful, broadcasting...");
                        let request = InstanceRequest::Op(map_op);
                        match datastore.broadcast::<Response<Instance>>(request, "/instance/update").await {
                            Ok(()) => {
                                let _ = write_datastore(&DB_HANDLE, &datastore.clone());
                                return Json(Response::Success(Success::Some(v.into())))
                            }
                            Err(e) => eprintln!("Error broadcasting Instance Update Request: {e}")
                        }
                        return Json(Response::Success(Success::Some(v.into())))
                    } else {
                        return Json(Response::Failure { reason: Some("update was rejected".to_string()) })
                    }
                }
            }
        }
        _ => {
            return Json(Response::Failure { reason: Some("Invalid request for update instance".into()) });
        }
    }
}

pub async fn get_instance(
    State(state): State<Arc<Mutex<DataStore>>>,
    Path(id): Path<String>,
) -> Json<Response<Instance>> {
    let datastore = state.lock().await;
    log::info!("Attempting to get instance {id}");
    if let Some(instance) = datastore.instance_state.get_instance(id.clone()) {
        return Json(Response::Success(Success::Some(instance)))
    }

    return Json(Response::Failure { reason: Some(format!("Unable to find instance with instance_id, node_id: {}", id))})
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
    Path(_id): Path<(String, String)>,
    Json(request): Json<InstanceRequest>
) -> Json<Response<Instance>> {
    let mut datastore = state.lock().await;
    match request {
        InstanceRequest::Op(map_op) => {
            log::info!("Delete Instance request is an Op from another peer");
            match &map_op {
                crdts::map::Op::Up { .. } => {
                    return Json(Response::Failure { reason: Some("Invalid Op type for delete dns".into()) });
                }
                crdts::map::Op::Rm { .. } => {
                    datastore.instance_state.instance_op(map_op);
                    return Json(Response::Success(Success::None))
                }
            }
        }
        InstanceRequest::Delete(id) => {
            log::info!("Delete Instance request was a direct request...");
            log::info!("Building Map Op...");
            let map_op = datastore.instance_state.remove_instance_local(id.clone());
            log::info!("Map op created... Applying...");
            datastore.instance_state.instance_op(map_op.clone());
            match &map_op {
                crdts::map::Op::Rm { .. } => {
                    let request = InstanceRequest::Op(map_op);
                    match datastore.broadcast::<Response<Instance>>(request, &format!("/instance/{}/delete", id.clone())).await {
                        Ok(()) => return Json(Response::Success(Success::None)),
                        Err(e) => eprintln!("Error broadcasting Delete Instance request: {e}")
                    }
                    return Json(Response::Success(Success::None));
                }
                crdts::map::Op::Up { .. } => {
                    return Json(Response::Failure { reason: Some("Map generated Add context instead of Rm context on Delete request".to_string()) });
                }
            }
        }
        _ => {
            return Json(Response::Failure { reason: Some("Invalid request for delete instance".into()) });
        }
    }
}

pub async fn list_instances(
    State(state): State<Arc<Mutex<DataStore>>>,
) -> Json<Response<Instance>> {
    let datastore = state.lock().await;
    let list: Vec<Instance> = datastore.instance_state.map().iter().filter_map(|ctx| {
        let (_, value) = ctx.val;
        match value.val() {
            Some(node) => {
                return Some(node.value())
            }
            None => return None
        }
    }).collect(); 

    return Json(Response::Success(Success::List(list)))
}
