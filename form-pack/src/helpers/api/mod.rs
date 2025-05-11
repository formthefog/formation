use crate::manager::FormPackManager;
use std::sync::Arc;
use tokio::sync::Mutex;
use axum::{Router, routing::{post, get}, middleware};
use std::net::SocketAddr;
use crate::auth::ecdsa_auth_middleware;

pub mod ping;
pub mod build;
pub mod health;
pub mod status;
pub mod write;

pub(crate) async fn serve(addr: String, manager: Arc<Mutex<FormPackManager>>) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("Building routes...");
    let routes = build_routes(manager).await;

    println!("binding listener to addr: {addr}");
    let listener = tokio::net::TcpListener::bind(
        &addr
    ).await?;

    println!("serving server on: {addr}");
    // Use the standard axum server with ConnectInfo
    let app = routes.into_make_service_with_connect_info::<SocketAddr>();
    
    if let Err(e) = axum::serve(listener, app).await {
        eprintln!("Error in FormPackManager API Server: {e}");
    }

    Ok(())
}

async fn build_routes(manager: Arc<Mutex<FormPackManager>>) -> Router {
    // Build routes with middlewares
    let app = Router::new()
        .route("/ping", post(ping::handle_ping))
        .route("/health", get(health::health_check))
        .route("/build", post(build::handle_pack))
        .route("/:build_id/get_status", get(status::get_status))
        .layer(middleware::from_fn_with_state(manager.clone(), ecdsa_auth_middleware))
        .with_state(manager.clone());
    
    app
}
