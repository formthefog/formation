// form-fuzzing/src/harness/mcp.rs

//! Test harness for MCP Server API testing

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

/// Result of an MCP operation
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MCPOperationResult {
    /// Operation succeeded
    Success(serde_json::Value),
    /// Authentication failed
    AuthenticationFailed,
    /// Permission denied
    PermissionDenied,
    /// Resource not found
    ResourceNotFound,
    /// Invalid input
    InvalidInput(String),
    /// Operation failed
    OperationFailed(String),
    /// Rate limited
    RateLimited,
    /// Internal error
    InternalError(String),
    /// Timeout
    Timeout,
}

/// VM instance status
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum VMStatus {
    /// VM is being created
    Creating,
    /// VM is running
    Running,
    /// VM is stopped
    Stopped,
    /// VM is deleted
    Deleted,
    /// VM creation failed
    Failed(String),
}

/// VM instance representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VMInstance {
    /// VM ID
    pub vm_id: String,
    /// VM name
    pub name: String,
    /// Owner ID
    pub owner_id: String,
    /// Number of vCPUs
    pub vcpus: u8,
    /// Memory size in MB
    pub memory_mb: u64,
    /// Disk size in GB
    pub disk_gb: u64,
    /// Status
    pub status: VMStatus,
    /// Creation time
    pub created_at: u64,
    /// IP address
    pub ip_address: Option<String>,
}

/// Build status
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum BuildStatus {
    /// Build is being created
    Creating,
    /// Build is running
    Running,
    /// Build is completed
    Completed,
    /// Build failed
    Failed(String),
}

/// Workload build
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkloadBuild {
    /// Build ID
    pub build_id: String,
    /// Owner ID
    pub owner_id: String,
    /// Formfile content
    pub formfile: String,
    /// Status
    pub status: BuildStatus,
    /// Creation time
    pub created_at: u64,
    /// Completion time
    pub completed_at: Option<u64>,
}

/// Workload deployment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkloadDeployment {
    /// Deployment ID
    pub deployment_id: String,
    /// Build ID
    pub build_id: String,
    /// Owner ID
    pub owner_id: String,
    /// VM ID
    pub vm_id: String,
    /// Status
    pub status: String,
    /// Creation time
    pub created_at: u64,
    /// Start time
    pub started_at: Option<u64>,
}

/// Tool parameter schema
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolParameter {
    /// Parameter name
    pub name: String,
    /// Parameter description
    pub description: String,
    /// Whether parameter is required
    pub required: bool,
    /// Parameter type
    pub parameter_type: String,
    /// Default value
    pub default: Option<serde_json::Value>,
    /// Enum values
    pub enum_values: Option<Vec<serde_json::Value>>,
}

/// Tool definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    /// Tool name
    pub name: String,
    /// Tool description
    pub description: String,
    /// Tool version
    pub version: String,
    /// Tool parameters
    pub parameters: Vec<ToolParameter>,
    /// Tool categories
    pub categories: Vec<String>,
}

/// Long-running operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Operation {
    /// Operation ID
    pub operation_id: String,
    /// Tool name
    pub tool_name: String,
    /// Status
    pub status: String,
    /// Creation time
    pub created_at: u64,
    /// Completion time
    pub completed_at: Option<u64>,
    /// Result
    pub result: Option<serde_json::Value>,
    /// Error
    pub error: Option<String>,
}

/// Mock MCP server state
pub struct MockMCPServer {
    /// JWT tokens
    tokens: Arc<Mutex<HashMap<String, String>>>,
    /// VMs
    vms: Arc<Mutex<HashMap<String, VMInstance>>>,
    /// Builds
    builds: Arc<Mutex<HashMap<String, WorkloadBuild>>>,
    /// Deployments
    deployments: Arc<Mutex<HashMap<String, WorkloadDeployment>>>,
    /// Operations
    operations: Arc<Mutex<HashMap<String, Operation>>>,
    /// Tools
    tools: Arc<Mutex<Vec<ToolDefinition>>>,
    /// User permissions
    permissions: Arc<Mutex<HashMap<String, Vec<String>>>>,
    /// Rate limiting counters
    rate_limits: Arc<Mutex<HashMap<String, usize>>>,
    /// Simulated operation latency
    operation_latency: Duration,
    /// Failure rate for simulating random failures
    failure_rate: f64,
}

