use crate::datastore::{DataStore, pong, complete_bootstrap, process_message, full_state};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use axum::{
    Router, 
    routing::{post, get}, 
    middleware, 
    Json,
    extract::{Path, State},
    response::{Response, IntoResponse},
    http::{Request, StatusCode},
    body::Body,
};
use crate::helpers::{
    network::*, 
    nodes::*, 
    instances::*, 
    account::*, 
    agent::*, 
    model::*,
    api_key_handlers::*,
};
use crate::auth::{
    JWKSManager, JwtClaims, jwt_auth_middleware, AuthError,
    verify_project_path_access, has_resource_access, extract_user_info
};
use crate::api_keys::{
    api_key_auth_middleware, ApiKeyAuth
};
use crate::billing::middleware::{check_agent_eligibility, check_token_eligibility};
use tokio::net::TcpListener;
use serde_json::json;
use crate::billing::middleware::EligibilityError;

// Simple node auth middleware to verify formation node key
async fn node_auth_middleware(
    State(state): State<Arc<Mutex<DataStore>>>,
    req: Request<Body>,
    next: middleware::Next,
) -> Result<Response, StatusCode> {
    // Check if we're running in dev mode with internal endpoints allowed
    let allow_internal = std::env::var("ALLOW_INTERNAL_ENDPOINTS")
        .unwrap_or_default()
        .to_lowercase() == "true";
        
    if allow_internal {
        // In dev mode, skip authentication
        log::info!("Dev mode: Skipping node authentication");
        return Ok(next.run(req).await);
    }
    
    // Extract the node key from header
    let node_key = req.headers()
        .get("X-Formation-Node-Key")
        .and_then(|v| v.to_str().ok());
        
    if let Some(key) = node_key {
        // Get trusted operator keys from environment variable
        let trusted_keys = std::env::var("TRUSTED_OPERATOR_KEYS")
            .unwrap_or_default();
            
        // Split comma-separated list of keys
        let trusted_keys: Vec<&str> = trusted_keys
            .split(',')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .collect();
            
        // If no trusted keys are configured, log a warning but allow the request in non-production
        if trusted_keys.is_empty() {
            log::warn!("No TRUSTED_OPERATOR_KEYS configured in environment. This is insecure in production!");
            
            // Check if we're in production
            let is_production = std::env::var("ENVIRONMENT")
                .unwrap_or_default()
                .to_lowercase() == "production";
                
            if is_production {
                log::error!("Rejecting node authentication in production with no trusted keys configured");
                return Err(StatusCode::UNAUTHORIZED);
            } else {
                log::warn!("Allowing request despite missing trusted keys (non-production environment)");
                return Ok(next.run(req).await);
            }
        }
        
        // Verify the key against trusted keys
        if trusted_keys.contains(&key) {
            log::info!("Node authentication successful with key: {}", key);
            return Ok(next.run(req).await);
        } else {
            log::warn!("Node authentication failed: Invalid key provided");
            return Err(StatusCode::UNAUTHORIZED);
        }
    }
    
    // No key provided
    log::warn!("Node authentication failed: No X-Formation-Node-Key header");
    Err(StatusCode::UNAUTHORIZED)
}

