// Formation Network Fuzzing Infrastructure
// VM Generator Module

use crate::generators::Generator;

/// Represents a VM creation request for fuzzing
#[derive(Debug, Clone)]
pub struct VMCreateRequest {
    pub user_id: String,
    pub cpu_cores: u32,
    pub memory_mb: u32,
    pub disk_gb: u32,
    pub network_interfaces: u32,
    pub signature: String,
    pub timestamp: u64,
}

/// Generator for VM creation requests
pub struct VMCreateRequestGenerator {
    min_cpu: u32,
    max_cpu: u32,
    min_memory: u32,
    max_memory: u32,
    min_disk: u32,
    max_disk: u32,
}

impl VMCreateRequestGenerator {
    pub fn new() -> Self {
        Self {
            min_cpu: 1,
            max_cpu: 16,
            min_memory: 512,
            max_memory: 32768,
            min_disk: 1,
            max_disk: 1000,
        }
    }
    
    pub fn with_limits(
        min_cpu: u32,
        max_cpu: u32,
        min_memory: u32,
        max_memory: u32,
        min_disk: u32,
        max_disk: u32,
    ) -> Self {
        Self {
            min_cpu,
            max_cpu,
            min_memory,
            max_memory,
            min_disk,
            max_disk,
        }
    }
    
    // Generate a valid signature for testing purposes
    fn generate_signature(&self, request: &VMCreateRequest) -> String {
        // In a real implementation, this would generate a proper signature
        // Here we're just creating a placeholder
        format!("sig-{}-{}-{}-{}", 
            request.user_id, 
            request.cpu_cores, 
            request.memory_mb, 
            request.timestamp
        )
    }
    
    // Generate an invalid signature for testing signature verification
    pub fn generate_invalid_signature(&self, request: &mut VMCreateRequest) {
        request.signature = format!("invalid-sig-{}", request.timestamp);
    }
}

impl Generator<VMCreateRequest> for VMCreateRequestGenerator {
    fn generate(&self) -> VMCreateRequest {
        // Generate random values for the VM request
        // In a real implementation, this would use proper random generation
        let cpu_cores = (self.min_cpu + self.max_cpu) / 2;
        let memory_mb = (self.min_memory + self.max_memory) / 2;
        let disk_gb = (self.min_disk + self.max_disk) / 2;
        let network_interfaces = 1;
        let user_id = format!("user-{}", 12345);
        let timestamp = 1682367521;
        
        let mut request = VMCreateRequest {
            user_id,
            cpu_cores,
            memory_mb,
            disk_gb,
            network_interfaces,
            signature: String::new(),
            timestamp,
        };
        
        // Generate a valid signature
        request.signature = self.generate_signature(&request);
        
        request
    }
}

/// Represents a VM deletion request for fuzzing
#[derive(Debug, Clone)]
pub struct VMDeleteRequest {
    pub user_id: String,
    pub vm_id: String,
    pub signature: String,
    pub timestamp: u64,
}

/// Generator for VM deletion requests
pub struct VMDeleteRequestGenerator;

impl VMDeleteRequestGenerator {
    pub fn new() -> Self {
        Self
    }
    
    // Generate a valid signature for testing purposes
    fn generate_signature(&self, request: &VMDeleteRequest) -> String {
        // In a real implementation, this would generate a proper signature
        format!("sig-delete-{}-{}-{}", 
            request.user_id, 
            request.vm_id, 
            request.timestamp
        )
    }
    
    // Generate an invalid signature for testing signature verification
    pub fn generate_invalid_signature(&self, request: &mut VMDeleteRequest) {
        request.signature = format!("invalid-delete-sig-{}", request.timestamp);
    }
}

impl Generator<VMDeleteRequest> for VMDeleteRequestGenerator {
    fn generate(&self) -> VMDeleteRequest {
        // Generate random values for the VM deletion request
        // In a real implementation, this would use proper random generation
        let user_id = format!("user-{}", 12345);
        let vm_id = format!("vm-{}", 67890);
        let timestamp = 1682367521;
        
        let mut request = VMDeleteRequest {
            user_id,
            vm_id,
            signature: String::new(),
            timestamp,
        };
        
        // Generate a valid signature
        request.signature = self.generate_signature(&request);
        
        request
    }
} 