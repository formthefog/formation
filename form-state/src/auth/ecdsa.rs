use axum::{
    async_trait,
    extract::{FromRequestParts, Request, ConnectInfo},
    http::{request::Parts, StatusCode, HeaderMap},
    response::{IntoResponse, Response},
    Json,
};
use k256::ecdsa::{RecoveryId, Signature};
use alloy_primitives::Address;
use serde::Serialize;
use serde_json::json;
use sha2::{Sha256, Digest};
use tiny_keccak::Hasher;
use hex;
use log;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::Mutex;
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
    mut request: Request,
    next: axum::middleware::Next,
) -> Result<Response, SignatureError> {
    // Check for localhost connection
    log::debug!("ECDSA_AUTH: Checking for localhost connection.");
    let is_localhost = {
        let connection_info = request.extensions().get::<ConnectInfo<SocketAddr>>();
        let remote_addr = connection_info.map(|c| c.0.to_string()).unwrap_or("".to_string());
        remote_addr.starts_with("127.0.0.1") || remote_addr.starts_with("::1")
    };

    if is_localhost {
        // Skip auth for localhost connections
        // Add a default service identity to the request extensions
        log::debug!("ECDSA_AUTH: Localhost connection detected. Skipping auth.");
        request.extensions_mut().insert(None::<RecoveredAddress>);
        return Ok(next.run(request).await);
    }
    
    let headers = request.headers().clone();
    if let Ok((signature_bytes, recovery_id, message)) = extract_signature_parts(&headers) {
        // Recover the address - this just verifies the signature is valid
        log::debug!("ECDSA_AUTH: Recovering address from signature.");
        let address = recover_address(&signature_bytes, recovery_id, &message)?;
        request.extensions_mut().insert(Some(
            RecoveredAddress {
                address,
                message,
            }
        ));
        // Authentication successful - let the handler handle authorization
        return Ok(next.run(request).await);
    } else {
        log::warn!("ECDSA_AUTH: Invalid signature format.");
        return Err(SignatureError::MissingSignature)
    }
}

/// Middleware to authenticate requests from active (non-disabled) Formation nodes.
/// Expects an ECDSA signature in the Authorization header, similar to ecdsa_auth_middleware.
pub async fn active_node_auth_middleware(
    axum::extract::State(state): axum::extract::State<Arc<Mutex<crate::datastore::DataStore>>>, // Fully qualified State
    mut req: Request<axum::body::Body>, 
    next: axum::middleware::Next,
) -> Result<Response, StatusCode> {
    let headers = req.headers().clone();
    log::debug!("ACTIVE_NODE_AUTH: Checking for active node auth.");
    let (signature_bytes, recovery_id, message_to_verify) = 
        match extract_signature_parts(&headers) {
            Ok(parts) => parts,
            Err(SignatureError::MissingSignature) => {
                log::warn!("ACTIVE_NODE_AUTH: Missing signature.");
                return Err(StatusCode::UNAUTHORIZED);
            }
            Err(e) => {
                log::warn!("ACTIVE_NODE_AUTH: Invalid signature format: {:?}.", e);
                return Err(StatusCode::BAD_REQUEST);
            }
        };

    let recovered_eth_address = 
        match recover_address(&signature_bytes, recovery_id, &message_to_verify) {
            Ok(addr) => addr,
            Err(e) => {
                log::warn!("ACTIVE_NODE_AUTH: Could not recover address: {:?}.", e);
                return Err(StatusCode::UNAUTHORIZED);
            }
        };
    
    log::debug!("ACTIVE_NODE_AUTH: Recovered address: 0x{}", hex::encode(recovered_eth_address.as_slice()));
    let recovered_address_hex = hex::encode(recovered_eth_address.as_slice());
    log::debug!("ACTIVE_NODE_AUTH: Recovered sender address for gossip: 0x{}", recovered_address_hex);
        
    let datastore = state.lock().await;
    
    // Check if the recovered address corresponds to an active, non-disabled peer
    match datastore.network_state.peers.get(&recovered_address_hex).val { // CrdtPeer ID is hex address
        Some(peer_reg) => {
            log::debug!("ACTIVE_NODE_AUTH: Peer found in datastore.");
            if let Some(peer_val) = peer_reg.val() {
                let peer = peer_val.value(); // Get the CrdtPeer struct
                if !peer.is_disabled {
                    log::info!("ACTIVE_NODE_AUTH: Auth success for active peer: 0x{}", recovered_address_hex);
                    req.extensions_mut().insert(RecoveredAddress {
                        address: recovered_eth_address,
                        message: message_to_verify.to_vec(), // Pass along the verified message if needed by handler
                    });
                    Ok(next.run(req).await)
                } else {
                    log::warn!("ACTIVE_NODE_AUTH: Auth failed: Peer 0x{} is disabled.", recovered_address_hex);
                    Err(StatusCode::FORBIDDEN)
                }
            } else {
                log::warn!("ACTIVE_NODE_AUTH: Auth failed: Peer 0x{} in map, but value is None.", recovered_address_hex);
                Err(StatusCode::INTERNAL_SERVER_ERROR) 
            }
        }
        None => {
            log::warn!("ACTIVE_NODE_AUTH: Auth failed: Peer 0x{} not found.", recovered_address_hex);
            Err(StatusCode::UNAUTHORIZED)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use k256::ecdsa::{SigningKey, signature::Signer};
    use rand::rngs::OsRng;
    
    #[test]
    fn test_signature_recovery() {
        // Generate a random signing key
        let signing_key = SigningKey::random(&mut OsRng);
        let verifying_key = signing_key.verifying_key();
        
        // Create a message
        let message = b"Test message for signature";
        
        // Hash the message
        let mut hasher = Sha256::new();
        hasher.update(message);
        let message_hash = hasher.finalize();
        
        // Sign the message
        let (signature, recovery_id) = signing_key.sign_recoverable(message_hash.as_slice()).unwrap();
        
        // Verify recovery
        let result = recover_address(signature.to_bytes().as_slice(), recovery_id, message).unwrap();
        
        // Convert verifying key to Ethereum address for comparison
        let mut keccak = tiny_keccak::Keccak::v256();
        let mut hash = [0u8; 32];
        
        keccak.update(&verifying_key.to_encoded_point(false).as_bytes()[1..]);
        keccak.finalize(&mut hash);
        
        let expected_address = Address::from_slice(&hash[12..32]);
        
        assert_eq!(result, expected_address);
    }
} 
