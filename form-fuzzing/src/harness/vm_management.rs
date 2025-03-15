// form-fuzzing/src/harness/vm_management.rs
//! Test harness for VM management and ownership verification

use crate::generators::vm::{VMCreateRequest, VMDeleteRequest};
use crate::harness::{FuzzingHarness, HarnessConfig, SystemState, with_timeout};
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use std::time::Instant;

/// Mock signature verifier for testing
pub struct MockSignatureVerifier {
    // Mapping of key IDs to public keys
    keys: HashMap<String, Vec<u8>>,
    // Valid signature algorithms
    valid_algorithms: Vec<String>,
    // Maximum allowed timestamp difference (in seconds)
    max_timestamp_diff: u64,
}

impl MockSignatureVerifier {
    pub fn new() -> Self {
        let mut keys = HashMap::new();
        // Add some mock keys
        keys.insert("user-12345".to_string(), vec![1, 2, 3, 4, 5]);
        keys.insert("user-67890".to_string(), vec![6, 7, 8, 9, 10]);
        
        Self {
            keys,
            valid_algorithms: vec!["ed25519".to_string(), "rsa2048".to_string()],
            max_timestamp_diff: 3600, // 1 hour
        }
    }
    
    pub fn verify(&self, data: &[u8], signature: &Signature) -> bool {
        // Check if key exists
        if !self.keys.contains_key(&signature.key_id) {
            println!("Key not found: {}", signature.key_id);
            return false;
        }
        
        // Check if algorithm is valid
        if !self.valid_algorithms.contains(&signature.algorithm) {
            println!("Invalid algorithm: {}", signature.algorithm);
            return false;
        }
        
        // Check timestamp
        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        
        let time_diff = if current_time > signature.timestamp {
            current_time - signature.timestamp
        } else {
            signature.timestamp - current_time
        };
        
        if time_diff > self.max_timestamp_diff {
            println!("Timestamp too old: diff={}", time_diff);
            return false;
        }
        
        // Real implementation would verify the signature cryptographically
        // For testing, we'll just check if it starts with "sig-"
        if signature.value.starts_with(b"sig-") {
            return true;
        }
        
        println!("Invalid signature value");
        false
    }
}

/// Simple signature struct for verification
pub struct Signature {
    pub key_id: String,
    pub algorithm: String,
    pub timestamp: u64,
    pub value: Vec<u8>,
}

/// Mock permission checker for testing
pub struct MockPermissionChecker {
    // Mapping of user IDs to permissions
    permissions: HashMap<String, Vec<String>>,
}

impl MockPermissionChecker {
    pub fn new() -> Self {
        let mut permissions = HashMap::new();
        // Add some mock permissions
        permissions.insert(
            "user-12345".to_string(),
            vec!["vm.create".to_string(), "vm.delete".to_string()],
        );
        permissions.insert(
            "user-67890".to_string(),
            vec!["vm.create".to_string()],
        );
        
        Self { permissions }
    }
    
    pub fn has_permission(&self, user_id: &str, permission: &str) -> bool {
        if let Some(perms) = self.permissions.get(user_id) {
            return perms.contains(&permission.to_string());
        }
        false
    }
}

/// Possible results of VM operations
#[derive(Debug, Clone)]
pub enum VMOperationResult {
    Success,
    InvalidSignature,
    PermissionDenied,
    ResourceError(String),
    Timeout,
    InternalError(String),
}

/// Mock VM manager for testing
pub struct MockVMManager {
    // VMs created during this test session
    vms: HashMap<String, CreateVmRequest>,
    // Maximum number of VMs allowed
    max_vms: usize,
    // Authentication failures to simulate
    auth_failures: Vec<String>,
}

impl MockVMManager {
    pub fn new() -> Self {
        Self {
            vms: HashMap::new(),
            max_vms: 100,
            auth_failures: Vec::new(),
        }
    }
    
    pub fn create_vm(&mut self, request: &CreateVmRequest) -> Result<String, String> {
        // Check resource limits
        if self.vms.len() >= self.max_vms {
            return Err("Resource limit exceeded: maximum number of VMs reached".to_string());
        }
        
        // Create a VM ID
        let vm_id = format!("vm-{}-{}", request.name, rand::random::<u16>());
        
        // Store the VM
        self.vms.insert(vm_id.clone(), request.clone());
        
        Ok(vm_id)
    }
    
