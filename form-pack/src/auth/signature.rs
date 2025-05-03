use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
    body::Body,
};
use std::{
    sync::Arc,
    collections::HashSet,
};
use log::{info, error, warn};
use form_auth::{
    signature::{SignatureData, verify_signature, recover_public_key},
    extractor::{extract_from_headers, SignatureConfig},
    AuthError,
};
use k256::ecdsa::VerifyingKey;

/// Configuration for signature-based authentication
#[derive(Clone, Debug)]
pub struct SignatureAuthConfig {
    /// List of authorized public keys (hex-encoded)
    pub authorized_pubkeys: HashSet<String>,
    
    /// Paths that should bypass authentication
    pub bypass_paths: HashSet<String>,
}

impl SignatureAuthConfig {
    /// Create a new configuration from environment variables
    pub fn from_env() -> Self {
        // Get authorized public keys from env
        let pubkeys_str = std::env::var("AUTH_PUBKEYS").unwrap_or_default();
        let authorized_pubkeys = pubkeys_str
            .split(',')
            .filter(|s| !s.is_empty())
            .map(|s| s.trim().to_string())
            .collect();

        // Get bypass paths from env (default to health and ping)
        let bypass_paths_str = std::env::var("AUTH_BYPASS_PATHS")
            .unwrap_or_else(|_| "/health,/ping".to_string());
        let bypass_paths = bypass_paths_str
            .split(',')
            .filter(|s| !s.is_empty())
            .map(|s| s.trim().to_string())
            .collect();

        Self {
            authorized_pubkeys,
            bypass_paths,
        }
    }

    /// Check if a path should bypass authentication
    pub fn should_bypass(&self, path: &str) -> bool {
        self.bypass_paths.iter().any(|p| path.starts_with(p))
    }
}

/// Authentication information extracted from a signature
#[derive(Clone, Debug)]
pub struct SignatureAuth {
    /// Hex-encoded public key
    pub public_key_hex: String,
}

/// Middleware for signature-based authentication
pub async fn signature_auth_middleware(
    State(config): State<Arc<SignatureAuthConfig>>,
    req: Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    let path = req.uri().path();
    
    // Allow bypass paths without authentication
    if config.should_bypass(path) {
        info!("Bypassing auth for path: {}", path);
        return Ok(next.run(req).await);
    }
    
    // Extract signature data
    let sig_config = SignatureConfig::default();
    let sig_data = match extract_from_headers(req.headers(), &sig_config) {
        Ok(Some(mut data)) => {
            // Use path as message for simple verification
            data.message = path.to_string();
            data
        },
        Ok(None) => {
            warn!("No signature data found in request");
            return Err(StatusCode::UNAUTHORIZED);
        },
        Err(e) => {
            error!("Error extracting signature data: {:?}", e);
            return Err(StatusCode::UNAUTHORIZED);
        },
    };
    
    // Verify signature and recover public key
    match verify_signature_and_pubkey(&sig_data, &config) {
        Ok(auth) => {
            info!("Successfully authenticated request from: {}", auth.public_key_hex);
            
            // Store auth info in request extensions
            let mut req = req;
            req.extensions_mut().insert(auth);
            
            Ok(next.run(req).await)
        },
        Err(e) => {
            error!("Authentication failed: {:?}", e);
            Err(StatusCode::UNAUTHORIZED)
        },
    }
}

/// Verify signature and check if the public key is authorized
fn verify_signature_and_pubkey(
    sig_data: &SignatureData,
    config: &SignatureAuthConfig,
) -> Result<SignatureAuth, AuthError> {
    // Recover public key from signature
    let pubkey = recover_public_key(sig_data)?;
    
    // Convert to hex for comparison
    let pubkey_hex = hex::encode(pubkey.to_sec1_bytes());
    
    // Check if this public key is authorized
    if !config.authorized_pubkeys.contains(&pubkey_hex) {
        return Err(AuthError::SignatureVerificationFailed);
    }
    
    // Verify the signature
    verify_signature(sig_data, &pubkey)?;
    
    Ok(SignatureAuth {
        public_key_hex: pubkey_hex,
    })
}

/// Extract signature auth from request extensions
pub fn extract_auth(req: &Request<Body>) -> Option<SignatureAuth> {
    req.extensions().get::<SignatureAuth>().cloned()
}

/// Require authorized signature and return error if not found
pub fn require_authorized(req: &Request<Body>) -> Result<SignatureAuth, StatusCode> {
    extract_auth(req).ok_or(StatusCode::UNAUTHORIZED)
}

/// Handle auth errors
pub fn create_auth_error_response(error: AuthError) -> impl IntoResponse {
    let status = match error {
        AuthError::MissingSignature => StatusCode::UNAUTHORIZED,
        AuthError::InvalidSignature => StatusCode::UNAUTHORIZED,
        AuthError::InvalidSignatureFormat => StatusCode::BAD_REQUEST,
        AuthError::InvalidHeader(_) => StatusCode::BAD_REQUEST,
        AuthError::MissingData(_) => StatusCode::BAD_REQUEST,
        AuthError::SignatureVerificationFailed => StatusCode::FORBIDDEN,
        AuthError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        _ => StatusCode::UNAUTHORIZED,
    };
    
    status
} 