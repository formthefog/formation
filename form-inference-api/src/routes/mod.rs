pub mod assistants;
pub mod audio;
pub mod chat;
pub mod embeddings;
pub mod images;
pub mod models;

use axum::{routing::get, Router};
use crate::AppState;

/// Creates a unified API router with all routes
pub fn api_router() -> Router<AppState> {
    Router::new()
        .merge(root_router())
        // .merge(assistants::router())
        // .merge(audio::router())
        .merge(chat::router())
        // .merge(embeddings::router())
        .merge(images::router())
        .merge(models::router())
}

/// Root router with basic info endpoint
fn root_router() -> Router<AppState> {
    Router::new().route("/", get(root))
}

async fn root() -> &'static str {
    "Welcome to the Form Inference API"
}
