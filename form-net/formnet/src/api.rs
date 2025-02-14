use std::{net::{IpAddr, SocketAddr}, sync::Arc};
use form_types::PeerType;
use formnet_server::{db::CrdtMap, DatabasePeer};
use serde::{Serialize, Deserialize};
use shared::{Endpoint, NetworkOpts, Peer, PeerContents};
use tokio::{net::TcpListener, sync::RwLock};
use axum::{extract::{ConnectInfo, Path, State}, routing::{get, post}, Json, Router};

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


pub async fn server(
    bootstrap_info: BootstrapInfo
) -> Result<(), Box<dyn std::error::Error>> {
    let bootstrap_info = Arc::new(RwLock::new(bootstrap_info));

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
    match add_peer(&NetworkOpts::default(), &request.peer_type, &request.id, request.pubkey, request.external_endpoint, addr).await {
        Ok(ip) => {
            log::info!("Added peer, returning IP {ip}");
            Json(Response::Join(JoinResponse::Success(ip)))
        }
        Err(e) => {
            Json(Response::Join(JoinResponse::Failure { reason: e.to_string() }))
        }
    } 
}

async fn members() -> Json<Response> {
    if let Ok(peers) = DatabasePeer::<String, CrdtMap>::list().await{
        Json(Response::Fetch(peers.iter().map(|p| p.inner.clone()).collect()))
    } else {
        Json(Response::Failure { reason: "Unable to retrieve peers from datastore".to_string() })
    }
}

async fn bootstrap(
    State(info): State<Arc<RwLock<BootstrapInfo>>>
) -> Json<Response> {
    let info_clone = info.read().await.clone();
    Json(Response::Bootstrap(info_clone))
}

async fn candidates(
    Path(ip): Path<String>,
    Json(contents): Json<Vec<Endpoint>> 
) {
    if let Ok(ip) = ip.parse::<IpAddr>() {
        if let Ok(mut selected_peer) = DatabasePeer::<String, CrdtMap>::get_from_ip(ip.clone()).await {
            if let Ok(_) = selected_peer.update(
                PeerContents {
                    candidates: contents,
                    ..selected_peer.contents.clone()
                },
            ).await {
                log::info!("Succesfully updated peer with candidates...");
            } else {
                log::info!("Unable to update peer with candidates...");
            }
        } else {
            log::info!("unable to acquiire peer with ip: {ip}");
        }
    } else {
        log::error!("Unable to parse provided ip");
    }
}
