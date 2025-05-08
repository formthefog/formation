use axum::{
    async_trait,
    extract::{FromRequestParts, Request},
    http::{request::Parts, StatusCode, HeaderMap, Method},
    response::{IntoResponse, Response},
    Json,
};
use k256::ecdsa::{RecoveryId, Signature};
use alloy_primitives::Address;
use serde::Serialize;
use serde_json::json;
use k256::sha2::{Sha256, Digest};
use tiny_keccak::Hasher;
use hex;
use log;
use std::sync::Arc;
use std::net::SocketAddr;

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
    fn into_response(self) -> Response {
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
) -> Result<Response, SignatureError> {
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
    let auth_header = format!("Signature {}", signature);
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
