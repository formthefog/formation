use crate::manager::FormPackManager;
use std::sync::Arc;
use tokio::sync::Mutex;
use axum::{Router, routing::{post, get}};

pub mod ping;
pub mod build;
pub mod health;
pub mod status;

pub(crate) async fn serve(addr: String, manager: Arc<Mutex<FormPackManager>>) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("Building routes...");
    let routes = build_routes(manager).await;

    println!("binding listener to addr: {addr}");
    let listener = tokio::net::TcpListener::bind(
        &addr
    ).await?;


    println!("serving server on: {addr}");
    if let Err(e) = axum::serve(listener, routes).await {
        eprintln!("Error in FormPackManager API Server: {e}");
    }

    Ok(())
}

async fn build_routes(manager: Arc<Mutex<FormPackManager>>) -> Router {
    Router::new()
        .route("/ping", post(ping::handle_ping))
        .route("/health", get(health::health_check))
        .route("/build", post(build::handle_pack))
        .route("/:build_id/get_status", get(status::get_status))
        .with_state(manager)
}
