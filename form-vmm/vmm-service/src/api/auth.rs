use alloy_primitives::Address;
use k256::ecdsa::{RecoveryId, Signature, VerifyingKey};
use form_state::instances::Instance;
use form_types::state::{Response, Success};
use axum::{
    async_trait,
    extract::{FromRequestParts, Request},
    http::{request::Parts, HeaderMap, StatusCode},
    response::{IntoResponse, Response as AxumResponse},
    Json,
};
use serde_json::json;
use reqwest::Client;
use std::sync::Arc;
use log;
use hex;
use serde::{Deserialize, Serialize};
use tiny_keccak::{self, Hasher};

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
        let (status, error_message_str) = match self {
            Self::MissingSignature => (StatusCode::UNAUTHORIZED, "Missing signature headers (X-Signature, X-Recovery-Id, X-Message)"),
            Self::InvalidSignature => (StatusCode::UNAUTHORIZED, "Invalid signature content"),
            Self::InvalidMessage => (StatusCode::BAD_REQUEST, "Invalid X-Message format (must be hex hash)"),
            Self::RecoveryFailed => (StatusCode::UNAUTHORIZED, "Failed to recover public key from signature"),
            Self::InvalidFormat => (StatusCode::BAD_REQUEST, "Invalid signature header format"),
        };
        let body = Json(json!({
            "error": error_message_str,
        }));
        (status, body).into_response()
    }
}

/// A struct containing the recovered public key/address and the message hash that was verified
#[derive(Debug, Clone)]
pub struct RecoveredAddress {
    pub address: Address,
    pub message_hash: Vec<u8>,
}

impl RecoveredAddress {
    /// Get the address as a hex string
    pub fn as_hex(&self) -> String {
        hex::encode(self.address.as_slice())
    }
}

/// Extract signature parts from X-Headers
pub fn extract_x_signature_parts(headers: &HeaderMap) -> Result<(Vec<u8>, RecoveryId, Vec<u8>), SignatureError> {
    let signature_hex = headers
        .get("X-Signature")
        .ok_or(SignatureError::MissingSignature)?
        .to_str()
        .map_err(|_| SignatureError::InvalidFormat)?;

    let recovery_id_str = headers
        .get("X-Recovery-Id")
        .ok_or(SignatureError::MissingSignature)?
        .to_str()
        .map_err(|_| SignatureError::InvalidFormat)?;

    let message_hash_hex = headers
        .get("X-Message")
        .ok_or(SignatureError::MissingSignature)?
        .to_str()
        .map_err(|_| SignatureError::InvalidFormat)?;

    let signature_bytes = hex::decode(signature_hex)
        .map_err(|_| SignatureError::InvalidFormat)?;

    let recovery_id_byte = recovery_id_str.parse::<u8>().map_err(|_| SignatureError::InvalidFormat)?;
    let recovery_id = RecoveryId::from_byte(recovery_id_byte)
        .ok_or(SignatureError::InvalidFormat)?;

    let cleaned_message_hash_hex = message_hash_hex.strip_prefix("0x").unwrap_or(message_hash_hex);
    let message_hash_bytes = hex::decode(cleaned_message_hash_hex)
        .map_err(|_| SignatureError::InvalidMessage)?;

    Ok((signature_bytes, recovery_id, message_hash_bytes))
}

/// Recover an address from a signature, recovery ID, and the message hash
pub fn recover_address_from_hash(signature_bytes: &[u8], recovery_id: RecoveryId, message_hash_bytes: &[u8]) -> Result<Address, SignatureError> {
    let signature = Signature::try_from(signature_bytes)
        .map_err(|_| SignatureError::InvalidSignature)?;

    log::debug!("Recovering address from signature using pre-computed hash.");
    log::debug!("Message hash (from X-Message, hex-decoded): {}", hex::encode(message_hash_bytes));
    log::debug!("Signature: {}", hex::encode(signature_bytes));
    log::debug!("Recovery ID: {}", recovery_id.to_byte());

    let recovered_key = VerifyingKey::recover_from_msg(
        message_hash_bytes,
        &signature,
        recovery_id,
    ).map_err(|e| {
        log::error!("Failed to recover public key: {:?}", e);
        SignatureError::RecoveryFailed
    })?;

    let address = Address::from_public_key(&recovered_key);
    log::debug!("Recovered address: 0x{}", hex::encode(address.as_slice()));
    Ok(address)
}

