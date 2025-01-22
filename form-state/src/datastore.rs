use std::{collections::HashMap, sync::Arc, time::{SystemTime, UNIX_EPOCH}};
use axum::{extract::{State, Path}, routing::{get, post}, Json, Router};
use ditto::{map::Op, MapState};
use reqwest::Client;
use shared::{Association, Cidr, Peer, PeerContents};
use tokio::{net::TcpListener, sync::Mutex};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use crate::network::{CrdtAssociation, CrdtCidr, CrdtPeer, NetworkState};

pub struct DataStore {
    next_site_id: u32,
    network_state: NetworkState,
    // Add Node State
    // Add Instance State
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum NewPeerRequest {
    Op { 
        site_id: u32,
        op: Op<String, CrdtPeer>,
    },
    Join(PeerContents),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum UpdatePeerRequest {
    Op {
        site_id: u32,
        op: Op<String, CrdtPeer>
    },
    Update(PeerContents)
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum DeletePeerRequest {
    Op {
        site_id: u32,
        op: Op<String, CrdtPeer>
    },
    Delete(String),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum PeerResponse {
    Success(Option<Peer>),
    Failure
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum GetPeerResponse {
    Success(CrdtPeer),
    Failure
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum GetPeerListResponse {
    Success(Vec<CrdtPeer>),
    Failure,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum DeleteExpiredResponse {
    Success,
    Failure
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum CreateCidrRequest {
    Op {
        site_id: u32,
        op: Op<String, CrdtCidr>
    },
    Create(Cidr)
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum UpdateCidrRequest {
    Op {
        site_id: u32,
        op: Op<String, CrdtCidr>
    },
    Update(Cidr)
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum DeleteCidrRequest {
    Op {
        site_id: u32,
        op: Op<String, CrdtCidr>
    },
    Delete(String)
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ListCidrResponse {
    Success(Vec<CrdtCidr>),
    Failure
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum CidrResponse {
    Success,
    Failure
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum GetCidrResponse {
    Success(CrdtCidr),
    Failure
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum CreateAssociationRequest {
    Op {
        site_id: u32,
        op: Op<String, CrdtAssociation>,
    },
    Create(Association),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum DeleteAssociationRequest {
    Op {
        site_id: u32,
        op: Op<String, CrdtAssociation>,
    },
    Delete(String),
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum AssociationResponse {
    Success,
    Failure
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ListAssociationResponse {
    Success(Vec<CrdtAssociation>),
    Failure
}


impl DataStore {
    pub fn new(site_id: Option<u32>) -> Result<Self, ditto::Error> {
        let network_state = if let Some(site_id) = site_id {
            NetworkState::new(site_id)?
        } else {
            NetworkState::new(1)?
        };

        Ok(Self {
            network_state,
            next_site_id: site_id.unwrap_or_else(|| 1) + 1, 
        })
    }

    pub fn new_from_state(
        network_state: NetworkState
    ) -> Self {
        let site_id = network_state.peers.site_id();
        Self {
            network_state,
            next_site_id: site_id + 1
        }
    }

    pub fn get_all_users(&self) -> HashMap<String, CrdtPeer> {
        self.network_state.peers.local_value()
    }

    pub fn get_all_active_admin(&mut self) -> HashMap<String, CrdtPeer> {
        let mut peers = self.network_state.peers.local_value();
        peers.retain(|_, v| {
            v.is_admin() && !v.is_disabled() && v.is_redeemed()
        });

        peers
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
            .route("/bootstrap/site_id", get(site_id))
            .route("/bootstrap/peer_state", get(peer_state))
            .route("/bootstrap/cidr_state", get(cidr_state))
            .route("/bootstrap/assoc_state", get(assoc_state))
            .route("/user/create", post(create_user))
            .route("/user/update", post(update_user))
            .route("/user/disable", post(disable_user))
            .route("/user/redeem", post(redeem_invite)) 
            .route("/user/delete", post(delete_user))
            .route("/user/:id/get", get(get_user))
            .route("/user/:ip/get_from_ip", get(get_user_from_ip))
            .route("/user/:id/get_all_allowed", get(get_all_allowed))
            .route("/user/list", get(list_users))
            .route("/user/delete_expired", post(delete_expired))
            .route("/cidr/create", post(create_cidr))
            .route("/cidr/update", post(update_cidr))
            .route("/cidr/delete", post(delete_cidr))
            .route("/cidr/:id/get", get(get_cidr))
            .route("/cidr/list", get(list_cidr))
            .route("/assoc/create", post(create_assoc))
            .route("/assoc/delete", post(delete_assoc))
            .route("/assoc/list", get(list_assoc))
            .with_state(state)
    }

    pub async fn run(self) -> Result<(), Box<dyn std::error::Error>> {
        let router = Self::app(Arc::new(Mutex::new(self)));
        let listener = TcpListener::bind("0.0.0.0:3004").await?;
        let _ = axum::serve(listener, router).await?;

        Ok(())
    }
}

async fn site_id(
    State(state): State<Arc<Mutex<DataStore>>>, 
) -> Json<u32> {
    let next_site_id = state.lock().await.next_site_id;
    Json(next_site_id)
}

async fn peer_state(
    State(state): State<Arc<Mutex<DataStore>>>, 
) -> Json<MapState<'static, String, CrdtPeer>> {
    let peer_state = state.lock().await.network_state.peers.clone_state();
    Json(peer_state)
}

async fn cidr_state(
    State(state): State<Arc<Mutex<DataStore>>>, 
) -> Json<MapState<'static, String, CrdtCidr>> {
    let cidr_state = state.lock().await.network_state.cidrs.clone_state();
    Json(cidr_state)
}

async fn assoc_state(
    State(state): State<Arc<Mutex<DataStore>>>, 
) -> Json<MapState<'static, String, CrdtAssociation>> {
    let assoc_state = state.lock().await.network_state.associations.clone_state();
    Json(assoc_state)
}

async fn create_user(
    State(state): State<Arc<Mutex<DataStore>>>,
    Json(user): Json<NewPeerRequest>
) -> Json<PeerResponse> {
    let mut datastore = state.lock().await;
    let sid = datastore.network_state.peers.site_id().clone();
    match user {
        NewPeerRequest::Op { site_id, op } => {
            match datastore.network_state.peer_op(op.clone(), site_id) {
                Ok(()) => {
                    if let Some(elem) = op.inserted_element() {
                        let peer = elem.value.clone();
                        if peer.is_admin() {
                            datastore.next_site_id += 1;
                        }
                        if let Ok(p) = peer.try_into() {
                            return Json(PeerResponse::Success(Some(p)));
                        } else {
                            return Json(PeerResponse::Failure);
                        }
                    } else {
                        return Json(PeerResponse::Failure);
                    }
                },
                Err(_) => return Json(PeerResponse::Failure),
            }
        },
        NewPeerRequest::Join(peer) => {
            let op = match datastore.network_state.add_peer_local(peer.clone()) {
                Ok(op) => {
                    if peer.is_admin {
                        datastore.next_site_id += 1;
                    }
                    op
                },
                Err(_e) => return Json(PeerResponse::Failure),
            }; 

            let peer = if let Some(elem) = op.inserted_element() {
                if let Ok(peer) = elem.value.clone().try_into() {
                    Some(peer)
                } else {
                    return Json(PeerResponse::Failure)
                }
            } else {
                return Json(PeerResponse::Failure)
            };
            let request = NewPeerRequest::Op { site_id: sid, op };
            match datastore.broadcast::<PeerResponse>(
                request,
                "/user/create"
            ).await {
                Ok(()) => return Json(PeerResponse::Success(peer)),
                Err(e) => {
                    eprintln!("broadcast_new_peer_request failed: {e}");
                } 
            }

            return Json(PeerResponse::Success(peer))
        },
    }
}

async fn update_user(
    State(state): State<Arc<Mutex<DataStore>>>,
    Json(user): Json<UpdatePeerRequest>
) -> Json<PeerResponse> {
    Json(handle_peer_updates(user, state).await)
}

async fn disable_user(
    State(state): State<Arc<Mutex<DataStore>>>,
    Json(user): Json<UpdatePeerRequest>
) -> Json<PeerResponse> {
    Json(handle_peer_updates(user, state).await)
}
async fn redeem_invite(
    State(state): State<Arc<Mutex<DataStore>>>,
    Json(user): Json<UpdatePeerRequest>
) -> Json<PeerResponse> {
    Json(handle_peer_updates(user, state).await)
}
async fn get_user(
    State(state): State<Arc<Mutex<DataStore>>>,
    Path(id): Path<String>
) -> Json<GetPeerResponse> {
    if let Some(peer) = state.lock().await.network_state.peers.get(&id) {
        return Json(GetPeerResponse::Success(peer.clone()))
    } else {
        return Json(GetPeerResponse::Failure)
    }
}

async fn get_user_from_ip(
    State(state): State<Arc<Mutex<DataStore>>>,
    Path(ip): Path<String>
) -> Json<GetPeerResponse> {
    let peers = state.lock().await.get_all_users();
    if let Some(peer) = peers.values().find(|peer| peer.ip().to_string() == ip) {
        Json(GetPeerResponse::Success(peer.clone()))
    } else {
        Json(GetPeerResponse::Failure)
    }
}

async fn get_all_allowed(
    State(state): State<Arc<Mutex<DataStore>>>,
    Path(id): Path<String>,
) -> Json<GetPeerListResponse> {
    let mut peers = state.lock().await.get_all_users();
    if let Some(peer) = state.lock().await.network_state.peers.get(&id) {
        let cidr = peer.cidr();
        peers.retain(|_, v| v.cidr() == cidr);
        let all_allowed = peers.iter().map(|(_, v)| v.clone()).collect();
        Json(GetPeerListResponse::Success(all_allowed))
    } else {
        Json(GetPeerListResponse::Failure)
    }
}

async fn list_users(
    State(state): State<Arc<Mutex<DataStore>>>,
) -> Json<GetPeerListResponse> {
    let peers = state.lock().await.get_all_users().iter().map(|(_, v)| v.clone()).collect();
    Json(GetPeerListResponse::Success(peers))
}
async fn delete_expired(
    State(state): State<Arc<Mutex<DataStore>>>
) -> Json<DeleteExpiredResponse> {
    let mut peers = state.lock().await.get_all_users();
    let now = match SystemTime::now()
        .duration_since(UNIX_EPOCH) {
            Ok(n) => n.as_secs(),
            Err(_) => return Json(DeleteExpiredResponse::Failure),
    };

    peers.retain(|_, v| {
        match v.invite_expires() {
            Some(expires) => {
                (expires < now) && (!v.is_redeemed())
            }
            None => false
        }
    });

    let mut datastore = state.lock().await;
    let sid = datastore.network_state.peers.site_id();
    for (id, _) in peers {
        if let Some(Ok(op)) = datastore.network_state.peers.remove(&id) {
            let request = DeletePeerRequest::Op { site_id: sid, op }; 
            match datastore.broadcast::<PeerResponse>(request, "/user/delete").await {
                Ok(()) => return Json(DeleteExpiredResponse::Success),
                Err(e) => eprintln!("Error broadcasting DeletePeerRequest: {e}")
            };
        }
    }

    Json(DeleteExpiredResponse::Success)
}

async fn delete_user(
    State(state): State<Arc<Mutex<DataStore>>>,
    Json(request): Json<DeletePeerRequest>,
) -> Json<PeerResponse> {
    let mut datastore = state.lock().await;
    let sid = datastore.network_state.peers.site_id().clone();
    match request {
        DeletePeerRequest::Op { site_id, op } => {
            match datastore.network_state.peer_op(op, site_id) {
                Ok(()) => return Json(PeerResponse::Success(None)),
                Err(_) => return Json(PeerResponse::Failure),
            }
        }
        DeletePeerRequest::Delete(peer) => {
            match datastore.network_state.remove_peer_local(peer) {
                Some(Ok(op)) => {
                    let request = DeletePeerRequest::Op { site_id: sid, op };
                    match datastore.broadcast::<PeerResponse>(request, "/user/delete").await {
                        Ok(()) => return Json(PeerResponse::Success(None)),
                        Err(e) => {
                            eprintln!("Error broadcasting DeletePeerRequest {e}");
                        }
                    }
                }
                Some(Err(e)) => {
                    eprintln!("Unable to remove peer locally: {e:?}");
                    return Json(PeerResponse::Failure)
                }
                None => {
                    return Json(PeerResponse::Failure)
                }
            }
        }
    }

    return Json(PeerResponse::Success(None))
}
async fn create_cidr(
    State(state): State<Arc<Mutex<DataStore>>>,
    Json(request): Json<CreateCidrRequest>,
) -> Json<CidrResponse> {
    let mut datastore = state.lock().await;
    let sid = datastore.network_state.cidrs.site_id().clone();
    match request {
        CreateCidrRequest::Op { site_id, op } => {
            match datastore.network_state.cidr_op(op, site_id) {
                Ok(()) => return Json(CidrResponse::Success),
                Err(e) => {
                    eprintln!("Failed to create Cidr locally: {e:?}");
                    return Json(CidrResponse::Failure)
                }
            }
        }
        CreateCidrRequest::Create(cidr) => {
            match datastore.network_state.add_cidr_local(cidr) {
                Ok(op) => {
                    let request = CreateCidrRequest::Op { site_id: sid, op };
                    match datastore.broadcast::<CidrResponse>(request, "/cidr/create").await {
                        Ok(()) => return Json(CidrResponse::Success),
                        Err(e) => {
                            eprintln!("Error broadcasting CreateCidrRequest: {e}");
                            return Json(CidrResponse::Success)
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Error adding CIDR locally: {e:?}");
                    return Json(CidrResponse::Failure)
                }
            }
        }
    }
} 

async fn update_cidr(
    State(state): State<Arc<Mutex<DataStore>>>,
    Json(request): Json<UpdateCidrRequest>,
) -> Json<CidrResponse> {
    let mut datastore = state.lock().await;
    let sid = datastore.network_state.cidrs.site_id().clone();
    match request {
        UpdateCidrRequest::Op { site_id, op } => {
            match datastore.network_state.cidr_op(op, site_id) {
                Ok(()) => return Json(CidrResponse::Success),
                Err(e) => {
                    eprintln!("Failed to create Cidr locally: {e:?}");
                    return Json(CidrResponse::Failure)
                }
            }
        }
        UpdateCidrRequest::Update(cidr) => {
            match datastore.network_state.update_cidr_local(cidr) {
                Ok(op) => {
                    let request = UpdateCidrRequest::Op { site_id: sid, op };
                    match datastore.broadcast::<CidrResponse>(request, "/cidr/update").await {
                        Ok(()) => return Json(CidrResponse::Success),
                        Err(e) => {
                            eprintln!("Error broadcasting UpdateCidrRequest : {e}");
                            return Json(CidrResponse::Success)
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Error updating CIDR locally: {e:?}");
                    return Json(CidrResponse::Failure)
                }
            }
        }
    }
} 

async fn delete_cidr(
    State(state): State<Arc<Mutex<DataStore>>>,
    Json(request): Json<DeleteCidrRequest>,
) -> Json<CidrResponse> {
    let mut datastore = state.lock().await;
    let sid = datastore.network_state.cidrs.site_id().clone();
    match request {
        DeleteCidrRequest::Op { site_id, op } => {
            match datastore.network_state.cidr_op(op, site_id) {
                Ok(()) => return Json(CidrResponse::Success),
                Err(e) => {
                    eprintln!("Failed to remove Cidr locally: {e:?}");
                    return Json(CidrResponse::Failure)
                }
            }
        }
        DeleteCidrRequest::Delete(id) => {
            match datastore.network_state.remove_cidr_local(id) {
                Some(Ok(op)) => {
                    let request = DeleteCidrRequest::Op { site_id: sid, op };
                    match datastore.broadcast::<CidrResponse>(request, "/cidr/delete").await {
                        Ok(()) => return Json(CidrResponse::Success),
                        Err(e) => {
                            eprintln!("Error broadcasting CreateCidrRequest: {e}");
                            return Json(CidrResponse::Success)
                        }
                    }
                }
                Some(Err(e)) => {
                    eprintln!("Error removing CIDR locally: {e:?}");
                    return Json(CidrResponse::Failure)
                }
                None => {
                    eprintln!("Error removing CIDR locally: NotFound");
                    return Json(CidrResponse::Failure)
                }
            }
        }
    }
} 

async fn get_cidr(
    State(state): State<Arc<Mutex<DataStore>>>,
    Path(id): Path<String>
) -> Json<GetCidrResponse> {
    if let Some(peer) = state.lock().await.network_state.cidrs.get(&id) {
        return Json(GetCidrResponse::Success(peer.clone()))
    } else {
        return Json(GetCidrResponse::Failure)
    }
} 

async fn list_cidr(
    State(state): State<Arc<Mutex<DataStore>>>,
) -> Json<ListCidrResponse> {
    let cidrs = state.lock().await.network_state.cidrs.local_value();
    let cidrs_list = cidrs.iter().map(|(_, v)| v.clone()).collect();
    Json(ListCidrResponse::Success(cidrs_list))
} 

async fn create_assoc(
    State(state): State<Arc<Mutex<DataStore>>>,
    Json(request): Json<CreateAssociationRequest>
) -> Json<AssociationResponse> {
    let mut datastore = state.lock().await;
    let sid = datastore.network_state.associations.site_id().clone();
    match request {
        CreateAssociationRequest::Op { site_id, op } => {
            match datastore.network_state.associations_op(op, site_id) {
                Ok(()) => return Json(AssociationResponse::Success),
                Err(e) => {
                    eprintln!("Failed to create Cidr locally: {e:?}");
                    return Json(AssociationResponse::Failure)
                }
            }
        }
        CreateAssociationRequest::Create(assoc) => {
            match datastore.network_state.add_association_local(assoc) {
                Ok(op) => {
                    let request = CreateAssociationRequest::Op { site_id: sid, op };
                    match datastore.broadcast::<AssociationResponse>(request, "/assoc/create").await {
                        Ok(()) => return Json(AssociationResponse::Success),
                        Err(e) => {
                            eprintln!("Error broadcasting CreateCidrRequest: {e}");
                            return Json(AssociationResponse::Success)
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Error adding CIDR locally: {e:?}");
                    return Json(AssociationResponse::Failure)
                }
            }
        }
    }
}

async fn delete_assoc(
    State(state): State<Arc<Mutex<DataStore>>>,
    Json(request): Json<DeleteAssociationRequest>,
) -> Json<AssociationResponse> {
    let mut datastore = state.lock().await;
    let sid = datastore.network_state.associations.site_id().clone();
    match request {
        DeleteAssociationRequest::Op { site_id, op } => {
            match datastore.network_state.associations_op(op, site_id) {
                Ok(()) => return Json(AssociationResponse::Success),
                Err(e) => {
                    eprintln!("Failed to create Cidr locally: {e:?}");
                    return Json(AssociationResponse::Failure)
                }
            }
        }
        DeleteAssociationRequest::Delete(id) => {
            match datastore.network_state.remove_association_local(id) {
                Some(Ok(op)) => {
                    let request = DeleteAssociationRequest::Op { site_id: sid, op };
                    match datastore.broadcast::<AssociationResponse>(request, "/assoc/delete").await {
                        Ok(()) => return Json(AssociationResponse::Success),
                        Err(e) => {
                            eprintln!("Error broadcasting DeleteAssociationRequest: {e}");
                            return Json(AssociationResponse::Success)
                        }
                    }
                }
                Some(Err(e)) => {
                    eprintln!("Error removing CIDR locally: {e:?}");
                    return Json(AssociationResponse::Failure)
                }
                None => {
                    eprintln!("Error removing CIDR locally: NotFound");
                    return Json(AssociationResponse::Failure)
                }
            }
        }
    }

}

async fn list_assoc(
    State(state): State<Arc<Mutex<DataStore>>>,
) -> Json<ListAssociationResponse> {
    let assocs = state.lock().await.network_state.associations.local_value();
    let assocs_list = assocs.iter().map(|(_, v)| v.clone()).collect();
    Json(ListAssociationResponse::Success(assocs_list))
}


async fn handle_peer_updates(update: UpdatePeerRequest, state: Arc<Mutex<DataStore>>) -> PeerResponse {
    let mut datastore = state.lock().await;
    let sid = datastore.network_state.peers.site_id().clone();
    match update {
        UpdatePeerRequest::Op { site_id, op } => {
            match datastore.network_state.peer_op(op, site_id) {
                Ok(()) => return PeerResponse::Success(None),
                Err(_) => return PeerResponse::Failure,
            }
        }
        UpdatePeerRequest::Update(peer) => {
            let op = match datastore.network_state.update_peer_local(peer) {
                Ok(op) => op,
                Err(_e) => return PeerResponse::Failure
            };

            let request = UpdatePeerRequest::Op { site_id: sid, op };
            match datastore.broadcast::<PeerResponse>(request, "/user/update").await {
                Ok(()) => return PeerResponse::Success(None),
                Err(e) => {
                    eprintln!("broadcast_peer_update_request failed: {e}");
                } 
            }
        }
    }

    PeerResponse::Success(None)
}

pub async fn request_site_id(to_dial: String) -> Result<u32, Box<dyn std::error::Error>> {
    let resp = Client::new()
        .get(format!("http://{to_dial}:3004/bootstrap/next_site_id"))
        .send().await?.json().await?;
    Ok(resp)
}

pub async fn request_peer_state(to_dial: String) -> Result<MapState<'static, String, CrdtPeer>, Box<dyn std::error::Error>> {
    let resp = Client::new()
        .get(format!("http://{to_dial}:3004/bootstrap/peer_state"))
        .send().await?.json().await?;
    Ok(resp)
}

pub async fn request_cidr_state(to_dial: String) -> Result<MapState<'static, String, CrdtCidr>, Box<dyn std::error::Error>> {
    let resp = Client::new()
        .get(format!("http://{to_dial}:3004/bootstrap/cidr_state"))
        .send().await?.json().await?;

    Ok(resp)
}

pub async fn request_associations_state(to_dial: String) -> Result<MapState<'static, String, CrdtAssociation>, Box<dyn std::error::Error>> {
    let resp = Client::new()
        .get(format!("http://{to_dial}:3004/bootstrap/assoc_state"))
        .send().await?.json().await?;

    Ok(resp)
}
