//! Authentication module for Form Pack service

pub mod claims;
pub mod config;
pub mod middleware;
pub mod jwt_client;
pub mod ecdsa;
pub mod authorization;

pub use claims::{JwtClaims, UserRole};
pub use config::AuthConfig;
pub use middleware::{
    jwt_auth_middleware, extract_auth_info, extract_token_from_header,
    AuthError, create_auth_error_response, has_resource_access,
    verify_project_access, verify_role
};
pub use jwt_client::JwtClient;
pub use ecdsa::{
    RecoveredAddress, OptionalRecoveredAddress, SignatureError,
    ecdsa_auth_middleware, extract_signature_parts, recover_address,
    create_auth_client
};
pub use authorization::{
    AuthorizationClient, AuthorizationError, extract_address_for_auth
}; 