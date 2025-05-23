use crate::datastore::{DataStore, NodeRequest, DB_HANDLE};
use crate::db::write_datastore;
use crate::nodes::Node;
use std::sync::Arc;
use form_node_metrics::metrics::NodeMetrics;
use tokio::sync::Mutex;
use axum::{extract::{State, Path}, Json};
use form_types::state::{Response, Success};

pub async fn create_node(
    State(state): State<Arc<Mutex<DataStore>>>,
    Json(request): Json<NodeRequest>
) -> Json<Response<Node>> {
    let mut datastore = state.lock().await;
    
    // Extract initial operator keys from environment or request header
    let initial_operator_keys = match std::env::var("TRUSTED_OPERATOR_KEYS") {
        Ok(keys) => keys.split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect::<Vec<String>>(),
        Err(_) => Vec::new(),
    };
    
    match request {
        NodeRequest::Op(map_op) => {
            log::info!("Create Node request is an Op from another peer");
            match &map_op {
                crdts::map::Op::Up { ref key, ref op, .. } => {
                    datastore.node_state.node_op(map_op.clone());
                    if let (true, v) = datastore.node_state.node_op_success(key.clone(), op.clone()) {
                        log::info!("Node Op succesffully applied...");
                        let _ = write_datastore(&DB_HANDLE, &datastore.clone());
                        return Json(Response::Success(Success::Some(v.into())))
                    } else {
                        log::info!("Node Op rejected...");
                        return Json(Response::Failure { reason: Some("update was rejected".to_string()) })
                    }
                }
                crdts::map::Op::Rm { .. } => {
                    return Json(Response::Failure { reason: Some("Invalid Op type for Create Node".into()) });
                }
            }
        }
        NodeRequest::Create(mut contents) => {
            log::info!("Create Node request was a direct request...");
            
            // Add initial operator keys from environment if available
            if !initial_operator_keys.is_empty() {
                for key in initial_operator_keys {
                    if !contents.operator_keys.contains(&key) {
                        contents.operator_keys.push(key);
                    }
                }
            }
            
            log::info!("Building Map Op...");
            let map_op = datastore.node_state.update_node_local(contents);
            log::info!("Map op created... Applying...");
            datastore.node_state.node_op(map_op.clone());
            match &map_op {
                crdts::map::Op::Rm { .. } => {
                    return Json(Response::Failure { reason: Some("Map generated RM context instead of Add context on Create request".to_string()) });
                }
                crdts::map::Op::Up { ref key, ref op, .. } => {
                    if let (true, v) = datastore.node_state.node_op_success(key.clone(), op.clone()) {
                        log::info!("Map Op was successful, broadcasting...");
                        let request = NodeRequest::Op(map_op);
                        match datastore.broadcast::<Response<Node>>(request, "/node/create").await {
                            Ok(()) => {
                                let _ = write_datastore(&DB_HANDLE, &datastore.clone());
                                return Json(Response::Success(Success::Some(v.into())))
                            }
                            Err(e) => eprintln!("Error broadcasting Node Create Request: {e}")
                        }
                        return Json(Response::Success(Success::Some(v.into())))
                    } else {
                        return Json(Response::Failure { reason: Some("update was rejected".to_string()) })
                    }
                }
            }
        }
        _ => {
            return Json(Response::Failure { reason: Some("Invalid request for create node".into()) });
        }
    }
}

pub async fn update_node(
    State(state): State<Arc<Mutex<DataStore>>>,
    Json(request): Json<NodeRequest>
) -> Json<Response<Node>> {
    let mut datastore = state.lock().await;
    match request {
        NodeRequest::Op(map_op) => {
            log::info!("Update Node request is an Op from another peer");
            match &map_op {
                crdts::map::Op::Up { ref key, ref op, .. } => {
                    datastore.node_state.node_op(map_op.clone());
                    if let (true, v) = datastore.node_state.node_op_success(key.clone(), op.clone()) {
                        log::info!("Node Op succesffully applied...");
                        let _ = write_datastore(&DB_HANDLE, &datastore.clone());
                        return Json(Response::Success(Success::Some(v.into())))
                    } else {
                        log::info!("Node Op rejected...");
                        return Json(Response::Failure { reason: Some("update was rejected".to_string()) })
                    }
                }
                crdts::map::Op::Rm { .. } => {
                    return Json(Response::Failure { reason: Some("Invalid Op type for Create Node".into()) });
                }
            }
        }
        NodeRequest::Update(contents) => {
            log::info!("Update Node request was a direct request...");
            log::info!("Building Map Op...");
            let map_op = datastore.node_state.update_node_local(contents);
            log::info!("Map op created... Applying...");
            datastore.node_state.node_op(map_op.clone());
            match &map_op {
                crdts::map::Op::Rm { .. } => {
                    return Json(Response::Failure { reason: Some("Map generated RM context instead of Add context on Create request".to_string()) });
                }
                crdts::map::Op::Up { ref key, ref op, .. } => {
                    if let (true, v) = datastore.node_state.node_op_success(key.clone(), op.clone()) {
                        log::info!("Map Op was successful, broadcasting...");
                        let request = NodeRequest::Op(map_op);
                        match datastore.broadcast::<Response<Node>>(request, "/node/update").await {
                            Ok(()) => {
                                let _ = write_datastore(&DB_HANDLE, &datastore.clone());
                                return Json(Response::Success(Success::Some(v.into())))
                            }
                            Err(e) => eprintln!("Error broadcasting Node Update Request: {e}")
                        }
                        return Json(Response::Success(Success::Some(v.into())))
                    } else {
                        return Json(Response::Failure { reason: Some("update was rejected".to_string()) })
                    }
                }
            }
        }
        _ => {
            return Json(Response::Failure { reason: Some("Invalid request for Update Node".into()) });
        }
    }
}