impl MockMCPServer {
    /// Create a new mock MCP server
    pub fn new() -> Self {
        // Create default tools
        let mut tools = Vec::new();
        
        // VM Create Tool
        tools.push(ToolDefinition {
            name: "vm.create".to_string(),
            description: "Create a new virtual machine".to_string(),
            version: "1.0".to_string(),
            parameters: vec![
                ToolParameter {
                    name: "name".to_string(),
                    description: "Name of the VM".to_string(),
                    required: true,
                    parameter_type: "string".to_string(),
                    default: None,
                    enum_values: None,
                },
                ToolParameter {
                    name: "vcpus".to_string(),
                    description: "Number of vCPUs".to_string(),
                    required: false,
                    parameter_type: "number".to_string(),
                    default: Some(serde_json::json!(1)),
                    enum_values: None,
                },
                ToolParameter {
                    name: "memory_mb".to_string(),
                    description: "Memory size in MB".to_string(),
                    required: false,
                    parameter_type: "number".to_string(),
                    default: Some(serde_json::json!(1024)),
                    enum_values: None,
                },
                ToolParameter {
                    name: "disk_gb".to_string(),
                    description: "Root disk size in GB".to_string(),
                    required: false,
                    parameter_type: "number".to_string(),
                    default: Some(serde_json::json!(10)),
                    enum_values: None,
                },
            ],
            categories: vec!["vm".to_string()],
        });
        
        // VM List Tool
        tools.push(ToolDefinition {
            name: "vm.list".to_string(),
            description: "List virtual machines".to_string(),
            version: "1.0".to_string(),
            parameters: vec![
                ToolParameter {
                    name: "status".to_string(),
                    description: "Filter by VM status".to_string(),
                    required: false,
                    parameter_type: "string".to_string(),
                    default: None,
                    enum_values: Some(vec![
                        serde_json::json!("creating"),
                        serde_json::json!("running"),
                        serde_json::json!("stopped"),
                    ]),
                },
            ],
            categories: vec!["vm".to_string()],
        });
        
        // Pack Build Tool
        tools.push(ToolDefinition {
            name: "form_pack_build".to_string(),
            description: "Build a workload from a Formfile".to_string(),
            version: "1.0".to_string(),
            parameters: vec![
                ToolParameter {
                    name: "formfile_content".to_string(),
                    description: "Content of the Formfile".to_string(),
                    required: true,
                    parameter_type: "string".to_string(),
                    default: None,
                    enum_values: None,
                },
            ],
            categories: vec!["pack".to_string()],
        });
        
        // Pack Ship Tool
        tools.push(ToolDefinition {
            name: "form_pack_ship".to_string(),
            description: "Deploy a built workload to a VM".to_string(),
            version: "1.0".to_string(),
            parameters: vec![
                ToolParameter {
                    name: "build_id".to_string(),
                    description: "ID of the build to deploy".to_string(),
                    required: true,
                    parameter_type: "string".to_string(),
                    default: None,
                    enum_values: None,
                },
                ToolParameter {
                    name: "instance_id".to_string(),
                    description: "ID of the VM to deploy to".to_string(),
                    required: true,
                    parameter_type: "string".to_string(),
                    default: None,
                    enum_values: None,
                },
            ],
            categories: vec!["pack".to_string()],
        });
        
        Self {
            tokens: Arc::new(Mutex::new(HashMap::new())),
            vms: Arc::new(Mutex::new(HashMap::new())),
            builds: Arc::new(Mutex::new(HashMap::new())),
            deployments: Arc::new(Mutex::new(HashMap::new())),
            operations: Arc::new(Mutex::new(HashMap::new())),
            tools: Arc::new(Mutex::new(tools)),
            permissions: Arc::new(Mutex::new(HashMap::new())),
            rate_limits: Arc::new(Mutex::new(HashMap::new())),
            operation_latency: Duration::from_millis(50),
            failure_rate: 0.05,
        }
    }
    