/// Axum extractor for recovering an address from X-Headers
#[async_trait]
impl<S> FromRequestParts<S> for RecoveredAddress
where
    S: Send + Sync,
{
    type Rejection = SignatureError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let (signature_bytes, recovery_id, message_hash_bytes) = extract_x_signature_parts(&parts.headers)?;
        let address = recover_address_from_hash(&signature_bytes, recovery_id, &message_hash_bytes)?;
        Ok(RecoveredAddress {
            address,
            message_hash: message_hash_bytes,
        })
    }
}

/// Middleware function to verify ECDSA signatures from X-Headers
pub async fn ecdsa_auth_middleware_x_headers(
    request: Request,
    next: axum::middleware::Next,
) -> Result<AxumResponse, SignatureError> {
    let path = request.uri().path().to_owned();
    if path == "/health" || path == "/ping" {
        return Ok(next.run(request).await);
    }
    
    let (mut http_parts, body) = request.into_parts();

    let recovered_address = match RecoveredAddress::from_request_parts(&mut http_parts, &()).await {
        Ok(addr) => addr,
        Err(e) => {
            log::warn!("ECDSA Authentication failed for path '{}': {:?}", path, e);
            return Err(e);
        }
    };
    
    let mut request_with_ext = Request::from_parts(http_parts, body);
    request_with_ext.extensions_mut().insert(Arc::new(recovered_address));
    
    log::info!("ECDSA Authentication successful for path '{}'", path);
    Ok(next.run(request_with_ext).await)
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
#[derive(Debug, thiserror::Error, Serialize)]
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

/// Need a function to sign a message (hash) with form-vmm's operator key
/// This is a placeholder for where this utility would live or be accessed from.
/// For now, assume it exists and can be called.
async fn sign_message_with_operator_key(message_hash_hex: &str) -> Result<(String, String), String> {
    // In a real scenario, this function would:
    // 1. Access the VMM operator's private signing key.
    // 2. Hex-decode message_hash_hex to bytes.
    // 3. Sign the hash bytes.
    // 4. Return (hex_signature, recovery_id_string).
    // Placeholder implementation:
    log::warn!("sign_message_with_operator_key is a placeholder and not signing with a real key!");
    Ok(("placeholder_signature_hex".to_string(), "0".to_string()))
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
        log::debug!("Verifying authorization for instance '{}', address '{}'", instance_id, address);
        match Self::get_instance(instance_id).await {
            Ok(instance) => {
                if instance.instance_owner.eq_ignore_ascii_case(address) {
                    log::debug!("Authorization successful: Address matches instance owner.");
                    Ok(true)
                } else {
                    log::warn!("Authorization failed: Address '{}' does not match instance owner '{}' for instance '{}'", address, instance.instance_owner, instance_id);
                    Ok(false)
                }
            },
            Err(e) => {
                log::error!("Error retrieving instance '{}' for authorization: {}. Assuming unauthorized.", instance_id, e);
                Err(VmmError::Config(format!("Failed to retrieve instance '{}' for auth check: {}", instance_id, e)))
            }
        }
    }
    
    /// Retrieves an instance by ID from the state store
    async fn get_instance(instance_id: &str) -> Result<Instance, Box<dyn std::error::Error + Send + Sync>> {
        let client = reqwest::Client::new();
        let path = format!("/instance/{}/get", instance_id);
        let url = format!("http://127.0.0.1:3004{}", path);
        log::debug!("OwnershipVerifier: Getting instance from {}", url);

        // Hash the path for X-Message (assuming Keccak256 for Ethereum-style, or SHA256 if that's the standard)
        // For consistency with README example, let's use a placeholder for hashing for now.
        // In form-state auth, X-Message is typically hash of body for POST, or path for GET.
        // Let's assume client for form-state expects path hash for GET /instance/{id}/get.
        // Hashing mechanism should align with form-state's expectation for this endpoint.
        let mut hasher = tiny_keccak::Keccak::v256(); // Or ::Sha256 if that's used by form-state for GET paths
        let mut output = [0u8; 32];
        hasher.update(path.as_bytes());
        hasher.finalize(&mut output);
        let path_hash_hex = format!("0x{}", hex::encode(output));

        // Sign this path_hash_hex with form-vmm's operator key
        let (operator_signature_hex, operator_recovery_id_str) = 
            sign_message_with_operator_key(&path_hash_hex).await.map_err(|e| Box::new(std::io::Error::new(std::io::ErrorKind::Other, e)))?;

        let response = client.get(&url)
            .header("X-Message", path_hash_hex)
            .header("X-Signature", operator_signature_hex)
            .header("X-Recovery-Id", operator_recovery_id_str)
            .send().await?;

        if !response.status().is_success() {
            let err_msg = format!("Error retrieving instance from form-state: Status {}, Body: {}", response.status(), response.text().await.unwrap_or_default());
            log::error!("{}", err_msg);
            return Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, err_msg)));
        }

        let state_response = response.json::<form_types::state::Response<form_state::instances::Instance>>().await?;
        
        match state_response {
            form_types::state::Response::Success(success_payload) => match success_payload {
                form_types::state::Success::Some(instance_data) => Ok(instance_data),
                form_types::state::Success::List(_) => Err(Box::new(std::io::Error::new(std::io::ErrorKind::InvalidInput, "Expected single instance from form-state, got a list"))),
                form_types::state::Success::Relationships(_) => Err(Box::new(std::io::Error::new(std::io::ErrorKind::InvalidInput, "Expected single instance from form-state, got relationships"))),
                form_types::state::Success::None => Err(Box::new(std::io::Error::new(std::io::ErrorKind::NotFound, "Instance not found in form-state (Success::None)"))),
            },
            form_types::state::Response::Failure { reason } => {
                 let err_msg = format!("form-state failed to get instance '{}': {:?}", instance_id, reason);
                 log::error!("{}", err_msg);
                 Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, err_msg)))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use k256::ecdsa::SigningKey;
    use rand::thread_rng;
    
    #[test]
    fn test_x_header_signature_flow() {
        let signing_key = SigningKey::random(&mut thread_rng());
        let verifying_key = VerifyingKey::from(&signing_key);
        
        let message_payload_hash_hex = "49a57138898700459722e900105355958830155d835118513628737314067633";
        let message_hash_bytes = hex::decode(message_payload_hash_hex).unwrap();

        // Use try_sign_prehash_recoverable to get both signature and recovery ID
        let (signature_ecdsa, recovery_id): (k256::ecdsa::Signature, RecoveryId) = 
            signing_key.sign_prehash_recoverable(&message_hash_bytes).unwrap();
        
        // Convert signature to bytes (Signature from k256::ecdsa can be converted to bytes directly)
        let signature_bytes = signature_ecdsa.to_bytes();
        
        // Test recover_address_from_hash
        let recovered_address = recover_address_from_hash(&signature_bytes, recovery_id, &message_hash_bytes).unwrap();
        assert_eq!(recovered_address, Address::from_public_key(&verifying_key));

        // Test with X-Header extraction logic (mock HeaderMap)
        let mut headers = HeaderMap::new();
        headers.insert("X-Signature", hex::encode(signature_bytes).parse().unwrap());
        headers.insert("X-Recovery-Id", recovery_id.to_byte().to_string().parse().unwrap());
        // Test with and without "0x" prefix for X-Message
        headers.insert("X-Message", format!("0x{}", message_payload_hash_hex).parse().unwrap());

        let (extracted_sig, extracted_rec_id, extracted_msg_hash) = extract_x_signature_parts(&headers).unwrap();
        assert_eq!(extracted_sig, signature_bytes.as_slice());
        assert_eq!(extracted_rec_id, recovery_id);
        assert_eq!(extracted_msg_hash, message_hash_bytes);

        let final_recovered_address = recover_address_from_hash(&extracted_sig, extracted_rec_id, &extracted_msg_hash).unwrap();
        assert_eq!(final_recovered_address, Address::from_public_key(&verifying_key));

        // Test X-Message without "0x" prefix
        headers.insert("X-Message", message_payload_hash_hex.parse().unwrap());
        let (_, _, extracted_msg_hash_no_prefix) = extract_x_signature_parts(&headers).unwrap();
        assert_eq!(extracted_msg_hash_no_prefix, message_hash_bytes);
    }
} 