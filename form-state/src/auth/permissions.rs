//! Role-based permission helpers for specific operations
//! 
//! This module provides helper functions that check if a user has the necessary
//! permissions to perform specific operations in the application based on their
//! role and project access.

use crate::auth::{
    DynamicClaims, UserRole, 
    middleware::{create_role_rejection, create_project_rejection, create_access_rejection, AuthError}
};
use axum::response::Response;

/// Represents different types of operations that can be performed in the application
pub enum Operation {
    // Model operations
    ViewModel,
    CreateModel,
    UpdateModel,
    DeleteModel,
    DeployModel,
    
    // Agent operations
    ViewAgent,
    CreateAgent,
    UpdateAgent,
    DeleteAgent,
    DeployAgent,
    
    // Billing operations
    ViewBilling,
    UpdateBillingInfo,
    ChangePlan,
    CancelSubscription,
    
    // Admin operations
    ManageUsers,
    ViewSystemStats,
    ConfigureSystem,
    
    // Project operations
    CreateProject,
    ManageProjectAccess,
    DeleteProject,
}

impl Operation {
    /// Get the minimum role required for this operation
    pub fn required_role(&self) -> UserRole {
        match self {
            // Model operations
            Operation::ViewModel => UserRole::User,
            Operation::CreateModel => UserRole::Developer,
            Operation::UpdateModel => UserRole::Developer,
            Operation::DeleteModel => UserRole::Developer,
            Operation::DeployModel => UserRole::Developer,
            
            // Agent operations
            Operation::ViewAgent => UserRole::User,
            Operation::CreateAgent => UserRole::Developer,
            Operation::UpdateAgent => UserRole::Developer,
            Operation::DeleteAgent => UserRole::Developer,
            Operation::DeployAgent => UserRole::Developer,
            
            // Billing operations
            Operation::ViewBilling => UserRole::Developer,
            Operation::UpdateBillingInfo => UserRole::Developer,
            Operation::ChangePlan => UserRole::Developer,
            Operation::CancelSubscription => UserRole::Developer,
            
            // Admin operations
            Operation::ManageUsers => UserRole::Admin,
            Operation::ViewSystemStats => UserRole::Admin,
            Operation::ConfigureSystem => UserRole::Admin,
            
            // Project operations
            Operation::CreateProject => UserRole::Developer,
            Operation::ManageProjectAccess => UserRole::Developer,
            Operation::DeleteProject => UserRole::Developer,
        }
    }
    
    /// Get a human-readable description of this operation
    pub fn description(&self) -> &'static str {
        match self {
            // Model operations
            Operation::ViewModel => "view AI models",
            Operation::CreateModel => "create AI models",
            Operation::UpdateModel => "update AI models",
            Operation::DeleteModel => "delete AI models",
            Operation::DeployModel => "deploy AI models",
            
            // Agent operations
            Operation::ViewAgent => "view AI agents",
            Operation::CreateAgent => "create AI agents",
            Operation::UpdateAgent => "update AI agents",
            Operation::DeleteAgent => "delete AI agents",
            Operation::DeployAgent => "deploy AI agents",
            
            // Billing operations
            Operation::ViewBilling => "view billing information",
            Operation::UpdateBillingInfo => "update billing information",
            Operation::ChangePlan => "change subscription plan",
            Operation::CancelSubscription => "cancel subscription",
            
            // Admin operations
            Operation::ManageUsers => "manage users",
            Operation::ViewSystemStats => "view system statistics",
            Operation::ConfigureSystem => "configure system settings",
            
            // Project operations
            Operation::CreateProject => "create new projects",
            Operation::ManageProjectAccess => "manage project access",
            Operation::DeleteProject => "delete projects",
        }
    }
    
    /// Returns true if this operation requires project context
    pub fn requires_project(&self) -> bool {
        match self {
            // These operations are global and don't require a specific project
            Operation::ManageUsers => false,
            Operation::ViewSystemStats => false,
            Operation::ConfigureSystem => false,
            Operation::CreateProject => false,
            
            // All other operations require a project context
            _ => true,
        }
    }
}

/// Check if a user has permission to perform a specific operation
/// 
/// This function checks if the user has the required role for the operation
/// and returns a Result with either () for success or a Response for rejection.
pub fn can_perform_operation(
    claims: &DynamicClaims,
    operation: Operation
) -> Result<(), Response> {
    let required_role = operation.required_role();
    
    if !claims.has_role(&required_role) {
        return Err(create_role_rejection(
            required_role,
            Some(claims.user_role())
        ));
    }
    
    Ok(())
}

