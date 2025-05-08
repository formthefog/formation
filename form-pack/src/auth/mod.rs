//! Authentication module for Form Pack service

mod middleware;
mod authorization;
mod ecdsa;

pub use authorization::{AuthorizationClient, AuthorizationError, extract_address_for_auth};
pub use ecdsa::{RecoveredAddress, OptionalRecoveredAddress, SignatureError, 
                ecdsa_auth_middleware, extract_signature_parts, recover_address,
                create_auth_client};
pub use form_state::auth::UserRole;
pub mod config;

pub use config::AuthConfig; 