//! # form-auth
//! 
//! ECDSA signature-based authentication for Formation services.
//! 
//! This crate provides functionality to authenticate API requests and messages
//! by verifying ECDSA signatures and recovering public keys.

pub mod error;
pub mod signature;
pub mod middleware;
pub mod extractor;

// Re-export the most commonly used types
pub use error::AuthError;
pub use signature::{verify_signature, recover_public_key, SignatureData};
pub use middleware::SignatureVerifyLayer;