pub fn app(state: Arc<Mutex<DataStore>>) -> Router {
    // Create the JWKS manager for JWT validation
    let jwks_manager = Arc::new(JWKSManager::new());
    
    // Define public routes (no authentication required)
    let public_api = Router::new()
        // Health check and bootstrap endpoints
        .route("/ping", get(pong))
        .route("/bootstrap/joined_formnet", post(complete_bootstrap))
        .route("/bootstrap/full_state", get(full_state))
        .route("/bootstrap/network_state", get(network_state))
        .route("/bootstrap/peer_state", get(peer_state))
        .route("/bootstrap/cidr_state", get(cidr_state))
        .route("/bootstrap/assoc_state", get(assoc_state));
    
    // Define network/infrastructure routes (node authentication)
    // These routes are only accessible to Formation nodes via operator key auth
    let network_api = Router::new()
        // User management for networking
        .route("/user/create", post(create_user))
        .route("/user/update", post(update_user))
        .route("/user/disable", post(disable_user))
        .route("/user/redeem", post(redeem_invite)) 
        .route("/user/:id/get", get(get_user))
        .route("/user/:ip/get_from_ip", get(get_user_from_ip))
        .route("/user/delete", post(delete_user))
        .route("/user/:id/get_all_allowed", get(get_all_allowed))
        .route("/user/list", get(list_users))
        .route("/user/list_admin", get(list_admin))
        .route("/user/:cidr/list", get(list_by_cidr))
        .route("/user/delete_expired", post(delete_expired))
        
        // CIDR management
        .route("/cidr/create", post(create_cidr))
        .route("/cidr/update", post(update_cidr))
        .route("/cidr/delete", post(delete_cidr))
        .route("/cidr/:id/get", get(get_cidr))
        .route("/cidr/list", get(list_cidr))
        
        // Association management
        .route("/assoc/create", post(create_assoc))
        .route("/assoc/delete", post(delete_assoc))
        .route("/assoc/list", get(list_assoc))
        .route("/assoc/:cidr_id/relationships", get(relationships))
        
        // DNS management
        .route("/dns/:domain/:build_id/request_vanity", post(request_vanity))
        .route("/dns/:domain/:build_id/request_public", post(request_public))
        .route("/dns/create", post(create_dns))
        .route("/dns/update", post(update_dns))
        .route("/dns/:domain/delete", post(delete_dns))
        .route("/dns/:domain/get", get(get_dns_record))
        .route("/dns/:node_ip/list", get(get_dns_records_by_node_ip))
        .route("/dns/list", get(list_dns_records))
        
        // Node management
        .route("/node/create", post(create_node))
        .route("/node/update", post(update_node))
        .route("/node/:id/get", get(get_node))
        .route("/node/:id/delete", post(delete_node))
        .route("/node/list", get(list_nodes))
        .route("/node/:id/metrics", get(get_node_metrics))
        .route("/node/list/metrics", get(list_node_metrics))
        
        // Apply node authentication middleware
        .layer(middleware::from_fn_with_state(
            state.clone(),
            node_auth_middleware,
        ));
    
    // Define account/user management routes (JWT authentication required)
    let account_api = Router::new()
        // Authentication test endpoints
        .route("/auth/test", get(protected_handler))
        .route("/projects/:project_id/resources/:resource_id", get(project_resource_handler))
        
        // Instance management
        .route("/instance/create", post(create_instance))
        .route("/instance/update", post(update_instance))
        .route("/instance/:instance_id/get", get(get_instance))
        .route("/instance/:build_id/get_by_build_id", get(get_instance_by_build_id))
        .route("/instance/:build_id/get_instance_ips", get(get_instance_ips))
        .route("/instance/:instance_id/delete", post(delete_instance))
        .route("/instance/:instance_id/metrics", get(get_instance_metrics))
        .route("/instance/list/metrics", get(list_instance_metrics))
        .route("/cluster/:build_id/metrics", get(get_cluster_metrics))
        .route("/instance/list", get(list_instances))
        
        // Account management
        .route("/account/:address/get", get(get_account))
        .route("/account/list", get(list_accounts))
        .route("/account/create", post(create_account))
        .route("/account/update", post(update_account))
        .route("/account/delete", post(delete_account))
        .route("/account/transfer-ownership", post(transfer_instance_ownership))
        
        // API key management
        .route("/api-keys", get(list_api_keys_handler))
        .route("/api-keys/create", post(create_api_key_handler))
        .route("/api-keys/:id", get(get_api_key_handler))
        .route("/api-keys/:id/revoke", post(revoke_api_key_handler))
        .route("/api-keys/:id/audit-logs", get(get_api_key_audit_logs))
        .route("/api-keys/audit-logs", get(get_account_api_key_audit_logs))
        
        // Billing and subscription management
        .route("/billing/subscription", get(crate::billing::handlers::get_subscription_status))
        .route("/billing/usage", get(crate::billing::handlers::get_usage_stats))
        .route("/billing/checkout/process", post(crate::billing::handlers::process_stripe_checkout_session))
        .route("/billing/credits/add", post(crate::billing::handlers::add_credits))
        
        // Apply JWT authentication middleware to all account management routes
        .layer(middleware::from_fn_with_state(
            jwks_manager.clone(),
            jwt_auth_middleware,
        ));
    
    // Define API routes (primarily for developers, using API key authentication)
    let api_routes = Router::new()
        // Agent management
        .route("/agents/create", post(create_agent))
        .route("/agents/update", post(update_agent))
        .route("/agents/delete", post(delete_agent))
        .route("/agents/:id", get(get_agent))
        .route("/agents", get(list_agent))
        .route("/agents/:id/hire", post(checked_agent_hire))
        
        // Model management
        .route("/models/create", post(create_model))
        .route("/models/update", post(update_model))
        .route("/models/delete", post(delete_model))
        .route("/models/:id", get(get_model))
        .route("/models", get(list_model))
        .route("/models/:id/inference", post(checked_model_inference))
        
        // Apply API key authentication middleware to all API routes
        .layer(middleware::from_fn_with_state(
            state.clone(),
            api_key_auth_middleware,
        ));
    
    // Merge all route groups into a single router
    Router::new()
        .merge(public_api)
        .merge(network_api)  // Add the node-authenticated network API
        .merge(account_api)
        .merge(api_routes)
        .with_state(state)
}

