use alloy_primitives::Address;
use k256::ecdsa::{RecoveryId, Signature, VerifyingKey};
use tiny_keccak::{Hasher, Sha3};
use form_state::instances::Instance;
use form_types::state::{Response, Success};
use axum::{
    async_trait,
    extract::{FromRequestParts, Request},
    http::{request::Parts, StatusCode, HeaderMap},
    response::{IntoResponse, Response as AxumResponse},
    Json,
};
use sha2::{Sha256, Digest};
use serde::{Serialize, Deserialize};
use serde_json::json;
use reqwest::Client;
use std::sync::Arc;
use log;
use hex;

use crate::error::VmmError;

/// Error type for signature verification failures
#[derive(Debug, Serialize)]
pub enum SignatureError {
    MissingSignature,
    InvalidSignature,
    InvalidMessage,
    RecoveryFailed,
    InvalidFormat,
}

impl IntoResponse for SignatureError {
    fn into_response(self) -> AxumResponse {
        let (status, message) = match self {
            Self::MissingSignature => (StatusCode::UNAUTHORIZED, "Missing signature"),
            Self::InvalidSignature => (StatusCode::UNAUTHORIZED, "Invalid signature"),
            Self::InvalidMessage => (StatusCode::BAD_REQUEST, "Invalid message format"),
            Self::RecoveryFailed => (StatusCode::UNAUTHORIZED, "Failed to recover public key"),
            Self::InvalidFormat => (StatusCode::BAD_REQUEST, "Invalid signature format"),
        };

        let body = Json(json!({
            "error": message,
        }));

        (status, body).into_response()
    }
}

/// A struct containing the recovered public key/address from a signature
#[derive(Debug, Clone)]
pub struct RecoveredAddress {
    pub address: Address,
    pub message: Vec<u8>,
}

impl RecoveredAddress {
    /// Get the address as a hex string
    pub fn as_hex(&self) -> String {
        hex::encode(self.address.as_slice())
    }
}

/// Extract a signature from the Authorization header
///
/// Expects the header to be in the format:
/// `Authorization: Signature <signature_hex>.<recovery_id>.<message_hex>`
pub fn extract_signature_parts(headers: &HeaderMap) -> Result<(Vec<u8>, RecoveryId, Vec<u8>), SignatureError> {
    // Get the authorization header
    let auth_header = headers
        .get("authorization")
        .ok_or(SignatureError::MissingSignature)?
        .to_str()
        .map_err(|_| SignatureError::InvalidFormat)?;
    
    // Check if it starts with "Signature "
    if !auth_header.starts_with("Signature ") {
        return Err(SignatureError::InvalidFormat);
    }
    
    // Parse the signature parts after "Signature "
    let signature_data = &auth_header["Signature ".len()..];
    let parts: Vec<&str> = signature_data.split('.').collect();
    
    if parts.len() != 3 {
        return Err(SignatureError::InvalidFormat);
    }
    
    // Parse signature, recovery ID, and message
    let signature_bytes = hex::decode(parts[0])
        .map_err(|_| SignatureError::InvalidFormat)?;
        
    let recovery_id_byte = parts[1].parse::<u8>().map_err(|_| SignatureError::InvalidFormat)?;
    let recovery_id = match RecoveryId::from_byte(recovery_id_byte) {
        Some(id) => id,
        None => return Err(SignatureError::InvalidFormat),
    };
    
    let message = hex::decode(parts[2])
        .map_err(|_| SignatureError::InvalidFormat)?;
    
    Ok((signature_bytes, recovery_id, message))
}

