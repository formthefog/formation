use std::sync::Arc;
use axum::{body::Body, extract::{Path, State}, routing::{get, post}, Json, Router};
use crdts::{bft_topic_queue::TopicQueue, merkle_reg::Sha3Hash};
use form_types::state::{Response as StateResponse, Success};
use reqwest::Client;
use shared::Peer;
use tiny_keccak::{Hasher, Sha3};
use tokio::{net::TcpListener, sync::RwLock};
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use bytes::Bytes;
use futures::StreamExt;
use crate::{db::{store_topic_queue, open_db}, queue::{FormMQ, QueueRequest, QueueResponse, QUEUE_PORT}};
use std::path::PathBuf;
use lazy_static::lazy_static;
use redb::Database;

lazy_static! {
    static ref DB_HANDLE: Arc<Database> = open_db(PathBuf::from("/var/lib/formation/db/form.db"));
}


pub async fn bootstrap_topic_queue(dial: String, queue: Arc<RwLock<FormMQ<Vec<u8>>>>) -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new();
    let url = format!("http://{dial}:{QUEUE_PORT}/queue/get");
    let resp = client.get(url).send().await?;

    if !resp.status().is_success() {
        return Err(format!("Request failed with status:{}", resp.status()).into());
    }

    let mut bytes = vec![];
    let mut stream = resp.bytes_stream();

    while let Some(chunk) = stream.next().await {
        match chunk {
            Ok(b) => bytes.extend_from_slice(&b),
            Err(e) => eprintln!("Error receiving chunk: {}", e)
        }
    }

    if !bytes.is_empty() {
        let received = serde_json::from_slice::<TopicQueue<Vec<u8>>>(&bytes)?;
        let mut guard = queue.write().await;
        guard.merge(received);
        drop(guard);
        return Ok(())
    }

    return Err(format!("Bytes were empty after stream").into());

}

pub fn build_routes(state: Arc<RwLock<FormMQ<Vec<u8>>>>) -> Router {
    Router::new()
        .route("/queue/health", get(health_check))
        .route("/queue/write_op", post(write_op))
        .route("/queue/write_local", post(write_local))
        .route("/queue/:topic/get", get(get_topic_all))
        .route("/queue/:topic/:n/get_n", get(get_topic_n))
        .route("/queue/:topic/:idx/get_after", get(get_topic_after))
        .route("/queue/:topic/:idx/:n/get_n_after", get(get_topic_n_after))
        .route("/queue/get", get(get_all))
        .route("/queue/joined_formnet", post(complete_bootstrap))
        .with_state(state)
}

pub async fn serve(state: Arc<RwLock<FormMQ<Vec<u8>>>>, bind: u16) -> Result<(), Box<dyn std::error::Error>> { 
    let tcp_listener = TcpListener::bind(format!("0.0.0.0:{bind}")).await?;
    if let Err(e) = axum::serve(tcp_listener, build_routes(state)).await {
        return Err(Box::new(e))
    }

    Ok(())
}

pub async fn health_check(
) -> String {
    "OK".to_string()
}

pub async fn complete_bootstrap(
    State(state): State<Arc<RwLock<FormMQ<Vec<u8>>>>>
) {
    let client = Client::new(); 
    match client.get("http://127.0.0.1:3004/user/list_admin")
        .send().await {
            Ok(resp) => match resp.json::<StateResponse<Peer<String>>>().await {
                Ok(r) => {
                    match r {
                        StateResponse::Success(Success::List(peers)) => {
                            let mut operator_iter = peers.iter();
                            while let Some(operator) = operator_iter.next() {
                                if let Ok(_) = bootstrap_topic_queue(operator.ip.to_string(), state.clone()).await {
                                    return;
                                }
                            }
                        }
                        _ => {
                            log::error!("Received response {r:?}. Invalid response for /user/list_admin response");
                        }
                    }
                }
                Err(e) => {
                    log::error!("Error trying to deserialize response from localhost:3004/user/list_admin: {e}");
                }
            }
            Err(e) => {
                log::error!("Error attempting to acquire peers from datastore {e}");
            }
        }
}

