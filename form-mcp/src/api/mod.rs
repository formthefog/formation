// API module for the MCP server
//
// This module contains the API endpoints, handlers, and middleware
// for the MCP server.

mod routes;
pub mod handlers;

use actix_web::{web, App, HttpServer, middleware};
use actix_cors::Cors;
use std::sync::Arc;
use crate::config::Settings;
use crate::tools::ToolRegistry;

/// Initialize the API server with the appropriate routes and middleware
pub async fn init_server(
    settings: Arc<Settings>,
    tool_registry: Arc<ToolRegistry>,
) -> std::io::Result<()> {
    // Get server settings
    let host = settings.server.host.clone();
    let port = settings.server.port;
    let workers = settings.server.workers;
    
    // Initialize HTTP server
    let server = HttpServer::new(move || {
        let settings = settings.clone();
        
        // Set up CORS
        let cors = if settings.server.cors_enabled {
            let mut cors = Cors::default()
                .allow_any_method()
                .allow_any_header()
                .max_age(3600);
            
            // Add allowed origins
            for origin in &settings.server.cors_origins {
                cors = cors.allowed_origin(origin);
            }
            
            cors
        } else {
            Cors::permissive()
        };
        
        App::new()
            // Add shared state
            .app_data(web::Data::new(tool_registry.clone()))
            .app_data(web::Data::new(settings.clone()))
            
            // Add middleware
            .wrap(middleware::Logger::default())
            .wrap(middleware::Compress::default())
            .wrap(middleware::NormalizePath::trim())
            .wrap(cors)
            
            // Configure routes
            .configure(routes::configure)
    })
    .workers(workers)
    .bind(format!("{}:{}", host, port))?;
    
    println!("Starting MCP server at http://{}:{}", host, port);
    
    server.run().await
}

/// Configure API routes for the service
pub fn configure_routes(cfg: &mut web::ServiceConfig) {
    routes::configure(cfg);
}

/// Health check handler
pub async fn health_check() -> actix_web::HttpResponse {
    actix_web::HttpResponse::Ok().json(serde_json::json!({
        "status": "ok",
        "version": crate::MCP_VERSION,
    }))
} 