    /// Set the simulated operation latency
    pub fn set_operation_latency(&mut self, latency: Duration) {
        self.operation_latency = latency;
    }
    
    /// Set the failure rate for simulating random failures
    pub fn set_failure_rate(&mut self, rate: f64) {
        self.failure_rate = rate;
    }
    
    /// Check if a JWT token is valid
    fn validate_token(&self, token: &str) -> Option<String> {
        let tokens = self.tokens.lock().unwrap();
        tokens.get(token).cloned()
    }
    
    /// Check rate limits for a user
    fn check_rate_limit(&self, user_id: &str, max_ops: usize) -> bool {
        let mut rate_limits = self.rate_limits.lock().unwrap();
        let count = rate_limits.entry(user_id.to_string()).or_insert(0);
        *count += 1;
        *count <= max_ops
    }
    
    /// Generate a random ID
    fn generate_id(&self, prefix: &str) -> String {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let random = rand::random::<u32>();
        format!("{}-{}-{}", prefix, now, random)
    }
    
    /// Create a new operation
    fn create_operation(&self, tool_name: &str, user_id: &str) -> String {
        let operation_id = self.generate_id("op");
        
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        
        let operation = Operation {
            operation_id: operation_id.clone(),
            tool_name: tool_name.to_string(),
            status: "running".to_string(),
            created_at: now,
            completed_at: None,
            result: None,
            error: None,
        };
        
        let mut operations = self.operations.lock().unwrap();
        operations.insert(operation_id.clone(), operation);
        
        operation_id
    }
    
    /// Complete an operation
    fn complete_operation(&self, operation_id: &str, result: Option<serde_json::Value>, error: Option<String>) -> bool {
        let mut operations = self.operations.lock().unwrap();
        
        if let Some(operation) = operations.get_mut(operation_id) {
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            
            operation.completed_at = Some(now);
            operation.result = result;
            operation.error = error.clone();
            operation.status = if error.is_some() { "failed".to_string() } else { "completed".to_string() };
            
            true
        } else {
            false
        }
    }
    
    /// Login with a wallet signature
    pub fn login(&self, address: &str, signed_message: &str, signature: &str) -> MCPOperationResult {
        // Simulate operation latency
        std::thread::sleep(self.operation_latency);
        
        // Check rate limits
        if !self.check_rate_limit(address, 60) {
            return MCPOperationResult::RateLimited;
        }
        
        // In a real implementation, we would verify the signature
        // For testing, we assume the signature is valid if it starts with "0x"
        if !signature.starts_with("0x") {
            return MCPOperationResult::AuthenticationFailed;
        }
        
        // Generate a JWT token
        let token = format!("jwt_{}_token", address);
        
        // Store the token
        let mut tokens = self.tokens.lock().unwrap();
        tokens.insert(token.clone(), address.to_string());
        
        // Return success with token
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        
        MCPOperationResult::Success(serde_json::json!({
            "token": token,
            "user_id": address,
            "expires_at": now + 3600, // 1 hour
        }))
    }
    
    /// Validate a JWT token
    pub fn validate_token_endpoint(&self, token: &str) -> MCPOperationResult {
        // Simulate operation latency
        std::thread::sleep(self.operation_latency);
        
        // Validate token
        if self.validate_token(token).is_some() {
            MCPOperationResult::Success(serde_json::json!({
                "valid": true
            }))
        } else {
            MCPOperationResult::AuthenticationFailed
        }
    }
    
