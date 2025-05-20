use crate::datastore::{DataStore, pong, process_message, full_state};
use std::sync::Arc;
use std::time::Duration;
use std::str::FromStr;
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
use serde::{Serialize, Deserialize};
use crate::helpers::{
    network::*, 
    nodes::*, 
    instances::*, 
    account::*, 
    agent::*, 
    model::*,
    agent_gateway::run_agent_task_handler,
};
use crate::auth::{
    RecoveredAddress, ecdsa_auth_middleware, active_node_auth_middleware
};

use serde_json::json;
use crate::billing::middleware::EligibilityError;
use hex;
use form_node_metrics::{capabilities::NodeCapabilities, capacity::NodeCapacity, metrics::NodeMetrics};
use crate::tasks::{TaskStatus as FormStateTaskStatus, TaskId as FormStateTaskId};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum HealthStatus {
    Healthy,
    Degraded { reason: String },
    Unhealthy { reason: String }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthResponse {
    status: HealthStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    uptime: Option<u64>
}

async fn health_check() -> Json<HealthResponse> {
    // Get the version from Cargo.toml if available
    let version = option_env!("CARGO_PKG_VERSION").map(String::from);
    
    // Return a healthy status
    Json(HealthResponse {
        status: HealthStatus::Healthy,
        version,
        uptime: None // Could add uptime calculation if needed
    })
}

// Node authentication middleware to verify formation node key
async fn node_auth_middleware(
    State(state): State<Arc<Mutex<DataStore>>>,
    req: Request<Body>,
    next: middleware::Next,
) -> Result<Response, StatusCode> {
    // Allow localhost access without authentication for bootstrap purposes
    if is_localhost_request(&req) {
        log::info!("Allowing localhost request without authentication");
        return Ok(next.run(req).await);
    }
    
    // Check if we're running in dev mode with internal endpoints allowed
    let allow_internal = std::env::var("ALLOW_INTERNAL_ENDPOINTS")
        .unwrap_or_default()
        .to_lowercase() == "true";
        
    if allow_internal {
        // In dev mode, skip authentication
        log::info!("Dev mode: Skipping authentication");
        return Ok(next.run(req).await);
    }
    
    // Extract headers for ECDSA verification
    let headers = req.headers().clone();
    
    // Extract signature parts and verify
    use crate::auth::{extract_signature_parts, recover_address, SignatureError};
    
    // Extract and verify the signature
    let (signature_bytes, recovery_id, message) = match extract_signature_parts(&headers) {
        Ok(parts) => parts,
        Err(SignatureError::MissingSignature) => {
            log::warn!("Authentication failed: Missing signature");
            return Err(StatusCode::UNAUTHORIZED);
        },
        Err(_) => {
            log::warn!("Authentication failed: Invalid signature format");
            return Err(StatusCode::UNAUTHORIZED);
        }
    };
    
    // Recover the address from the signature
    let address = match recover_address(&signature_bytes, recovery_id, &message) {
        Ok(addr) => addr,
        Err(_) => {
            log::warn!("Authentication failed: Could not recover address from signature");
            return Err(StatusCode::UNAUTHORIZED);
        }
    };
    
    // Convert address to hex string
    let address_hex = hex::encode(address.as_slice());
    log::debug!("Recovered address from signature: 0x{}", address_hex);
        
    // Lock the datastore to check admin status
    let datastore = state.lock().await;
    
    // Check if this address belongs to an account with global admin privileges
    let is_system_admin = datastore.account_state.get_account(&address_hex)
        .map_or(false, |acc| acc.is_global_admin);
    
    if is_system_admin {
        log::info!("Node authentication successful: Address 0x{} is a global system admin", address_hex);
        Ok(next.run(req).await)
    } else {
        log::warn!("Authentication failed: Address 0x{} is not a global system admin", address_hex);
        Err(StatusCode::UNAUTHORIZED)
    }
}

// Helper function to check if a request is coming from localhost
pub fn is_localhost_request(req: &Request<Body>) -> bool {
    log::info!("Checking if request is from localhost");
    
    // Check from connection info (direct connections)
    if let Some(addr) = req.extensions().get::<axum::extract::ConnectInfo<std::net::SocketAddr>>() {
        let ip = addr.ip();
        log::info!("  Connection info IP: {}", ip);
        if ip.is_loopback() {
            log::info!("  Localhost detected via connection info (loopback)");
            return true;
        }
    } else {
        log::info!("  No connection info available");
    }
    
    // Check headers for proxy info
    if let Some(forwarded) = req.headers().get("x-forwarded-for") {
        if let Ok(addr) = forwarded.to_str() {
            let first_ip = addr.split(',').next().unwrap_or("").trim();
            log::info!("  X-Forwarded-For header: {}", first_ip);
            if first_ip == "127.0.0.1" || first_ip == "::1" || first_ip.starts_with("localhost") {
                log::info!("  Localhost detected via X-Forwarded-For");
                return true;
            }
        } else {
            log::info!("  X-Forwarded-For header exists but couldn't be parsed");
        }
    } else {
        log::info!("  No X-Forwarded-For header");
    }
    
    // Check if host header indicates localhost
    if let Some(host) = req.headers().get("host") {
        if let Ok(host_str) = host.to_str() {
            log::info!("  Host header: {}", host_str);
            if host_str.starts_with("localhost:") || host_str == "localhost" || host_str.starts_with("127.0.0.1") || host_str.starts_with("::1") {
                log::info!("  Localhost detected via Host header");
                return true;
            }
        } else {
            log::info!("  Host header exists but couldn't be parsed");
        }
    } else {
        log::info!("  No Host header");
    }
    
    log::info!("  Not a localhost request");
    false
}

// Helper function to determine if a path is for a public endpoint
pub fn is_public_endpoint(path: &str) -> bool {
    log::info!("Checking if path '{}' is a public endpoint", path);
    
    // List of paths that are safe for public access
    let public_paths = [
        // Agents endpoints
        "/agents",
        "/agents/",
        "/agent/",
        
        // Instance endpoints
        "/instances",
        "/instance/list",
        "/instance/",
        
        // Model endpoints
        "/models",
        "/models/",
        "/model/",
        
        // Account endpoints (minimal info only)
        "/accounts",
        
        // Node endpoints
        "/nodes",
        "/node/list"
    ];
    
    // Log all path checks for debugging
    for &prefix in &public_paths {
        let matches = path.starts_with(prefix);
        log::info!("  Checking against '{}': {}", prefix, if matches { "MATCH" } else { "no match" });
        if matches {
            return true;
        }
    }
    
    log::info!("  No public path match found for '{}'", path);
    false
}

pub fn app(state: Arc<Mutex<DataStore>>) -> Router {
    
    // Define public routes (no authentication required)
    let public_api = Router::new()
        // Health check and bootstrap endpoints
        .route("/ping", get(pong))
        .route("/health", get(health_check))
        .route("/bootstrap/joined_formnet", post(crate::datastore::complete_bootstrap))
        .route("/bootstrap/full_state", get(full_state))
        .route("/bootstrap/network_state", get(network_state))
        .route("/bootstrap/peer_state", get(peer_state))
        .route("/bootstrap/cidr_state", get(cidr_state))
        .route("/bootstrap/assoc_state", get(assoc_state))
        .route("/bootstrap/ensure_admin_account", post(ensure_admin_account))
        // Add read-only endpoints for non-sensitive data
        .route("/agents", get(list_agents))
        .route("/agents/:id", get(get_agent))
        .route("/models", get(list_model))
        .route("/models/:id", get(get_model))
        .route("/node/list", get(list_nodes))
        .route("/instance/:instance_id/metrics", get(get_instance_metrics))
        .route("/instance/list/metrics", get(list_instance_metrics))
        .route("/cluster/:build_id/metrics", get(get_cluster_metrics));
    
    // Keep the original network API routes
    let network_writers_api = Router::new()
        .route("/user/create", post(create_user))
        .route("/user/update", post(update_user))
        .route("/user/disable", post(disable_user))
        .route("/user/delete", post(delete_user))
        .route("/user/delete_expired", post(delete_expired))
        .route("/cidr/create", post(create_cidr))
        .route("/cidr/update", post(update_cidr))
        .route("/cidr/delete", post(delete_cidr))
        .route("/assoc/create", post(create_assoc))
        .route("/assoc/delete", post(delete_assoc))
        .route("/assoc/list", get(list_assoc))
        .route("/dns/create", post(create_dns))
        .route("/dns/update", post(update_dns))
        .route("/dns/:domain/delete", post(delete_dns))
        .route("/node/create", post(create_node))
        .route("/node/update", post(update_node))
        .route("/node/:id/get", get(get_node))
        .route("/node/:id/delete", post(delete_node))
        .route("/node/:id/report_metrics", post(report_node_metrics))
        .route("/user/redeem", post(redeem_invite))
        // Task Update Endpoint
        .route("/task/update_status", post(update_task_status_handler))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            node_auth_middleware,
        ));
    
    // Define network/infrastructure routes (node authentication)
    // These routes are only accessible to Formation nodes via operator key auth
    let network_readers_api = Router::new()
        // User management for networking
        .route("/user/:id/get", get(get_user))
        .route("/user/:ip/get_from_ip", get(get_user_from_ip))
        .route("/user/:id/get_all_allowed", get(get_all_allowed))
        .route("/user/list", get(list_users))
        .route("/user/list_admin", get(list_admin))
        .route("/peer/list_active", get(list_active_peers))
        .route("/user/:cidr/list", get(list_by_cidr))        
        // CIDR management
        .route("/cidr/:id/get", get(get_cidr))
        .route("/cidr/list", get(list_cidr))
        
        // Association management
        .route("/assoc/:cidr_id/relationships", get(relationships))
        
        // DNS management
        .route("/dns/:domain/:build_id/request_vanity", post(request_vanity))
        .route("/dns/:domain/:build_id/request_public", post(request_public))
        .route("/dns/:domain/get", get(get_dns_record))
        .route("/dns/:node_ip/list", get(get_dns_records_by_node_ip))
        .route("/dns/list", get(list_dns_records))
        
        // Node management
        .route("/node/:id/metrics", get(get_node_metrics))
        .route("/node/list/metrics", get(list_node_metrics))
        // Task related endpoints
        .route("/task/:task_id/is_responsible/:node_id_to_check", get(check_task_responsibility))
        
        // Node authentication key management
        .route("/node/:id/operator-key", post(add_node_operator_key))
        .route("/node/:id/operator-key/:key", post(remove_node_operator_key))
        // Task query endpoints
        .route("/tasks", get(list_tasks_handler))
        .route("/task/:task_id/get", get(get_task_handler));
        
    let account_api = Router::new()
        // Account management
        .route("/account/:address/get", get(get_account))
        .route("/account/list", get(list_accounts))
        .route("/account/create", post(create_account))
        .route("/account/update", post(update_account))
        .route("/account/delete", post(delete_account))
        .route("/account/:address/is_global_admin", get(is_global_admin_handler))
        .route("/account/transfer-ownership", post(transfer_instance_ownership))
        // Apply ECDSA auth middleware to all account management routes
        .layer(middleware::from_fn_with_state(
            state.clone(),
            ecdsa_auth_middleware
        ));
    
    // User-authenticated instance API routes 
    let instance_api = Router::new()
        .route("/instance/create", post(create_instance))
        .route("/instance/update", post(update_instance))
        .route("/instance/:instance_id/delete", post(delete_instance))
        .route("/instance/list", get(list_instances))
        .route("/instance/:instance_id/get", get(get_instance))
        .route("/instance/:build_id/get_by_build_id", get(get_instance_by_build_id))
        .route("/instance/:build_id/get_instance_ips", get(get_instance_ips))
        // Apply ECDSA auth middleware to all instance API routes
        .layer(middleware::from_fn_with_state(
            state.clone(),
            ecdsa_auth_middleware
        ));

    // Define API routes (primarily for developers, using API key authentication)
    let api_routes = Router::new()
        // Agent management
        .route("/agents/create", post(create_agent))
        .route("/agents/update", post(update_agent))
        .route("/agents/delete", post(delete_agent))
        .route("/agents/:id/hire", post(checked_agent_hire))
        .route("/agents/:agent_id/run_task", post(run_agent_task_handler))
        
        // Model management
        .route("/models/create", post(create_model))
        .route("/models/update", post(update_model))
        .route("/models/delete", post(delete_model))
        .route("/models/:id/inference", post(checked_model_inference))
        
        // Apply ECDSA authentication middleware to all API routes
        .layer(middleware::from_fn_with_state(
            state.clone(),
            ecdsa_auth_middleware
        ));
    
    // New router for devnet gossip, protected by active_node_auth_middleware
    let devnet_gossip_api = Router::new()
        .route("/apply_op", post(devnet_apply_op_handler))
        .layer(middleware::from_fn_with_state(
            state.clone(), 
            crate::auth::active_node_auth_middleware,
        ));
    
    // Merge all route groups into a single router
    Router::new()
        .merge(public_api)
        .merge(network_writers_api)  
        .merge(network_readers_api)
        .nest("/devnet_gossip", devnet_gossip_api) // Nest the new router
        .merge(account_api)
        .merge(instance_api)  
        .merge(api_routes)
        .with_state(state)
}