/// Check if a user has permission to perform a specific operation on a project
/// 
/// This function checks if the user has the required role for the operation
/// and has access to the specified project.
pub fn can_perform_project_operation(
    claims: &DynamicClaims,
    operation: Operation,
    project_id: &str
) -> Result<(), Response> {
    // First check if user has access to the project
    if operation.requires_project() && !claims.is_for_project(project_id) {
        return Err(create_project_rejection(project_id, claims.project_id()));
    }
    
    // Then check if user has the required role
    let required_role = operation.required_role();
    if !claims.has_role(&required_role) {
        return Err(create_role_rejection(required_role, Some(claims.user_role())));
    }
    
    Ok(())
}

/// Check if a user can manage a model (create, update, delete)
pub fn can_manage_model(claims: &DynamicClaims, project_id: &str) -> Result<(), Response> {
    can_perform_project_operation(claims, Operation::UpdateModel, project_id)
}

/// Check if a user can view models in a project
pub fn can_view_models(claims: &DynamicClaims, project_id: &str) -> Result<(), Response> {
    can_perform_project_operation(claims, Operation::ViewModel, project_id)
}

/// Check if a user can deploy a model
pub fn can_deploy_model(claims: &DynamicClaims, project_id: &str) -> Result<(), Response> {
    can_perform_project_operation(claims, Operation::DeployModel, project_id)
}

/// Check if a user can manage an agent (create, update, delete)
pub fn can_manage_agent(claims: &DynamicClaims, project_id: &str) -> Result<(), Response> {
    can_perform_project_operation(claims, Operation::UpdateAgent, project_id)
}

/// Check if a user can view agents in a project
pub fn can_view_agents(claims: &DynamicClaims, project_id: &str) -> Result<(), Response> {
    can_perform_project_operation(claims, Operation::ViewAgent, project_id)
}

/// Check if a user can deploy an agent
pub fn can_deploy_agent(claims: &DynamicClaims, project_id: &str) -> Result<(), Response> {
    can_perform_project_operation(claims, Operation::DeployAgent, project_id)
}

/// Check if a user can manage billing information
pub fn can_manage_billing(claims: &DynamicClaims, project_id: &str) -> Result<(), Response> {
    can_perform_project_operation(claims, Operation::UpdateBillingInfo, project_id)
}

/// Check if a user can view billing information
pub fn can_view_billing(claims: &DynamicClaims, project_id: &str) -> Result<(), Response> {
    can_perform_project_operation(claims, Operation::ViewBilling, project_id)
}

/// Check if a user can modify subscription plans
pub fn can_modify_subscription(claims: &DynamicClaims, project_id: &str) -> Result<(), Response> {
    can_perform_project_operation(claims, Operation::ChangePlan, project_id)
}

/// Check if a user can manage other users (admin only)
pub fn can_manage_users(claims: &DynamicClaims) -> Result<(), Response> {
    can_perform_operation(claims, Operation::ManageUsers)
}

/// Check if a user can view system statistics (admin only)
pub fn can_view_system_stats(claims: &DynamicClaims) -> Result<(), Response> {
    can_perform_operation(claims, Operation::ViewSystemStats)
}

/// Check if a user can configure system settings (admin only)
pub fn can_configure_system(claims: &DynamicClaims) -> Result<(), Response> {
    can_perform_operation(claims, Operation::ConfigureSystem)
}

/// Check if a user can manage project access
pub fn can_manage_project_access(claims: &DynamicClaims, project_id: &str) -> Result<(), Response> {
    can_perform_project_operation(claims, Operation::ManageProjectAccess, project_id)
}

/// Check if a user can delete the project
pub fn can_delete_project(claims: &DynamicClaims, project_id: &str) -> Result<(), Response> {
    can_perform_project_operation(claims, Operation::DeleteProject, project_id)
}

/// Check if a user has ownership or admin rights to a resource
/// 
/// This is useful for operations where a user should be able to only modify
/// their own resources, but admins can modify any resource.
pub fn has_owner_or_admin_access(
    claims: &DynamicClaims,
    owner_id: &str,
    project_id: &str
) -> Result<(), Response> {
    // If user is admin, they can access anything
    if claims.is_admin() {
        return Ok(());
    }
    
    // If user belongs to the project but is not the owner
    if claims.is_for_project(project_id) && claims.sub != owner_id {
        return Err(create_access_rejection(
            project_id,
            UserRole::Developer,
            claims
        ));
    }
    
    // If user doesn't belong to the project
    if !claims.is_for_project(project_id) {
        return Err(create_project_rejection(
            project_id,
            claims.project_id()
        ));
    }
    
    Ok(())
}

