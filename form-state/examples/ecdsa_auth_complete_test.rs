use alloy_primitives::Address;
use k256::ecdsa::SigningKey;
use rand::rngs::OsRng;
use sha2::{Sha256, Digest};
use reqwest::{header::{HeaderMap, HeaderValue, AUTHORIZATION}};
use hex;
use serde_json::Value;
use tiny_keccak::{Keccak, Hasher};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    simple_logger::init_with_level(log::Level::Info).unwrap();
    
    // Parse arguments or use defaults
    let server_url = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "http://localhost:3004".to_string());
    
    // Generate a key - will be used for all requests
    log::info!("Generating a random key pair for testing...");
    let signing_key = SigningKey::random(&mut OsRng);
    let address = generate_address_from_key(&signing_key);
    let address_hex = hex::encode(address.as_slice());
    
    log::info!("Generated address: 0x{}", address_hex);
    log::info!("Private key: {}", hex::encode(signing_key.to_bytes()));
    
    // Create a client
    let client = reqwest::Client::new();
    
    // 1. Try the public health endpoint
    log::info!("Step 1: Testing public endpoint...");
    let public_resp = client
        .get(format!("{}/health", server_url))
        .send()
        .await?;
    
    log::info!("Health check status: {}", public_resp.status());
    log::info!("Health check response: {}", public_resp.text().await?);
    
    // 2. Create an account
    log::info!("Step 2: Creating an account...");
    let create_message = b"Create account request";
    let create_header = create_auth_header(&signing_key, create_message)?;
    
    let mut create_headers = HeaderMap::new();
    create_headers.insert(
        AUTHORIZATION,
        HeaderValue::from_str(&create_header)
            .map_err(|e| format!("Invalid header value: {}", e))?
    );
    
    // Send the create account request
    let create_url = format!("{}/account/create", server_url);
    log::info!("Creating account at: {}", create_url);
    
    let create_resp = client
        .post(&create_url)
        .headers(create_headers)
        .send()
        .await?;
    
    log::info!("Create account status: {}", create_resp.status());
    
    if !create_resp.status().is_success() {
        log::error!("Failed to create account!");
        return Ok(());
    }
    
    // Get and parse the create account response
    let create_body = create_resp.text().await?;
    log::info!("Create account response: {}", create_body);
    log::info!("✅ Account creation successful!");
    
    // Parse out the actual address from the response
    let create_json: Value = serde_json::from_str(&create_body)?;
    let server_address = match create_json.get("address").and_then(|a| a.as_str()) {
        Some(addr) => addr,
        None => {
            log::error!("Could not parse account address from response");
            return Ok(());
        }
    };
    
    // 3. Now get the account with correct address
    log::info!("Step 3: Getting the account...");
    
    // Create message and signature
    let get_message = b"Get account request";
    let get_auth_header = create_auth_header(&signing_key, get_message)?;
    
    let mut get_headers = HeaderMap::new();
    get_headers.insert(
        AUTHORIZATION,
        HeaderValue::from_str(&get_auth_header)
            .map_err(|e| format!("Invalid header value: {}", e))?
    );
    
    // Make authenticated request to get account info using the address from the server
    let account_url = format!("{}/account/{}/get", server_url, server_address);
    log::info!("Getting account from: {}", account_url);
    
    let get_resp = client
        .get(&account_url)
        .headers(get_headers)
        .send()
        .await?;
    
    log::info!("Get account status: {}", get_resp.status());
    
    if get_resp.status().is_success() {
        let account_body = get_resp.text().await?;
        log::info!("Get account response: {}", account_body);
        
        let account_json: Value = serde_json::from_str(&account_body)?;
        if account_json.get("success").and_then(|s| s.as_bool()).unwrap_or(false) {
            log::info!("✅ Get account successful!");
            log::info!("✅ ECDSA Authentication fully verified!");
        } else {
            log::error!("❌ Get account returned success status but response indicates error: {}", account_body);
        }
    } else {
        let error_text = get_resp.text().await?;
        log::error!("❌ Get account failed: {}", error_text);
    }
    
    // 4. List all accounts
    log::info!("Step 4: Listing all accounts...");
    
    let list_message = b"List accounts request";
    let list_auth_header = create_auth_header(&signing_key, list_message)?;
    
    let mut list_headers = HeaderMap::new();
    list_headers.insert(
        AUTHORIZATION,
        HeaderValue::from_str(&list_auth_header)
            .map_err(|e| format!("Invalid header value: {}", e))?
    );
    
    let list_url = format!("{}/account/list", server_url);
    log::info!("Listing accounts from: {}", list_url);
    
    let list_resp = client
        .get(&list_url)
        .headers(list_headers)
        .send()
        .await?;
    
    log::info!("List accounts status: {}", list_resp.status());
    
    if list_resp.status().is_success() {
        let list_body = list_resp.text().await?;
        log::info!("List accounts response: {}", list_body);
        log::info!("✅ List accounts successful!");
    } else {
        let error_text = list_resp.text().await?;
        log::error!("❌ List accounts failed: {}", error_text);
    }
    
    Ok(())
}

// Helper function to generate an Ethereum-compatible address from a private key
fn generate_address_from_key(signing_key: &SigningKey) -> Address {
    let verifying_key = signing_key.verifying_key();
    
    // Log information for debugging
    log::info!("Generating address from verifying key");
    log::info!("Verifying key (encoded): {}", hex::encode(verifying_key.to_encoded_point(false).as_bytes()));
    
    let address = Address::from_public_key(&verifying_key);
    log::info!("Generated address bytes: {}", hex::encode(address.as_slice()));
    
    address
}

// Create an Authorization header with an ECDSA signature
fn create_auth_header(signing_key: &SigningKey, message: &[u8]) -> Result<String, Box<dyn std::error::Error>> {
    // Hash the message with SHA-256
    let mut hasher = Sha256::new();
    hasher.update(message);
    let message_hash = hasher.finalize();
    
    log::info!("Creating signature for message: {}", String::from_utf8_lossy(message));
    log::info!("Message hash: {}", hex::encode(message_hash));
    
    // Sign the message
    let (signature, recovery_id) = signing_key
        .sign_recoverable(message_hash.as_slice())
        .map_err(|e| format!("Failed to sign message: {}", e))?;
    
    log::info!("Signature: {}", hex::encode(signature.to_bytes()));
    log::info!("Recovery ID: {}", recovery_id.to_byte());
    
    // Format the signature as "Signature <signature_hex>.<recovery_id>.<message_hex>"
    let auth_value = format!(
        "Signature {}.{}.{}",
        hex::encode(signature.to_bytes()),
        recovery_id.to_byte(),
        hex::encode(message)
    );
    
    Ok(auth_value)
} 