/// Run the API server without queue processing
pub async fn run_api(datastore: Arc<Mutex<DataStore>>) -> Result<(), Box<dyn std::error::Error>> {
    let router = app(datastore.clone());
    let addr = "0.0.0.0:3004".parse::<std::net::SocketAddr>()?;
    
    let socket = tokio::net::TcpListener::bind(addr).await?;
    log::info!("Running API server only at {}", addr);
    
    if let Err(e) = axum::serve(
        socket,
        router.into_make_service_with_connect_info::<std::net::SocketAddr>()
    ).await {
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
    let addr = "0.0.0.0:3004".parse::<std::net::SocketAddr>()?;
    
    let socket = tokio::net::TcpListener::bind(addr).await?;
    log::info!("Running datastore server with API and queue reader at {}", addr);
    
    // Start API server
    tokio::spawn(async move {
        if let Err(e) = axum::serve(
            socket,
            router.into_make_service_with_connect_info::<std::net::SocketAddr>()
        ).await {
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
    recovered: RecoveredAddress,
    Path(agent_id): Path<String>,
    payload: Json<serde_json::Value>,
) -> Result<Response, EligibilityError> {
    // Run eligibility check first
    let mut datastore = state.lock().await;
    
    // Get the account address
    let account_address = recovered.as_hex();
    
    // Check if the agent exists
    if datastore.agent_state.get_agent(&agent_id).is_none() {
        return Err(EligibilityError::OperationNotAllowed(format!("Agent not found: {}", agent_id)));
    }
    
    // Get or create the account
    let account = match datastore.account_state.get_account(&account_address) {
        Some(acc) => acc,
        None => {
            // Create a new account if it doesn't exist
            let new_account = crate::accounts::Account::new(account_address.clone());
            let op = datastore.account_state.update_account_local(new_account.clone());
            if let Err(e) = datastore.handle_account_op(op).await {
                return Err(EligibilityError::OperationNotAllowed(
                    format!("Failed to create account: {}", e)
                ));
            }
            new_account
        }
    };
    
    // Use the new centralized credit checking function
    use crate::billing::middleware::{check_operation_credits, OperationType};
    check_operation_credits(&account, OperationType::AgentHire { 
        agent_id: agent_id.clone() 
    })?;
    
    // If we got here, eligibility check passed, so call the actual handler
    drop(datastore); // Release the lock before calling handler
    
    // Call the actual handler with the account context
    let response = agent_hire(State(state), recovered, Path(agent_id), payload).await;
    Ok(response.into_response())
}

// Wrapper function that performs eligibility check before calling model_inference
async fn checked_model_inference(
    State(state): State<Arc<Mutex<DataStore>>>,
    recovered: RecoveredAddress,
    Path(model_id): Path<String>, 
    Json(json_payload): Json<serde_json::Value>,
) -> Result<Response, EligibilityError> {
    // Run eligibility check first
    let mut datastore = state.lock().await;
    
    // Get the account address
    let account_address = recovered.as_hex();
    
    // Get or create the account
    let account = match datastore.account_state.get_account(&account_address) {
        Some(acc) => acc,
        None => {
            // Create a new account if it doesn't exist
            let new_account = crate::accounts::Account::new(account_address.clone());
            let op = datastore.account_state.update_account_local(new_account.clone());
            if let Err(e) = datastore.handle_account_op(op).await {
                return Err(EligibilityError::OperationNotAllowed(
                    format!("Failed to create account: {}", e)
                ));
            }
            new_account
        }
    };
    
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
    let response = model_inference(State(state), recovered, Path(model_id), Json(typed_payload)).await;
    Ok(response.into_response())
}

/// Add an operator key to a node
async fn add_node_operator_key(
    State(state): State<Arc<Mutex<DataStore>>>,
    Path(node_id): Path<String>,
    Json(payload): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    // Extract the operator key from the payload
    let operator_key = match payload.get("operator_key").and_then(|v| v.as_str()) {
        Some(key) => key.to_string(),
        None => return Json(json!({
            "success": false,
            "error": "Missing operator_key in request body"
        })),
    };
    
    // Lock the datastore
    let mut datastore = match state.try_lock() {
        Ok(ds) => ds,
        Err(_) => return Json(json!({
            "success": false,
            "error": "Server is busy, try again later"
        })),
    };
    
    // Verify the node exists
    if datastore.node_state.get_node(node_id.clone()).is_none() {
        return Json(json!({
            "success": false,
            "error": "Node not found"
        }));
    }
    
    // Add the operator key to the node
    match datastore.node_state.add_operator_key(node_id.clone(), operator_key.clone()) {
        Some(op) => {
            log::info!("Added operator key to node {}", node_id);
            // Apply the operation locally
            if let Some((actor, key)) = datastore.node_state.node_op(op.clone()) {
                // Could broadcast the change here if needed
                Json(json!({
                    "success": true,
                    "message": "Operator key added successfully",
                    "node_id": node_id,
                    "operator_key": operator_key
                }))
            } else {
                Json(json!({
                    "success": false,
                    "error": "Failed to apply node operation"
                }))
            }
        },
        None => Json(json!({
            "success": false,
            "error": "Failed to add operator key to node"
        })),
    }
}

/// Remove an operator key from a node
async fn remove_node_operator_key(
    State(state): State<Arc<Mutex<DataStore>>>,
    Path((node_id, key)): Path<(String, String)>,
) -> Json<serde_json::Value> {
    // Lock the datastore
    let mut datastore = match state.try_lock() {
        Ok(ds) => ds,
        Err(_) => return Json(json!({
            "success": false,
            "error": "Server is busy, try again later"
        })),
    };
    
    // Verify the node exists
    if datastore.node_state.get_node(node_id.clone()).is_none() {
        return Json(json!({
            "success": false,
            "error": "Node not found"
        }));
    }
    
    // Remove the operator key from the node
    match datastore.node_state.remove_operator_key(node_id.clone(), &key) {
        Some(op) => {
            log::info!("Removed operator key from node {}", node_id);
            // Apply the operation locally
            if let Some((actor, key)) = datastore.node_state.node_op(op.clone()) {
                // Could broadcast the change here if needed
                Json(json!({
                    "success": true,
                    "message": "Operator key removed successfully",
                    "node_id": node_id
                }))
            } else {
                Json(json!({
                    "success": false,
                    "error": "Failed to apply node operation"
                }))
            }
        },
        None => Json(json!({
            "success": false,
            "error": "Failed to remove operator key from node"
        })),
    }
}

#[derive(Deserialize)]
struct EnsureAdminPayload {
    admin_public_key: String,
}

async fn ensure_admin_account(
    State(state): State<Arc<Mutex<DataStore>>>,
    Json(payload): Json<EnsureAdminPayload>,
) -> impl IntoResponse {
    let admin_key = payload.admin_public_key;
    log::info!("Ensuring admin account for key: {}", admin_key);
    let mut datastore = state.lock().await;

    let account_exists = datastore.account_state.get_account(&admin_key).is_some();
    
    let mut account_to_save = if account_exists {
        let mut acc = datastore.account_state.get_account(&admin_key).unwrap(); // Safe unwrap due to check
        acc.is_global_admin = true;
        acc.updated_at = chrono::Utc::now().timestamp();
        acc
    } else {
        let mut new_acc = crate::accounts::Account::new(admin_key.clone());
        new_acc.is_global_admin = true;
        new_acc
    };

    let op = datastore.account_state.update_account_local(account_to_save);
    match datastore.handle_account_op(op).await {
        Ok(_) => (StatusCode::OK, Json(json!({ "status": "success", "message": "Admin account ensured."}))),
        Err(e) => {
            log::error!("Failed to ensure admin account for {}: {}", admin_key, e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "status": "error", "message": e.to_string()})))
        }
    }
}

// Add new handler function
async fn is_global_admin_handler(
    State(state): State<Arc<Mutex<DataStore>>>,
    Path(address): Path<String>,
) -> impl IntoResponse {
    let datastore = state.lock().await;
    match datastore.account_state.get_account(&address) {
        Some(account) => (StatusCode::OK, Json(json!({ "address": address, "is_global_admin": account.is_global_admin }))),
        None => (StatusCode::NOT_FOUND, Json(json!({ "error": "Account not found" }))),
    }
}

// New handler for listing all active (non-disabled) peers
async fn list_active_peers(State(state): State<Arc<Mutex<DataStore>>>) -> impl IntoResponse {
    let datastore = state.lock().await;
    let active_peer_ips = datastore.network_state.peers.iter()
        .filter_map(|entry| {
            let (_, reg) = entry.val;
            reg.val().map(|reg_val| reg_val.value())
        })
        .filter(|peer| !peer.is_disabled)
        .filter_map(|peer| {
            peer.endpoint.as_ref().and_then(|ep| {
                ep.resolve().ok().map(|socket_addr| socket_addr.ip().to_string())
            })
        })
        .collect::<Vec<String>>();
    (StatusCode::OK, Json(active_peer_ips))
}

async fn check_task_responsibility(
    State(state): State<Arc<Mutex<DataStore>>>,
    Path((task_id, node_id_to_check)): Path<(String, String)>,
) -> impl IntoResponse {
    let datastore = state.lock().await;
    match datastore.task_state.get_task(&task_id) {
        Some(task) => {
            let is_responsible = task.responsible_nodes.as_ref()
                .map_or(false, |nodes_set| nodes_set.contains(&node_id_to_check));
            
            // Additionally, ensure the node actually has the capabilities for the task
            // This is a sanity check, as PoC should only select capable nodes.
            let node_is_capable = datastore.node_state.get_node(node_id_to_check.clone())
                .map_or(false, |node| {
                    // Check against node.metadata.tags
                    task.required_capabilities.iter().all(|cap| node.metadata.tags.contains(cap))
                });

            if !node_is_capable && is_responsible {
                log::warn!("Node {} was marked responsible for task {} but does not have required capabilities.", node_id_to_check, task_id);
                 // Decide if this discrepancy should make it not responsible for execution.
                 // For now, if PoC assigned it, we trust that (but log warning).
            }

            (StatusCode::OK, Json(json!({ "task_id": task_id, "node_id": node_id_to_check, "is_responsible": is_responsible && node_is_capable })))
        }
        None => (StatusCode::NOT_FOUND, Json(json!({ "error": "Task not found" }))),
    }
}

#[derive(Deserialize, Debug)]
struct ReportMetricsPayload {
    capacity: NodeCapacity,
    metrics: NodeMetrics,
    // Optionally, could include capabilities if they can change dynamically
    // capabilities: Option<NodeCapabilities> 
}

async fn report_node_metrics(
    State(state): State<Arc<Mutex<DataStore>>>,
    Path(node_id): Path<String>,
    Json(payload): Json<ReportMetricsPayload>,
) -> impl IntoResponse {
    log::info!("Received metrics report for node_id: {}", node_id);
    let mut datastore = state.lock().await;

    // It's better to have a dedicated method in DataStore that calls node_state.update_node_metrics
    // and then handle_node_op for atomicity and proper op generation.
    // For now, directly using node_state and then queueing.
    match datastore.node_state.update_node_metrics(node_id.clone(), payload.capacity, payload.metrics) {
        Some(node_op) => {
            match datastore.handle_node_op(node_op).await {
                Ok(_) => (StatusCode::OK, Json(json!({ "status": "success", "message": "Metrics reported."}))),
                Err(e) => {
                    log::error!("Failed to handle node_op for metrics report {}: {}", node_id, e);
                    (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "status": "error", "message": e.to_string()})))
                }
            }
        }
        None => {
            log::warn!("Failed to update metrics for node_id: {} (node not found or no change)", node_id);
            (StatusCode::NOT_FOUND, Json(json!({ "status": "error", "message": "Node not found or no metrics change to report"})))
        }
    }
}

