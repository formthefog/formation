// API routes for the MCP server
//
// This file defines the routing for the MCP server API endpoints.

use actix_web::{web, HttpResponse, Responder};
use crate::api::health_check;
use crate::api::handlers::{tools, operations, auth};
use crate::models::operations::{OperationsRepository, create_repository};

/// Configure API routes for the MCP server
pub fn configure(cfg: &mut web::ServiceConfig) {
    // Create and register the operations repository
    let operations_repository = create_repository();
    cfg.app_data(web::Data::new(operations_repository));
    
    cfg
        // Health check endpoint
        .route("/health", web::get().to(health_check))
        
        // MCP protocol endpoints
        .service(
            web::scope("/api")
                // Authentication endpoints
                .service(
                    web::scope("/auth")
                        .route("/login", web::post().to(auth::login))
                        .route("/validate", web::post().to(auth::validate_token))
                )
                
                // Tool discovery and execution
                .route("/tools", web::get().to(tools::list_tools))
                .route("/tools/{name}", web::post().to(tools::execute_tool))
                
                // Operation status endpoints
                .route("/operations/{id}", web::get().to(operations::get_operation_status))
                .route("/operations", web::get().to(operations::list_operations))
        )
        
        // Version endpoint
        .route("/version", web::get().to(version))
        
        // Fallback for undefined routes
        .default_service(web::route().to(not_found));
}

/// Handler for version endpoint
async fn version() -> impl Responder {
    HttpResponse::Ok().json(serde_json::json!({
        "server": "form-mcp",
        "version": crate::MCP_VERSION,
        "protocol_version": "MCP/0.1"
    }))
}

/// Handler for undefined routes
async fn not_found() -> impl Responder {
    HttpResponse::NotFound().json(serde_json::json!({
        "status": "error",
        "message": "Resource not found",
    }))
} 