    pub fn delete_vm(&mut self, name: &str) -> Result<(), String> {
        if self.vms.remove(name).is_none() {
            return Err(format!("VM not found: {}", name));
        }
        Ok(())
    }
    
    pub fn simulate_auth_failure(&mut self, vm_name: &str) {
        self.auth_failures.push(vm_name.to_string());
    }
    
    pub fn should_fail_auth(&self, vm_name: &str) -> bool {
        self.auth_failures.contains(&vm_name.to_string())
    }
}

/// Simple VM creation request for the mock VM manager
#[derive(Debug, Clone)]
pub struct CreateVmRequest {
    pub name: String,
    pub cpu_count: u32,
    pub memory_mb: u32,
    pub user_id: String,
}

/// Test harness for VM management operations
pub struct VMManagementHarness {
    // Configuration for the harness
    config: HarnessConfig,
    // Mock components for testing
    signature_verifier: MockSignatureVerifier,
    permission_checker: MockPermissionChecker,
    vm_manager: Arc<Mutex<MockVMManager>>,
    // Track operations for analysis
    operations: Vec<(String, VMOperationResult)>,
}

impl VMManagementHarness {
    /// Create a new VM management harness with default configuration
    pub fn new() -> Self {
        Self::new_with_config(HarnessConfig::default())
    }
    
    /// Create a new VM management harness with custom configuration
    pub fn new_with_config(config: HarnessConfig) -> Self {
        Self {
            config,
            signature_verifier: MockSignatureVerifier::new(),
            permission_checker: MockPermissionChecker::new(),
            vm_manager: Arc::new(Mutex::new(MockVMManager::new())),
            operations: Vec::new(),
        }
    }
    
    /// Test signature verification
    pub fn test_signature_verification(&mut self, request: VMCreateRequest, signature: Signature) -> VMOperationResult {
        let operation = "SignatureVerification";
        
        // Convert request to bytes for verification
        let request_bytes = format!(
            "{}:{}:{}:{}",
            request.user_id, request.cpu_cores, request.memory_mb, request.timestamp
        ).into_bytes();
        
        // Verify signature
        let result = if self.signature_verifier.verify(&request_bytes, &signature) {
            VMOperationResult::Success
        } else {
            VMOperationResult::InvalidSignature
        };
        
        // Record the operation
        self.record_operation(operation, result.clone());
        
        result
    }
    
    /// Test permission checks
    pub fn test_permission_checks(&mut self, user_id: &str, permission: &str) -> VMOperationResult {
        let operation = format!("PermissionCheck:{}", permission);
        
        // Check permission
        let result = if self.permission_checker.has_permission(user_id, permission) {
            VMOperationResult::Success
        } else {
            VMOperationResult::PermissionDenied
        };
        
        // Record the operation
        self.record_operation(&operation, result.clone());
        
        result
    }
    
    /// Test VM creation
    pub fn test_vm_creation(&mut self, request: VMCreateRequest, signature: Signature) -> VMOperationResult {
        let operation = "VMCreation";
        
        // Convert VMCreateRequest to our internal CreateVmRequest
        let create_request = CreateVmRequest {
            name: format!("vm-{}-{}", request.user_id, request.timestamp),
            cpu_count: request.cpu_cores,
            memory_mb: request.memory_mb,
            user_id: request.user_id.clone(),
        };
        
        // Convert request to bytes for verification
        let request_bytes = format!(
            "{}:{}:{}:{}",
            request.user_id, request.cpu_cores, request.memory_mb, request.timestamp
        ).into_bytes();
        
        // First, verify signature
        if !self.signature_verifier.verify(&request_bytes, &signature) {
            let result = VMOperationResult::InvalidSignature;
            self.record_operation(operation, result.clone());
            return result;
        }
        
        // Then, check permission
        if !self.permission_checker.has_permission(&request.user_id, "vm.create") {
            let result = VMOperationResult::PermissionDenied;
            self.record_operation(operation, result.clone());
            return result;
        }
        
        // Finally, create the VM with timeout protection
        let vm_manager = self.vm_manager.clone();
        let result = with_timeout(self.config.timeout_ms, move || {
            let mut manager = vm_manager.lock().unwrap();
            manager.create_vm(&create_request)
        });
        
        // Process the result
        let operation_result = match result {
            Ok(Ok(_)) => VMOperationResult::Success,
            Ok(Err(e)) => VMOperationResult::ResourceError(e),
            Err(e) => VMOperationResult::Timeout,
        };
        
        // Record the operation
        self.record_operation(operation, operation_result.clone());
        
        operation_result
    }
    
