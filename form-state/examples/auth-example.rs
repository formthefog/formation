use axum::{
    Router,
    routing::{get, post},
    middleware::from_fn_with_state,
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use form_state::auth::{
    JWKSManager,
    jwt_auth_middleware, JwtClaims, 
    AdminClaims, UserRole,
    verify_role,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::net::SocketAddr;
use tokio::net::TcpListener;

#[tokio::main]
async fn main() {
    // Initialize logging
    simple_logger::init_with_level(log::Level::Debug).unwrap();
    
    // Load environment variables (in a real app, use dotenv or similar)
    std::env::set_var("DYNAMIC_JWKS_URL", "https://app.example.com/.well-known/jwks");
    std::env::set_var("DYNAMIC_ISSUER", "https://app.example.com");
    std::env::set_var("DYNAMIC_AUDIENCE", "form-network-api");
    
    // Initialize the JWKS manager (could also use init_jwks_manager from form_state::auth::jwks)
    let jwks_manager = Arc::new(JWKSManager::new());
    
    // Build our application with routes
    let app = Router::new()
        // Public route with no auth
        .route("/health", get(health_check))
        
        // Protected routes requiring auth
        .route("/api/public", get(authenticated_endpoint))
        .route("/api/admin", get(admin_only_endpoint))
        .route("/api/projects", post(create_project))
        
        // Use middleware to protect all API routes
        .layer(from_fn_with_state(
            jwks_manager.clone(), 
            jwt_auth_middleware
        ))
        
        // Add the JWKS manager to app state
        .with_state(jwks_manager);
    
    // Run our app
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    log::info!("Listening on {}", addr);
    
    let listener = TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

// Health check endpoint (public, no auth required)
async fn health_check() -> &'static str {
    "OK"
}

// Protected endpoint that requires authentication
async fn authenticated_endpoint(
    claims: JwtClaims,
) -> impl IntoResponse {
    let user_info = format!(
        "Hello, {}! You are authenticated with role: {:?}",
        claims.0.dynamic_user_id.clone().unwrap_or_default(),
        claims.0.user_role()
    );
    
    Json(serde_json::json!({
        "message": "You are authenticated!",
        "user_info": user_info,
    }))
}

// Admin-only endpoint
async fn admin_only_endpoint(
    claims: AdminClaims,
) -> impl IntoResponse {
    let admin_info = format!(
        "Admin: {} ({})",
        claims.0.dynamic_user_id.clone().unwrap_or_default(),
        claims.0.email().unwrap_or("no email")
    );
    
    Json(serde_json::json!({
        "message": "Admin access granted",
        "admin_info": admin_info,
    }))
}

// Data type for creating a project
#[derive(Debug, Deserialize)]
struct CreateProjectRequest {
    name: String,
    description: String,
}

#[derive(Debug, Serialize)]
struct ProjectResponse {
    id: String,
    name: String,
    description: String,
    created_by: String,
}

// Endpoint to create a project (requires authentication)
async fn create_project(
    claims: JwtClaims,
    Json(request): Json<CreateProjectRequest>,
) -> Result<impl IntoResponse, StatusCode> {
    // Check if user has appropriate role
    if let Err(_) = verify_role(&claims.0, UserRole::Developer) {
        return Err(StatusCode::FORBIDDEN);
    }
    
    // Create the project (this would normally interact with a database)
    let project_id = format!("proj_{}", uuid::Uuid::new_v4());
    
    // Return the created project
    let response = ProjectResponse {
        id: project_id,
        name: request.name,
        description: request.description,
        created_by: claims.0.dynamic_user_id.unwrap_or_default(),
    };
    
    Ok((StatusCode::CREATED, Json(response)))
} 