/// Check if the user is allowed to access a resource with specific requirements
/// 
/// This provides a more flexible way to check permissions with custom requirements.
pub fn check_custom_access<F>(
    claims: &DynamicClaims,
    project_id: Option<&str>,
    required_role: UserRole,
    custom_check: F
) -> Result<(), Response> 
where 
    F: FnOnce() -> bool
{
    // Check project access if project_id is provided
    if let Some(pid) = project_id {
        if !claims.is_for_project(pid) {
            return Err(create_project_rejection(pid, claims.project_id()));
        }
    }
    
    // Check role requirements
    if !claims.has_role(&required_role) {
        return Err(create_role_rejection(required_role, Some(claims.user_role())));
    }
    
    // Run the custom check
    if !custom_check() {
        return Err(create_access_rejection(
            project_id.unwrap_or(""),
            required_role,
            claims
        ));
    }
    
    Ok(())
}

/// Trait for objects that have an owner ID
pub trait Owned {
    fn owner_id(&self) -> &str;
}

/// Trait for objects that belong to a project
pub trait ProjectScoped {
    fn project_id(&self) -> &str;
}

/// Check if a user can manage a specific resource that is both owned and project-scoped
/// 
/// This is useful for checking if a user can manage (update, delete) a specific resource.
pub fn can_manage_resource<T>(
    claims: &DynamicClaims,
    resource: &T
) -> Result<(), Response> 
where 
    T: Owned + ProjectScoped
{
    // First check if user has access to the project
    let project_id = resource.project_id();
    if !claims.is_for_project(project_id) {
        // Admins can access resources from any project
        if claims.is_admin() {
            return Ok(());
        }
        return Err(create_project_rejection(project_id, claims.project_id()));
    }
    
    // Then check if user is the owner or an admin/developer
    let owner_id = resource.owner_id();
    if claims.sub != owner_id && !claims.is_developer() {
        return Err(create_role_rejection(
            UserRole::Developer,
            Some(claims.user_role())
        ));
    }
    
    Ok(())
}

// Example usage:

/*
// Using in route handlers with JwtClaims extractor:
async fn create_model_handler(
    JwtClaims(claims): JwtClaims,
    Path(project_id): Path<String>,
    Json(model_data): Json<ModelData>
) -> Result<impl IntoResponse, Response> {
    // Check if the user can create models in this project
    permissions::can_perform_project_operation(&claims, Operation::CreateModel, &project_id)?;
    
    // Proceed with model creation
    // ...
    
    Ok(StatusCode::CREATED)
}

// Using in route handlers with ProjectRoleExtractor:
async fn update_model_handler(
    extractor: ProjectRoleExtractor,
    Path(model_id): Path<String>,
    State(model_state): State<Arc<ModelState>>,
    Json(model_data): Json<ModelData>
) -> Result<impl IntoResponse, Response> {
    // Get the existing model
    let model = model_state.get_model(&model_id)
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Model not found").into_response())?;
    
    // Check if the user can manage this specific model
    permissions::can_manage_resource(&extractor.claims, &model)?;
    
    // Proceed with model update
    // ...
    
    Ok(StatusCode::OK)
}

// Using with the Owned and ProjectScoped traits:
impl Owned for AIModel {
    fn owner_id(&self) -> &str {
        &self.owner_id
    }
}

impl ProjectScoped for AIModel {
    fn project_id(&self) -> &str {
        &self.project_id
    }
}

// Then in your handler:
async fn delete_model_handler(
    JwtClaims(claims): JwtClaims,
    Path((project_id, model_id)): Path<(String, String)>,
    State(model_state): State<Arc<ModelState>>
) -> Result<impl IntoResponse, Response> {
    // Get the existing model
    let model = model_state.get_model(&model_id)
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Model not found").into_response())?;
    
    // Check if model belongs to the specified project
    if model.project_id != project_id {
        return Err((StatusCode::NOT_FOUND, "Model not found in this project").into_response());
    }
    
    // Check if the user can delete this specific model
    permissions::can_manage_resource(&claims, &model)?;
    
    // Proceed with model deletion
    // ...
    
    Ok(StatusCode::NO_CONTENT)
}
*/ 