    /// Test VM deletion
    pub fn test_vm_deletion(&mut self, name: &str, signature: Signature) -> VMOperationResult {
        let operation = "VMDeletion";
        
        // Convert request to bytes for verification
        let request_bytes = format!("delete:{}", name).into_bytes();
        
        // First, verify signature
        if !self.signature_verifier.verify(&request_bytes, &signature) {
            let result = VMOperationResult::InvalidSignature;
            self.record_operation(operation, result.clone());
            return result;
        }
        
        // Extract user ID from signature
        let user_id = signature.key_id.clone();
        
        // Then, check permission
        if !self.permission_checker.has_permission(&user_id, "vm.delete") {
            let result = VMOperationResult::PermissionDenied;
            self.record_operation(operation, result.clone());
            return result;
        }
        
        // Finally, delete the VM with timeout protection
        let vm_manager = self.vm_manager.clone();
        let vm_name = name.to_string();
        let result = with_timeout(self.config.timeout_ms, move || {
            let mut manager = vm_manager.lock().unwrap();
            if manager.should_fail_auth(&vm_name) {
                Err("Authentication failure".to_string())
            } else {
                manager.delete_vm(&vm_name)
            }
        });
        
        // Process the result
        let operation_result = match result {
            Ok(Ok(_)) => VMOperationResult::Success,
            Ok(Err(e)) => VMOperationResult::ResourceError(e),
            Err(e) => VMOperationResult::Timeout,
        };
        
        // Record the operation
        self.record_operation(operation, operation_result.clone());
        
        operation_result
    }
    
    /// Test VM lifecycle (create and delete operations)
    pub fn test_vm_lifecycle(&mut self, operations: Vec<(String, CreateVmRequest, Signature)>) -> Vec<VMOperationResult> {
        let mut results = Vec::new();
        
        for (op, request, signature) in operations {
            let result = match op.as_str() {
                "create" => {
                    // Convert CreateVmRequest to VMCreateRequest
                    let vm_request = VMCreateRequest {
                        user_id: request.user_id.clone(),
                        cpu_cores: request.cpu_count,
                        memory_mb: request.memory_mb,
                        disk_gb: 10, // Default value
                        network_interfaces: 1, // Default value
                        signature: String::new(), // Will be ignored
                        timestamp: signature.timestamp,
                    };
                    self.test_vm_creation(vm_request, signature)
                },
                "delete" => self.test_vm_deletion(&request.name, signature),
                _ => VMOperationResult::InternalError(format!("Unknown operation: {}", op)),
            };
            
            results.push(result);
        }
        
        results
    }
    
    /// Capture the current system state for comparison
    pub fn capture_system_state(&self) -> SystemState {
        // In a real implementation, this would capture actual system metrics
        // For now, just create a placeholder
        let vm_count = if let Ok(manager) = self.vm_manager.lock() {
            manager.vms.len()
        } else {
            0
        };
        
        SystemState {
            memory_usage: vm_count * 1024, // Mock memory usage based on VM count
            process_count: 1 + vm_count, // Mock process count
            open_files: 10 + vm_count * 2, // Mock open file count
            network_connections: 5 + vm_count, // Mock network connection count
        }
    }
    
    /// Record an operation for later analysis
    fn record_operation(&mut self, operation: &str, result: VMOperationResult) {
        if self.config.verbose {
            println!("Operation: {}, Result: {:?}", operation, result);
        }
        self.operations.push((operation.to_string(), result));
    }
}

impl FuzzingHarness for VMManagementHarness {
    fn setup(&mut self) {
        // Reset the VM manager
        if let Ok(mut manager) = self.vm_manager.lock() {
            *manager = MockVMManager::new();
        }
        
        // Clear operation history
        self.operations.clear();
        
        if self.config.verbose {
            println!("VM Management harness set up");
        }
    }
    
    fn teardown(&mut self) {
        if self.config.verbose {
            println!("VM Management harness torn down");
        }
    }
    
    fn reset(&mut self) {
        self.setup();
    }
} 