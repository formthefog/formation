use std::{collections::HashMap, net::{IpAddr, SocketAddr}, str::FromStr, sync::Arc};
use form_types::PeerType;
use formnet_server::{db::CrdtMap, DatabasePeer};
use serde::{Serialize, Deserialize};
use shared::{Endpoint, NetworkOpts, Peer, PeerContents};
use tokio::{net::TcpListener, sync::RwLock};
use axum::{extract::{ConnectInfo, Path, State}, routing::{get, post}, Json, Router};
use wireguard_control::{AllowedIp, Device, InterfaceName};

use crate::{add_peer, handle_leave_request};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BootstrapInfo {
    pub id: String,
    pub peer_type: PeerType,
    pub cidr_id: String,
    pub pubkey: String,
    pub internal_endpoint: Option<IpAddr>,
    pub external_endpoint: Option<SocketAddr>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Response {
    Join(JoinResponse),
    Bootstrap(BootstrapInfo),
    Fetch(Vec<Peer<String>>),
    Leave,
    Failure { reason: String }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum JoinResponse {
    Success(IpAddr),
    Failure { reason: String }
}

#[derive(Clone, Debug)]
pub struct FormnetApiState {
    pub info: BootstrapInfo,
    pub endpoints: Arc<RwLock<HashMap<String, SocketAddr>>>
}


pub async fn server(
    bootstrap_info: BootstrapInfo,
    endpoints: Arc<RwLock<HashMap<String, SocketAddr>>>
) -> Result<(), Box<dyn std::error::Error>> {
    let bootstrap_info = Arc::new(RwLock::new(FormnetApiState { info: bootstrap_info, endpoints}));

    let router = Router::new()
        .route("/join", post(join))
        .route("/leave", post(handle_leave_request))
        .route("/fetch", get(members))
        .route("/bootstrap", get(bootstrap))
        .route("/:ip/candidates", post(candidates))
        .with_state(bootstrap_info);

    let listener = TcpListener::bind("0.0.0.0:51820").await?;

    axum::serve(listener, router.into_make_service_with_connect_info::<SocketAddr>()).await?;

    Ok(())
}

async fn join(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Json(request): Json<BootstrapInfo>,
) -> Json<Response> {
    log::info!("Received join request");
    match add_peer(&NetworkOpts::default(), &request.peer_type, &request.id, request.external_endpoint, request.pubkey, addr).await {
        Ok(ip) => {
            log::info!("Added peer, returning IP {ip}");
            Json(Response::Join(JoinResponse::Success(ip)))
        }
        Err(e) => {
            Json(Response::Join(JoinResponse::Failure { reason: e.to_string() }))
        }
    } 
}

async fn members(
    State(state): State<Arc<RwLock<FormnetApiState>>>,
) -> Json<Response> {
    if let Ok(ref mut peers) = DatabasePeer::<String, CrdtMap>::list().await{
        inject_endpoints(state.clone(), peers).await;
        Json(Response::Fetch(peers.iter().map(|p| p.inner.clone()).collect()))
    } else {
        Json(Response::Failure { reason: "Unable to retrieve peers from datastore".to_string() })
    }
}

async fn bootstrap(
    State(info): State<Arc<RwLock<FormnetApiState>>>
) -> Json<Response> {
    let reader = info.read().await;
    let info_clone = reader.info.clone();
    drop(reader);
    Json(Response::Bootstrap(info_clone))
}

async fn candidates(
    State(state): State<Arc<RwLock<FormnetApiState>>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Path(ip): Path<String>,
    Json(contents): Json<Vec<Endpoint>> 
) {

    let public_ip = addr.ip();
    let contents = contents.iter().filter_map(|ep| {
        match ep.resolve() {
            Ok(socket_addr) => {
                let ep_port = socket_addr.port(); 
                let pub_ep = Endpoint::from(SocketAddr::new(public_ip, ep_port));
                Some(pub_ep)
            }
            Err(_) => None
        }
    }).collect::<Vec<Endpoint>>();

    if let Ok(ip) = ip.parse::<IpAddr>() {
        if let Ok(device) = Device::get(&InterfaceName::from_str("formnet").unwrap(), NetworkOpts::default().backend) {
            if let Some(peer_info) = device.peers.iter().find(|p| {
                p.config.allowed_ips.contains(&AllowedIp { address: ip, cidr: 8 })
            }) {
                if let Some(current_endpoint) = peer_info.config.endpoint {
                    let mut selected_peer = DatabasePeer::<String, CrdtMap>::get_from_ip(ip).await;
                    match selected_peer {
                        Ok(ref mut dbpeer) => {
                            let _ = dbpeer.update(
                                PeerContents {
                                    endpoint: Some(current_endpoint.into()),
                                    candidates: contents.clone(),
                                    ..dbpeer.contents.clone()
                                }
                            ).await;
                        }
                        Err(e) => {
                            log::error!("Error getting peer, peer may not exist in datatore: {e}");
                        }
                    }
                } else {
                    let mut selected_peer = DatabasePeer::<String, CrdtMap>::get_from_ip(ip).await;
                    if let Some(current_endpoint) = contents.clone().first() {
                        match selected_peer {
                            Ok(ref mut dbpeer) => {
                                let _ = dbpeer.update(
                                    PeerContents {
                                        endpoint: Some(current_endpoint.clone()),
                                        candidates: contents.clone(),
                                        ..dbpeer.contents.clone()
                                    }
                                ).await;
                                let endpoints = state.write().await.endpoints.clone();
                                let mut guard = endpoints.write().await;
                                guard.insert(dbpeer.public_key.clone(), current_endpoint.resolve().unwrap());
                                drop(guard);
                                drop(endpoints);
                            }
                            Err(e) => {
                                log::error!("Error getting peer, peer may not exist in datatore: {e}");
                            }
                        }
                    }
                }
            }
        } else {
            log::info!("unable to acquiire peer with ip: {ip}");
        }
    } else {
        log::error!("Unable to parse provided ip");
    }
}

async fn inject_endpoints(state: Arc<RwLock<FormnetApiState>>, peers: &mut [DatabasePeer<String, CrdtMap>]) {
    let guard = state.read().await;
    let endpoints = guard.endpoints.clone();
    drop(guard);
    let reader = endpoints.read().await;
    for peer in peers {
        if let Some(wg_endpoint) = reader.get(&peer.public_key) {
            if peer.contents.endpoint.is_none() {
                peer.contents.endpoint = Some(wg_endpoint.to_owned().into());
            } else {
                peer.contents.candidates.push(wg_endpoint.to_owned().into());
            }
            let new_contents = peer.contents.clone();
            if let Err(e) = peer.update(new_contents).await {
                log::error!("Error attempting to update peer contents: {e}");
            }
        }
    }
}