#[derive(Serialize, Deserialize, Debug)] // Add Debug for logging
struct DevnetGossipOpContainer {
    op_type: String, // e.g., "PeerOp", "NodeOp"
    op_payload_json: String, // The specific Op (PeerOp, NodeOp, etc.) serialized as JSON string
}

async fn devnet_apply_op_handler(
    State(state): State<Arc<Mutex<DataStore>>>,
    Json(payload): Json<DevnetGossipOpContainer>,
) -> impl IntoResponse {
    log::info!("DEVNET: Received direct gossip op: type '{}'", payload.op_type);
    let mut datastore = state.lock().await;

    // Sub-task 5.2.2.1: Deserialize the received Op (op_payload_json based on op_type)
    // Sub-task 5.2.2.2: Verify signature (inherent in Op apply methods)
    // Sub-task 5.2.2.3: Apply the Op to local DataStore
    // Sub-task 5.2.2.4: Ensure no re-gossip
    
    // Actual deserialization and application logic will be in 5.2.2
    // For now, just acknowledge receipt for 5.2.1
    match payload.op_type.as_str() {
        "PeerOp" => {
            match serde_json::from_str::<crate::network::PeerOp<String>>(&payload.op_payload_json) {
                Ok(peer_op) => {
                    // Apply the Op directly to the network state's CRDT map.
                    // The peer_op method in NetworkState should handle signature verification implicitly via map.apply.
                    datastore.network_state.peer_op(peer_op); 
                    // Persist the entire datastore state after applying the op.
                    match crate::db::write_datastore(&crate::datastore::DB_HANDLE, &*datastore) { // Pass datastore by reference
                        Ok(_) => {
                            log::info!("DEVNET: Successfully applied and persisted PeerOp.");
                            (StatusCode::OK, Json(json!({"status": "success", "message": "PeerOp applied."})))
                        }
                        Err(e) => {
                            log::error!("DEVNET: Failed to persist datastore after applying PeerOp: {}", e);
                            (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"status": "error", "message": "Failed to persist state after applying PeerOp"})))
                        }
                    }
                }
                Err(e) => {
                    log::error!("DEVNET: Failed to deserialize PeerOp: {}", e);
                    (StatusCode::BAD_REQUEST, Json(json!({"status": "error", "message": "Failed to deserialize PeerOp"})))
                }
            }
        }
        "NodeOp" => {
            match serde_json::from_str::<crate::nodes::NodeOp>(&payload.op_payload_json) {
                Ok(node_op) => {
                    datastore.node_state.node_op(node_op); 
                    match crate::db::write_datastore(&crate::datastore::DB_HANDLE, &*datastore) {
                        Ok(_) => {
                            log::info!("DEVNET: Successfully applied and persisted NodeOp.");
                            (StatusCode::OK, Json(json!({"status": "success", "message": "NodeOp applied."})))
                        }
                        Err(e) => {
                            log::error!("DEVNET: Failed to persist datastore after applying NodeOp: {}", e);
                            (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"status": "error", "message": "Failed to persist state after applying NodeOp"})))
                        }
                    }
                }
                Err(e) => {
                    log::error!("DEVNET: Failed to deserialize NodeOp: {}", e);
                    (StatusCode::BAD_REQUEST, Json(json!({"status": "error", "message": "Failed to deserialize NodeOp"})))
                }
            }
        }
        "AccountOp" => {
            match serde_json::from_str::<crate::accounts::AccountOp>(&payload.op_payload_json) {
                Ok(account_op) => {
                    datastore.account_state.account_op(account_op); 
                    match crate::db::write_datastore(&crate::datastore::DB_HANDLE, &*datastore) {
                        Ok(_) => {
                            log::info!("DEVNET: Successfully applied and persisted AccountOp.");
                            (StatusCode::OK, Json(json!({"status": "success", "message": "AccountOp applied."})))
                        }
                        Err(e) => {
                            log::error!("DEVNET: Failed to persist datastore after applying AccountOp: {}", e);
                            (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"status": "error", "message": "Failed to persist state after applying AccountOp"})))
                        }
                    }
                }
                Err(e) => {
                    log::error!("DEVNET: Failed to deserialize AccountOp: {}", e);
                    (StatusCode::BAD_REQUEST, Json(json!({"status": "error", "message": "Failed to deserialize AccountOp"})))
                }
            }
        }
        "CidrOp" => {
            match serde_json::from_str::<crate::network::CidrOp<String>>(&payload.op_payload_json) {
                Ok(cidr_op) => {
                    datastore.network_state.cidr_op(cidr_op); 
                    match crate::db::write_datastore(&crate::datastore::DB_HANDLE, &*datastore) {
                        Ok(_) => {
                            log::info!("DEVNET: Successfully applied and persisted CidrOp.");
                            (StatusCode::OK, Json(json!({"status": "success", "message": "CidrOp applied."})))
                        }
                        Err(e) => {
                            log::error!("DEVNET: Failed to persist datastore after applying CidrOp: {}", e);
                            (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"status": "error", "message": "Failed to persist state after applying CidrOp"})))
                        }
                    }
                }
                Err(e) => {
                    log::error!("DEVNET: Failed to deserialize CidrOp: {}", e);
                    (StatusCode::BAD_REQUEST, Json(json!({"status": "error", "message": "Failed to deserialize CidrOp"})))
                }
            }
        }
        "AssocOp" => {
            match serde_json::from_str::<crate::network::AssocOp<String>>(&payload.op_payload_json) {
                Ok(assoc_op) => {
                    datastore.network_state.associations_op(assoc_op); 
                    match crate::db::write_datastore(&crate::datastore::DB_HANDLE, &*datastore) {
                        Ok(_) => {
                            log::info!("DEVNET: Successfully applied and persisted AssocOp.");
                            (StatusCode::OK, Json(json!({"status": "success", "message": "AssocOp applied."})))
                        }
                        Err(e) => {
                            log::error!("DEVNET: Failed to persist datastore after applying AssocOp: {}", e);
                            (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"status": "error", "message": "Failed to persist state after applying AssocOp"})))
                        }
                    }
                }
                Err(e) => {
                    log::error!("DEVNET: Failed to deserialize AssocOp: {}", e);
                    (StatusCode::BAD_REQUEST, Json(json!({"status": "error", "message": "Failed to deserialize AssocOp"})))
                }
            }
        }
        "DnsOp" => {
            match serde_json::from_str::<crate::network::DnsOp>(&payload.op_payload_json) {
                Ok(dns_op) => {
                    datastore.network_state.dns_op(dns_op); 
                    match crate::db::write_datastore(&crate::datastore::DB_HANDLE, &*datastore) {
                        Ok(_) => {
                            log::info!("DEVNET: Successfully applied and persisted DnsOp.");
                            (StatusCode::OK, Json(json!({"status": "success", "message": "DnsOp applied."})))
                        }
                        Err(e) => {
                            log::error!("DEVNET: Failed to persist datastore after applying DnsOp: {}", e);
                            (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"status": "error", "message": "Failed to persist state after applying DnsOp"})))
                        }
                    }
                }
                Err(e) => {
                    log::error!("DEVNET: Failed to deserialize DnsOp: {}", e);
                    (StatusCode::BAD_REQUEST, Json(json!({"status": "error", "message": "Failed to deserialize DnsOp"})))
                }
            }
        }
        "InstanceOp" => {
            match serde_json::from_str::<crate::instances::InstanceOp>(&payload.op_payload_json) {
                Ok(instance_op) => {
                    datastore.instance_state.instance_op(instance_op); 
                    match crate::db::write_datastore(&crate::datastore::DB_HANDLE, &*datastore) {
                        Ok(_) => {
                            log::info!("DEVNET: Successfully applied and persisted InstanceOp.");
                            (StatusCode::OK, Json(json!({"status": "success", "message": "InstanceOp applied."})))
                        }
                        Err(e) => {
                            log::error!("DEVNET: Failed to persist datastore after applying InstanceOp: {}", e);
                            (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"status": "error", "message": "Failed to persist state after applying InstanceOp"})))
                        }
                    }
                }
                Err(e) => {
                    log::error!("DEVNET: Failed to deserialize InstanceOp: {}", e);
                    (StatusCode::BAD_REQUEST, Json(json!({"status": "error", "message": "Failed to deserialize InstanceOp"})))
                }
            }
        }
        "AgentOp" => {
            match serde_json::from_str::<crate::agent::AgentOp>(&payload.op_payload_json) {
                Ok(agent_op) => {
                    // The handle_agent_op in datastore.rs currently only applies locally.
                    // If it were to be gossiped, it would also need the #[cfg] logic.
                    // For receiving, we just apply locally.
                    datastore.agent_state.agent_op(agent_op); // Directly apply to the map
                    match crate::db::write_datastore(&crate::datastore::DB_HANDLE, &*datastore) {
                        Ok(_) => {
                            log::info!("DEVNET: Successfully applied and persisted AgentOp.");
                            (StatusCode::OK, Json(json!({"status": "success", "message": "AgentOp applied."})))
                        }
                        Err(e) => {
                            log::error!("DEVNET: Failed to persist datastore after applying AgentOp: {}", e);
                            (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"status": "error", "message": "Failed to persist state after applying AgentOp"})))
                        }
                    }
                }
                Err(e) => {
                    log::error!("DEVNET: Failed to deserialize AgentOp: {}", e);
                    (StatusCode::BAD_REQUEST, Json(json!({"status": "error", "message": "Failed to deserialize AgentOp"})))
                }
            }
        }
        "ModelOp" => {
            match serde_json::from_str::<crate::model::ModelOp>(&payload.op_payload_json) {
                Ok(model_op) => {
                    datastore.model_state.model_op(model_op); // Directly apply to the map
                    match crate::db::write_datastore(&crate::datastore::DB_HANDLE, &*datastore) {
                        Ok(_) => {
                            log::info!("DEVNET: Successfully applied and persisted ModelOp.");
                            (StatusCode::OK, Json(json!({"status": "success", "message": "ModelOp applied."})))
                        }
                        Err(e) => {
                            log::error!("DEVNET: Failed to persist datastore after applying ModelOp: {}", e);
                            (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"status": "error", "message": "Failed to persist state after applying ModelOp"})))
                        }
                    }
                }
                Err(e) => {
                    log::error!("DEVNET: Failed to deserialize ModelOp: {}", e);
                    (StatusCode::BAD_REQUEST, Json(json!({"status": "error", "message": "Failed to deserialize ModelOp"})))
                }
            }
        }
        _ => {
            log::warn!("DEVNET: Received unknown op_type for direct gossip: {}", payload.op_type);
            (StatusCode::BAD_REQUEST, Json(json!({ "status": "error", "message": "Unknown op_type"})))
        }
    }
}

