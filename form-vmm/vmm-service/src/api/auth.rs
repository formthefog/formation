use alloy_primitives::Address;
use k256::ecdsa::{Signature, VerifyingKey};
use tiny_keccak::{Hasher, Sha3};
use form_state::instances::Instance;
use form_types::state::{Response, Success};
use form_auth::{AuthError, signature::{SignatureData, create_message_hash}};

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
        // Create a SignatureData struct for form-auth
        let timestamp = chrono::Utc::now().timestamp(); // We're not using timestamp validation here
        let sig_data = SignatureData {
            signature: signature.to_string(),
            recovery_id: recovery_id.to_string(),
            timestamp,
            message: String::from_utf8_lossy(message.as_ref()).to_string(),
        };
        
        // Recover the public key
        let verifying_key = form_auth::signature::recover_public_key(&sig_data)
            .map_err(|e| VmmError::Config(format!("Signature verification failed: {}", e)))?;
        
        // Convert to Ethereum address
        let address = Address::from_public_key(&verifying_key);
        
        // Return the address as a hex string
        Ok(format!("{:x}", address))
    }
    
    /// Creates a standardized message for VM operations to be used in signature verification
    pub fn create_operation_message(op_type: &str, instance_id: &str) -> String {
        format!("{}:{}", op_type, instance_id)
    }
    
    /// Generates the message hash for a VM operation
    pub fn hash_operation_message(op_type: &str, instance_id: &str) -> [u8; 32] {
        let message = Self::create_operation_message(op_type, instance_id);
        let digest = create_message_hash(&message, 0);
        let mut hash = [0u8; 32];
        hash.copy_from_slice(&digest[0..32]);
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
        required_permission: Permission
    ) -> Result<bool, VmmError> {
        // Get the instance details
        let instance = Self::get_instance(instance_id).await
            .map_err(|e| VmmError::Config(format!("Error retrieving instance: {}", e)))?;
        
        // Check if the address matches the instance owner
        if instance.instance_owner.to_lowercase() == address.to_lowercase() {
            return Ok(true);
        }
        
        // For ReadOnly permission, we could check if the address is an authorized viewer
        if required_permission == Permission::ReadOnly {
            // Check if this user has read access (implementation depends on your authorization model)
            // This could involve checking a list of authorized viewers, team members, etc.
            // For now, we'll fall back to the default permission check
        }
        
        // For Operator permission, check if the address is an authorized operator
        if required_permission == Permission::Operator || required_permission == Permission::Manager {
            // Check if this user has operator access
            // This could involve checking teams, authorized developers, etc.
            // For now, we're implementing a basic check; expand as needed
            
            // Example: Check if the address is in a list of collaborators (if your Instance model has this)
            if let Some(collaborators) = instance.metadata.annotations.additional_data.get("authorized_collaborators") {
                if collaborators.contains(&address.to_lowercase()) {
                    return Ok(true);
                }
            }
        }
        
        // If we reach here, the user doesn't have the required permission
        Ok(false)
    }
    
    /// Retrieves an instance by ID from the state store
    async fn get_instance(instance_id: &str) -> Result<Instance, Box<dyn std::error::Error + Send + Sync>> {
        // Query the instance from the state service
        let client = reqwest::Client::new();
        let response = client.get(&format!("http://127.0.0.1:3004/instance/{}/get", instance_id))
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
        
        // Sign the message using form-auth
        let timestamp = chrono::Utc::now().timestamp();
        let (signature, recovery_id) = form_auth::signature::sign_message(&message, timestamp, &signing_key)
            .expect("Failed to sign message");
        
        // Verify the signature
        let recovered_address = SignatureVerifier::verify_signature(
            message.clone(),
            &signature,
            u8::from_str_radix(&recovery_id, 16).unwrap() as u32
        ).expect("Failed to verify signature");
        
        // Generate the expected address
        let expected_address = format!("{:x}", Address::from_public_key(&verifying_key));
        
        // Compare the addresses
        assert_eq!(recovered_address, expected_address);
    }
} 