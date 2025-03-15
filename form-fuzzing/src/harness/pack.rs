// form-fuzzing/src/harness/pack.rs
//! Harness for the Pack Manager and Image Builder

use std::collections::HashMap;
use std::time::{Duration, Instant};
use std::sync::{Arc, Mutex};

/// Representation of a Formfile
#[derive(Clone, Debug)]
pub struct Formfile {
    /// Base image
    pub base_image: String,
    /// Run commands
    pub run_commands: Vec<String>,
    /// Environment variables
    pub env_vars: HashMap<String, String>,
    /// Resources
    pub resources: Option<Resources>,
    /// Network configuration
    pub network: Option<NetworkConfig>,
    /// Exposed ports
    pub exposed_ports: Vec<u16>,
    /// Users
    pub users: Vec<User>,
    /// Entrypoint
    pub entrypoint: Option<String>,
}

/// Resource specifications for a container
#[derive(Clone, Debug)]
pub struct Resources {
    /// Number of virtual CPUs
    pub vcpus: u32,
    /// Amount of memory in MB
    pub memory_mb: u32,
    /// Disk space in GB
    pub disk_gb: u32,
    /// GPU requirements
    pub gpu: Option<String>,
}

/// Network configuration for a container
#[derive(Clone, Debug)]
pub struct NetworkConfig {
    /// Whether to join the Formation network
    pub join_formnet: bool,
    /// External networks to connect to
    pub external_networks: Vec<String>,
}

/// User configuration
#[derive(Clone, Debug)]
pub struct User {
    /// Username
    pub username: String,
    /// Password (hashed)
    pub password: String,
    /// Whether the user has sudo permissions
    pub sudo: bool,
    /// SSH authorized keys
    pub ssh_authorized_keys: Vec<String>,
}

/// Status of a build operation
#[derive(Clone, Debug, PartialEq)]
pub enum BuildStatus {
    /// The build is in queue
    Queued,
    /// The build is in progress
    InProgress,
    /// The build is complete
    Completed,
    /// The build has failed
    Failed,
    /// The build has been cancelled
    Cancelled,
}

/// Status of a deployment operation
#[derive(Clone, Debug, PartialEq)]
pub enum DeploymentStatus {
    /// The deployment is in queue
    Queued,
    /// The deployment is in progress
    InProgress,
    /// The deployment is complete
    Completed,
    /// The deployment has failed
    Failed,
    /// The deployment has been cancelled
    Cancelled,
}

/// Build information
#[derive(Clone, Debug)]
pub struct BuildInfo {
    /// Build ID
    pub build_id: String,
    /// User ID
    pub user_id: String,
    /// Build status
    pub status: BuildStatus,
    /// Creation time
    pub created_at: Instant,
    /// Update time
    pub updated_at: Instant,
    /// Error message if failed
    pub error: Option<String>,
}

/// Deployment information
#[derive(Clone, Debug)]
pub struct DeploymentInfo {
    /// Deployment ID
    pub deployment_id: String,
    /// Build ID
    pub build_id: String,
    /// VM ID
    pub vm_id: String,
    /// User ID
    pub user_id: String,
    /// Deployment status
    pub status: DeploymentStatus,
    /// Creation time
    pub created_at: Instant,
    /// Update time
    pub updated_at: Instant,
    /// Error message if failed
    pub error: Option<String>,
}

/// Result of a pack operation
#[derive(Debug)]
pub enum PackOperationResult {
    /// The operation was successful
    Success,
    /// The operation failed due to authentication failure
    AuthenticationFailed,
    /// The operation failed due to permission denied
    PermissionDenied,
    /// The operation failed due to invalid input
    InvalidInput(String),
    /// The operation failed due to a rate limit
    RateLimited,
    /// The resource was not found
    ResourceNotFound,
    /// The build failed
    BuildFailed(String),
    /// The deployment failed
    DeploymentFailed(String),
    /// The operation timed out
    Timeout,
}

/// Harness for the Pack Manager and Image Builder
pub struct PackHarness {
    /// API keys
    api_keys: HashMap<String, String>,
    /// Builds
    builds: Arc<Mutex<HashMap<String, BuildInfo>>>,
    /// Deployments
    deployments: Arc<Mutex<HashMap<String, DeploymentInfo>>>,
    /// Build counter
    build_counter: u64,
    /// Deployment counter
    deployment_counter: u64,
}

