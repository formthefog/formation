use alloy_primitives::Address;
use k256::ecdsa::{SigningKey, signature::Signer};
use rand::rngs::OsRng;
use sha2::{Sha256, Digest};
use hex;
use serde_json::json;

fn main() {
    // Initialize logging
    simple_logger::init_with_level(log::Level::Info).unwrap();
    
    log::info!("=== ECDSA Authentication Test ===");
    log::info!("");
    
    // Generate a key pair for testing
    test_key_generation();
    
    // Test the signature creation
    test_signature_creation();
    
    // Test the final format
    test_auth_header_format();
    
    log::info!("");
    log::info!("✅ All tests passed successfully!");
    log::info!("");
    log::info!("Your ECDSA authentication implementation is working correctly.");
    log::info!("To use it in real API requests, include the Authorization header");
    log::info!("with the format: 'Signature <sig>.<recovery_id>.<message>'");
}

fn test_key_generation() {
    log::info!("Test: Key Generation");
    
    // Generate a random key pair
    let signing_key = SigningKey::random(&mut OsRng);
    let verifying_key = signing_key.verifying_key();
    
    // Generate an Ethereum-compatible address
    let address = Address::from_public_key(&verifying_key);
    let address_hex = hex::encode(address.as_slice());
    
    log::info!("  Generated address: 0x{}", address_hex);
    log::info!("  Private key: {}", hex::encode(signing_key.to_bytes()));
    
    // Verify address is 20 bytes (40 hex chars)
    assert_eq!(address_hex.len(), 40);
    log::info!("  ✓ Address has correct length (20 bytes)");
    
    log::info!("  ✓ Key generation passed");
    log::info!("");
}

fn test_signature_creation() {
    log::info!("Test: Signature Creation");
    
    // Create a key and message
    let signing_key = SigningKey::random(&mut OsRng);
    let message = "Test message for authentication";
    log::info!("  Message to sign: {}", message);
    
    // Hash the message with SHA-256
    let mut hasher = Sha256::new();
    hasher.update(message.as_bytes());
    let message_hash = hasher.finalize();
    
    // Sign the message
    let (signature, recovery_id) = signing_key
        .sign_recoverable(message_hash.as_slice())
        .unwrap();
    
    // Verify signature components
    let signature_bytes = signature.to_bytes();
    let recovery_byte = recovery_id.to_byte();
    
    assert_eq!(signature_bytes.len(), 64); // r and s are 32 bytes each
    assert!(recovery_byte == 0 || recovery_byte == 1, "Recovery ID must be 0 or 1");
    
    log::info!("  Signature: {}", hex::encode(signature_bytes));
    log::info!("  Recovery ID: {}", recovery_byte);
    log::info!("  ✓ Signature has correct length (64 bytes)");
    log::info!("  ✓ Recovery ID is valid");
    log::info!("  ✓ Signature creation passed");
    log::info!("");
}

fn test_auth_header_format() {
    log::info!("Test: Authorization Header Format");
    
    // Create a key and message
    let signing_key = SigningKey::random(&mut OsRng);
    let address = Address::from_public_key(&signing_key.verifying_key());
    let address_hex = hex::encode(address.as_slice());
    
    let message = "Test message for authentication";
    
    // Create the signature
    let auth_header = create_auth_header(&signing_key, message.as_bytes())
        .expect("Failed to create auth header");
    
    log::info!("  Generated address: 0x{}", address_hex);
    log::info!("  Authorization header: {}", auth_header);
    
    // Verify the format
    assert!(auth_header.starts_with("Signature "));
    
    let signature_part = auth_header.strip_prefix("Signature ").unwrap();
    let parts: Vec<&str> = signature_part.split('.').collect();
    
    assert_eq!(parts.len(), 3, "Header should have 3 parts separated by dots");
    
    // Verify each part can be decoded
    let signature_hex = parts[0];
    let recovery_id = parts[1];
    let message_hex = parts[2];
    
    let signature_bytes = hex::decode(signature_hex).expect("Invalid signature hex");
    let recovery_byte = recovery_id.parse::<u8>().expect("Invalid recovery ID");
    let message_bytes = hex::decode(message_hex).expect("Invalid message hex");
    
    assert_eq!(signature_bytes.len(), 64);
    assert!(recovery_byte == 0 || recovery_byte == 1);
    assert_eq!(String::from_utf8_lossy(&message_bytes), message);
    
    log::info!("  ✓ Header has correct format");
    log::info!("  ✓ Signature part is valid and decodable");
    log::info!("  ✓ Recovery ID part is valid");
    log::info!("  ✓ Message part is valid and decodable");
    log::info!("  ✓ Authorization header format passed");
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