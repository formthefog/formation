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

/// Initialize the API server with the appropriate routes and middleware
pub async fn init_server(settings: Arc<Settings>) -> std::io::Result<()> {
    // This will be implemented in a future sub-task
    Ok(())
}

/// Define the API routes
pub fn configure_routes(cfg: &mut web::ServiceConfig) {
    // This will be implemented in a future sub-task
}

/// Health check handler
pub async fn health_check() -> actix_web::HttpResponse {
    actix_web::HttpResponse::Ok().json(serde_json::json!({
        "status": "ok",
        "version": crate::MCP_VERSION,
    }))
} 