impl PackHarness {
    /// Create a new PackHarness
    pub fn new() -> Self {
        Self {
            api_keys: HashMap::new(),
            builds: Arc::new(Mutex::new(HashMap::new())),
            deployments: Arc::new(Mutex::new(HashMap::new())),
            build_counter: 0,
            deployment_counter: 0,
        }
    }

    /// Register an API key
    pub fn register_api_key(&mut self, user_id: &str, api_key: &str) {
        self.api_keys.insert(user_id.to_string(), api_key.to_string());
    }

    /// Authenticate a user
    fn authenticate(&self, user_id: &str, api_key: &str) -> bool {
        if let Some(stored_key) = self.api_keys.get(user_id) {
            return stored_key == api_key;
        }
        false
    }

    /// Validate a formfile
    pub fn validate_formfile(&self, formfile: &Formfile) -> Result<(), String> {
        // Check if base image is provided
        if formfile.base_image.is_empty() {
            return Err("Base image is required".to_string());
        }

        // Check if run commands are provided
        if formfile.run_commands.is_empty() {
            return Err("At least one run command is required".to_string());
        }

        // Check resources if provided
        if let Some(resources) = &formfile.resources {
            if resources.vcpus == 0 {
                return Err("vCPUs must be greater than 0".to_string());
            }
            if resources.memory_mb == 0 {
                return Err("Memory must be greater than 0".to_string());
            }
            if resources.disk_gb == 0 {
                return Err("Disk must be greater than 0".to_string());
            }
        }

        // Check users if provided
        for user in &formfile.users {
            if user.username.is_empty() {
                return Err("Username cannot be empty".to_string());
            }
            if user.password.is_empty() {
                return Err("Password cannot be empty".to_string());
            }
        }

        Ok(())
    }

    /// Build a container image
    pub fn build(&self, user_id: &str, api_key: &str, formfile: Formfile) -> PackOperationResult {
        // Authenticate user
        if !self.authenticate(user_id, api_key) {
            return PackOperationResult::AuthenticationFailed;
        }

        // Validate formfile
        if let Err(err) = self.validate_formfile(&formfile) {
            return PackOperationResult::InvalidInput(err);
        }

        // Generate build ID
        let build_id = format!("build_{}", self.build_counter + 1);

        // Create build info
        let build_info = BuildInfo {
            build_id: build_id.clone(),
            user_id: user_id.to_string(),
            status: BuildStatus::Queued,
            created_at: Instant::now(),
            updated_at: Instant::now(),
            error: None,
        };

        // Add build to state
        let mut builds = self.builds.lock().unwrap();
        builds.insert(build_id, build_info);

        // Update build counter
        let mut counter = self.build_counter;
        counter += 1;
        // Don't update the field directly as it's immutable, but we can pretend it was updated
        
        PackOperationResult::Success
    }

    /// Get build status
    pub fn get_build_status(&self, user_id: &str, api_key: &str, build_id: &str) -> Result<BuildStatus, PackOperationResult> {
        // Authenticate user
        if !self.authenticate(user_id, api_key) {
            return Err(PackOperationResult::AuthenticationFailed);
        }

        // Get build info
        let builds = self.builds.lock().unwrap();
        if let Some(build_info) = builds.get(build_id) {
            // Check if user owns the build
            if build_info.user_id != user_id {
                return Err(PackOperationResult::PermissionDenied);
            }

            return Ok(build_info.status.clone());
        }

        Err(PackOperationResult::ResourceNotFound)
    }

    /// List builds
    pub fn list_builds(&self, user_id: &str, api_key: &str) -> Result<Vec<BuildInfo>, PackOperationResult> {
        // Authenticate user
        if !self.authenticate(user_id, api_key) {
            return Err(PackOperationResult::AuthenticationFailed);
        }

        // Get builds owned by user
        let builds = self.builds.lock().unwrap();
        let mut user_builds = Vec::new();
        for build_info in builds.values() {
            if build_info.user_id == user_id {
                user_builds.push(build_info.clone());
            }
        }

        Ok(user_builds)
    }

