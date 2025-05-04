use axum::{
    Router,
    routing::{get, post},
    middleware::from_fn,
    extract::State,
    response::IntoResponse,
    Json,
};
use form_state::auth::{
    RecoveredAddress,
    OptionalRecoveredAddress,
    ecdsa_auth_middleware,
};
use k256::ecdsa::{SigningKey, signature::Signer};
use sha2::{Sha256, Digest};
use alloy_primitives::Address;
use serde_json::json;
use std::net::SocketAddr;
use tokio::net::TcpListener;
use std::str::FromStr;
use rand::rngs::OsRng;
use hex;
use tiny_keccak::Hasher;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    simple_logger::init_with_level(log::Level::Info).unwrap();
    
    log::info!("Initializing ECDSA authentication example server...");
    
    // Generate a sample key for testing
    let signing_key = SigningKey::random(&mut OsRng);
    let address = generate_address_from_key(&signing_key);
    
    let address_hex = hex::encode(address.as_slice());
    log::info!("Generated sample address: 0x{}", address_hex);
    
    // Build the application with routes
    let app = Router::new()
        // Public endpoint
        .route("/", get(root_handler))
        
        // Protected endpoints
        .route("/protected", get(protected_handler))
        .route("/optional-auth", get(optional_auth_handler))
        
        // Protected route group
        .route("/api/resource", get(api_resource_handler))
        .layer(from_fn(ecdsa_auth_middleware));
    
    // Run the server
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    log::info!("Starting server on http://{}", addr);
    log::info!("Generate a signature with the private key:");
    log::info!("Private key: {}", hex::encode(signing_key.to_bytes()));
    
    let listener = TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    
    Ok(())
}

// Public root handler
async fn root_handler() -> impl IntoResponse {
    Json(json!({
        "message": "Welcome to the ECDSA authentication example server",
        "endpoints": {
            "/": "This public endpoint",
            "/protected": "Protected endpoint requiring ECDSA signature",
            "/optional-auth": "Endpoint that works with or without authentication",
            "/api/resource": "Protected API resource"
        },
        "authentication_format": "Authorization: Signature <signature_hex>.<recovery_id>.<message_hex>"
    }))
}

// Protected endpoint requiring authentication
async fn protected_handler(
    recovered: RecoveredAddress,
) -> impl IntoResponse {
    Json(json!({
        "message": "Access granted to protected resource",
        "authenticated_address": format!("0x{}", recovered.as_hex()),
        "original_message": String::from_utf8_lossy(&recovered.message)
    }))
}

// Endpoint that works with or without authentication
async fn optional_auth_handler(
    OptionalRecoveredAddress(recovered): OptionalRecoveredAddress,
) -> impl IntoResponse {
    match recovered {
        Some(address) => Json(json!({
            "message": "Authenticated access",
            "authenticated_address": format!("0x{}", address.as_hex()),
            "original_message": String::from_utf8_lossy(&address.message)
        })),
        None => Json(json!({
            "message": "Unauthenticated access",
            "note": "You can provide authentication for additional information"
        })),
    }
}

// Protected API resource
async fn api_resource_handler(
    recovered: RecoveredAddress,
) -> impl IntoResponse {
    Json(json!({
        "resource_id": "resource_123",
        "owner": format!("0x{}", recovered.as_hex()),
        "access_level": "full",
        "data": {
            "value": "This is sensitive data",
            "timestamp": chrono::Utc::now().to_rfc3339()
        }
    }))
}

// Helper function to generate an Ethereum-compatible address from a private key
fn generate_address_from_key(signing_key: &SigningKey) -> Address {
    let verifying_key = signing_key.verifying_key();
    
    // Hash the public key using keccak256
    let mut keccak = tiny_keccak::Keccak::v256();
    let mut hash = [0u8; 32];
    
    // Remove the first byte (0x04 prefix) and hash the public key
    keccak.update(&verifying_key.to_encoded_point(false).as_bytes()[1..]);
    keccak.finalize(&mut hash);
    
    // Take the last 20 bytes as the address
    Address::from_slice(&hash[12..32])
} 