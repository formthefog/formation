use alloy_primitives::Address;
use k256::ecdsa::{SigningKey, signature::Signer};
use rand::rngs::OsRng;
use sha2::{Sha256, Digest};
use reqwest::{header::{HeaderMap, HeaderValue, AUTHORIZATION}};
use hex;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    simple_logger::init_with_level(log::Level::Info).unwrap();
    
    // Parse arguments or use defaults
    let server_url = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "http://localhost:3004".to_string());
    
    // Generate or load key
    log::info!("Generating a random key pair for testing...");
    let signing_key = SigningKey::random(&mut OsRng);
    let address = generate_address_from_key(&signing_key);
    let address_hex = hex::encode(address.as_slice());
    
    log::info!("Generated address: 0x{}", address_hex);
    log::info!("Private key: {}", hex::encode(signing_key.to_bytes()));
    
    // Create a client
    let client = reqwest::Client::new();
    
    // 1. Try the public endpoint first
    log::info!("Testing public endpoint...");
    let public_resp = client
        .get(format!("{}/health", server_url))
        .send()
        .await?;
    
    log::info!("Health check status: {}", public_resp.status());
    log::info!("Health check response: {}", public_resp.text().await?);
    
    // 2. Now try an authenticated endpoint
    log::info!("Testing authenticated endpoint...");
    
    // Create message and signature
    let message = format!("Auth request at {}", chrono::Utc::now());
    log::info!("Message to sign: {}", message);
    
    let auth_header = create_auth_header(&signing_key, message.as_bytes())?;
    let mut headers = HeaderMap::new();
    headers.insert(
        AUTHORIZATION,
        HeaderValue::from_str(&auth_header)
            .map_err(|e| format!("Invalid header value: {}", e))?
    );
    
    // Make authenticated request to get account info
    let account_url = format!("{}/account/{}/get", server_url, address_hex);
    log::info!("Requesting: {}", account_url);
    
    let auth_resp = client
        .get(&account_url)
        .headers(headers)
        .send()
        .await?;
    
    log::info!("Auth request status: {}", auth_resp.status());
    
    if auth_resp.status().is_success() {
        log::info!("Auth response: {}", auth_resp.text().await?);
        log::info!("✅ ECDSA Authentication successful!");
    } else {
        let error_text = auth_resp.text().await?;
        log::error!("❌ Authentication failed: {}", error_text);
    }
    
    Ok(())
}

// Helper function to generate an Ethereum-compatible address from a private key
fn generate_address_from_key(signing_key: &SigningKey) -> Address {
    let verifying_key = signing_key.verifying_key();
    Address::from_public_key(&verifying_key)
}

// Create an Authorization header with an ECDSA signature
fn create_auth_header(signing_key: &SigningKey, message: &[u8]) -> Result<String, Box<dyn std::error::Error>> {
    // Hash the message with SHA-256
    let mut hasher = Sha256::new();
    hasher.update(message);
    let message_hash = hasher.finalize();
    
    // Sign the message
    let (signature, recovery_id) = signing_key
        .sign_recoverable(message_hash.as_slice())
        .map_err(|e| format!("Failed to sign message: {}", e))?;
    
    // Format the signature as "Signature <signature_hex>.<recovery_id>.<message_hex>"
    let auth_value = format!(
        "Signature {}.{}.{}",
        hex::encode(signature.to_bytes()),
        recovery_id.to_byte(),
        hex::encode(message)
    );
    
    Ok(auth_value)
} 