    /// List available tools
    pub fn list_tools(&self, token: &str, category: Option<&str>) -> MCPOperationResult {
        // Simulate operation latency
        std::thread::sleep(self.operation_latency);
        
        // Validate token
        let user_id = match self.validate_token(token) {
            Some(id) => id,
            None => return MCPOperationResult::AuthenticationFailed,
        };
        
        // Check rate limits
        if !self.check_rate_limit(&user_id, 60) {
            return MCPOperationResult::RateLimited;
        }
        
        // Get tools
        let tools = self.tools.lock().unwrap();
        
        // Filter by category if specified
        let filtered_tools: Vec<ToolDefinition> = if let Some(cat) = category {
            tools.iter()
                .filter(|tool| tool.categories.contains(&cat.to_string()))
                .cloned()
                .collect()
        } else {
            tools.clone()
        };
        
        // Get unique categories
        let categories: Vec<String> = tools.iter()
            .flat_map(|tool| tool.categories.clone())
            .collect::<std::collections::HashSet<String>>()
            .into_iter()
            .collect();
        
        // Return success with tools
        MCPOperationResult::Success(serde_json::json!({
            "tools": filtered_tools,
            "categories": categories,
        }))
    }
    
    /// Execute a tool
    pub fn execute_tool(&self, token: &str, tool_name: &str, params: serde_json::Value) -> MCPOperationResult {
        // Simulate operation latency
        std::thread::sleep(self.operation_latency);
        
        // Validate token
        let user_id = match self.validate_token(token) {
            Some(id) => id,
            None => return MCPOperationResult::AuthenticationFailed,
        };
        
        // Check rate limits
        if !self.check_rate_limit(&user_id, 60) {
            return MCPOperationResult::RateLimited;
        }
        
        // Check if tool exists
        let tools = self.tools.lock().unwrap();
        let tool = match tools.iter().find(|t| t.name == tool_name) {
            Some(t) => t.clone(),
            None => return MCPOperationResult::ResourceNotFound,
        };
        drop(tools);
        
        // Create operation
        let operation_id = self.create_operation(tool_name, &user_id);
        
        // Execute tool with a delay to simulate async operation
        let tool_name = tool_name.to_string();
        let user_id_clone = user_id.clone();
        let params_clone = params.clone();
        let self_clone = self.clone();
        
        // In a real implementation, this would be spawned in a new thread
        // or handled by a task queue. For simplicity, we'll execute it inline.
        let result = match tool_name.as_str() {
            "vm.create" => self.handle_vm_create(&user_id, params),
            "vm.list" => self.handle_vm_list(&user_id, params),
            "form_pack_build" => self.handle_pack_build(&user_id, params),
            "form_pack_ship" => self.handle_pack_ship(&user_id, params),
            _ => MCPOperationResult::InvalidInput(format!("Unsupported tool: {}", tool_name)),
        };
        
        // Update operation based on result
        match &result {
            MCPOperationResult::Success(value) => {
                self.complete_operation(&operation_id, Some(value.clone()), None);
            },
            MCPOperationResult::InvalidInput(msg) => {
                self.complete_operation(&operation_id, None, Some(msg.clone()));
            },
            MCPOperationResult::OperationFailed(msg) => {
                self.complete_operation(&operation_id, None, Some(msg.clone()));
            },
            MCPOperationResult::InternalError(msg) => {
                self.complete_operation(&operation_id, None, Some(msg.clone()));
            },
            _ => {
                self.complete_operation(&operation_id, None, Some(format!("Operation failed: {:?}", result)));
            }
        };
        
        // Return success with operation ID
        MCPOperationResult::Success(serde_json::json!({
            "operation_id": operation_id,
        }))
    }
    