pub async fn write_op(
    State(state): State<Arc<RwLock<FormMQ<Vec<u8>>>>>,
    Json(request): Json<QueueRequest>
) -> Json<QueueResponse> {
    let mut queue = state.write().await;
    match request {
        QueueRequest::Op(op) => {
            queue.apply(op.clone());
            queue.op_success(op);
            drop(queue);
            let queue = state.read().await.queue().clone();
            let _ = store_topic_queue(&DB_HANDLE, "form-queue", &queue);
            return Json(QueueResponse::OpSuccess)
        }
        _ => {
            return Json(QueueResponse::Failure { reason: Some("Invalid request for write_op endpoint".to_string()) })
        }
    }
}
pub async fn write_local(
    State(state): State<Arc<RwLock<FormMQ<Vec<u8>>>>>,
    Json(request): Json<QueueRequest>
) -> Json<QueueResponse> {
    log::info!("Received write local request");
    let mut queue = state.write().await;
    match request {
        QueueRequest::Write { content, topic } => {
            log::info!("For topic: {topic:?}");
            match queue.write_local(topic, content) {
                Ok(op) => if queue.op_success(op.clone()) {
                    tokio::spawn(async move {
                        if let Err(e) = FormMQ::broadcast_op(op.clone()).await {
                            eprintln!("Error broadcasting op: {e}");
                        }
                    });
                    drop(queue);
                    let inner_state = state.clone();
                    tokio::spawn(async move {
                        let queue = inner_state.read().await.queue().clone();
                        let _ = store_topic_queue(&DB_HANDLE, "form-queue", &queue);
                    });
                    return Json(QueueResponse::OpSuccess)
                } else {
                    return Json(QueueResponse::Failure { reason: Some(format!("Error trying to write local: Op not successfully written to queue.")) })
                }
                Err(e) => return Json(QueueResponse::Failure { reason: Some(format!("Error trying to write local: {e}")) })
            }
        }
        _ => {
            return Json(QueueResponse::Failure { reason: Some("Invalid request for write_op endpoint".to_string()) })
        }
    }
}
pub async fn get_topic_all(
    State(state): State<Arc<RwLock<FormMQ<Vec<u8>>>>>,
    Path(topic): Path<String>
) -> Json<QueueResponse> {
    let queue = state.read().await;
    let mut hasher = Sha3::v256();
    topic.hash(&mut hasher); 
    let mut topic_hash = [0u8; 32];
    hasher.finalize(&mut topic_hash);
    let messages = queue.read(hex::encode(topic_hash));
    if let Some(contents) = messages {
        return Json(QueueResponse::List(contents.iter().map(|m| m.content.clone()).collect()));
    }

    return Json(QueueResponse::Failure { reason: Some(format!("Unable to acquire messages for {topic}")) });
}

pub async fn get_topic_n(
    State(state): State<Arc<RwLock<FormMQ<Vec<u8>>>>>,
    Path(topic): Path<String>,
    Path(n): Path<usize>
) -> Json<QueueResponse> {
    let queue = state.read().await;
    let mut hasher = Sha3::v256();
    topic.hash(&mut hasher); 
    let mut topic_hash = [0u8; 32];
    hasher.finalize(&mut topic_hash);
    let messages = queue.read(hex::encode(topic_hash));
    if let Some(contents) = messages {
        let list = if contents.len() - 1 >= n {
            contents[..n].iter().map(|m| m.content.clone()).collect()
        } else {
            contents.iter().map(|m| m.content.clone()).collect()
        };
        return Json(QueueResponse::List(list));
    }

    return Json(QueueResponse::Failure { reason: Some(format!("Unable to acquire message for {topic}")) })
}
pub async fn get_topic_after(
    State(state): State<Arc<RwLock<FormMQ<Vec<u8>>>>>,
    Path((topic, idx)): Path<(String, usize)>,
) -> Json<QueueResponse> {
    let queue = state.read().await;
    let mut hasher = Sha3::v256();
    topic.hash(&mut hasher); 
    let mut topic_hash = [0u8; 32];
    hasher.finalize(&mut topic_hash);
    let messages = queue.read(hex::encode(topic_hash));
    if let Some(contents) = messages {
        let list = if (contents.len() - 1) >= idx {
            contents[idx..].iter().map(|m| m.content.clone()).collect()
        } else {
            return Json(QueueResponse::Failure { reason: Some(format!("Queue is shorter than {idx} for topic {topic}")) })
        };
        return Json(QueueResponse::List(list));

    }

    return Json(QueueResponse::Failure { reason: Some(format!("Unable to acquire message for {topic}")) })
}

pub async fn get_topic_n_after(
    State(state): State<Arc<RwLock<FormMQ<Vec<u8>>>>>,
    Path(topic): Path<String>,
    Path(idx): Path<usize>,
    Path(n): Path<usize>
) -> Json<QueueResponse> {
    let queue = state.read().await;
    let mut hasher = Sha3::v256();
    topic.hash(&mut hasher); 
    let mut topic_hash = [0u8; 32];
    hasher.finalize(&mut topic_hash);
    let messages = queue.read(hex::encode(topic_hash));
    if let Some(contents) = messages {
        let list = if (contents.len() - 1) >= idx {
            let contents_after = &contents[idx..];
            if (contents_after.len() - 1) >= n {
                contents_after[..n].iter().map(|m| m.content.clone()).collect()
            } else {
                contents_after.iter().map(|m| m.content.clone()).collect()
            }
        } else {
            return Json(QueueResponse::Failure { reason: Some(format!("Queue is shorter than {idx} for topic {topic}")) })
        };
        return Json(QueueResponse::List(list));

    }

    return Json(QueueResponse::Failure { reason: Some(format!("Unable to acquire message for {topic}")) })
}

/// Returns a streaming response where the full TopicQueue is written as a JSON array.
/// In this example we assume that each topic in the TopicQueue will be sent as a tuple of (topic_name, bft_queue).
pub async fn get_all(
    State(state): State<Arc<RwLock<FormMQ<Vec<u8>>>>>
) -> impl IntoResponse {
    log::info!("Received request for full queue");
    // Read the current queue (or clone what you need)
    log::info!("acquiring read only lock");
    let queue = state.read().await;
    log::info!("cloning queue");
    let topic_queue: TopicQueue<Vec<u8>> = queue.queue().clone();
    log::info!("converting queue into body");
    let body = Body::from(Bytes::copy_from_slice(&serde_json::to_vec(&topic_queue).unwrap()));
    
    log::info!("Building response");
    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .body(body.into_data_stream())
        .unwrap()
}
