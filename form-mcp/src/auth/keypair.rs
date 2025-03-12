// Keypair module for authentication
//
// This module handles cryptographic keypairs for authentication in the MCP server.

use k256::ecdsa::{SigningKey, VerifyingKey};
use rand::rngs::OsRng;
use std::str::FromStr;
use crate::errors::AuthError;
use k256::elliptic_curve::sec1::ToEncodedPoint;
use sha3::{Digest, Keccak256};
use hex;

/// Represents a cryptographic keypair
pub struct KeyPair {
    /// Private signing key
    pub signing_key: SigningKey,
    /// Public verification key
    pub verifying_key: VerifyingKey,
    /// Public address derived from the verifying key
    pub address: String,
}

impl KeyPair {
    /// Create a new random keypair
    pub fn random() -> Self {
        let signing_key = SigningKey::random(&mut OsRng);
        let verifying_key = VerifyingKey::from(&signing_key);
        let address = derive_address(&verifying_key);
        
        Self {
            signing_key,
            verifying_key,
            address,
        }
    }
    
    /// Create a keypair from an existing private key
    pub fn from_private_key(private_key: &str) -> Result<Self, AuthError> {
        // Remove '0x' prefix if present
        let private_key = private_key.trim_start_matches("0x");
        
        // Decode hex string to bytes
        let private_key_bytes = hex::decode(private_key)
            .map_err(|_e| AuthError::InvalidCredentials)?;
        
        // Create SigningKey from bytes
        let signing_key = SigningKey::from_slice(&private_key_bytes)
            .map_err(|_| AuthError::InvalidCredentials)?;
        
        // Derive VerifyingKey from SigningKey
        let verifying_key = VerifyingKey::from(&signing_key);
        
        // Derive Ethereum-style address
        let address = derive_address(&verifying_key);
        
        Ok(Self {
            signing_key,
            verifying_key,
            address,
        })
    }
}

/// Derive an Ethereum-style address from a verifying key
pub fn derive_address(verifying_key: &VerifyingKey) -> String {
    // Get the public key in uncompressed SEC1 format
    let public_key = verifying_key.to_encoded_point(false);
    
    // Remove the '04' prefix (represents uncompressed point format)
    let public_key_bytes = public_key.as_bytes();
    // Skip the prefix byte which indicates uncompressed format (0x04)
    let key_without_prefix = &public_key_bytes[1..];
    
    // Hash the public key with Keccak256
    let mut hasher = Keccak256::new();
    hasher.update(key_without_prefix);
    let hash = hasher.finalize();
    
    // Take the last 20 bytes of the hash as the address (Ethereum address = last 20 bytes)
    let address_bytes = &hash[hash.len() - 20..];
    
    // Convert to hex string with 0x prefix
    format!("0x{}", hex::encode(address_bytes))
}

// ... existing code ...