#[derive(Deserialize, Debug)]
pub struct ListTasksQuery {
    task_type: Option<String>,
    status: Option<String>,
    assigned_to_node_id: Option<String>,
    submitted_by: Option<String>,
}

async fn list_tasks_handler(
    State(state): State<Arc<Mutex<DataStore>>>,
    axum::extract::Query(query): axum::extract::Query<ListTasksQuery>,
) -> impl IntoResponse {
    let datastore = state.lock().await;
    let mut tasks = datastore.task_state.list_tasks(); // Gets Vec<Task>

    if let Some(task_type_filter) = query.task_type {
        tasks.retain(|task| 
            match &task.task_variant {
                crate::tasks::TaskVariant::BuildImage(_) => task_type_filter == "BuildImage",
                crate::tasks::TaskVariant::LaunchInstance(_) => task_type_filter == "LaunchInstance",
                // Add other variants if/when they exist
            }
        );
    }

    if let Some(status_filter_str) = query.status {
        // This requires TaskStatus to be easily convertible from/to string, or more complex matching
        // For now, assuming direct string match on debug representation (not robust)
        tasks.retain(|task| format!("{:?}", task.status) == status_filter_str);
    }

    if let Some(node_id_filter) = query.assigned_to_node_id {
        tasks.retain(|task| task.assigned_to_node_id.as_deref() == Some(node_id_filter.as_str()));
    }

    if let Some(submitter_filter) = query.submitted_by {
        tasks.retain(|task| task.submitted_by == submitter_filter);
    }

    (StatusCode::OK, Json(tasks))
}

