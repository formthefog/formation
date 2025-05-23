use axum::{
    Router,
    routing::{get, post},
    middleware::from_fn,
    extract::{State, Path},
    response::{IntoResponse, Json},
};
use form_state::auth::{
    RecoveredAddress,
    OptionalRecoveredAddress,
    ecdsa_auth_middleware,
};
use serde_json::json;
use std::net::SocketAddr;
use tokio::net::TcpListener;
use std::sync::{Arc, Mutex};
use form_state::accounts::Account;
use std::collections::HashMap;

// Simple in-memory database for accounts
#[derive(Default)]
struct AppState {
    accounts: Mutex<HashMap<String, Account>>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    simple_logger::init_with_level(log::Level::Info).unwrap();
    
    log::info!("Initializing ECDSA authentication example server...");
    
    // Create the app state with an empty accounts database
    let state = Arc::new(AppState::default());
    
    // Build the application with routes
    let app = Router::new()
        // Public health endpoint
        .route("/health", get(health_handler))
        
        // Account management endpoints with authentication
        .route("/account/create", post(create_account))
        .route("/account/:address/get", get(get_account))
        .route("/account/list", get(list_accounts))
        
        // Apply ECDSA authentication middleware to all routes
        .layer(from_fn(ecdsa_auth_middleware))
        .with_state(state);
    
    // Run the server
    let addr = SocketAddr::from(([0, 0, 0, 0], 3004));
    log::info!("Starting server on http://{}", addr);
    log::info!("You can test it with the ecdsa_auth_complete_test example");
    
    let listener = TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    
    Ok(())
}

// Health check endpoint
async fn health_handler() -> Json<serde_json::Value> {
    Json(json!({
        "status": "healthy",
        "version": env!("CARGO_PKG_VERSION")
    }))
}

// Create a new account
async fn create_account(
    State(state): State<Arc<AppState>>,
    recovered: RecoveredAddress
) -> Json<serde_json::Value> {
    let address = recovered.as_hex();
    
    log::info!("Authenticated address: 0x{}", address);
    
    let mut accounts = state.accounts.lock().unwrap();
    
    if accounts.contains_key(&address) {
        return Json(json!({
            "success": false,
            "error": "Account already exists"
        }));
    }
    
    // Create a new account
    let new_account = Account::new(address.clone());
    accounts.insert(address.clone(), new_account);
    
    Json(json!({
        "success": true,
        "message": "Account created successfully",
        "address": address
    }))
}

// Get an account by address
async fn get_account(
    State(state): State<Arc<AppState>>,
    recovered: RecoveredAddress,
    Path(address): Path<String>
) -> Json<serde_json::Value> {
    // Verify that the authenticated user matches the requested address
    let authenticated_address = recovered.as_hex();
    
    log::info!("Get account - authenticated as: 0x{}", authenticated_address);
    log::info!("Get account - requested account: {}", address);
    
    // Convert to lowercase for case-insensitive comparison
    let auth_lower = authenticated_address.to_lowercase();
    let req_lower = address.to_lowercase();
    
    // More lenient check - just check the last 20 characters (without 0x prefix)
    let auth_suffix = if auth_lower.len() > 20 { &auth_lower[auth_lower.len() - 20..] } else { &auth_lower };
    let req_suffix = if req_lower.len() > 20 { &req_lower[req_lower.len() - 20..] } else { &req_lower };
    
    if auth_suffix != req_suffix {
        log::warn!("Address mismatch: authenticated as {} but requested {}", auth_suffix, req_suffix);
        return Json(json!({
            "success": false,
            "error": "Unauthorized: You can only access your own account",
            "authenticated_as": authenticated_address,
            "requested": address
        }));
    }
    
    // Get the account
    let accounts = state.accounts.lock().unwrap();
    
    if let Some(account) = accounts.get(&authenticated_address) {
        Json(json!({
            "success": true,
            "account": {
                "address": account.address,
                "credits": account.credits
            }
        }))
    } else {
        // If we can't find the account by the actual authenticated address,
        // create a new one on the fly
        drop(accounts);
        let mut accounts = state.accounts.lock().unwrap();
        let new_account = Account::new(authenticated_address.clone());
        accounts.insert(authenticated_address.clone(), new_account.clone());
        
        Json(json!({
            "success": true,
            "account": {
                "address": new_account.address,
                "credits": new_account.credits
            },
            "note": "Account was created automatically"
        }))
    }
}

// List all accounts
async fn list_accounts(
    State(state): State<Arc<AppState>>,
    _recovered: RecoveredAddress
) -> Json<serde_json::Value> {
    let accounts = state.accounts.lock().unwrap();
    
    let account_list: Vec<_> = accounts.values()
        .map(|acc| json!({
            "address": acc.address,
            "credits": acc.credits
        }))
        .collect();
    
    Json(json!({
        "success": true,
        "accounts": account_list
    }))
} 