/// Recover an address from a signature, recovery ID, and message
pub fn recover_address(signature_bytes: &[u8], recovery_id: RecoveryId, message: &[u8]) -> Result<Address, SignatureError> {
    // Create a recoverable signature
    let signature = Signature::try_from(signature_bytes)
        .map_err(|_| SignatureError::InvalidSignature)?;
    
    // Hash the message with SHA-256
    let mut hasher = Sha256::new();
    hasher.update(message);
    let message_hash = hasher.finalize();
    
    log::debug!("Recovering address from signature. Message: {}", String::from_utf8_lossy(message));
    log::debug!("Message hash: {}", hex::encode(message_hash));
    log::debug!("Signature: {}", hex::encode(signature_bytes));
    log::debug!("Recovery ID: {}", recovery_id.to_byte());
    
    // Recover the public key from the signature
    let recovery_result = k256::ecdsa::VerifyingKey::recover_from_msg(
        message_hash.as_slice(),
        &signature,
        recovery_id,
    ).map_err(|_| SignatureError::RecoveryFailed)?;
    
    // Take the last 20 bytes as the address
    let address = Address::from_public_key(&recovery_result);
    log::debug!("Recovered address: 0x{}", hex::encode(address.as_slice()));
    
    Ok(address)
}

/// Axum extractor for recovering an address from a signature
#[async_trait]
impl<S> FromRequestParts<S> for RecoveredAddress
where
    S: Send + Sync,
{
    type Rejection = SignatureError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let (signature_bytes, recovery_id, message) = extract_signature_parts(&parts.headers)?;
        
        let address = recover_address(&signature_bytes, recovery_id, &message)?;
        
        Ok(RecoveredAddress {
            address,
            message,
        })
    }
}

/// Optional address recovery that doesn't reject the request if authentication fails
#[derive(Debug, Clone)]
pub struct OptionalRecoveredAddress(pub Option<RecoveredAddress>);

#[async_trait]
impl<S> FromRequestParts<S> for OptionalRecoveredAddress
where
    S: Send + Sync,
{
    type Rejection = SignatureError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        match RecoveredAddress::from_request_parts(parts, state).await {
            Ok(address) => Ok(OptionalRecoveredAddress(Some(address))),
            Err(SignatureError::MissingSignature) => Ok(OptionalRecoveredAddress(None)),
            Err(other) => Err(other),
        }
    }
}

/// Middleware function to verify ECDSA signatures
pub async fn ecdsa_auth_middleware(
    request: Request,
    next: axum::middleware::Next,
) -> Result<AxumResponse, SignatureError> {
    // Extract headers for verification
    let headers = request.headers().clone();
    
    // Skip authentication for specific endpoints
    if request.uri().path() == "/health" || request.uri().path() == "/ping" {
        return Ok(next.run(request).await);
    }
    
    // Extract signature parts and verify
    let (signature_bytes, recovery_id, message) = extract_signature_parts(&headers)?;
    
    // Recover the address - this just verifies the signature is valid
    let address = recover_address(&signature_bytes, recovery_id, &message)?;
    
    // Store the recovered address in request extensions
    let mut request = request;
    request.extensions_mut().insert(RecoveredAddress {
        address,
        message: message.to_vec(),
    });
    
    // Authentication successful - let the handler handle authorization
    Ok(next.run(request).await)
}

/// Function to create a client that includes the signature in requests to form-state
pub fn create_auth_client(signature: &str, recovery_id: u8, message: &str) -> reqwest::Client {
    let auth_header = format!("Signature {}.{}.{}", signature, recovery_id, message);
    let client = reqwest::Client::builder()
        .default_headers({
            let mut headers = reqwest::header::HeaderMap::new();
            headers.insert(
                reqwest::header::AUTHORIZATION,
                reqwest::header::HeaderValue::from_str(&auth_header).unwrap(),
            );
            headers
        })
        .build()
        .unwrap();
    
    client
}

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

/// Error type for authorization failures
#[derive(Debug, thiserror::Error)]
pub enum AuthorizationError {
    #[error("Resource not found")]
    ResourceNotFound,
    
    #[error("Access denied")]
    AccessDenied,
    
    #[error("Network error: {0}")]
    NetworkError(String),
    
