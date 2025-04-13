//! Authentication module for Form Network services

pub mod config;
pub mod jwks;
pub mod claims;
pub mod middleware;
pub mod permissions;

pub use config::AuthConfig;
pub use jwks::JWKSManager;
pub use claims::{DynamicClaims, UserRole};
pub use middleware::{
    jwt_auth_middleware, JwtClaims, AuthError,
    AdminClaims, DeveloperOrAdminClaims, DeveloperOnlyClaims, ProjectRoleExtractor,
    verify_project_access, verify_role, verify_project_and_role,
    get_wallet_address, get_user_email, is_token_valid,
    verify_project_path_access, create_auth_error_response,
    extract_token_from_header, has_resource_access, extract_user_info,
    create_role_rejection, create_project_rejection, create_access_rejection
};
pub use permissions::{
    Operation, Owned, ProjectScoped,
    can_perform_operation, can_perform_project_operation,
    can_manage_model, can_view_models, can_deploy_model,
    can_manage_agent, can_view_agents, can_deploy_agent,
    can_manage_billing, can_view_billing, can_modify_subscription,
    can_manage_users, can_view_system_stats, can_configure_system,
    can_manage_project_access, can_delete_project,
    has_owner_or_admin_access, check_custom_access, can_manage_resource
}; 