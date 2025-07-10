use alloy_primitives::Address;
use axum::{
    async_trait,
    extract::{ConnectInfo, FromRequestParts, Request},
    http::{request::Parts, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use hex;
use k256::ecdsa::{RecoveryId, Signature};
use log;
use serde::Serialize;
use serde_json::json;
use sha2::{Digest, Sha256};
use std::net::SocketAddr;
use std::sync::{Arc, RwLock};
use std::collections::HashMap;
use tokio::sync::Mutex;
use thiserror;
use lazy_static::lazy_static;
use tiny_keccak::Hasher;

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

/// A struct containing admin authentication context from a verified admin signature
#[derive(Debug, Clone)]
pub struct AdminRecoveredAddress {
    /// The admin address that signed the request
    pub address: Address,
    /// The original message that was signed
    pub message: Vec<u8>,
    /// Human-readable description of the admin (if configured)
    pub description: Option<String>,
    /// Timestamp when admin authentication occurred
    pub authenticated_at: std::time::SystemTime,
    /// Service that performed the authentication
    pub service_name: String,
}

impl AdminRecoveredAddress {
    /// Create new admin recovered address
    pub fn new(
        address: Address,
        message: Vec<u8>,
        description: Option<String>,
        service_name: String,
    ) -> Self {
        Self {
            address,
            message,
            description,
            authenticated_at: std::time::SystemTime::now(),
            service_name,
        }
    }

    /// Get the address as a hex string
    pub fn as_hex(&self) -> String {
        format!("0x{}", hex::encode(self.address.as_slice()))
    }

    /// Get description or fallback
    pub fn display_description(&self) -> &str {
        self.description.as_deref().unwrap_or("unknown admin")
    }

    /// Convert to a standard RecoveredAddress for backward compatibility
    pub fn to_recovered_address(&self) -> RecoveredAddress {
        RecoveredAddress {
            address: self.address,
            message: self.message.clone(),
        }
    }
}

/// Unified authentication context for request extensions
///
/// This enum represents the result of the authentication precedence hierarchy:
/// 1. Localhost access (highest precedence)
/// 2. Admin key authentication (second precedence)
/// 3. Normal user authentication (lowest precedence)
///
/// Sub-sub-task 3.5.1.1: Implement unified `AuthenticationContext` enum
#[derive(Debug, Clone)]
pub enum AuthenticationContext {
    /// Localhost request (no signature required)
    Localhost {
        timestamp: std::time::SystemTime,
    },

    /// Admin request with full context
    Admin {
        context: AdminRecoveredAddress,
        precedence_level: u8, // Always 2
    },

    /// Normal user request
    User {
        address: RecoveredAddress,
        precedence_level: u8, // Always 3
    },
}

impl AuthenticationContext {
    /// Get the precedence level for this authentication
    pub fn precedence_level(&self) -> u8 {
        match self {
            Self::Localhost { .. } => 1,
            Self::Admin { .. } => 2,
            Self::User { .. } => 3,
        }
    }

    /// Check if this context has admin privileges
    pub fn is_admin(&self) -> bool {
        matches!(self, Self::Localhost { .. } | Self::Admin { .. })
    }

    /// Get address if available
    pub fn address(&self) -> Option<Address> {
        match self {
            Self::Admin { context, .. } => Some(context.address),
            Self::User { address, .. } => Some(address.address),
            Self::Localhost { .. } => None,
        }
    }

    /// Get address as hex string if available
    pub fn address_hex(&self) -> Option<String> {
        self.address().map(|addr| format!("0x{}", hex::encode(addr.as_slice())))
    }

    /// Get a description of the authenticated entity for logging
    pub fn description(&self) -> String {
        match self {
            Self::Localhost { .. } => "localhost".to_string(),
            Self::Admin { context, .. } =>
                format!("admin:{}", context.display_description()),
            Self::User { address, .. } =>
                format!("user:0x{}", hex::encode(address.address.as_slice())),
        }
    }

    /// Convert to RecoveredAddress for backward compatibility
    /// Returns None for localhost requests
    pub fn as_recovered_address(&self) -> Option<&RecoveredAddress> {
        match self {
            Self::User { address, .. } => Some(address),
            Self::Admin { context, .. } => {
                // For admin context, we need to return a reference to a RecoveredAddress
                // This is a bit tricky since AdminRecoveredAddress contains the data
                // but doesn't have a RecoveredAddress field. We'll handle this in the
                // backward compatibility extractor instead.
                None
            }
            Self::Localhost { .. } => None,
        }
    }

    /// Create localhost authentication context
    pub fn localhost() -> Self {
        Self::Localhost {
            timestamp: std::time::SystemTime::now(),
        }
    }

    /// Create admin authentication context
    pub fn admin(admin_context: AdminRecoveredAddress) -> Self {
        Self::Admin {
            context: admin_context,
            precedence_level: 2,
        }
    }

    /// Create user authentication context
    pub fn user(recovered_address: RecoveredAddress) -> Self {
        Self::User {
            address: recovered_address,
            precedence_level: 3,
        }
    }
}

/// Extract a signature from the Authorization header
///
/// Expects the header to be in the format:
/// `Authorization: Signature <signature_hex>.<recovery_id>.<message_hex>`
pub fn extract_signature_parts(
    headers: &HeaderMap,
) -> Result<(Vec<u8>, RecoveryId, Vec<u8>), SignatureError> {
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
    let signature_bytes = hex::decode(parts[0]).map_err(|_| SignatureError::InvalidFormat)?;

    let recovery_id_byte = parts[1]
        .parse::<u8>()
        .map_err(|_| SignatureError::InvalidFormat)?;
    let recovery_id = match RecoveryId::from_byte(recovery_id_byte) {
        Some(id) => id,
        None => return Err(SignatureError::InvalidFormat),
    };

    let message = hex::decode(parts[2]).map_err(|_| SignatureError::InvalidFormat)?;

    Ok((signature_bytes, recovery_id, message))
}

/// Recover an address from a signature, recovery ID, and message
pub fn recover_address(
    signature_bytes: &[u8],
    recovery_id: RecoveryId,
    message: &[u8],
) -> Result<Address, SignatureError> {
    // Create a recoverable signature
    let signature =
        Signature::try_from(signature_bytes).map_err(|_| SignatureError::InvalidSignature)?;

    // Hash the message with SHA-256
    let mut hasher = Sha256::new();
    hasher.update(message);
    let message_hash = hasher.finalize();

    log::debug!(
        "Recovering address from signature. Message: {}",
        String::from_utf8_lossy(message)
    );
    log::debug!("Message hash: {}", hex::encode(message_hash));
    log::debug!("Signature: {}", hex::encode(signature_bytes));
    log::debug!("Recovery ID: {}", recovery_id.to_byte());

    // Recover the public key from the signature
    let recovery_result = k256::ecdsa::VerifyingKey::recover_from_msg(
        message_hash.as_slice(),
        &signature,
        recovery_id,
    )
    .map_err(|_| SignatureError::RecoveryFailed)?;

    // Take the last 20 bytes as the address
    let address = Address::from_public_key(&recovery_result);
    log::debug!("Recovered address: 0x{}", hex::encode(address.as_slice()));

    Ok(address)
}

/// Check if an address is configured as an admin key
///
/// This function verifies if the given address matches any of the configured admin addresses.
/// It uses environment variables to determine admin addresses, making it suitable for
/// deployment-time configuration without requiring persistent storage.
///
/// # Arguments
/// * `address` - The address to check for admin status
///
/// # Returns
/// * `true` if the address is configured as an admin key
/// * `false` if the address is not an admin or if admin authentication is disabled
///
/// # Environment Variables
/// * `FORMATION_ADMIN_KEY` - Single admin address (hex format with or without 0x prefix)
/// * `FORMATION_ADMIN_KEYS` - Multiple admin addresses (comma-separated)
/// * `FORMATION_ADMIN_ENABLED` - Whether admin authentication is enabled (default: true)
pub fn is_admin_key(address: &Address) -> bool {
    // Check if admin authentication is enabled
    let admin_enabled = std::env::var("FORMATION_ADMIN_ENABLED")
        .unwrap_or_else(|_| "true".to_string())
        .parse::<bool>()
        .unwrap_or(true);

    if !admin_enabled {
        log::debug!("Admin authentication disabled via FORMATION_ADMIN_ENABLED");
        return false;
    }

    // PRECEDENCE 1: Check runtime admin keys first (highest precedence)
    // Sub-sub-task 3.4.1.3: Add runtime admin key rotation capability
    if is_runtime_admin_key(address) {
        log::info!("Admin key verified: {} matches runtime admin key", format!("0x{}", hex::encode(address.as_slice())));
        return true;
    }

    // PRECEDENCE 2: Check environment variable admin keys (lower precedence)
    // Get admin addresses from environment variables
    let admin_addresses = if let Ok(single_key) = std::env::var("FORMATION_ADMIN_KEY") {
        vec![single_key]
    } else if let Ok(multiple_keys) = std::env::var("FORMATION_ADMIN_KEYS") {
        multiple_keys.split(',').map(|s| s.trim().to_string()).collect()
    } else {
        log::debug!("No environment admin keys configured (FORMATION_ADMIN_KEY or FORMATION_ADMIN_KEYS not set)");
        return false;
    };

    // Convert input address to hex string for comparison
    let address_hex = format!("0x{}", hex::encode(address.as_slice()));
    let address_hex_no_prefix = hex::encode(address.as_slice());

    // Check if any configured admin address matches
    for admin_addr_str in admin_addresses {
        let admin_addr_cleaned = admin_addr_str.trim();

        // Handle both with and without 0x prefix
        let normalized_admin = if admin_addr_cleaned.starts_with("0x") {
            admin_addr_cleaned.to_lowercase()
        } else {
            format!("0x{}", admin_addr_cleaned.to_lowercase())
        };

        if normalized_admin == address_hex.to_lowercase() ||
           admin_addr_cleaned.to_lowercase() == address_hex_no_prefix.to_lowercase() {
            log::info!("Admin key verified: {} matches environment admin address", address_hex);
            return true;
        }
    }

    log::debug!("Address {} is not configured as an admin key", address_hex);
    false
}

/// Verify admin signature and return AdminRecoveredAddress if valid
///
/// This function combines signature verification with admin status checking to create
/// a complete admin authentication flow. It extracts the signature from headers,
/// recovers the address, and checks if it's an admin key.
///
/// # Arguments
/// * `headers` - HTTP headers containing the signature
/// * `service_name` - Name of the service performing verification (for audit logging)
///
/// # Returns
/// * `Ok(AdminRecoveredAddress)` if signature is valid and from admin key
/// * `Err(SignatureError)` if signature is invalid or not from admin key
pub fn verify_admin_signature(headers: &HeaderMap, service_name: &str) -> Result<AdminRecoveredAddress, SignatureError> {
    // Extract signature parts from headers
    let (signature_bytes, recovery_id, message) = extract_signature_parts(headers)?;

    // Recover address from signature
    let recovered_address = recover_address(&signature_bytes, recovery_id, &message)?;

    // Check if recovered address is an admin key
    if !is_admin_key(&recovered_address) {
        log::debug!("Valid signature but not from admin key: 0x{}", hex::encode(recovered_address.as_slice()));
        return Err(SignatureError::RecoveryFailed); // Reuse existing error type for now
    }

    // Get admin description from environment if available
    let description = get_admin_description(&recovered_address);

    log::info!("Admin signature verified: 0x{} ({})",
               hex::encode(recovered_address.as_slice()),
               description.as_deref().unwrap_or("unknown admin"));

    Ok(AdminRecoveredAddress::new(
        recovered_address,
        message,
        description,
        service_name.to_string(),
    ))
}

/// Get description for an admin address from runtime or environment configuration
///
/// Sub-sub-task 3.4.1.3: Add runtime admin key rotation capability
///
/// This function checks runtime admin keys first, then falls back to environment variables.
fn get_admin_description(address: &Address) -> Option<String> {
    // PRECEDENCE 1: Check runtime admin keys first (highest precedence)
    if let Some(description) = get_runtime_admin_description(address) {
        return Some(description);
    }

    // PRECEDENCE 2: Check environment variable descriptions (lower precedence)
    // Get admin descriptions from environment variable
    let descriptions = std::env::var("FORMATION_ADMIN_KEY_DESCRIPTIONS").ok()?;
    let description_list: Vec<&str> = descriptions.split(',').map(|s| s.trim()).collect();

    // Get admin addresses to match with descriptions
    let admin_addresses = if let Ok(single_key) = std::env::var("FORMATION_ADMIN_KEY") {
        vec![single_key]
    } else if let Ok(multiple_keys) = std::env::var("FORMATION_ADMIN_KEYS") {
        multiple_keys.split(',').map(|s| s.trim().to_string()).collect()
    } else {
        return None;
    };

    let address_hex = format!("0x{}", hex::encode(address.as_slice()));

    // Find matching admin address and return corresponding description
    for (i, admin_addr_str) in admin_addresses.iter().enumerate() {
        let admin_addr_cleaned = admin_addr_str.trim();
        let normalized_admin = if admin_addr_cleaned.starts_with("0x") {
            admin_addr_cleaned.to_lowercase()
        } else {
            format!("0x{}", admin_addr_cleaned.to_lowercase())
        };

        if normalized_admin == address_hex.to_lowercase() {
            return description_list.get(i).map(|s| s.to_string());
        }
    }

    None
}

// Note: RecoveredAddress extractor moved below for backward compatibility with AuthenticationContext

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

/// Backward compatibility extractor for RecoveredAddress
///
/// This extractor maintains backward compatibility with existing handlers that expect
/// RecoveredAddress by extracting it from the unified AuthenticationContext.
///
/// Sub-sub-task 3.5.1.3: Create backward compatibility extractors for existing handlers
#[async_trait]
impl<S> FromRequestParts<S> for RecoveredAddress
where
    S: Send + Sync,
{
    type Rejection = SignatureError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        // First try to extract from unified AuthenticationContext
        if let Some(auth_context) = parts.extensions.get::<AuthenticationContext>() {
            match auth_context {
                AuthenticationContext::User { address, .. } => {
                    log::debug!("BACKWARD_COMPAT: Extracted RecoveredAddress from User context");
                    return Ok(address.clone());
                }
                AuthenticationContext::Admin { context, .. } => {
                    log::debug!("BACKWARD_COMPAT: Extracted RecoveredAddress from Admin context");
                    return Ok(context.to_recovered_address());
                }
                AuthenticationContext::Localhost { .. } => {
                    log::debug!("BACKWARD_COMPAT: Localhost request - no RecoveredAddress available");
                    return Err(SignatureError::MissingSignature);
                }
            }
        }

        // Fallback to legacy extraction for compatibility with old middleware
        if let Some(recovered) = parts.extensions.get::<RecoveredAddress>() {
            log::debug!("BACKWARD_COMPAT: Found legacy RecoveredAddress in extensions");
            return Ok(recovered.clone());
        }

        // If no context found, try direct signature extraction (legacy behavior)
        log::debug!("BACKWARD_COMPAT: No AuthenticationContext found, attempting direct signature extraction");
        let (signature_bytes, recovery_id, message) = extract_signature_parts(&parts.headers)?;
        let address = recover_address(&signature_bytes, recovery_id, &message)?;

        Ok(RecoveredAddress {
            address,
            message,
        })
    }
}

/// Extractor for unified AuthenticationContext
///
/// This extractor allows handlers to access the full authentication context
/// including precedence level and admin status information.
#[async_trait]
impl<S> FromRequestParts<S> for AuthenticationContext
where
    S: Send + Sync,
{
    type Rejection = SignatureError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        // Try to extract from request extensions
        if let Some(auth_context) = parts.extensions.get::<AuthenticationContext>() {
            log::debug!("AUTH_CONTEXT: Found unified AuthenticationContext in extensions");
            return Ok(auth_context.clone());
        }

        // Fallback: try to construct from legacy extensions
        if let Some(admin_context) = parts.extensions.get::<AdminRecoveredAddress>() {
            log::debug!("AUTH_CONTEXT: Constructing from legacy AdminRecoveredAddress");
            return Ok(AuthenticationContext::admin(admin_context.clone()));
        }

        if let Some(recovered) = parts.extensions.get::<RecoveredAddress>() {
            log::debug!("AUTH_CONTEXT: Constructing from legacy RecoveredAddress");
            return Ok(AuthenticationContext::user(recovered.clone()));
        }

        // No authentication context found
        log::debug!("AUTH_CONTEXT: No authentication context found");
        Err(SignatureError::MissingSignature)
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
        let remote_addr = connection_info
            .map(|c| c.0.to_string())
            .unwrap_or("".to_string());
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
        request
            .extensions_mut()
            .insert(Some(RecoveredAddress { address, message }));
        // Authentication successful - let the handler handle authorization
        return Ok(next.run(request).await);
    } else {
        log::warn!("ECDSA_AUTH: Invalid signature format.");
        return Err(SignatureError::MissingSignature);
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
    let (signature_bytes, recovery_id, message_to_verify) = match extract_signature_parts(&headers)
    {
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

    log::debug!(
        "ACTIVE_NODE_AUTH: Recovered address: 0x{}",
        hex::encode(recovered_eth_address.as_slice())
    );
    let recovered_address_hex = hex::encode(recovered_eth_address.as_slice());
    log::debug!(
        "ACTIVE_NODE_AUTH: Recovered sender address for gossip: 0x{}",
        recovered_address_hex
    );

    let datastore = state.lock().await;

    // Check if the recovered address corresponds to an active, non-disabled peer
    match datastore
        .network_state
        .peers
        .get(&recovered_address_hex)
        .val
    {
        // CrdtPeer ID is hex address
        Some(peer_reg) => {
            log::debug!("ACTIVE_NODE_AUTH: Peer found in datastore.");
            if let Some(peer_val) = peer_reg.val() {
                let peer = peer_val.value(); // Get the CrdtPeer struct
                if !peer.is_disabled {
                    log::info!(
                        "ACTIVE_NODE_AUTH: Auth success for active peer: 0x{}",
                        recovered_address_hex
                    );
                    req.extensions_mut().insert(RecoveredAddress {
                        address: recovered_eth_address,
                        message: message_to_verify.to_vec(), // Pass along the verified message if needed by handler
                    });
                    Ok(next.run(req).await)
                } else {
                    log::warn!(
                        "ACTIVE_NODE_AUTH: Auth failed: Peer 0x{} is disabled.",
                        recovered_address_hex
                    );
                    Err(StatusCode::FORBIDDEN)
                }
            } else {
                log::warn!(
                    "ACTIVE_NODE_AUTH: Auth failed: Peer 0x{} in map, but value is None.",
                    recovered_address_hex
                );
                Err(StatusCode::INTERNAL_SERVER_ERROR)
            }
        }
        None => {
            log::warn!(
                "ACTIVE_NODE_AUTH: Auth failed: Peer 0x{} not found.",
                recovered_address_hex
            );
            Err(StatusCode::UNAUTHORIZED)
        }
    }
}

/// Global admin authentication middleware with precedence rules
///
/// This middleware implements the authentication precedence hierarchy:
/// 1. Localhost access (highest precedence)
/// 2. Admin key authentication (second precedence)
/// 3. Normal user authentication (lowest precedence)
///
/// Sub-sub-task 3.5.1.2: Update `global_admin_auth_middleware` to use unified context
/// For sub-sub-task 3.2.2.1.1: Check localhost first (existing behavior)
/// For sub-sub-task 3.2.2.1.2: Extract and verify ECDSA signature
/// For sub-sub-task 3.2.2.1.3: Check if recovered address matches admin key
/// For sub-sub-task 3.2.2.1.4: Allow all requests if admin key verified
/// For sub-sub-task 3.2.2.1.5: Fall back to normal auth flow if not admin
pub async fn global_admin_auth_middleware(
    mut request: axum::extract::Request,
    next: axum::middleware::Next,
) -> Result<axum::response::Response, axum::http::StatusCode> {

    let endpoint = request.uri().path();
    let method = request.method();

    // PRECEDENCE LEVEL 1: Localhost Check (Highest Priority)
    // Based on design documents: localhost has highest precedence
    if crate::api::is_localhost_request(&request) {
        log::info!("GLOBAL_ADMIN_AUTH: Localhost access granted for {} {}", method, endpoint);

        // Store unified localhost context in request extensions for handlers
        let auth_context = AuthenticationContext::localhost();
        request.extensions_mut().insert(auth_context);

        log::debug!("GLOBAL_ADMIN_AUTH: Stored localhost AuthenticationContext");
        return Ok(next.run(request).await);
    }

    log::debug!("GLOBAL_ADMIN_AUTH: Not localhost, checking for ECDSA signature");

    // PRECEDENCE LEVEL 2: Admin Key Authentication (sub-sub-tasks 3.2.2.1.2, 3.2.2.1.3 & 3.2.2.1.4)
    let headers = request.headers().clone();

    match extract_signature_parts(&headers) {
        Ok((signature_bytes, recovery_id, message)) => {
            log::debug!("GLOBAL_ADMIN_AUTH: ECDSA signature found, attempting to recover address");

            match recover_address(&signature_bytes, recovery_id, &message) {
                Ok(recovered_address) => {
                    log::debug!("GLOBAL_ADMIN_AUTH: Successfully recovered address: 0x{}",
                               hex::encode(recovered_address.as_slice()));

                    // PRECEDENCE LEVEL 2: Check if recovered address is admin key (sub-sub-task 3.2.2.1.3)
                    if is_admin_key(&recovered_address) {
                        log::info!("GLOBAL_ADMIN_AUTH: Admin key verified: 0x{}",
                                  hex::encode(recovered_address.as_slice()));

                        // Get admin description for audit logging
                        let description = get_admin_description(&recovered_address);

                        // Create admin context for audit and authorization
                        let admin_context = AdminRecoveredAddress::new(
                            recovered_address,
                            message,
                            description,
                            "form-state".to_string(),
                        );

                        log::warn!("ADMIN ACCESS: {} accessing {} {} on form-state",
                                  admin_context.display_description(), method, endpoint);

                        // Store unified admin context in request extensions for handlers
                        let auth_context = AuthenticationContext::admin(admin_context);
                        request.extensions_mut().insert(auth_context);

                        // PRECEDENCE LEVEL 2: Allow all requests if admin key verified (sub-sub-task 3.2.2.1.4)
                        log::info!("GLOBAL_ADMIN_AUTH: Admin request allowed, proceeding to handler");
                        return Ok(next.run(request).await);
                    } else {
                        log::debug!("GLOBAL_ADMIN_AUTH: Valid signature but not from admin key: 0x{}",
                                   hex::encode(recovered_address.as_slice()));

                        // Not an admin key - create user context for normal authentication
                        // This will be used in PRECEDENCE LEVEL 3 (sub-sub-task 3.2.2.1.5)
                        let recovered_context = RecoveredAddress {
                            address: recovered_address,
                            message,
                        };

                        // Store unified user context in request extensions
                        let auth_context = AuthenticationContext::user(recovered_context);
                        request.extensions_mut().insert(auth_context);

                        log::debug!("GLOBAL_ADMIN_AUTH: Non-admin signature stored as user context");

                        // Valid non-admin signature - allow request to proceed with normal authorization
                        log::info!("GLOBAL_ADMIN_AUTH: Normal user request allowed, proceeding to handler");
                        return Ok(next.run(request).await);
                    }
                }
                Err(e) => {
                    log::debug!("GLOBAL_ADMIN_AUTH: Failed to recover address from signature: {:?}", e);
                    // Invalid signature - authentication failed
                }
            }
        }
        Err(SignatureError::MissingSignature) => {
            log::debug!("GLOBAL_ADMIN_AUTH: No ECDSA signature found in request");
            // No signature present - authentication failed
        }
        Err(e) => {
            log::debug!("GLOBAL_ADMIN_AUTH: Invalid signature format: {:?}", e);
            // Invalid signature format - authentication failed
        }
    }

    // PRECEDENCE LEVEL 3: Authentication Failed
    // No valid signature found - authentication failed
    log::info!("GLOBAL_ADMIN_AUTH: Authentication failed - no valid signature found for {} {}", method, endpoint);
    Err(axum::http::StatusCode::UNAUTHORIZED)
}

/// Authorization helper functions
///
/// Sub-sub-task 3.5.5.1: Create `check_authorization()` function with admin bypass
/// Sub-sub-task 3.5.5.2: Create admin context utilities for payload extraction
/// Sub-sub-task 3.5.5.3: Implement comprehensive authorization logging

/// Authorization error types
#[derive(Debug, thiserror::Error)]
pub enum AuthorizationError {
    #[error("Access denied: insufficient permissions")]
    InsufficientPermissions,

    #[error("Access denied: resource not found")]
    ResourceNotFound,

    #[error("Access denied: not resource owner")]
    NotResourceOwner,

    #[error("Access denied: authentication required")]
    AuthenticationRequired,

    #[error("Authorization check failed: {0}")]
    CheckFailed(String),
}

/// Resource types for authorization checking
#[derive(Debug, Clone, PartialEq)]
pub enum ResourceType {
    Account,
    Instance,
    Agent,
    Model,
    Node,
    Network,
    Task,
    Admin,
}

/// Operation types for authorization checking
#[derive(Debug, Clone, PartialEq)]
pub enum OperationType {
    Read,
    Create,
    Update,
    Delete,
    Execute,
    Manage,
}

/// Authorization context for logging and decision making
#[derive(Debug, Clone)]
pub struct AuthorizationContext {
    pub resource_type: ResourceType,
    pub resource_id: Option<String>,
    pub operation: OperationType,
    pub requested_by: String,
    pub access_level: String,
    pub timestamp: std::time::SystemTime,
}

impl AuthorizationContext {
    pub fn new(
        resource_type: ResourceType,
        resource_id: Option<String>,
        operation: OperationType,
        auth_context: &AuthenticationContext,
    ) -> Self {
        Self {
            resource_type,
            resource_id,
            operation,
            requested_by: auth_context.description(),
            access_level: match auth_context {
                AuthenticationContext::Localhost { .. } => "localhost".to_string(),
                AuthenticationContext::Admin { .. } => "admin".to_string(),
                AuthenticationContext::User { .. } => "user".to_string(),
            },
            timestamp: std::time::SystemTime::now(),
        }
    }
}

/// Sub-sub-task 3.5.5.1: Create `check_authorization()` function with admin bypass
///
/// Comprehensive authorization checker that respects authentication precedence
/// and implements admin bypass logic for all resource types.
pub fn check_authorization(
    auth_context: &AuthenticationContext,
    resource_type: ResourceType,
    resource_id: Option<&str>,
    operation: OperationType,
    resource_owner: Option<&str>,
) -> Result<AuthorizationContext, AuthorizationError> {

    let auth_ctx = AuthorizationContext::new(
        resource_type.clone(),
        resource_id.map(|s| s.to_string()),
        operation.clone(),
        auth_context,
    );

    // Admin bypass: Localhost and Admin contexts have full access
    if auth_context.is_admin() {
        log_authorization_decision(&auth_ctx, true, "Admin bypass granted");
        return Ok(auth_ctx);
    }

    // Normal user authorization checks
    match auth_context {
        AuthenticationContext::User { address, .. } => {
            let user_address = address.as_hex();

            // Check resource-specific authorization
            let authorized = match resource_type {
                ResourceType::Account => {
                    // Users can only access their own accounts
                    resource_id.map_or(false, |id| id.to_lowercase() == user_address.to_lowercase())
                }
                ResourceType::Instance | ResourceType::Agent | ResourceType::Model => {
                    // Users can only access resources they own
                    resource_owner.map_or(false, |owner| {
                        let normalized_owner = owner.strip_prefix("0x").unwrap_or(owner).to_lowercase();
                        let normalized_user = user_address.strip_prefix("0x").unwrap_or(&user_address).to_lowercase();
                        normalized_owner == normalized_user
                    })
                }
                ResourceType::Node | ResourceType::Network | ResourceType::Task => {
                    // These require admin privileges
                    false
                }
                ResourceType::Admin => {
                    // Admin operations require admin privileges
                    false
                }
            };

            if authorized {
                log_authorization_decision(&auth_ctx, true, "Resource ownership verified");
                Ok(auth_ctx)
            } else {
                log_authorization_decision(&auth_ctx, false, "Insufficient permissions or not resource owner");
                Err(AuthorizationError::InsufficientPermissions)
            }
        }
        _ => {
            // This shouldn't happen since middleware should handle authentication
            log_authorization_decision(&auth_ctx, false, "No valid authentication context");
            Err(AuthorizationError::AuthenticationRequired)
        }
    }
}

/// Sub-sub-task 3.5.5.2: Create admin context utilities for payload extraction
///
/// Utility functions for extracting and validating data from request payloads
/// with admin context awareness.

/// Extract owner ID from payload with admin context handling
pub fn extract_owner_from_payload(
    payload: &serde_json::Value,
    auth_context: &AuthenticationContext,
    default_field: &str,
) -> Result<String, AuthorizationError> {

    // Admin bypass: Admins can specify any owner
    if auth_context.is_admin() {
        if let Some(owner) = payload.get(default_field).and_then(|v| v.as_str()) {
            log::info!("PAYLOAD_EXTRACT: Admin {} specified owner: {}",
                      auth_context.description(), owner);
            return Ok(owner.to_string());
        } else {
            log::warn!("PAYLOAD_EXTRACT: Admin {} did not specify owner in field '{}'",
                      auth_context.description(), default_field);
            return Err(AuthorizationError::CheckFailed(
                format!("Admin must specify owner in field '{}'", default_field)
            ));
        }
    }

    // Normal users: force owner to be their own address
    match auth_context {
        AuthenticationContext::User { address, .. } => {
            let user_address = address.as_hex();
            log::info!("PAYLOAD_EXTRACT: User {} forced as owner (ignoring payload)", user_address);
            Ok(user_address)
        }
        _ => {
            log::error!("PAYLOAD_EXTRACT: Invalid authentication context for payload extraction");
            Err(AuthorizationError::AuthenticationRequired)
        }
    }
}

/// Validate resource ownership for updates with admin bypass
pub fn validate_resource_ownership(
    auth_context: &AuthenticationContext,
    resource_type: ResourceType,
    resource_id: &str,
    current_owner: &str,
    new_owner: Option<&str>,
) -> Result<(), AuthorizationError> {

    // Admin bypass: Admins can change ownership
    if auth_context.is_admin() {
        if let Some(new_owner) = new_owner {
            log::info!("OWNERSHIP_VALIDATE: Admin {} changing {} {} ownership from {} to {}",
                      auth_context.description(),
                      format!("{:?}", resource_type).to_lowercase(),
                      resource_id, current_owner, new_owner);
        }
        return Ok(());
    }

    // Normal users: verify current ownership and prevent ownership changes
    match auth_context {
        AuthenticationContext::User { address, .. } => {
            let user_address = address.as_hex();
            let normalized_user = user_address.strip_prefix("0x").unwrap_or(&user_address).to_lowercase();
            let normalized_current = current_owner.strip_prefix("0x").unwrap_or(current_owner).to_lowercase();

            // Check current ownership
            if normalized_user != normalized_current {
                log::warn!("OWNERSHIP_VALIDATE: User {} does not own {} {}",
                          user_address, format!("{:?}", resource_type).to_lowercase(), resource_id);
                return Err(AuthorizationError::NotResourceOwner);
            }

            // Check for ownership change attempts
            if let Some(new_owner) = new_owner {
                let normalized_new = new_owner.strip_prefix("0x").unwrap_or(new_owner).to_lowercase();
                if normalized_new != normalized_current {
                    log::warn!("OWNERSHIP_VALIDATE: User {} attempted to change {} {} ownership to {}",
                              user_address, format!("{:?}", resource_type).to_lowercase(),
                              resource_id, new_owner);
                    return Err(AuthorizationError::InsufficientPermissions);
                }
            }

            log::debug!("OWNERSHIP_VALIDATE: User {} ownership verified for {} {}",
                       user_address, format!("{:?}", resource_type).to_lowercase(), resource_id);
            Ok(())
        }
        _ => {
            log::error!("OWNERSHIP_VALIDATE: Invalid authentication context");
            Err(AuthorizationError::AuthenticationRequired)
        }
    }
}

/// Extract and validate resource ID from request path or payload
pub fn extract_resource_id(
    path_id: Option<&str>,
    payload: &serde_json::Value,
    id_field: &str,
) -> Result<String, AuthorizationError> {

    // Prefer path parameter over payload
    if let Some(id) = path_id {
        if !id.is_empty() {
            return Ok(id.to_string());
        }
    }

    // Fall back to payload field
    if let Some(id) = payload.get(id_field).and_then(|v| v.as_str()) {
        if !id.is_empty() {
            return Ok(id.to_string());
        }
    }

    Err(AuthorizationError::CheckFailed(
        format!("Resource ID not found in path or payload field '{}'", id_field)
    ))
}

/// Sub-sub-task 3.5.5.3: Implement comprehensive authorization logging
///
/// Comprehensive logging system for authorization decisions with structured
/// output for security auditing and monitoring.

/// Log authorization decision with full context
pub fn log_authorization_decision(
    auth_ctx: &AuthorizationContext,
    granted: bool,
    reason: &str,
) {
    let decision = if granted { "GRANTED" } else { "DENIED" };
    let resource_desc = if let Some(ref id) = auth_ctx.resource_id {
        format!("{:?}:{}", auth_ctx.resource_type, id)
    } else {
        format!("{:?}", auth_ctx.resource_type)
    };

    if granted {
        log::info!(
            target: "authorization",
            "AUTHORIZATION_{} resource={} operation={:?} requested_by={} access_level={} reason=\"{}\"",
            decision, resource_desc, auth_ctx.operation,
            auth_ctx.requested_by, auth_ctx.access_level, reason
        );
    } else {
        log::warn!(
            target: "authorization",
            "AUTHORIZATION_{} resource={} operation={:?} requested_by={} access_level={} reason=\"{}\"",
            decision, resource_desc, auth_ctx.operation,
            auth_ctx.requested_by, auth_ctx.access_level, reason
        );
    }
}

/// Log admin operation with enhanced context
pub fn log_admin_operation(
    auth_context: &AuthenticationContext,
    operation: &str,
    resource: &str,
    details: Option<&str>,
) {
    if let AuthenticationContext::Admin { context, .. } = auth_context {
        let detail_str = details.unwrap_or("none");
        log::warn!(
            target: "admin_operations",
            "ADMIN_OPERATION admin={} operation=\"{}\" resource=\"{}\" details=\"{}\" service=\"{}\"",
            context.display_description(), operation, resource, detail_str, context.service_name
        );
    } else if let AuthenticationContext::Localhost { .. } = auth_context {
        let detail_str = details.unwrap_or("none");
        log::info!(
            target: "admin_operations",
            "LOCALHOST_OPERATION operation=\"{}\" resource=\"{}\" details=\"{}\"",
            operation, resource, detail_str
        );
    }
}

/// Log security event for monitoring and alerting
pub fn log_security_event(
    event_type: &str,
    auth_context: &AuthenticationContext,
    resource: &str,
    severity: &str,
    details: &str,
) {
    log::warn!(
        target: "security_events",
        "SECURITY_EVENT type=\"{}\" severity=\"{}\" requested_by=\"{}\" access_level=\"{}\" resource=\"{}\" details=\"{}\"",
        event_type, severity, auth_context.description(),
        match auth_context {
            AuthenticationContext::Localhost { .. } => "localhost",
            AuthenticationContext::Admin { .. } => "admin",
            AuthenticationContext::User { .. } => "user",
        },
        resource, details
    );
}

/// Log authorization failure with enhanced context for security monitoring
pub fn log_authorization_failure(
    auth_context: &AuthenticationContext,
    resource_type: ResourceType,
    resource_id: Option<&str>,
    operation: OperationType,
    failure_reason: &str,
) {
    let resource_desc = if let Some(id) = resource_id {
        format!("{:?}:{}", resource_type, id)
    } else {
        format!("{:?}", resource_type)
    };

    log_security_event(
        "authorization_failure",
        auth_context,
        &resource_desc,
        "medium",
        &format!("Operation {:?} failed: {}", operation, failure_reason)
    );
}

/// Helper function to create authorization context for common operations
pub fn create_auth_context_for_operation(
    auth_context: &AuthenticationContext,
    resource_type: ResourceType,
    resource_id: Option<&str>,
    operation: OperationType,
) -> AuthorizationContext {
    AuthorizationContext::new(
        resource_type,
        resource_id.map(|s| s.to_string()),
        operation,
        auth_context,
    )
}

/// Runtime admin key storage for dynamic key rotation
///
/// Sub-sub-task 3.4.1.3: Add runtime admin key rotation capability
///
/// This provides thread-safe storage for admin keys that can be modified at runtime
/// without requiring service restart. Keys are stored in memory and take precedence
/// over environment variable configuration.
#[derive(Debug, Clone)]
pub struct RuntimeAdminKeys {
    /// Map of admin address (hex) to description
    pub keys: HashMap<String, Option<String>>,
    /// Timestamp when keys were last modified
    pub last_modified: std::time::SystemTime,
}

impl RuntimeAdminKeys {
    pub fn new() -> Self {
        Self {
            keys: HashMap::new(),
            last_modified: std::time::SystemTime::now(),
        }
    }

    /// Add an admin key with optional description
    pub fn add_key(&mut self, address: &str, description: Option<String>) -> Result<(), String> {
        // Normalize address format
        let normalized_address = if address.starts_with("0x") {
            address.to_lowercase()
        } else {
            format!("0x{}", address.to_lowercase())
        };

        // Validate address format (basic hex check)
        if normalized_address.len() != 42 || !normalized_address[2..].chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(format!("Invalid address format: {}", address));
        }

        self.keys.insert(normalized_address, description);
        self.last_modified = std::time::SystemTime::now();
        Ok(())
    }

    /// Remove an admin key
    pub fn remove_key(&mut self, address: &str) -> bool {
        let normalized_address = if address.starts_with("0x") {
            address.to_lowercase()
        } else {
            format!("0x{}", address.to_lowercase())
        };

        let removed = self.keys.remove(&normalized_address).is_some();
        if removed {
            self.last_modified = std::time::SystemTime::now();
        }
        removed
    }

    /// Check if an address is an admin key
    pub fn is_admin_key(&self, address: &str) -> bool {
        let normalized_address = if address.starts_with("0x") {
            address.to_lowercase()
        } else {
            format!("0x{}", address.to_lowercase())
        };

        self.keys.contains_key(&normalized_address)
    }

    /// Get description for an admin key
    pub fn get_description(&self, address: &str) -> Option<String> {
        let normalized_address = if address.starts_with("0x") {
            address.to_lowercase()
        } else {
            format!("0x{}", address.to_lowercase())
        };

        self.keys.get(&normalized_address).and_then(|desc| desc.clone())
    }

    /// List all admin keys with their descriptions
    pub fn list_keys(&self) -> Vec<(String, Option<String>)> {
        self.keys.iter().map(|(addr, desc)| (addr.clone(), desc.clone())).collect()
    }

    /// Clear all runtime admin keys
    pub fn clear(&mut self) {
        self.keys.clear();
        self.last_modified = std::time::SystemTime::now();
    }

    /// Get count of runtime admin keys
    pub fn count(&self) -> usize {
        self.keys.len()
    }
}

/// Global runtime admin key storage
///
/// This provides thread-safe access to runtime admin keys across the application.
/// Keys stored here take precedence over environment variable configuration.
lazy_static! {
    static ref RUNTIME_ADMIN_KEYS: Arc<RwLock<RuntimeAdminKeys>> = Arc::new(RwLock::new(RuntimeAdminKeys::new()));
}

/// Runtime admin key management functions
///
/// Sub-sub-task 3.4.1.3: Add runtime admin key rotation capability

/// Add an admin key at runtime
///
/// This function allows adding admin keys without restarting the service.
/// Runtime keys take precedence over environment variable configuration.
pub fn add_runtime_admin_key(address: &str, description: Option<String>) -> Result<(), String> {
    let mut runtime_keys = RUNTIME_ADMIN_KEYS.write()
        .map_err(|e| format!("Failed to acquire write lock: {}", e))?;

    let description_for_log = description.as_deref().unwrap_or("no description").to_string();
    runtime_keys.add_key(address, description)?;

    log::info!("ADMIN_KEY_ROTATION: Added runtime admin key: {} ({})",
               address, description_for_log);

    Ok(())
}

/// Remove an admin key at runtime
///
/// This function allows removing admin keys without restarting the service.
/// Note: This only removes runtime keys, not environment variable keys.
pub fn remove_runtime_admin_key(address: &str) -> Result<bool, String> {
    let mut runtime_keys = RUNTIME_ADMIN_KEYS.write()
        .map_err(|e| format!("Failed to acquire write lock: {}", e))?;

    let removed = runtime_keys.remove_key(address);

    if removed {
        log::info!("ADMIN_KEY_ROTATION: Removed runtime admin key: {}", address);
    } else {
        log::warn!("ADMIN_KEY_ROTATION: Attempted to remove non-existent runtime admin key: {}", address);
    }

    Ok(removed)
}

/// List all runtime admin keys
///
/// Returns a list of all runtime admin keys with their descriptions.
pub fn list_runtime_admin_keys() -> Result<Vec<(String, Option<String>)>, String> {
    let runtime_keys = RUNTIME_ADMIN_KEYS.read()
        .map_err(|e| format!("Failed to acquire read lock: {}", e))?;

    Ok(runtime_keys.list_keys())
}

/// Clear all runtime admin keys
///
/// This removes all runtime admin keys but does not affect environment variable keys.
pub fn clear_runtime_admin_keys() -> Result<(), String> {
    let mut runtime_keys = RUNTIME_ADMIN_KEYS.write()
        .map_err(|e| format!("Failed to acquire write lock: {}", e))?;

    let count = runtime_keys.count();
    runtime_keys.clear();

    log::info!("ADMIN_KEY_ROTATION: Cleared {} runtime admin keys", count);

    Ok(())
}

/// Get runtime admin key statistics
///
/// Returns information about runtime admin key storage.
pub fn get_runtime_admin_key_stats() -> Result<(usize, std::time::SystemTime), String> {
    let runtime_keys = RUNTIME_ADMIN_KEYS.read()
        .map_err(|e| format!("Failed to acquire read lock: {}", e))?;

    Ok((runtime_keys.count(), runtime_keys.last_modified))
}

/// Check if an address is a runtime admin key
///
/// Internal function used by the main is_admin_key function.
fn is_runtime_admin_key(address: &Address) -> bool {
    let runtime_keys = match RUNTIME_ADMIN_KEYS.read() {
        Ok(keys) => keys,
        Err(e) => {
            log::error!("Failed to acquire read lock for runtime admin keys: {}", e);
            return false;
        }
    };

    let address_hex = format!("0x{}", hex::encode(address.as_slice()));
    runtime_keys.is_admin_key(&address_hex)
}

/// Get description for a runtime admin key
///
/// Internal function used by the main get_admin_description function.
fn get_runtime_admin_description(address: &Address) -> Option<String> {
    let runtime_keys = match RUNTIME_ADMIN_KEYS.read() {
        Ok(keys) => keys,
        Err(e) => {
            log::error!("Failed to acquire read lock for runtime admin keys: {}", e);
            return None;
        }
    };

    let address_hex = format!("0x{}", hex::encode(address.as_slice()));
    runtime_keys.get_description(&address_hex)
}

#[cfg(test)]
mod tests {
    use super::*;
    use k256::ecdsa::{signature::Signer, SigningKey};
    use rand::rngs::OsRng;
    use tiny_keccak::Hasher;

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
        let (signature, recovery_id) = signing_key
            .sign_recoverable(message_hash.as_slice())
            .unwrap();

        // Verify recovery
        let result =
            recover_address(signature.to_bytes().as_slice(), recovery_id, message).unwrap();

        // Convert verifying key to Ethereum address for comparison
        let mut keccak = tiny_keccak::Keccak::v256();
        let mut hash = [0u8; 32];

        keccak.update(&verifying_key.to_encoded_point(false).as_bytes()[1..]);
        keccak.finalize(&mut hash);

        let expected_address = Address::from_slice(&hash[12..32]);

        assert_eq!(result, expected_address);
    }
}

