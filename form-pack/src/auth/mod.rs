//! Authentication module for Form Pack service
// Current signature-based authentication
pub mod signature;

// Re-export only signature auth
pub use signature::{
    SignatureAuthConfig, SignatureAuth, signature_auth_middleware,
    extract_auth, require_authorized
}; 