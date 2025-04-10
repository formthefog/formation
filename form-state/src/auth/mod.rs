//! Authentication module for Form Network services

pub mod config;
pub mod jwks;
pub mod claims;
pub mod middleware;

pub use config::AuthConfig;
pub use jwks::JWKSManager;
pub use claims::{DynamicClaims, UserRole};
pub use middleware::{
    jwt_auth_middleware, JwtClaims, AuthError,
    AdminClaims, DeveloperOrAdminClaims,
    verify_project_access, verify_role, verify_project_and_role,
    get_wallet_address, get_user_email, is_token_valid,
    verify_project_path_access, create_auth_error_response,
    extract_token_from_header, has_resource_access, extract_user_info
}; 