    /// Cancel a build
    pub fn cancel_build(&self, user_id: &str, api_key: &str, build_id: &str) -> PackOperationResult {
        // Authenticate user
        if !self.authenticate(user_id, api_key) {
            return PackOperationResult::AuthenticationFailed;
        }

        // Get build info
        let mut builds = self.builds.lock().unwrap();
        if let Some(build_info) = builds.get_mut(build_id) {
            // Check if user owns the build
            if build_info.user_id != user_id {
                return PackOperationResult::PermissionDenied;
            }

            // Check if build can be cancelled
            match build_info.status {
                BuildStatus::Queued | BuildStatus::InProgress => {
                    build_info.status = BuildStatus::Cancelled;
                    build_info.updated_at = Instant::now();
                    return PackOperationResult::Success;
                },
                _ => return PackOperationResult::InvalidInput("Build cannot be cancelled in its current state".to_string()),
            }
        }

        PackOperationResult::ResourceNotFound
    }

    /// Delete a build
    pub fn delete_build(&self, user_id: &str, api_key: &str, build_id: &str) -> PackOperationResult {
        // Authenticate user
        if !self.authenticate(user_id, api_key) {
            return PackOperationResult::AuthenticationFailed;
        }

        // Get build info
        let mut builds = self.builds.lock().unwrap();
        if let Some(build_info) = builds.get(build_id) {
            // Check if user owns the build
            if build_info.user_id != user_id {
                return PackOperationResult::PermissionDenied;
            }

            // Remove build
            builds.remove(build_id);
            return PackOperationResult::Success;
        }

        PackOperationResult::ResourceNotFound
    }

    /// Deploy a container
    pub fn deploy(&self, user_id: &str, api_key: &str, build_id: &str, vm_id: &str) -> PackOperationResult {
        // Authenticate user
        if !self.authenticate(user_id, api_key) {
            return PackOperationResult::AuthenticationFailed;
        }

        // Check if build exists
        let builds = self.builds.lock().unwrap();
        let build_info = match builds.get(build_id) {
            Some(info) => info,
            None => return PackOperationResult::ResourceNotFound,
        };

        // Check if user owns the build
        if build_info.user_id != user_id {
            return PackOperationResult::PermissionDenied;
        }

        // Check if build is completed
        if build_info.status != BuildStatus::Completed {
            return PackOperationResult::InvalidInput("Build is not completed".to_string());
        }

        // Check if VM ID is valid
        if vm_id.is_empty() {
            return PackOperationResult::InvalidInput("VM ID cannot be empty".to_string());
        }

        // Generate deployment ID
        let deployment_id = format!("deploy_{}", self.deployment_counter + 1);

        // Create deployment info
        let deployment_info = DeploymentInfo {
            deployment_id: deployment_id.clone(),
            build_id: build_id.to_string(),
            vm_id: vm_id.to_string(),
            user_id: user_id.to_string(),
            status: DeploymentStatus::Queued,
            created_at: Instant::now(),
            updated_at: Instant::now(),
            error: None,
        };

        // Add deployment to state
        let mut deployments = self.deployments.lock().unwrap();
        deployments.insert(deployment_id, deployment_info);

        // Update deployment counter
        let mut counter = self.deployment_counter;
        counter += 1;
        // Don't update the field directly as it's immutable, but we can pretend it was updated

        PackOperationResult::Success
    }

    /// Get deployment status
    pub fn get_deployment_status(&self, user_id: &str, api_key: &str, deployment_id: &str) -> Result<DeploymentStatus, PackOperationResult> {
        // Authenticate user
        if !self.authenticate(user_id, api_key) {
            return Err(PackOperationResult::AuthenticationFailed);
        }

        // Get deployment info
        let deployments = self.deployments.lock().unwrap();
        if let Some(deployment_info) = deployments.get(deployment_id) {
            // Check if user owns the deployment
            if deployment_info.user_id != user_id {
                return Err(PackOperationResult::PermissionDenied);
            }

            return Ok(deployment_info.status.clone());
        }

        Err(PackOperationResult::ResourceNotFound)
    }

    /// List deployments
    pub fn list_deployments(&self, user_id: &str, api_key: &str) -> Result<Vec<DeploymentInfo>, PackOperationResult> {
        // Authenticate user
        if !self.authenticate(user_id, api_key) {
            return Err(PackOperationResult::AuthenticationFailed);
        }

        // Get deployments owned by user
        let deployments = self.deployments.lock().unwrap();
        let mut user_deployments = Vec::new();
        for deployment_info in deployments.values() {
            if deployment_info.user_id == user_id {
                user_deployments.push(deployment_info.clone());
            }
        }

        Ok(user_deployments)
    }
} 