pub async fn delete_node(
    State(state): State<Arc<Mutex<DataStore>>>,
    Path(node_id): Path<String>,
    Json(request): Json<NodeRequest>
) -> Json<Response<Node>> {
    let mut datastore = state.lock().await;
    match request {
        NodeRequest::Op(map_op) => {
            log::info!("Delete Node request is an Op from another peer");
            match &map_op {
                crdts::map::Op::Up { .. } => {
                    return Json(Response::Failure { reason: Some("Invalid Op type for delete dns".into()) });
                }
                crdts::map::Op::Rm { .. } => {
                    datastore.node_state.node_op(map_op);
                    return Json(Response::Success(Success::None))
                }
            }
        }
        NodeRequest::Delete(_id) => {
            log::info!("Delete Node request was a direct request...");
            log::info!("Building Map Op...");
            let map_op = datastore.node_state.remove_node_local(node_id.clone());
            log::info!("Map op created... Applying...");
            datastore.node_state.node_op(map_op.clone());
            match &map_op {
                crdts::map::Op::Rm { .. } => {
                    let request = NodeRequest::Op(map_op);
                    match datastore.broadcast::<Response<Node>>(request, &format!("/node/{}/delete", node_id.clone())).await {
                        Ok(()) => return Json(Response::Success(Success::None)),
                        Err(e) => eprintln!("Error broadcasting Delete Node request: {e}")
                    }
                    return Json(Response::Success(Success::None));
                }
                crdts::map::Op::Up { .. } => {
                    return Json(Response::Failure { reason: Some("Map generated Add context instead of Rm context on Delete request".to_string()) });
                }
            }
        }
        _ => {
            return Json(Response::Failure { reason: Some("Invalid request for delete Node".into()) });
        }
    }
}

pub async fn get_node(
    State(state): State<Arc<Mutex<DataStore>>>,
    Path(node_id): Path<String>,
) -> Json<Response<Node>> {
    let datastore = state.lock().await;
    if let Some(node) = datastore.node_state.get_node(node_id.clone()) {
        return Json(Response::Success(Success::Some(node)))
    }

    return Json(Response::Failure { reason: Some(format!("Unable to find node with id: {node_id}"))})
}

pub async fn get_node_metrics(
    State(state): State<Arc<Mutex<DataStore>>>,
    Path(node_id): Path<String>,
) -> Json<Response<NodeMetrics>> {
    let datastore = state.lock().await;
    if let Some(node) = datastore.node_state.get_node(node_id.clone()) {
        return Json(Response::Success(Success::Some(node.metrics)))
    }

    return Json(Response::Failure { reason: Some(format!("Unable to node instance with id: {node_id}"))})
}

pub async fn list_node_metrics(
    State(state): State<Arc<Mutex<DataStore>>>,
) -> Json<Response<NodeMetrics>> {
    let datastore = state.lock().await;
    let list: Vec<NodeMetrics> = datastore.node_state.map().iter().filter_map(|ctx| {
        let (_, value) = ctx.val;
        match value.val() {
            Some(node) => {
                return Some(node.value().metrics.clone())
            }
            None => return None
        }
    }).collect(); 

    return Json(Response::Success(Success::List(list)))
}

pub async fn list_nodes(
    State(state): State<Arc<Mutex<DataStore>>>,
) -> Json<Response<Node>> {
    let datastore = state.lock().await;
    let list: Vec<Node> = datastore.node_state.map().iter().filter_map(|ctx| {
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
