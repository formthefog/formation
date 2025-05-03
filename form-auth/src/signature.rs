use k256::ecdsa::{
    RecoveryId, Signature, signature::Verifier, 
    VerifyingKey, signature::Signer, SigningKey
};
use k256::Secp256k1;
use sha2::{Sha256, Digest};
use std::time::{SystemTime, UNIX_EPOCH};
use serde::{Serialize, Deserialize};
use crate::error::AuthError;

/// Maximum age for a signature timestamp in seconds
const MAX_TIMESTAMP_AGE_SECONDS: u64 = 300; // 5 minutes

/// Signature data extracted from a request
#[derive(Debug, Clone)]
pub struct SignatureData {
    /// The signature as a hex string
    pub signature: String,
    /// The recovery ID as a hex string
    pub recovery_id: String,
    /// The timestamp from the request
    pub timestamp: i64,
    /// The message to verify
    pub message: String,
}

impl SignatureData {
    /// Create a new SignatureData
    pub fn new(signature: String, recovery_id: String, timestamp: i64, message: String) -> Self {
        Self {
            signature,
            recovery_id,
            timestamp,
            message,
        }
    }
}

/// Create a message hash from a message and timestamp
/// 
/// This creates a unique digest that combines the message and timestamp
pub fn create_message_hash(message: &str, timestamp: i64) -> Vec<u8> {
    let mut hasher = Sha256::new();
    hasher.update(message.as_bytes());
    hasher.update(timestamp.to_string().as_bytes());
    hasher.finalize().to_vec()
}

/// Recover the public key from a signature and message hash
pub fn recover_public_key(
    sig_data: &SignatureData
) -> Result<VerifyingKey, AuthError> {
    // Create the message hash
    let message_hash = create_message_hash(&sig_data.message, sig_data.timestamp);
    
    // Parse the signature
    let signature_bytes = hex::decode(&sig_data.signature)
        .map_err(|_| AuthError::InvalidSignatureFormat)?;
    
    if signature_bytes.len() != 64 {
        return Err(AuthError::InvalidSignatureFormat);
    }
    
    let recovery_id = u8::from_str_radix(&sig_data.recovery_id, 16)
        .map_err(|_| AuthError::InvalidSignatureFormat)?;
    
    if recovery_id > 1 {
        return Err(AuthError::InvalidSignatureFormat);
    }
    
    let recovery_id = match RecoveryId::from_byte(recovery_id) {
        Some(id) => id,
        None => return Err(AuthError::InvalidSignatureFormat),
    };
    
    // Create a recoverable signature
    let sig = Signature::from_slice(&signature_bytes)
        .map_err(|_| AuthError::InvalidSignatureFormat)?;
    
    // Recover the public key
    match VerifyingKey::recover_from_prehash(
        &message_hash,
        &sig, 
        recovery_id
    ) {
        Ok(key) => Ok(key),
        Err(_) => Err(AuthError::InvalidSignature),
    }
}

/// Verify a signature
pub fn verify_signature(sig_data: &SignatureData, verifying_key: &VerifyingKey) -> Result<bool, AuthError> {
    // Hex-decode the signature
    let signature_bytes = hex::decode(&sig_data.signature)
        .map_err(|_| AuthError::InvalidSignature)?;
    
    // Create a signature object
    let signature = Signature::try_from(signature_bytes.as_slice())
        .map_err(|_| AuthError::InvalidSignature)?;
    
    // Create message hash
    let message_hash = create_message_hash(&sig_data.message, sig_data.timestamp);
    
    // Verify the signature
    match verifying_key.verify(message_hash.as_slice(), &signature) {
        Ok(_) => Ok(true),
        Err(_) => Err(AuthError::SignatureVerificationFailed),
    }
}

/// Sign a message with a private key
/// 
/// Returns a tuple of (signature, recovery_id) where both are hex strings
pub fn sign_message(message: &str, timestamp: i64, signing_key: &SigningKey) -> Result<(String, String), AuthError> {
    // Create message hash
    let digest = create_message_hash(message, timestamp);
    
    // Sign the message
    let signature: Signature = signing_key.sign(digest.as_slice());
    
    // Get the recovery ID
    // Note: In a real implementation, you would use the actual recovery ID from the signature
    // This is simplified for testing purposes
    let recovery_id = RecoveryId::from_byte(0).unwrap();
    
    // Convert to hex strings
    let signature_hex = hex::encode(signature.to_bytes());
    let recovery_hex = hex::encode([recovery_id.to_byte()]);
    
    Ok((signature_hex, recovery_hex))
}

/// Validate that a timestamp is within an allowed window
pub fn validate_timestamp(timestamp: i64, allowed_window_secs: i64) -> bool {
    let now = chrono::Utc::now().timestamp();
    let diff = now - timestamp;
    diff.abs() <= allowed_window_secs
}

#[cfg(test)]
mod tests {
    use super::*;
    use k256::SecretKey;
    use rand::rngs::OsRng;
    use std::str::FromStr;
    
    #[test]
    fn test_signature_verification() {
        // Generate a random key pair
        let signing_key = SigningKey::random(&mut OsRng);
        let verifying_key = VerifyingKey::from(&signing_key);
        
        // Create a test message and timestamp
        let message = "test message";
        let timestamp = chrono::Utc::now().timestamp();
        
        // Sign the message
        let (signature, recovery_id) = sign_message(message, timestamp, &signing_key).unwrap();
        
        // Create signature data
        let sig_data = SignatureData {
            signature,
            recovery_id,
            timestamp,
            message: message.to_string(),
        };
        
        // Verify the signature
        let result = verify_signature(&sig_data, &verifying_key).unwrap();
        assert!(result);
        
        // Verify that a wrong key fails
        let wrong_key = VerifyingKey::from(&SigningKey::random(&mut OsRng));
        let result = verify_signature(&sig_data, &wrong_key);
        assert!(result.is_err());
    }
    
    #[test]
    fn test_message_hash() {
        let message = "test message";
        let timestamp = 1625097600i64;
        
        // Generate hash
        let hash1 = create_message_hash(message, timestamp);
        
        // Generate again - should be deterministic
        let hash2 = create_message_hash(message, timestamp);
        
        // Hashes should match
        assert_eq!(hash1, hash2);
        
        // Changing message should change hash
        let hash3 = create_message_hash("different message", timestamp);
        assert_ne!(hash1, hash3);
        
        // Changing timestamp should change hash
        let hash4 = create_message_hash(message, timestamp + 1);
        assert_ne!(hash1, hash4);
    }
    
    #[test]
    fn test_timestamp_validation() {
        let now = chrono::Utc::now().timestamp();
        
        // Current timestamp should be valid
        assert!(validate_timestamp(now, 30));
        
        // Timestamp 15 seconds in the past should be valid with 30s window
        assert!(validate_timestamp(now - 15, 30));
        
        // Timestamp 15 seconds in the future should be valid with 30s window
        assert!(validate_timestamp(now + 15, 30));
        
        // Timestamp 60 seconds in the past should be invalid with 30s window
        assert!(!validate_timestamp(now - 60, 30));
        
        // Timestamp 60 seconds in the future should be invalid with 30s window
        assert!(!validate_timestamp(now + 60, 30));
    }
} 