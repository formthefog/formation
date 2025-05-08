use k256::ecdsa::{SigningKey, signature::Signer};
use sha2::{Sha256, Digest};
use alloy_primitives::Address;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION};
use serde_json::Value;
use std::error::Error;
use std::str::FromStr;
use hex;
use rand::rngs::OsRng;
use tiny_keccak::Hasher;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Initialize logging
    simple_logger::init_with_level(log::Level::Info).unwrap();
    
    // Parse the server URL from the command line or use default
    let server_url = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "http://localhost:3000".to_string());
    
    // Parse the private key from the command line or generate a new one
    let private_key_hex = std::env::args().nth(2);
    
    let signing_key = match private_key_hex {
        Some(hex_key) => {
            let key_bytes = hex::decode(&hex_key)?;
            SigningKey::from_slice(&key_bytes)?
        },
        None => {
            log::info!("No private key provided, generating a new random key");
            SigningKey::random(&mut OsRng)
        }
    };
    
    let address = generate_address_from_key(&signing_key);
    log::info!("Using address: 0x{}", hex::encode(address.as_slice()));
    
    // Create a reqwest client
    let client = reqwest::Client::new();
    
    // Demonstration of accessing different endpoints
    
    // 1. Access the public endpoint
    log::info!("Making request to public endpoint...");
    let public_response = client
        .get(format!("{}/", server_url))
        .send()
        .await?
        .json::<Value>()
        .await?;
    
    log::info!("Public endpoint response: {}", serde_json::to_string_pretty(&public_response)?);
    
    // 2. Access a protected endpoint
    log::info!("Making authenticated request to protected endpoint...");
    let message = format!("Authenticated request at {}", chrono::Utc::now());
    
    let auth_header = create_auth_header(&signing_key, message.as_bytes())?;
    
    let protected_response = client
        .get(format!("{}/protected", server_url))
        .headers(auth_header)
        .send()
        .await?
        .json::<Value>()
        .await?;
    
    log::info!("Protected endpoint response: {}", serde_json::to_string_pretty(&protected_response)?);
    
    // 3. Access the optional auth endpoint without authentication
    log::info!("Making unauthenticated request to optional-auth endpoint...");
    let optional_unauth_response = client
        .get(format!("{}/optional-auth", server_url))
        .send()
        .await?
        .json::<Value>()
        .await?;
    
    log::info!("Optional endpoint (unauthenticated) response: {}", 
        serde_json::to_string_pretty(&optional_unauth_response)?);
    
    // 4. Access the optional auth endpoint with authentication
    log::info!("Making authenticated request to optional-auth endpoint...");
    let message = format!("Optional auth request at {}", chrono::Utc::now());
    
    let auth_header = create_auth_header(&signing_key, message.as_bytes())?;
    
    let optional_auth_response = client
        .get(format!("{}/optional-auth", server_url))
        .headers(auth_header)
        .send()
        .await?
        .json::<Value>()
        .await?;
    
    log::info!("Optional endpoint (authenticated) response: {}", 
        serde_json::to_string_pretty(&optional_auth_response)?);
    
    // 5. Access the protected API resource
    log::info!("Making authenticated request to API resource...");
    let message = format!("API resource request at {}", chrono::Utc::now());
    
    let auth_header = create_auth_header(&signing_key, message.as_bytes())?;
    
    let api_response = client
        .get(format!("{}/api/resource", server_url))
        .headers(auth_header)
        .send()
        .await?
        .json::<Value>()
        .await?;
    
    log::info!("API resource response: {}", serde_json::to_string_pretty(&api_response)?);
    
    log::info!("Client demonstration completed successfully");
    
    Ok(())
}

// Create an Authorization header with an ECDSA signature
fn create_auth_header(signing_key: &SigningKey, message: &[u8]) -> Result<HeaderMap, Box<dyn Error>> {
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
    
    // Create the header map
    let mut headers = HeaderMap::new();
    headers.insert(
        AUTHORIZATION,
        HeaderValue::from_str(&auth_value)
            .map_err(|e| format!("Invalid header value: {}", e))?
    );
    
    Ok(headers)
}

// Helper function to generate an Ethereum-compatible address from a private key
fn generate_address_from_key(signing_key: &SigningKey) -> Address {
    let verifying_key = signing_key.verifying_key();
    Address::from_public_key(&verifying_key)
} 