    #[error("Unknown error: {0}")]
    Unknown(String),
}

/// Authorization client for checking permissions with form-state
pub struct AuthorizationClient {
    /// Base URL for the form-state API
    base_url: String,
    /// HTTP client
    client: Client,
}

impl AuthorizationClient {
    /// Create a new authorization client
    pub fn new(base_url: String) -> Self {
        Self {
            base_url,
            client: Client::new(),
        }
    }
    
    /// Check if a user has access to a resource
    pub async fn check_resource_access(
        &self,
        address: &str,
        resource_id: &str,
        resource_type: &str,
    ) -> Result<bool, AuthorizationError> {
        let url = format!("{}/auth/check_access", self.base_url);
        
        let payload = serde_json::json!({
            "address": address,
            "resource_id": resource_id,
            "resource_type": resource_type,
        });
        
        let response = self.client.post(&url)
            .json(&payload)
            .send()
            .await
            .map_err(|e| AuthorizationError::NetworkError(e.to_string()))?;
        
        match response.status() {
            StatusCode::OK => {
                let body: serde_json::Value = response.json()
                    .await
                    .map_err(|e| AuthorizationError::Unknown(e.to_string()))?;
                
                // Parse the response to check if access is granted
                if let Some(has_access) = body.get("has_access").and_then(|v| v.as_bool()) {
                    Ok(has_access)
                } else {
                    log::error!("Unexpected response format: {:?}", body);
                    Err(AuthorizationError::Unknown("Unexpected response format".to_string()))
                }
            },
            StatusCode::NOT_FOUND => Err(AuthorizationError::ResourceNotFound),
            StatusCode::FORBIDDEN => Err(AuthorizationError::AccessDenied),
            _ => {
                let error_msg = format!(
                    "Unexpected status code: {}", 
                    response.status()
                );
                log::error!("{}", error_msg);
                Err(AuthorizationError::Unknown(error_msg))
            }
        }
    }
    
    /// Forward a request to form-state with the authenticated user's address
    /// but with the admin node's credentials
    pub async fn forward_authenticated_request<T, R>(
        &self,
        endpoint: &str,
        user_address: &str,
        payload: &T,
        admin_signature: &str,
        admin_recovery_id: u8,
        admin_message: &str,
    ) -> Result<R, AuthorizationError>
    where
        T: Serialize,
        R: for<'de> Deserialize<'de>,
    {
        // Create a client with admin credentials
        let client = create_auth_client(admin_signature, admin_recovery_id, admin_message);
        
        // Add the user's address to the payload
        let mut payload_with_user = serde_json::to_value(payload)
            .map_err(|e| AuthorizationError::Unknown(e.to_string()))?;
        
        if let serde_json::Value::Object(ref mut map) = payload_with_user {
            map.insert(
                "original_user_address".to_string(), 
                serde_json::Value::String(user_address.to_string())
            );
        }
        
        let url = format!("{}{}", self.base_url, endpoint);
        
        let response = client.post(&url)
            .json(&payload_with_user)
            .send()
            .await
            .map_err(|e| AuthorizationError::NetworkError(e.to_string()))?;
        
        match response.status() {
            StatusCode::OK | StatusCode::CREATED => {
                response.json::<R>()
                    .await
                    .map_err(|e| AuthorizationError::Unknown(e.to_string()))
            },
            StatusCode::NOT_FOUND => Err(AuthorizationError::ResourceNotFound),
            StatusCode::FORBIDDEN => Err(AuthorizationError::AccessDenied),
            _ => {
                let error_msg = format!(
                    "Unexpected status code: {}", 
                    response.status()
                );
                log::error!("{}", error_msg);
                Err(AuthorizationError::Unknown(error_msg))
            }
        }
    }
}

/// Helper function to extract an address for authorization
pub fn extract_address_for_auth(recovered: &RecoveredAddress) -> String {
    recovered.as_hex()
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