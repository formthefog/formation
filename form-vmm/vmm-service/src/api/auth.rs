use alloy_primitives::Address;
use k256::ecdsa::{RecoveryId, Signature, VerifyingKey};
use tiny_keccak::{Hasher, Sha3};
use form_state::instances::Instance;
use form_types::state::{Response, Success};

use crate::error::VmmError;

/// Utilities for verifying signatures and authorization for VM operations
pub struct SignatureVerifier;

impl SignatureVerifier {
    /// Verifies a signature and returns the signer's Ethereum address
    pub fn verify_signature<T: AsRef<[u8]>>(
        message: T,
        signature: &str,
        recovery_id: u32
    ) -> Result<String, VmmError> {
        // Decode the signature
        let sig_bytes = hex::decode(signature)
            .map_err(|e| VmmError::Config(format!("Invalid signature format: {}", e)))?;
        
        let signature = Signature::from_slice(&sig_bytes)
            .map_err(|e| VmmError::Config(format!("Invalid signature: {}", e)))?;
        
        // Convert recovery_id to RecoveryId
        let rec_id = RecoveryId::from_byte(recovery_id.to_be_bytes()[3])
            .ok_or_else(|| VmmError::Config("Invalid recovery ID".to_string()))?;
        
        // Hash the message
        let mut hasher = Sha3::v256();
        let mut hash = [0u8; 32];
        hasher.update(message.as_ref());
        hasher.finalize(&mut hash);
        
        // Recover the public key
        let verifying_key = VerifyingKey::recover_from_msg(
            &hash,
            &signature,
            rec_id
        ).map_err(|e| VmmError::Config(format!("Failed to recover public key: {}", e)))?;
        
        // Convert to Ethereum address
        let address = Address::from_public_key(&verifying_key);
        
        Ok(format!("{:x}", address))
    }
    
    /// Creates a standardized message for VM operations to be used in signature verification
    pub fn create_operation_message(op_type: &str, instance_id: &str) -> String {
        format!("{}:{}", op_type, instance_id)
    }
    
    /// Generates the message hash for a VM operation
    pub fn hash_operation_message(op_type: &str, instance_id: &str) -> [u8; 32] {
        let message = Self::create_operation_message(op_type, instance_id);
        let mut hasher = Sha3::v256();
        let mut hash = [0u8; 32];
        hasher.update(message.as_bytes());
        hasher.finalize(&mut hash);
        hash
    }
}

/// Permission levels for VM operations
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Permission {
    /// Can only view instance details
    ReadOnly,
    /// Can start and stop instances
    Operator,
    /// Can modify instance configuration
    Manager,
    /// Full control including ownership transfer
    Owner,
}

/// Utilities for verifying instance ownership and authorization
pub struct OwnershipVerifier;

impl OwnershipVerifier {
    /// Verifies if an address is authorized to perform an operation on an instance
    pub async fn verify_authorization(
        instance_id: &str,
        address: &str,
        _required_permission: Permission
    ) -> Result<bool, VmmError> {
        // Get the instance details
        let instance = Self::get_instance(instance_id).await
            .map_err(|e| VmmError::Config(format!("Error retrieving instance: {}", e)))?;
        
        // Check if the address matches the instance owner
        // For now, we only check direct ownership - in future phases, we'll expand this
        if instance.instance_owner.to_lowercase() == address.to_lowercase() {
            return Ok(true);
        }
        
        // For now, only the owner has access
        // In future phases, we'll implement more sophisticated authorization checks
        Ok(false)
    }
    
    /// Retrieves an instance by ID from the state store
    async fn get_instance(instance_id: &str) -> Result<Instance, Box<dyn std::error::Error + Send + Sync>> {
        // Query the instance from the state service
        let client = reqwest::Client::new();
        let response = client.get(&format!("http://127.0.0.1:3000/instances/{}", instance_id))
            .send()
            .await?
            .json::<Response<Instance>>()
            .await?;
        
        match response {
            Response::Success(success) => {
                match success {
                    Success::Some(data) => Ok(data),
                    _ => Err(Box::new(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        "Unexpected response format"
                    )))
                }
            },
            Response::Failure { reason } => Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Error retrieving instance: {:?}", reason)
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use k256::ecdsa::SigningKey;
    use rand::thread_rng;
    
    #[test]
    fn test_signature_verification() {
        // Generate a random key for testing
        let signing_key = SigningKey::random(&mut thread_rng());
        let verifying_key = VerifyingKey::from(&signing_key);
        
        // Create a test message
        let operation = "TestOperation";
        let instance_id = "test-instance-123";
        let message = SignatureVerifier::create_operation_message(operation, instance_id);
        
        // Hash the message
        let mut hasher = Sha3::v256();
        let mut hash = [0u8; 32];
        hasher.update(message.as_bytes());
        hasher.finalize(&mut hash);
        
        // Sign the message
        let (signature, recovery_id) = signing_key.sign_prehash_recoverable(&hash)
            .expect("Failed to sign message");
        
        // Verify the signature
        let signature_hex = hex::encode(signature.to_bytes());
        let recovered_address = SignatureVerifier::verify_signature(
            message.clone(),
            &signature_hex,
            recovery_id.to_byte() as u32
        ).expect("Failed to verify signature");
        
        // Generate the expected address
        let expected_address = format!("{:x}", Address::from_public_key(&verifying_key));
        
        // Compare the addresses
        assert_eq!(recovered_address, expected_address);
    }
} 