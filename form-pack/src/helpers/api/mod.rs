use crate::manager::FormPackManager;
use crate::auth::{jwt_auth_middleware, AuthConfig, JwtClient};
use crate::api_keys::{api_key_auth_middleware, ApiKeyClient};
use std::sync::Arc;
use tokio::sync::Mutex;
use axum::{Router, routing::{post, get}, middleware};

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
    // Initialize auth and API key clients
    let auth_config = AuthConfig::from_env();
    let jwt_client = Arc::new(JwtClient::new(auth_config));
    let api_key_client = Arc::new(ApiKeyClient::from_env());

    // Build routes with middlewares
    let app = Router::new()
        .route("/ping", post(ping::handle_ping))
        .route("/health", get(health::health_check))
        .route("/build", post(build::handle_pack))
        .route("/:build_id/get_status", get(status::get_status))
        .with_state(manager.clone())
        .with_state(jwt_client.clone())
        .with_state(api_key_client.clone());
    
    // Apply middlewares
    // We use branch to allow either JWT or API key authentication
    let app = app
        .route_layer(middleware::from_fn_with_state(
            jwt_client.clone(),
            jwt_auth_middleware
        ))
        .route_layer(middleware::from_fn_with_state(
            api_key_client,
            api_key_auth_middleware
        ));

    app
}