    /// Get operation status
    pub fn get_operation_status(&self, token: &str, operation_id: &str) -> MCPOperationResult {
        // Simulate operation latency
        std::thread::sleep(self.operation_latency);
        
        // Validate token
        let user_id = match self.validate_token(token) {
            Some(id) => id,
            None => return MCPOperationResult::AuthenticationFailed,
        };
        
        // Check rate limits
        if !self.check_rate_limit(&user_id, 60) {
            return MCPOperationResult::RateLimited;
        }
        
        // Get operation
        let operations = self.operations.lock().unwrap();
        let operation = match operations.get(operation_id) {
            Some(op) => op.clone(),
            None => return MCPOperationResult::ResourceNotFound,
        };
        
        // Return success with operation status
        MCPOperationResult::Success(serde_json::json!({
            "operation_id": operation.operation_id,
            "status": operation.status,
            "created_at": operation.created_at,
            "completed_at": operation.completed_at,
            "result": operation.result,
            "error": operation.error,
        }))
    }
    
    /// Handle VM creation
    fn handle_vm_create(&self, user_id: &str, params: serde_json::Value) -> MCPOperationResult {
        // Extract parameters
        let name = match params.get("name").and_then(|v| v.as_str()) {
            Some(n) => n,
            None => return MCPOperationResult::InvalidInput("Missing required parameter: name".to_string()),
        };
        
        let vcpus = params.get("vcpus").and_then(|v| v.as_u64()).unwrap_or(1) as u8;
        let memory_mb = params.get("memory_mb").and_then(|v| v.as_u64()).unwrap_or(1024);
        let disk_gb = params.get("disk_gb").and_then(|v| v.as_u64()).unwrap_or(10);
        
        // Generate VM ID
        let vm_id = self.generate_id("vm");
        
        // Create VM instance
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        
        let vm = VMInstance {
            vm_id: vm_id.clone(),
            name: name.to_string(),
            owner_id: user_id.to_string(),
            vcpus,
            memory_mb,
            disk_gb,
            status: VMStatus::Creating,
            created_at: now,
            ip_address: None,
        };
        
        // Add VM to list
        let mut vms = self.vms.lock().unwrap();
        vms.insert(vm_id.clone(), vm);
        
        // Return success
        MCPOperationResult::Success(serde_json::json!({
            "vm_id": vm_id,
            "status": "creating",
            "message": format!("VM '{}' creation initiated", name),
        }))
    }
    
    /// Handle VM listing
    fn handle_vm_list(&self, user_id: &str, params: serde_json::Value) -> MCPOperationResult {
        // Extract filter parameters
        let status_filter = params.get("status").and_then(|v| v.as_str());
        
        // Get VMs owned by user
        let vms = self.vms.lock().unwrap();
        let user_vms: Vec<VMInstance> = vms.values()
            .filter(|vm| vm.owner_id == user_id)
            .filter(|vm| {
                if let Some(status) = status_filter {
                    match (status, &vm.status) {
                        ("creating", VMStatus::Creating) => true,
                        ("running", VMStatus::Running) => true,
                        ("stopped", VMStatus::Stopped) => true,
                        _ => false,
                    }
                } else {
                    true
                }
            })
            .cloned()
            .collect();
        
        // Return success with VMs
        MCPOperationResult::Success(serde_json::json!({
            "vms": user_vms,
            "total": user_vms.len(),
        }))
    }
    
    /// Handle Pack Build
    fn handle_pack_build(&self, user_id: &str, params: serde_json::Value) -> MCPOperationResult {
        // Extract parameters
        let formfile = match params.get("formfile_content").and_then(|v| v.as_str()) {
            Some(f) => f,
            None => return MCPOperationResult::InvalidInput("Missing required parameter: formfile_content".to_string()),
        };
        
        // Generate build ID
        let build_id = self.generate_id("build");
        
        // Create build
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        
        let build = WorkloadBuild {
            build_id: build_id.clone(),
            owner_id: user_id.to_string(),
            formfile: formfile.to_string(),
            status: BuildStatus::Creating,
            created_at: now,
            completed_at: None,
        };
        
        // Add build to list
        let mut builds = self.builds.lock().unwrap();
        builds.insert(build_id.clone(), build);
        
        // Return success
        MCPOperationResult::Success(serde_json::json!({
            "build_id": build_id,
            "status": "creating",
            "message": "Build initiated",
        }))
    }
    
