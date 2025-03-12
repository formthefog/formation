// Signature verification for authentication
//
// This module handles cryptographic signature verification for the MCP server.

use k256::ecdsa::{Signature, VerifyingKey, signature::{Signer, Verifier}};
use std::str::FromStr;
use crate::auth::keypair::{KeyPair, derive_address};
use crate::errors::AuthError;
use hex;

/// Verifies a signature against a message and a verifying key
pub fn verify_signature(
    message: &[u8],
    signature: &[u8],
    verifying_key: &VerifyingKey
) -> Result<bool, AuthError> {
    // Parse the signature from bytes
    let signature = Signature::from_slice(signature)
        .map_err(|_| AuthError::SignatureVerification)?;
    
    // Verify the signature
    match verifying_key.verify(message, &signature) {
        Ok(_) => Ok(true),
        Err(_) => Ok(false)
    }
}

/// Verifies a signed message against a public address
pub fn verify_signed_message(
    message: &[u8],
    signature: &[u8],
    address: &str
) -> Result<bool, AuthError> {
    // Parse the signature from bytes
    let signature = Signature::from_slice(signature)
        .map_err(|_| AuthError::SignatureVerification)?;
    
    // Extract the recovery ID from the signature if available
    // Note: This is a simplified implementation, in a complete system
    // you would need to handle proper recovery of the public key from the signature
    
    // For now, let's assume we can't recover the public key
    // In a production system, you'd extract the recovery ID and recover the public key
    
    return Err(AuthError::NotImplemented("Public key recovery from signature not implemented".to_string()));
    
    /* 
    // Placeholder for recovery logic:
    let recovered_key = recover_public_key(message, signature)?;
    let recovered_address = derive_address(&recovered_key);
    
    // Compare the recovered address with the provided address
    Ok(address.to_lowercase() == recovered_address.to_lowercase())
    */
}

/// Creates a signature for a message using a keypair
pub fn sign_message(
    message: &[u8],
    keypair: &KeyPair
) -> Result<Vec<u8>, AuthError> {
    // Sign the message
    let signature: k256::ecdsa::Signature = keypair.signing_key.sign(message);
    
    // Convert the signature to bytes
    Ok(signature.to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::keypair::KeyPair;
    
    #[test]
    fn test_signature_verification() {
        // Create a keypair
        let keypair = KeyPair::random();
        
        // Message to sign
        let message = b"Test message";
        
        // Sign the message
        let signature = sign_message(message, &keypair).unwrap();
        
        // Verify the signature
        let result = verify_signature(message, &signature, &keypair.verifying_key).unwrap();
        
        assert!(result, "Signature verification failed");
    }
    
    #[test]
    fn test_invalid_signature() {
        // Create a keypair
        let keypair = KeyPair::random();
        
        // Message to sign
        let message = b"Test message";
        
        // Sign the message
        let signature = sign_message(message, &keypair).unwrap();
        
        // Different message
        let different_message = b"Different message";
        
        // Verify the signature with the wrong message
        let result = verify_signature(different_message, &signature, &keypair.verifying_key).unwrap();
        
        assert!(!result, "Signature verification should have failed");
    }
} 