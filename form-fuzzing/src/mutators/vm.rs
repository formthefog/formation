// Formation Network Fuzzing Infrastructure
// VM Mutators Module

use crate::generators::vm::{VMCreateRequest, VMDeleteRequest};
use crate::mutators::Mutator;

/// Mutator for VM creation requests
pub struct VMMutator;

impl VMMutator {
    pub fn new() -> Self {
        Self
    }
}

impl Mutator<VMCreateRequest> for VMMutator {
    fn mutate(&self, input: &mut VMCreateRequest) {
        // Simplified mutation - in a real system, this would use proper fuzzing strategies
        // Here we're just tweaking the resource values
        
        // Randomly choose what to mutate (simplified approach)
        let mutation_type = 0; // In real implementation, this would be random
        
        match mutation_type {
            0 => {
                // Mutate CPU cores
                input.cpu_cores = match input.cpu_cores {
                    1 => 2,
                    _ => input.cpu_cores * 2,
                };
            }
            1 => {
                // Mutate memory
                input.memory_mb = input.memory_mb + 1024;
            }
            2 => {
                // Mutate disk
                input.disk_gb = input.disk_gb + 10;
            }
            3 => {
                // Mutate user ID
                input.user_id = format!("different-user-{}", input.user_id);
            }
            4 => {
                // Mutate signature (make it invalid)
                input.signature = format!("invalid-mutated-{}", input.timestamp);
            }
            _ => {
                // Mutate timestamp
                input.timestamp += 1000;
            }
        }
    }
}

/// Specific mutator for VM creation requests that targets resource limits
pub struct VMResourceMutator;

impl VMResourceMutator {
    pub fn new() -> Self {
        Self
    }
    
    /// Mutate to push resource values to the limits
    pub fn mutate_to_max_resources(&self, input: &mut VMCreateRequest) {
        input.cpu_cores = 256;
        input.memory_mb = 1024 * 1024; // 1 TB
        input.disk_gb = 10000;
        input.network_interfaces = 100;
    }
    
    /// Mutate to push resource values to zero or negative (invalid values)
    pub fn mutate_to_invalid_resources(&self, input: &mut VMCreateRequest) {
        input.cpu_cores = 0;
        input.memory_mb = 0;
        input.disk_gb = 0;
        input.network_interfaces = 0;
    }
}

impl Mutator<VMCreateRequest> for VMResourceMutator {
    fn mutate(&self, input: &mut VMCreateRequest) {
        // Randomly choose between max and invalid resources (simplified approach)
        let use_max = true; // In real implementation, this would be random
        
        if use_max {
            self.mutate_to_max_resources(input);
        } else {
            self.mutate_to_invalid_resources(input);
        }
    }
}

/// Specific mutator for VM deletion requests
pub struct VMDeleteMutator;

impl VMDeleteMutator {
    pub fn new() -> Self {
        Self
    }
}

impl Mutator<VMDeleteRequest> for VMDeleteMutator {
    fn mutate(&self, input: &mut VMDeleteRequest) {
        // Simplified mutation - in a real system, this would use proper fuzzing strategies
        
        // Randomly choose what to mutate (simplified approach)
        let mutation_type = 0; // In real implementation, this would be random
        
        match mutation_type {
            0 => {
                // Mutate VM ID
                input.vm_id = format!("different-vm-{}", input.vm_id);
            }
            1 => {
                // Mutate user ID
                input.user_id = format!("different-user-{}", input.user_id);
            }
            2 => {
                // Mutate signature (make it invalid)
                input.signature = format!("invalid-delete-mutated-{}", input.timestamp);
            }
            _ => {
                // Mutate timestamp
                input.timestamp += 1000;
            }
        }
    }
} 