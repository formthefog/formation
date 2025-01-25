use std::{collections::{HashMap, HashSet}, sync::Arc, time::{SystemTime, UNIX_EPOCH}};
use axum::{extract::{State, Path}, routing::{get, post}, Json, Router};
use reqwest::Client;
use serde_json::Value;
use shared::{Association, AssociationContents, Cidr, CidrContents, Peer, PeerContents};
use tokio::{net::TcpListener, sync::Mutex};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use crdts::{Map, BFTReg, CvRDT};
use crate::network::{AssocOp, CidrOp, CrdtAssociation, CrdtCidr, CrdtPeer, NetworkState, PeerOp};

pub type PeerMap = Map<String, BFTReg<CrdtPeer<String>, String>, String>;
pub type CidrMap = Map<String, BFTReg<CrdtCidr<String>, String>, String>;
pub type AssocMap = Map<(String, String), BFTReg<CrdtAssociation<String>, String>, String>;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DataStore {
    network_state: NetworkState,
    // Add Node State
    // Add Instance State
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum PeerRequest {
    Op(PeerOp<String>),
    Join(PeerContents<String>),
    Update(PeerContents<String>),
    Delete(String),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Success<T> {
    Some(T),
    List(Vec<T>),
    Relationships(Vec<(Cidr<String>, Cidr<String>)>),
    None,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Response<T> {
    Success(Success<T>),
    Failure { reason: Option<String> }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum CidrRequest {
    Op(CidrOp<String>),
    Create(CidrContents<String>),
    Update(CidrContents<String>),
    Delete(String),
}


#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum AssocRequest {
    Op(AssocOp<String>), 
    Create(AssociationContents<String>),
    Delete((String, String)),
}

impl DataStore {
    pub fn new(node_id: String, pk: String) -> Self {
        let network_state = NetworkState::new(node_id, pk);

        Self { network_state }
    }

    pub fn new_from_state(
        node_id: String,
        pk: String,
        other: DataStore,
    ) -> Self {
        let mut local = Self::new(node_id, pk); 
        local.network_state.peers.merge(other.network_state.peers);
        local.network_state.cidrs.merge(other.network_state.cidrs);
        local.network_state.associations.merge(other.network_state.associations);
        local.network_state.dns_state.zones.merge(other.network_state.dns_state.zones);
        local
    }

    pub fn get_all_users(&self) -> HashMap<String, CrdtPeer<String>> {
        self.network_state.peers.iter().filter_map(|item| {
            match item.val.1.val() {
                Some(v) => Some((item.val.0.clone(), v.value().clone())),
                None => None
            }
        }).collect()
    }

    pub fn get_all_cidrs(&self) -> HashMap<String, CrdtCidr<String>> {
        self.network_state.cidrs.iter().filter_map(|item| {
            match item.val.1.val() {
                Some(v) => Some((item.val.0.clone(), v.value().clone())),
                None => None
            }
        }).collect()
    }

    pub fn get_all_assocs(&self) -> HashMap<(String, String), CrdtAssociation<String>> {
        self.network_state.associations.iter().filter_map(|item| {
            match item.val.1.val() {
                Some(v) => Some((item.val.0.clone(), v.value().clone())),
                None => None
            }
        }).collect()
    }

    pub fn get_relationships(&self, cidr_id: String) -> Vec<(Cidr<String>, Cidr<String>)> {
        let mut assoc_ids = self.get_all_assocs();
        assoc_ids.retain(|k, _| k.0 == cidr_id || k.1 == cidr_id);
        let ids: HashSet<(String, String)> = assoc_ids.iter().map(|(k, _)| k.clone()).collect();
        ids.iter().filter_map(|(cidr1, cidr2)| {
            let cidr_1 = self.network_state.cidrs.get(cidr1).val;
            let cidr_2 = self.network_state.cidrs.get(cidr2).val;
            match (cidr_1, cidr_2) {
                (Some(reg_1), Some(reg_2)) => {
                    let val1 = reg_1.val();
                    let val2 = reg_2.val();
                    match (val1, val2) {
                        (Some(node1), Some(node2)) => {
                            return Some((node1.value().into(), node2.value().into()))
                        }
                        _ => None,
                    }
                }
                _ => None,
            }
        }).collect()
    }

    pub fn get_all_active_admin(&mut self) -> HashMap<String, CrdtPeer<String>> {
        self.network_state.peers.iter().filter_map(|item| {
            match item.val.1.val() {
                Some(v) => {
                    if v.value().is_admin() {
                        Some((item.val.0.clone(), v.value().clone()))
                    } else {
                        None
                    }
                }
                None => None,
            }
        }).collect()
    }

    pub async fn broadcast<R: DeserializeOwned>(
        &mut self,
        request: impl Serialize + Clone,
        endpoint: &str
    ) -> Result<(), Box<dyn std::error::Error>> {
        let peers = self.get_all_active_admin();
        for (id, peer) in peers {
            if let Err(e) = self.send::<R>(&peer.ip().to_string(), endpoint, request.clone()).await {
                eprintln!("Error sending {endpoint} request to {id}: {}: {e}", peer.ip().to_string());
            };
        }

        Ok(())
    }

    pub async fn send<R: DeserializeOwned>(&mut self, ip: &str, endpoint: &str, request: impl Serialize) -> Result<(), Box<dyn std::error::Error>> {
        match Client::new()
            .post(format!("http://{ip}:3004/{endpoint}"))
            .json(&request)
            .send()
            .await {
                Ok(resp) => match resp.json::<R>().await {
                    Ok(_) => println!("Succesfully shared request with {ip}"),
                    Err(e) => eprintln!("Unable to decode response to request from {ip}: {e}")
                }
                Err(e) => {
                    eprintln!("Unable to share request with {ip}: {e}");
                }
            };

        Ok(())
    }

    pub fn app(state: Arc<Mutex<DataStore>>) -> Router {
        Router::new()
            .route("/ping", get(pong))
            .route("/bootstrap/full_state", get(full_state))
            .route("/bootstrap/network_state", get(network_state))
            .route("/bootstrap/peer_state", get(peer_state))
            .route("/bootstrap/cidr_state", get(cidr_state))
            .route("/bootstrap/assoc_state", get(assoc_state))
            .route("/user/create", post(create_user))
            .route("/user/update", post(update_user))
            .route("/user/disable", post(disable_user))
            .route("/user/redeem", post(redeem_invite)) 
            .route("/user/:id/get", get(get_user))
            .route("/user/:ip/get_from_ip", get(get_user_from_ip))
            .route("/user/delete", post(delete_user))
            .route("/user/:id/get_all_allowed", get(get_all_allowed))
            .route("/user/list", get(list_users))
            .route("/user/:cidr/list", get(list_by_cidr))
            .route("/user/delete_expired", post(delete_expired))
            .route("/cidr/create", post(create_cidr))
            .route("/cidr/update", post(update_cidr))
            .route("/cidr/delete", post(delete_cidr))
            .route("/cidr/:id/get", get(get_cidr))
            .route("/cidr/list", get(list_cidr))
            .route("/assoc/create", post(create_assoc))
            .route("/assoc/delete", post(delete_assoc))
            .route("/assoc/list", get(list_assoc))
            .route("/assoc/:cidr_id/relationships", get(relationships))
            .with_state(state)
    }

    pub async fn run(self) -> Result<(), Box<dyn std::error::Error>> {
        let router = Self::app(Arc::new(Mutex::new(self)));
        let listener = TcpListener::bind("0.0.0.0:3004").await?;
        let _ = axum::serve(listener, router).await?;

        Ok(())
    }
}

async fn pong() -> Json<Value> {
    Json(serde_json::json!({"ping":"pong"}))
}

async fn full_state(
    State(state): State<Arc<Mutex<DataStore>>>,
) -> Json<DataStore> {
    let datastore = state.lock().await.clone();
    Json(datastore)
}

async fn network_state(
    State(state): State<Arc<Mutex<DataStore>>>,
) -> Json<NetworkState> {
    let network_state = state.lock().await.network_state.clone();
    Json(network_state)
}

async fn peer_state(
    State(state): State<Arc<Mutex<DataStore>>>, 
) -> Json<PeerMap> {
    let peer_state = state.lock().await.network_state.peers.clone();
    Json(peer_state)
}

async fn cidr_state(
    State(state): State<Arc<Mutex<DataStore>>>, 
) -> Json<CidrMap> {
    let cidr_state = state.lock().await.network_state.cidrs.clone();
    Json(cidr_state)
}

async fn assoc_state(
    State(state): State<Arc<Mutex<DataStore>>>, 
) -> Json<AssocMap> {
    let assoc_state = state.lock().await.network_state.associations.clone();
    Json(assoc_state)
}

async fn create_user(
    State(state): State<Arc<Mutex<DataStore>>>,
    Json(user): Json<PeerRequest>
) -> Json<Response<Peer<String>>> {
    let mut datastore = state.lock().await;
    match user {
        PeerRequest::Op(map_op) => {
            match &map_op {
                crdts::map::Op::Up { ref key, ref op, .. } => {
                    datastore.network_state.peer_op(map_op.clone());
                    if let (true, v) = datastore.network_state.peer_op_success(key.clone(), op.clone()) {
                        return Json(Response::Success(Success::Some(v.into())))
                    } else {
                        return Json(Response::Failure { reason: Some("update was rejected".to_string()) })
                    }
                }
                crdts::map::Op::Rm { .. } => {
                    return Json(Response::Failure { reason: Some("Invalid Op type for Create User".into()) });
                }
            }
        }
        PeerRequest::Join(contents) => {
            let map_op = datastore.network_state.update_peer_local(contents);
            datastore.network_state.peer_op(map_op.clone());
            match &map_op {
                crdts::map::Op::Rm { .. } => {
                    return Json(Response::Failure { reason: Some("Map generated RM context instead of Add context on Join request".to_string()) });
                }
                crdts::map::Op::Up { ref key, ref op, .. } => {
                    if let (true, v) = datastore.network_state.peer_op_success(key.clone(), op.clone()) {
                        let request = PeerRequest::Op(map_op);
                        match datastore.broadcast::<Response<Peer<String>>>(request, "/user/create").await {
                            Ok(()) => return Json(Response::Success(Success::Some(v.into()))),
                            Err(e) => eprintln!("Error broadcasting DeletePeerRequest: {e}")
                        }
                        return Json(Response::Success(Success::Some(v.into())))
                    } else {
                        return Json(Response::Failure { reason: Some("update was rejected".to_string()) })
                    }
                }
            }
        }
        _ => {
            return Json(Response::Failure { reason: Some("Invalid request for create user".into()) });
        }
    }
}

async fn update_user(
    State(state): State<Arc<Mutex<DataStore>>>,
    Json(user): Json<PeerRequest>
) -> Json<Response<Peer<String>>> {
    let mut datastore = state.lock().await;
    match user {
        PeerRequest::Op(map_op) => {
            match &map_op {
                crdts::map::Op::Up { ref key, ref op, .. } => {
                    datastore.network_state.peer_op(map_op.clone());
                    if let (true, v) = datastore.network_state.peer_op_success(key.clone(), op.clone()) {
                        return Json(Response::Success(Success::Some(v.into())))
                    } else {
                        return Json(Response::Failure { reason: Some("update was rejected".to_string()) })
                    }
                }
                crdts::map::Op::Rm { .. } => {
                    return Json(Response::Failure { reason: Some("Invalid Op type for Create User".into()) });
                }
            }
        }
        PeerRequest::Update(contents) => {
            let map_op = datastore.network_state.update_peer_local(contents);
            datastore.network_state.peer_op(map_op.clone());
            match &map_op {
                crdts::map::Op::Rm { .. } => {
                    return Json(Response::Failure { reason: Some("Map generated RM context instead of Add context on Join request".to_string()) });
                }
                crdts::map::Op::Up { ref key, ref op, .. } => {
                    if let (true, v) = datastore.network_state.peer_op_success(key.clone(), op.clone()) {
                        let request = PeerRequest::Op(map_op);
                        match datastore.broadcast::<Response<Peer<String>>>(request, "/user/update").await {
                            Ok(()) => return Json(Response::Success(Success::Some(v.into()))),
                            Err(e) => eprintln!("Error broadcasting DeletePeerRequest: {e}")
                        }
                        return Json(Response::Success(Success::Some(v.into())))
                    } else {
                        return Json(Response::Failure { reason: Some("update was rejected".to_string()) })
                    }
                }
            }
        }
        _ => {
            return Json(Response::Failure { reason: Some("Invalid request for update user".into()) });
        }
    }
}

async fn disable_user(
    State(state): State<Arc<Mutex<DataStore>>>,
    Json(user): Json<PeerRequest>
) -> Json<Response<Peer<String>>> {
    let mut datastore = state.lock().await;
    match user {
        PeerRequest::Op(map_op) => {
            match &map_op {
                crdts::map::Op::Up { ref key, ref op, .. } => {
                    datastore.network_state.peer_op(map_op.clone());
                    if let (true, v) = datastore.network_state.peer_op_success(key.clone(), op.clone()) {
                        return Json(Response::Success(Success::Some(v.into())))
                    } else {
                        return Json(Response::Failure { reason: Some("update was rejected".to_string()) })
                    }
                }
                crdts::map::Op::Rm { .. } => {
                    return Json(Response::Failure { reason: Some("Invalid Op type for Create User".into()) });
                }
            }
        }
        PeerRequest::Update(contents) => {
            let map_op = datastore.network_state.update_peer_local(contents);
            datastore.network_state.peer_op(map_op.clone());
            match &map_op {
                crdts::map::Op::Rm { .. } => {
                    return Json(Response::Failure { reason: Some("Map generated RM context instead of Add context on Join request".to_string()) });
                }
                crdts::map::Op::Up { ref key, ref op, .. } => {
                    if let (true, v) = datastore.network_state.peer_op_success(key.clone(), op.clone()) {
                        let request = PeerRequest::Op(map_op);
                        match datastore.broadcast::<Response<Peer<String>>>(request, "/user/disable").await {
                            Ok(()) => return Json(Response::Success(Success::Some(v.into()))),
                            Err(e) => eprintln!("Error broadcasting DeletePeerRequest: {e}")
                        }
                        return Json(Response::Success(Success::Some(v.into())))
                    } else {
                        return Json(Response::Failure { reason: Some("update was rejected".to_string()) })
                    }
                }
            }
        }
        _ => {
            return Json(Response::Failure { reason: Some("Invalid request for update user".into()) });
        }
    }
}

async fn redeem_invite(
    State(state): State<Arc<Mutex<DataStore>>>,
    Json(user): Json<PeerRequest>
) -> Json<Response<Peer<String>>> {
    let mut datastore = state.lock().await;
    match user {
        PeerRequest::Op(map_op) => {
            match &map_op {
                crdts::map::Op::Up { ref key, ref op, .. } => {
                    datastore.network_state.peer_op(map_op.clone());
                    if let (true, v) = datastore.network_state.peer_op_success(key.clone(), op.clone()) {
                        return Json(Response::Success(Success::Some(v.into())))
                    } else {
                        return Json(Response::Failure { reason: Some("update was rejected".to_string()) })
                    }
                }
                crdts::map::Op::Rm { .. } => {
                    return Json(Response::Failure { reason: Some("Invalid Op type for Create User".into()) });
                }
            }
        }
        PeerRequest::Update(contents) => {
            let map_op = datastore.network_state.update_peer_local(contents);
            datastore.network_state.peer_op(map_op.clone());
            match &map_op {
                crdts::map::Op::Rm { .. } => {
                    return Json(Response::Failure { reason: Some("Map generated RM context instead of Add context on Join request".to_string()) });
                }
                crdts::map::Op::Up { ref key, ref op, .. } => {
                    if let (true, v) = datastore.network_state.peer_op_success(key.clone(), op.clone()) {
                        let request = PeerRequest::Op(map_op);
                        match datastore.broadcast::<Response<Peer<String>>>(request, "/user/redeem").await {
                            Ok(()) => return Json(Response::Success(Success::Some(v.into()))),
                            Err(e) => eprintln!("Error broadcasting DeletePeerRequest: {e}")
                        }
                        return Json(Response::Success(Success::Some(v.into())))
                    } else {
                        return Json(Response::Failure { reason: Some("update was rejected".to_string()) })
                    }
                }
            }
        }
        _ => {
            return Json(Response::Failure { reason: Some("Invalid request for update user".into()) });
        }
    }
}

async fn delete_user(
    State(state): State<Arc<Mutex<DataStore>>>,
    Json(user): Json<PeerRequest>
) -> Json<Response<Peer<String>>> {
    let mut datastore = state.lock().await;
    match user {
        PeerRequest::Op(map_op) => {
            match &map_op {
                crdts::map::Op::Up { .. } => {
                    return Json(Response::Failure { reason: Some("Invalid Op type for delete User".into()) });
                }
                crdts::map::Op::Rm { .. } => {
                    datastore.network_state.peer_op(map_op);
                    return Json(Response::Success(Success::None));
                }
            }
        }
        PeerRequest::Delete(contents) => {
            let map_op = datastore.network_state.remove_peer_local(contents);
            datastore.network_state.peer_op(map_op.clone());
            match &map_op {
                crdts::map::Op::Rm { .. } => {
                    let request = PeerRequest::Op(map_op);
                    match datastore.broadcast::<Response<Peer<String>>>(request, "/user/delete").await {
                        Ok(()) => return Json(Response::Success(Success::None)),
                        Err(e) => eprintln!("Error broadcasting DeletePeerRequest: {e}")
                    }
                    return Json(Response::Success(Success::None));
                }
                crdts::map::Op::Up { .. } => {
                    return Json(Response::Failure { reason: Some("Map generated Add context instead of Rm context on Delete request".to_string()) });
                }
            }
        }
        _ => {
            return Json(Response::Failure { reason: Some("Invalid request for update user".into()) });
        }
    }
}


async fn get_user(
    State(state): State<Arc<Mutex<DataStore>>>,
    Path(id): Path<String>
) -> Json<Response<Peer<String>>> {
    if let Some(peer) = state.lock().await.network_state.peers.get(&id).val {
        if let Some(val) = peer.val() {
            return Json(Response::Success(Success::Some(val.value().into())))
        } else {
            return Json(Response::Failure { reason: Some("Entry exists, but value is None".into()) })
        }
    } 
    Json(Response::Failure { reason: Some("Entry does not exist in the CrdtMap".to_string()) })
}

async fn get_user_from_ip( 
    State(state): State<Arc<Mutex<DataStore>>>,
    Path(ip): Path<String>
) -> Json<Response<Peer<String>>> {
    if let Some(peer) = state.lock().await.network_state.get_peer_by_ip(ip.clone()) {
        return Json(Response::Success(Success::Some(peer.into())))
    } 

    return Json(Response::Failure { reason: Some(format!("Peer with IP {ip} does not exist")) })
}

async fn get_all_allowed(
    State(state): State<Arc<Mutex<DataStore>>>,
    Path(id): Path<String>,
) -> Json<Response<Peer<String>>> {
    let mut peers = state.lock().await.get_all_users();
    if let Some(peer) = state.lock().await.network_state.peers.get(&id).val {
        if let Some(node) = peer.val() {
            let peer = node.value();
            let cidr = peer.cidr();
            peers.retain(|_, v| v.cidr() == cidr);
            let all_allowed = peers.iter().map(|(_, v)| v.clone().into()).collect();
            return Json(Response::Success(Success::List(all_allowed)))
        }
    } 

    return Json(Response::Failure { reason: Some("Peer for which allowed peers was requested does not exist".into()) })
}

async fn list_users(
    State(state): State<Arc<Mutex<DataStore>>>,
) -> Json<Response<Peer<String>>> {
    let peers = state.lock().await.get_all_users().iter().map(|(_, v)| v.clone().into()).collect();
    Json(Response::Success(Success::List(peers)))
}

async fn list_by_cidr(
    State(state): State<Arc<Mutex<DataStore>>>,
    Path(cidr): Path<String>
) -> Json<Response<Peer<String>>> {
    let mut peers = state.lock().await.get_all_users();
    peers.retain(|_, v| v.cidr() == cidr);
    Json(Response::Success(Success::List(peers.iter().map(|(_, v)| v.clone().into()).collect())))
}

async fn delete_expired(
    State(state): State<Arc<Mutex<DataStore>>>
) -> Json<Response<Peer<String>>> {
    let all_peers = state.lock().await.get_all_users();
    let now = match SystemTime::now()
        .duration_since(UNIX_EPOCH) {
            Ok(n) => n.as_secs(),
            Err(_) => return Json(Response::Failure { reason: Some("Unable to get current timestamp".to_string()) }),
    };

    let mut removed_peers = all_peers.clone();
    removed_peers.retain(|_, v| {
        match v.invite_expires() {
            Some(expires) => {
                (expires < now) && (!v.is_redeemed())
            }
            None => false
        }
    });

    let mut datastore = state.lock().await;
    for (id, _) in &removed_peers {
        let op = datastore.network_state.remove_peer_local(id.clone());
        datastore.network_state.peer_op(op);
        let request = PeerRequest::Delete(id.clone()); 
        match datastore.broadcast::<Response<Peer<String>>>(request, "/user/delete").await {
            Ok(()) => return Json(Response::Success(Success::List(removed_peers.iter().map(|(_, v)| v.clone().into()).collect()))),
            Err(e) => eprintln!("Error broadcasting DeletePeerRequest: {e}")
        }
    }

    Json(Response::Success(Success::List(removed_peers.iter().map(|(_, v)| v.clone().into()).collect())))
}

async fn create_cidr(
    State(state): State<Arc<Mutex<DataStore>>>,
    Json(cidr): Json<CidrRequest>,
) -> Json<Response<Cidr<String>>> {
    let mut datastore = state.lock().await;
    match cidr {
        CidrRequest::Op(map_op) => {
            match &map_op {
                crdts::map::Op::Up { ref key, ref op, .. } => {
                    datastore.network_state.cidr_op(map_op.clone());
                    if let (true, v) = datastore.network_state.cidr_op_success(key.clone(), op.clone()) {
                        return Json(Response::Success(Success::Some(v.into())))
                    } else {
                        return Json(Response::Failure { reason: Some("update was rejected".to_string()) })
                    }
                }
                crdts::map::Op::Rm { .. } => {
                    return Json(Response::Failure { reason: Some("Invalid Op type for Create CIDR".into()) });
                }
            }
        }
        CidrRequest::Create(contents) => {
            let map_op = datastore.network_state.update_cidr_local(contents);
            datastore.network_state.cidr_op(map_op.clone());
            match &map_op {
                crdts::map::Op::Rm { .. } => {
                    return Json(Response::Failure { reason: Some("Map generated RM context instead of Add context on Join request".to_string()) });
                }
                crdts::map::Op::Up { ref key, ref op, .. } => {
                    if let (true, v) = datastore.network_state.cidr_op_success(key.clone(), op.clone()) {
                        let request = CidrRequest::Op(map_op);
                        match datastore.broadcast::<Response<Cidr<String>>>(request, "/cidr/create").await {
                            Ok(()) => return Json(Response::Success(Success::Some(v.into()))),
                            Err(e) => eprintln!("Error broadcasting DeletePeerRequest: {e}")
                        }
                        return Json(Response::Success(Success::Some(v.into())))
                    } else {
                        return Json(Response::Failure { reason: Some("update was rejected".to_string()) })
                    }
                }
            }
        }
        _ => {
            return Json(Response::Failure { reason: Some("Invalid request for create cidr".into()) });
        }
    }
} 

async fn update_cidr(
    State(state): State<Arc<Mutex<DataStore>>>,
    Json(cidr): Json<CidrRequest>,
) -> Json<Response<Cidr<String>>> {
    let mut datastore = state.lock().await;
    match cidr {
        CidrRequest::Op(map_op) => {
            match &map_op {
                crdts::map::Op::Up { ref key, ref op, .. } => {
                    datastore.network_state.cidr_op(map_op.clone());
                    if let (true, v) = datastore.network_state.cidr_op_success(key.clone(), op.clone()) {
                        return Json(Response::Success(Success::Some(v.into())))
                    } else {
                        return Json(Response::Failure { reason: Some("update was rejected".to_string()) })
                    }
                }
                crdts::map::Op::Rm { .. } => {
                    return Json(Response::Failure { reason: Some("Invalid Op type for Create CIDR".into()) });
                }
            }
        }
        CidrRequest::Update(contents) => {
            let map_op = datastore.network_state.update_cidr_local(contents);
            datastore.network_state.cidr_op(map_op.clone());
            match &map_op {
                crdts::map::Op::Rm { .. } => {
                    return Json(Response::Failure { reason: Some("Map generated RM context instead of Add context on Join request".to_string()) });
                }
                crdts::map::Op::Up { ref key, ref op, .. } => {
                    if let (true, v) = datastore.network_state.cidr_op_success(key.clone(), op.clone()) {
                        let request = CidrRequest::Op(map_op);
                        match datastore.broadcast::<Response<Cidr<String>>>(request, "/cidr/update").await {
                            Ok(()) => return Json(Response::Success(Success::Some(v.into()))),
                            Err(e) => eprintln!("Error broadcasting DeletePeerRequest: {e}")
                        }
                        return Json(Response::Success(Success::Some(v.into())))
                    } else {
                        return Json(Response::Failure { reason: Some("update was rejected".to_string()) })
                    }
                }
            }
        }
        _ => {
            return Json(Response::Failure { reason: Some("Invalid request for update cidr".into()) });
        }
    }
} 

async fn delete_cidr(
    State(state): State<Arc<Mutex<DataStore>>>,
    Json(cidr): Json<CidrRequest>,
) -> Json<Response<Cidr<String>>> {
    let mut datastore = state.lock().await;
    match cidr {
        CidrRequest::Op(map_op) => {
            match &map_op {
                crdts::map::Op::Up { .. } => {
                    return Json(Response::Failure { reason: Some("Invalid Op type for delete cidr".into()) });
                }
                crdts::map::Op::Rm { .. } => {
                    datastore.network_state.cidr_op(map_op);
                    return Json(Response::Success(Success::None));
                }
            }
        }
        CidrRequest::Delete(contents) => {
            let map_op = datastore.network_state.remove_cidr_local(contents);
            datastore.network_state.cidr_op(map_op.clone());
            match &map_op {
                crdts::map::Op::Rm { .. } => {
                    let request = CidrRequest::Op(map_op);
                    match datastore.broadcast::<Response<Cidr<String>>>(request, "/cidr/delete").await {
                        Ok(()) => return Json(Response::Success(Success::None)),
                        Err(e) => eprintln!("Error broadcasting DeletePeerRequest: {e}")
                    }
                    return Json(Response::Success(Success::None));
                }
                crdts::map::Op::Up { .. } => {
                    return Json(Response::Failure { reason: Some("Map generated Add context instead of Rm context on Delete request".to_string()) });
                }
            }
        }
        _ => {
            return Json(Response::Failure { reason: Some("Invalid request for remove cidr".into()) });
        }
    }
} 

async fn get_cidr(
    State(state): State<Arc<Mutex<DataStore>>>,
    Path(id): Path<String>
) -> Json<Response<Cidr<String>>> {
    if let Some(cidr) = state.lock().await.network_state.cidrs.get(&id).val {
        if let Some(val) = cidr.val() {
            return Json(Response::Success(Success::Some(val.value().into())))
        } else {
            return Json(Response::Failure { reason: Some("Entry exists, but value is None".into()) })
        }
    } 
    Json(Response::Failure { reason: Some("Entry does not exist in the CrdtMap".to_string()) })
} 

async fn list_cidr(
    State(state): State<Arc<Mutex<DataStore>>>,
) -> Json<Response<Cidr<String>>> {
    let cidrs = state.lock().await.get_all_cidrs().iter().map(|(_, v)| v.clone().into()).collect();
    Json(Response::Success(Success::List(cidrs)))
} 

async fn create_assoc(
    State(state): State<Arc<Mutex<DataStore>>>,
    Json(assoc): Json<AssocRequest>
) -> Json<Response<Association<String, (String, String)>>> {
    let mut datastore = state.lock().await;
    match assoc {
        AssocRequest::Op(map_op) => {
            match &map_op {
                crdts::map::Op::Up { ref key, ref op, .. } => {
                    datastore.network_state.associations_op(map_op.clone());
                    if let (true, v) = datastore.network_state.associations_op_success(key.clone(), op.clone()) {
                        return Json(Response::Success(Success::Some(v.into())))
                    } else {
                        return Json(Response::Failure { reason: Some("update was rejected".to_string()) })
                    }
                }
                crdts::map::Op::Rm { .. } => {
                    return Json(Response::Failure { reason: Some("Invalid Op type for Create Association".into()) });
                }
            }
        }
        AssocRequest::Create(contents) => {
            let map_op = datastore.network_state.update_association_local(contents);
            datastore.network_state.associations_op(map_op.clone());
            match &map_op {
                crdts::map::Op::Rm { .. } => {
                    return Json(Response::Failure { reason: Some("Map generated RM context instead of Add context on Join request".to_string()) });
                }
                crdts::map::Op::Up { ref key, ref op, .. } => {
                    if let (true, v) = datastore.network_state.associations_op_success(key.clone(), op.clone()) {
                        let request = AssocRequest::Op(map_op);
                        match datastore.broadcast::<Response<Association<String, (String, String)>>>(request, "/assoc/create").await {
                            Ok(()) => return Json(Response::Success(Success::Some(v.into()))),
                            Err(e) => eprintln!("Error broadcasting DeletePeerRequest: {e}")
                        }
                        return Json(Response::Success(Success::Some(v.into())))
                    } else {
                        return Json(Response::Failure { reason: Some("update was rejected".to_string()) })
                    }
                }
            }
        }
        _ => {
            return Json(Response::Failure { reason: Some("Invalid request for create Association".into()) });
        }
    }
}

async fn delete_assoc(
    State(state): State<Arc<Mutex<DataStore>>>,
    Json(assoc): Json<AssocRequest>,
) -> Json<Response<Association<String, (String, String)>>> {
    let mut datastore = state.lock().await;
    match assoc {
        AssocRequest::Op(map_op) => {
            match &map_op {
                crdts::map::Op::Up { .. } => {
                    return Json(Response::Failure { reason: Some("Invalid Op type for delete association".into()) });
                }
                crdts::map::Op::Rm { .. } => {
                    datastore.network_state.associations_op(map_op);
                    return Json(Response::Success(Success::None));
                }
            }
        }
        AssocRequest::Delete(contents) => {
            let map_op = datastore.network_state.remove_association_local(contents);
            datastore.network_state.associations_op(map_op.clone());
            match &map_op {
                crdts::map::Op::Rm { .. } => {
                    let request = AssocRequest::Op(map_op);
                    match datastore.broadcast::<Response<Association<String, (String, String)>>>(request, "/assoc/delete").await {
                        Ok(()) => return Json(Response::Success(Success::None)),
                        Err(e) => eprintln!("Error broadcasting DeletePeerRequest: {e}")
                    }
                    return Json(Response::Success(Success::None));
                }
                crdts::map::Op::Up { .. } => {
                    return Json(Response::Failure { reason: Some("Map generated Add context instead of Rm context on Delete request".to_string()) });
                }
            }
        }
        _ => {
            return Json(Response::Failure { reason: Some("Invalid request for remove association".into()) });
        }
    }
}

async fn list_assoc(
    State(state): State<Arc<Mutex<DataStore>>>,
) -> Json<Response<Association<String, (String, String)>>> {
    let assocs = state.lock().await.get_all_assocs().iter().map(|(_, v)| v.clone().into()).collect();
    Json(Response::Success(Success::List(assocs)))
}

async fn relationships(
    State(state): State<Arc<Mutex<DataStore>>>,
    Path(cidr_id): Path<String>
) -> Json<Response<Vec<(Cidr<String>, Cidr<String>)>>> {
    let ships = state.lock().await.get_relationships(cidr_id);
    Json(Response::Success(Success::Relationships(ships)))
}

pub async fn request_full_state(to_dial: String) -> Result<DataStore, Box<dyn std::error::Error>> {
    let resp = Client::new()
        .get(format!("http://{to_dial}:3004/bootstrap/full_state"))
        .send().await?.json().await?;
    Ok(resp)
}

pub async fn request_netwok_state(to_dial: String) -> Result<NetworkState, Box<dyn std::error::Error>> {
    let resp = Client::new()
        .get(format!("http://{to_dial}:3004/bootstrap/network_state"))
        .send().await?.json().await?;
    Ok(resp)
}

pub async fn request_peer_state(to_dial: String) -> Result<PeerMap, Box<dyn std::error::Error>> {
    let resp = Client::new()
        .get(format!("http://{to_dial}:3004/bootstrap/peer_state"))
        .send().await?.json().await?;
    Ok(resp)
}

pub async fn request_cidr_state(to_dial: String) -> Result<CidrMap, Box<dyn std::error::Error>> {
    let resp = Client::new()
        .get(format!("http://{to_dial}:3004/bootstrap/cidr_state"))
        .send().await?.json().await?;

    Ok(resp)
}

pub async fn request_associations_state(to_dial: String) -> Result<AssocMap, Box<dyn std::error::Error>> {
    let resp = Client::new()
        .get(format!("http://{to_dial}:3004/bootstrap/assoc_state"))
        .send().await?.json().await?;

    Ok(resp)
}
