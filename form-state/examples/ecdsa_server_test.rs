use axum::{
    Router,
    routing::{get, post},
    middleware::from_fn,
    extract::{State, Path},
    response::IntoResponse,
    Json,
};
use form_state::auth::{
    RecoveredAddress,
    ecdsa_auth_middleware,
};
use serde_json::json;
use std::net::SocketAddr;
use tokio::net::TcpListener;
use std::sync::{Arc, Mutex};
use std::collections::HashMap;

// Simple in-memory store for accounts
type AccountStore = Arc<Mutex<HashMap<String, Account>>>;

#[derive(Clone, Debug)]
struct Account {
    address: String,
    balance: u64,
    created_at: chrono::DateTime<chrono::Utc>,
}

impl Account {
    fn new(address: String) -> Self {
        Self {
            address,
            balance: 100, // Initial free credits
            created_at: chrono::Utc::now(),
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    simple_logger::init_with_level(log::Level::Info).unwrap();
    
    log::info!("Initializing ECDSA authentication test server...");
    
    // Create an in-memory account store
    let account_store = Arc::new(Mutex::new(HashMap::new()));
    
    // Build the application with routes
    let app = Router::new()
        // Public endpoints
        .route("/", get(root_handler))
        .route("/health", get(health_handler))
        
        // Protected endpoints
        .route("/account/:address/get", get(get_account_handler))
        .route("/account/create", post(create_account_handler))
        .route("/account/list", get(list_accounts_handler))
        
        // Apply ECDSA authentication middleware to protected routes
        .layer(from_fn(ecdsa_auth_middleware))
        .with_state(account_store);
    
    // Run the server
    let addr = SocketAddr::from(([127, 0, 0, 1], 3004));
    log::info!("Starting server on http://{}", addr);
    
    let listener = TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    
    Ok(())
}

// Public root handler
async fn root_handler() -> impl IntoResponse {
    Json(json!({
        "message": "Welcome to the ECDSA authentication test server",
        "endpoints": {
            "/": "This public endpoint",
            "/health": "Health check endpoint",
            "/account/:address/get": "Get account details (authenticated)",
            "/account/create": "Create a new account (authenticated)",
            "/account/list": "List all accounts (authenticated)"
        },
        "authentication_format": "Authorization: Signature <signature_hex>.<recovery_id>.<message_hex>"
    }))
}

// Health check endpoint
async fn health_handler() -> impl IntoResponse {
    Json(json!({
        "status": "healthy",
        "version": "0.1.0",
        "timestamp": chrono::Utc::now()
    }))
}

// Get account handler
async fn get_account_handler(
    State(store): State<AccountStore>,
    recovered: RecoveredAddress,
    Path(address): Path<String>,
) -> impl IntoResponse {
    let store = store.lock().unwrap();
    
    // Check authorization (users can only access their own accounts)
    let caller_address = recovered.as_hex();
    if caller_address != address {
        return Json(json!({
            "error": "Unauthorized - you can only access your own account",
            "caller_address": caller_address,
            "requested_address": address
        }));
    }
    
    match store.get(&address) {
        Some(account) => Json(json!({
            "address": account.address,
            "balance": account.balance,
            "created_at": account.created_at
        })),
        None => Json(json!({
            "error": "Account not found",
            "address": address
        })),
    }
}

// Create account handler
async fn create_account_handler(
    State(store): State<AccountStore>,
    recovered: RecoveredAddress,
) -> impl IntoResponse {
    let address = recovered.as_hex();
    
    let mut store = store.lock().unwrap();
    
    // Check if account already exists
    if store.contains_key(&address) {
        return Json(json!({
            "error": "Account already exists",
            "address": address
        }));
    }
    
    // Create a new account
    let account = Account::new(address.clone());
    store.insert(address.clone(), account.clone());
    
    Json(json!({
        "success": true,
        "message": "Account created successfully",
        "account": {
            "address": account.address,
            "balance": account.balance,
            "created_at": account.created_at
        }
    }))
}

// List accounts handler
async fn list_accounts_handler(
    State(store): State<AccountStore>,
    recovered: RecoveredAddress,
) -> impl IntoResponse {
    let store = store.lock().unwrap();
    
    let accounts: Vec<_> = store.values()
        .map(|account| json!({
            "address": account.address,
            "balance": account.balance,
            "created_at": account.created_at
        }))
        .collect();
    
    Json(json!({
        "accounts": accounts,
        "total": accounts.len(),
        "caller": recovered.as_hex()
    }))
} 