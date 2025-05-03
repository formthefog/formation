use crate::manager::FormPackManager;
use crate::auth::SignatureAuthConfig;
use crate::auth::signature_auth_middleware;
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
    // Initialize auth config
    let signature_auth_config = Arc::new(SignatureAuthConfig::from_env());

    // Create public routes with no auth
    let public_routes = Router::new()
        .route("/ping", post(ping::handle_ping))
        .route("/health", get(health::health_check));
    
    // Create protected routes that require auth
    let protected_routes = Router::new()
        .route("/build", post(build::handle_pack))
        .route("/:build_id/get_status", get(status::get_status));
    
    // Combine all routes
    Router::new()
        .merge(public_routes)
        .merge(
            // Apply signature auth middleware to protected routes only
            protected_routes
                .route_layer(middleware::from_fn_with_state(
                    signature_auth_config.clone(),
                    signature_auth_middleware
                ))
        )
        .with_state(manager.clone())
        .with_state(signature_auth_config)
}
