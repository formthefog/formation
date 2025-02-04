#![allow(unused)]
use std::sync::Arc;
use axum::{extract::{Path, State}, routing::{get, post}, Json, Router};
use tokio::sync::Mutex;
use crate::queue::{FormMQ, QueueRequest, QueueResponse};

pub async fn build_routes(state: Arc<Mutex<FormMQ<Vec<u8>>>>) -> Router<Arc<Mutex<FormMQ<Vec<u8>>>>> {
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

pub async fn write_op(
    State(state): State<Arc<Mutex<FormMQ<Vec<u8>>>>>,
    Json(request): Json<QueueRequest>
) -> Json<QueueResponse> {
    todo!()
}
pub async fn write_local(
    State(state): State<Arc<Mutex<FormMQ<Vec<u8>>>>>,
    Json(request): Json<QueueRequest>
) -> Json<QueueResponse> {
    todo!()
}
pub async fn get_topic_all(
    State(state): State<Arc<Mutex<FormMQ<Vec<u8>>>>>,
    Path(topic): Path<String>
) {
    todo!()
}
pub async fn get_topic_n(
    State(state): State<Arc<Mutex<FormMQ<Vec<u8>>>>>,
    Path(topic): Path<String>,
    Path(n): Path<usize>
) {
    todo!()
}
pub async fn get_topic_after(
    State(state): State<Arc<Mutex<FormMQ<Vec<u8>>>>>,
    Path(topic): Path<String>,
    Path(idx): Path<usize>
) {}

pub async fn get_topic_n_after(
    State(state): State<Arc<Mutex<FormMQ<Vec<u8>>>>>,
    Path(topic): Path<String>,
    Path(idx): Path<usize>,
    Path(n): Path<usize>
) {
    todo!()
}
pub async fn get_all(
    State(state): State<Arc<Mutex<FormMQ<Vec<u8>>>>>
) {
    todo!()
}
