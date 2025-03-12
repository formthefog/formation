// API routes for the MCP server
//
// This file defines the routing for the MCP server API endpoints.

use actix_web::{web, HttpResponse, Responder};
use crate::api::health_check;

/// Configure API routes for the MCP server
pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg
        // Health check endpoint
        .route("/health", web::get().to(health_check))
        
        // MCP protocol endpoints will be added here in future sub-tasks
        
        // Fallback for undefined routes
        .default_service(web::route().to(not_found));
}

/// Handler for undefined routes
async fn not_found() -> impl Responder {
    HttpResponse::NotFound().json(serde_json::json!({
        "status": "error",
        "message": "Resource not found",
    }))
} 