async fn get_task_handler(
    State(state): State<Arc<Mutex<DataStore>>>,
    Path(task_id): Path<String>,
) -> impl IntoResponse {
    let datastore = state.lock().await;
    match datastore.task_state.get_task(&task_id) {
        Some(task) => (StatusCode::OK, Json(json!(task))),
        None => (StatusCode::NOT_FOUND, Json(json!({ "error": "Task not found" }))),
    }
}

#[derive(Deserialize, Debug)] // Added Debug
struct UpdateTaskStatusPayload {
    task_id: FormStateTaskId,
    status: String, // Will be parsed into FormStateTaskStatus
    progress: Option<u8>,
    result_info: Option<String>,
}

async fn update_task_status_handler(
    State(state): State<Arc<Mutex<DataStore>>>,
    // This endpoint is in network_writers_api, so node_auth_middleware applies.
    // The recovered address here would be the node performing the update.
    // We might want to verify that this node is assigned_to_node_id for the task.
    _recovered_address: Option<RecoveredAddress>, // From node_auth_middleware or ecdsa_auth_middleware if switched
    Json(payload): Json<UpdateTaskStatusPayload>,
) -> impl IntoResponse {
    log::info!("Received task status update for task_id: {}", payload.task_id);
    
    // Parse status string to TaskStatus enum
    let task_status = match FormStateTaskStatus::from_str(&payload.status) { // Assuming FromStr for TaskStatus
        Ok(s) => s,
        Err(_) => {
            log::error!("Invalid task status string: {}", payload.status);
            return (StatusCode::BAD_REQUEST, Json(json!({ "status": "error", "message": "Invalid task status provided"}))).into_response();
        }
    };

    let mut datastore = state.lock().await;
    let task_request = crate::datastore::TaskRequest::UpdateStatus {
        task_id: payload.task_id.clone(),
        status: task_status,
        progress: payload.progress,
        result_info: payload.result_info,
    };

    match datastore.handle_task_request(task_request).await {
        Ok(_) => (StatusCode::OK, Json(json!({ "status": "success", "message": "Task status updated."}))).into_response(),
        Err(e) => {
            log::error!("Failed to update task status for {}: {}", payload.task_id, e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "status": "error", "message": e.to_string()}))).into_response()
        }
    }
}
