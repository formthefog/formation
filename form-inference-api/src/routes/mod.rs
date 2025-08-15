pub mod assistants;
pub mod audio;
pub mod chat;
pub mod embeddings;
pub mod models;

use axum::{routing::get, Router};

/// Creates a unified API router with all routes
pub fn api_router() -> Router {
    Router::new()
        .merge(root_router())
        // .merge(assistants::router())
        // .merge(audio::router())
        .merge(chat::router())
        // .merge(embeddings::router())
        .merge(models::router())
}

/// Root router with basic info endpoint
fn root_router() -> Router {
    Router::new()
        .route("/", get(root))
}

async fn root() -> &'static str {
    "Welcome to the Form Inference API"
}