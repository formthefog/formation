use std::sync::Arc;
use axum::{extract::{Path, State}, routing::{get, post}, Json, Router};
use crdts::merkle_reg::Sha3Hash;
use tiny_keccak::{Hasher, Sha3};
use tokio::{net::TcpListener, sync::RwLock};
use crate::queue::{FormMQ, QueueRequest, QueueResponse};

pub fn build_routes(state: Arc<RwLock<FormMQ<Vec<u8>>>>) -> Router {
    Router::new()
        .route("/queue/write_op", post(write_op))
        .route("/queue/write_local", post(write_local))
        .route("/queue/:topic/get", get(get_topic_all))
        .route("/queue/:topic/:n/get_n", get(get_topic_n))
        .route("/queue/:topic/:idx/get_after", get(get_topic_after))
        .route("/queue/:topic/:idx/:n/get_n_after", get(get_topic_n_after))
        .route("/queue/get", get(get_all))
        .with_state(state)
}

pub async fn serve(state: Arc<RwLock<FormMQ<Vec<u8>>>>, bind: u16) -> Result<(), Box<dyn std::error::Error>> { 
    let tcp_listener = TcpListener::bind(format!("0.0.0.0:{bind}")).await?;
    if let Err(e) = axum::serve(tcp_listener, build_routes(state)).await {
        return Err(Box::new(e))
    }

    Ok(())
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
    let mut queue = state.write().await;
    match request {
        QueueRequest::Write { content, topic } => {
            match queue.write_local(topic, content) {
                Ok(op) => if queue.op_success(op.clone()) {
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
    let messages = queue.read(topic_hash);
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
    let messages = queue.read(topic_hash);
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
    Path(topic): Path<String>,
    Path(idx): Path<usize>
) -> Json<QueueResponse> {
    let queue = state.read().await;
    let mut hasher = Sha3::v256();
    topic.hash(&mut hasher); 
    let mut topic_hash = [0u8; 32];
    hasher.finalize(&mut topic_hash);
    let messages = queue.read(topic_hash);
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
    let messages = queue.read(topic_hash);
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
pub async fn get_all(
    State(state): State<Arc<RwLock<FormMQ<Vec<u8>>>>>
) -> Json<QueueResponse> {
    let queue = state.read().await;
    let full = queue.queue();

    return Json(QueueResponse::Full(full.topics.iter().map(|ctx| {
        let (topic, queue) = ctx.val; 
        (*topic, queue.messages.iter().map(|(_, v)| {
            v.content.clone()
        }).collect())
    }).collect()))
}