    /// Handle Pack Ship
    fn handle_pack_ship(&self, user_id: &str, params: serde_json::Value) -> MCPOperationResult {
        // Extract parameters
        let build_id = match params.get("build_id").and_then(|v| v.as_str()) {
            Some(id) => id,
            None => return MCPOperationResult::InvalidInput("Missing required parameter: build_id".to_string()),
        };
        
        let instance_id = match params.get("instance_id").and_then(|v| v.as_str()) {
            Some(id) => id,
            None => return MCPOperationResult::InvalidInput("Missing required parameter: instance_id".to_string()),
        };
        
        // Check if build exists
        let builds = self.builds.lock().unwrap();
        let build = match builds.get(build_id) {
            Some(b) => b.clone(),
            None => return MCPOperationResult::ResourceNotFound,
        };
        drop(builds);
        
        // Check if build belongs to user
        if build.owner_id != user_id {
            return MCPOperationResult::PermissionDenied;
        }
        
        // Check if VM exists
        let vms = self.vms.lock().unwrap();
        let vm = match vms.get(instance_id) {
            Some(v) => v.clone(),
            None => return MCPOperationResult::ResourceNotFound,
        };
        drop(vms);
        
        // Check if VM belongs to user
        if vm.owner_id != user_id {
            return MCPOperationResult::PermissionDenied;
        }
        
        // Generate deployment ID
        let deployment_id = self.generate_id("deploy");
        
        // Create deployment
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        
        let deployment = WorkloadDeployment {
            deployment_id: deployment_id.clone(),
            build_id: build_id.to_string(),
            owner_id: user_id.to_string(),
            vm_id: instance_id.to_string(),
            status: "deploying".to_string(),
            created_at: now,
            started_at: None,
        };
        
        // Add deployment to list
        let mut deployments = self.deployments.lock().unwrap();
        deployments.insert(deployment_id.clone(), deployment);
        
        // Return success
        MCPOperationResult::Success(serde_json::json!({
            "deployment_id": deployment_id,
            "status": "deploying",
            "message": format!("Deploying build {} to VM {}", build_id, instance_id),
        }))
    }
}

impl Clone for MockMCPServer {
    fn clone(&self) -> Self {
        Self {
            tokens: self.tokens.clone(),
            vms: self.vms.clone(),
            builds: self.builds.clone(),
            deployments: self.deployments.clone(),
            operations: self.operations.clone(),
            tools: self.tools.clone(),
            permissions: self.permissions.clone(),
            rate_limits: self.rate_limits.clone(),
            operation_latency: self.operation_latency,
            failure_rate: self.failure_rate,
        }
    }
}

/// MCP server harness for testing
pub struct MCPHarness {
    /// MCP server
    pub server: MockMCPServer,
}

impl MCPHarness {
    /// Create a new MCP harness
    pub fn new() -> Self {
        Self {
            server: MockMCPServer::new(),
        }
    }
    
    /// Login with wallet signature
    pub fn login(&self, address: &str, signed_message: &str, signature: &str) -> MCPOperationResult {
        self.server.login(address, signed_message, signature)
    }
    
    /// Validate a JWT token
    pub fn validate_token(&self, token: &str) -> MCPOperationResult {
        self.server.validate_token_endpoint(token)
    }
    
    /// List available tools
    pub fn list_tools(&self, token: &str, category: Option<&str>) -> MCPOperationResult {
        self.server.list_tools(token, category)
    }
    
    /// Execute a tool
    pub fn execute_tool(&self, token: &str, tool_name: &str, params: serde_json::Value) -> MCPOperationResult {
        self.server.execute_tool(token, tool_name, params)
    }
    
    /// Get operation status
    pub fn get_operation_status(&self, token: &str, operation_id: &str) -> MCPOperationResult {
        self.server.get_operation_status(token, operation_id)
    }
}