// Protected route handler example - requires valid JWT
async fn protected_handler(
    claims: JwtClaims,
) -> Json<serde_json::Value> {
    // Access the validated claims
    Json(json!({
        "message": "You have access to this protected route",
        "user_id": claims.0.sub,
        "project": claims.0.project_id(),
        "role": format!("{:?}", claims.0.user_role()),
    }))
}

// Example of a project resource handler using our helper functions
async fn project_resource_handler(
    claims: JwtClaims,
    Path((project_id, resource_id)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, AuthError> {
    // Verify that the user has access to this project
    verify_project_path_access(&claims.0, &project_id)?;
    
    // Let's assume we looked up the resource and found it belongs to this project
    // Now we verify user has access to this specific resource
    has_resource_access(&claims.0, &resource_id, &project_id)?;
    
    // For audit logging, extract user info
    let user_info = extract_user_info(&claims.0);
    
    // Log the access (just printing here, but would log to file/database in a real app)
    log::info!(
        "User accessed resource: project_id={}, resource_id={}, user={}",
        project_id,
        resource_id,
        serde_json::to_string(&user_info).unwrap_or_default()
    );
    
    // Return some data about the resource
    Ok(Json(json!({
        "project_id": project_id,
        "resource_id": resource_id,
        "name": "Example Resource",
        "description": "This is a protected resource that requires authentication and authorization",
        "user": user_info
    })))
}

/// Run the API server without queue processing
pub async fn run_api(datastore: Arc<Mutex<DataStore>>) -> Result<(), Box<dyn std::error::Error>> {
    let router = app(datastore.clone());
    let listener = TcpListener::bind("0.0.0.0:3004").await?;
    log::info!("Running API server only...");
    
    if let Err(e) = axum::serve(listener, router).await {
        eprintln!("Error serving State API Server: {e}");
        return Err(Box::new(e));
    }
    
    Ok(())
}

/// Run the queue reader without the API server
pub async fn run_queue_reader(datastore: Arc<Mutex<DataStore>>, mut shutdown: tokio::sync::broadcast::Receiver<()>) -> Result<(), Box<dyn std::error::Error>> {
    log::info!("Running queue reader only...");
    
    let mut n = 0;
    let polling_interval = 100;
    loop {
        tokio::select! {
            Ok(messages) = DataStore::read_from_queue(Some(n), None) => {
                n += messages.len();
                for message in messages {
                    log::info!("pulled message from queue");
                    let ds = datastore.clone();
                    tokio::spawn(async move {
                        if let Err(e) = process_message(message, ds).await {
                            eprintln!("Error processing message: {e}");
                        }
                    });
                }
            }
            _ = tokio::time::sleep(Duration::from_millis(polling_interval)) => {
            }
            _ = shutdown.recv() => {
                break;
            }
        }
    }

    Ok(())
}

/// Run both the API server and queue reader
pub async fn run(datastore: Arc<Mutex<DataStore>>, mut shutdown: tokio::sync::broadcast::Receiver<()>) -> Result<(), Box<dyn std::error::Error>> {
    let router = app(datastore.clone());
    let listener = TcpListener::bind("0.0.0.0:3004").await?;
    log::info!("Running datastore server with API and queue reader...");
    
    // Start API server
    tokio::spawn(async move {
        if let Err(e) = axum::serve(listener, router).await {
            eprintln!("Error serving State API Server: {e}");
        }
    });

    // Start queue reader
    let mut n = 0;
    let polling_interval = 100;
    loop {
        tokio::select! {
            Ok(messages) = DataStore::read_from_queue(Some(n), None) => {
                n += messages.len();
                for message in messages {
                    log::info!("pulled message from queue");
                    let ds = datastore.clone();
                    tokio::spawn(async move {
                        if let Err(e) = process_message(message, ds).await {
                            eprintln!("Error processing message: {e}");
                        }
                    });
                }
            }
            _ = tokio::time::sleep(Duration::from_millis(polling_interval)) => {
            }
            _ = shutdown.recv() => {
                break;
            }
        }
    }

    Ok(())
}

// Wrapper function that performs eligibility check before calling agent_hire
async fn checked_agent_hire(
    State(state): State<Arc<Mutex<DataStore>>>,
    auth: ApiKeyAuth,
    Path(agent_id): Path<String>,
    payload: Json<serde_json::Value>,
) -> Result<Response, EligibilityError> {
    // Run eligibility check first
    let datastore = state.lock().await;
    let account = auth.account.clone();
    
    // Check if the agent exists
    if datastore.agent_state.get_agent(&agent_id).is_none() {
        return Err(EligibilityError::OperationNotAllowed(format!("Agent not found: {}", agent_id)));
    }
    
    // Use the new centralized credit checking function
    use crate::billing::middleware::{check_operation_credits, OperationType};
    check_operation_credits(&account, OperationType::AgentHire { 
        agent_id: agent_id.clone() 
    })?;
    
    // If we got here, eligibility check passed, so call the actual handler
    drop(datastore); // Release the lock before calling handler
    
    // Call the actual handler with the account context
    let response = agent_hire(State(state), auth, Path(agent_id), payload).await;
    Ok(response.into_response())
}

// Wrapper function that performs eligibility check before calling model_inference
async fn checked_model_inference(
    State(state): State<Arc<Mutex<DataStore>>>,
    auth: ApiKeyAuth,
    Path(model_id): Path<String>, 
    Json(json_payload): Json<serde_json::Value>,
) -> Result<Response, EligibilityError> {
    // Run eligibility check first
    let datastore = state.lock().await;
    let account = auth.account.clone();
    
    // Extract token count from payload
    let input_tokens = json_payload.get("input_tokens")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    
    let output_tokens = json_payload.get("output_tokens")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    
    // Use the new centralized credit checking function
    use crate::billing::middleware::{check_operation_credits, OperationType};
    check_operation_credits(&account, OperationType::TokenConsumption { 
        model_id: model_id.clone(),
        input_tokens,
        output_tokens
    })?;
    
    // If we got here, eligibility check passed, so call the actual handler
    drop(datastore); // Release the lock before calling handler
    
    // Convert the generic JSON to the expected type
    let typed_payload = match serde_json::from_value::<ModelInferenceRequest>(json_payload) {
        Ok(request) => request,
        Err(err) => {
            return Ok(
                Json(json!({
                    "error": "Invalid request format",
                    "details": err.to_string()
                })).into_response()
            );
        }
    };
    
    // Call the actual handler with the properly typed payload
    let response = model_inference(State(state), auth, Path(model_id), Json(typed_payload)).await;
    Ok(response.into_response())
}
