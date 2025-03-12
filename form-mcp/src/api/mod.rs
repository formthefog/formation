// API module for the MCP server
//
// This module handles the HTTP API endpoints for the MCP server,
// including request handling, routing, and response formatting.

pub mod routes;
pub mod handlers;

use std::sync::Arc;
use actix_web::{web, App, HttpServer, middleware};
use actix_web::middleware::Compress;
use actix_cors::Cors;
use log::info;
use crate::config::Settings;
use crate::tools::ToolRegistry;
use crate::auth;

/// Initialize the API server with the appropriate routes and middleware
pub async fn init_server(
    settings: Arc<Settings>,
    tool_registry: Arc<ToolRegistry>,
) -> std::io::Result<()> {
    // Create a tool registry data object
    let tool_registry_data = web::Data::new(tool_registry);
    
    // Get server settings
    let host = settings.server.host.clone();
    let port = settings.server.port;
    let workers = settings.server.workers;
    
    // Configure authentication
    let enable_auth = settings.auth.enabled;
    let auth_middleware = auth::AuthenticationMiddleware::new(enable_auth);
    
    // Log startup information
    info!("Starting MCP server on {}:{}", host, port);
    info!("Authentication enabled: {}", enable_auth);
    
    // Create and start the HTTP server
    HttpServer::new(move || {
        // Configure CORS if enabled
        let cors = if settings.server.cors_enabled {
            // Create a permissive CORS configuration for development
            // In production, this should be more restrictive
            Cors::default()
                .allow_any_origin()
                .allow_any_method()
                .allow_any_header()
                .max_age(3600)
        } else {
            // Create a default CORS configuration that denies all cross-origin requests
            Cors::default()
        };
        
        App::new()
            // Register the tool registry
            .app_data(tool_registry_data.clone())
            // Set request timeout
            .app_data(web::PayloadConfig::new(settings.server.request_timeout as usize))
            // Enable compression
            .wrap(Compress::default())
            // Add CORS middleware
            .wrap(cors)
            // Add authentication middleware
            .wrap(auth_middleware.clone())
            // Configure routes
            .configure(routes::configure)
    })
    .workers(workers)
    .bind((host, port))?
    .run()
    .await
}

/// Simple health check endpoint
pub async fn health_check() -> actix_web::HttpResponse {
    actix_web::HttpResponse::Ok().json(serde_json::json!({
        "status": "ok",
        "version": crate::MCP_VERSION
    }))
} 