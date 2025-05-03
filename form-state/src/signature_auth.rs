use axum::{
    async_trait,
    extract::{FromRequestParts, State},
    http::{request::Parts, Request, StatusCode},
    middleware::Next,
    response::Response,
    body::Body,
};
use std::sync::Arc;
use tokio::sync::Mutex;
use k256::ecdsa::VerifyingKey;
use form_auth::{
    signature::{SignatureData, verify_signature},
    extractor::{extract_from_headers, SignatureConfig},
};

use crate::datastore::DataStore;
use crate::accounts::Account;
use crate::api::is_localhost_request;

/// Structure containing validated signature and account
#[derive(Clone)]
pub struct SignatureAuth {
    /// The public key that was validated
    pub public_key: String,
    /// The account associated with this public key
    pub account: Account,
}

/// Simple middleware for signature-based authentication
pub async fn signature_auth_middleware(
    State(state): State<Arc<Mutex<DataStore>>>,
    mut request: Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    // Bypass auth for localhost requests
    if is_localhost_request(&request) {
        return Ok(next.run(request).await);
    }
    
    // Bypass auth for health check endpoints
    let path = request.uri().path();
    if path == "/health" || path == "/ping" {
        return Ok(next.run(request).await);
    }
    
    // Extract signature data
    let config = SignatureConfig::default();
    
    let sig_data = match extract_from_headers(request.headers(), &config) {
        Ok(Some(mut data)) => {
            // Use path as message for simple verification
            data.message = path.to_string();
            data
        },
        Ok(None) => return Err(StatusCode::UNAUTHORIZED),
        Err(_) => return Err(StatusCode::UNAUTHORIZED),
    };
    
    // Find account with matching public key
    let auth_data = {
        let datastore = state.lock().await;
        
        // For the initial implementation, we'll use a single trusted public key
        // In a real implementation, we'd look up keys from accounts
        let trusted_pubkey = std::env::var("TRUSTED_PUBLIC_KEY").ok();
        
        // Check against the trusted key
        if let Some(key_hex) = trusted_pubkey {
            if let Ok(key_bytes) = hex::decode(&key_hex) {
                if let Ok(verifying_key) = VerifyingKey::from_sec1_bytes(&key_bytes) {
                    // Verify signature
                    if let Ok(true) = verify_signature(&sig_data, &verifying_key) {
                        // For simplicity, just use the first account (replace with proper lookup)
                        if let Some(account) = datastore.account_state.list_accounts().first().cloned() {
                            return {
                                // Store auth data
                                request.extensions_mut().insert(SignatureAuth {
                                    public_key: key_hex,
                                    account,
                                });
                                
                                // Continue processing
                                Ok(next.run(request).await)
                            };
                        }
                    }
                }
            }
        }
        
        // Authentication failed
        Err(StatusCode::UNAUTHORIZED)
    };
    
    auth_data
}

/// Extractor for signature auth
#[async_trait]
impl<S> FromRequestParts<S> for SignatureAuth
where
    S: Send + Sync,
{
    type Rejection = StatusCode;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        // Extract SignatureAuth from request extensions
        parts.extensions
            .get::<SignatureAuth>()
            .cloned()
            .ok_or(StatusCode::UNAUTHORIZED)
    }
} 