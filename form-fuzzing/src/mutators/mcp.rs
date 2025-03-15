// form-fuzzing/src/mutators/mcp.rs

//! Mutators for MCP API requests

use crate::generators::mcp::{
    LoginRequest, VMCreateRequest, VMListRequest, 
    PackBuildRequest, PackShipRequest
};
use crate::mutators::Mutator;
use rand::Rng;
use serde_json::{json, Value};

/// Mutator for login requests
pub struct LoginRequestMutator;

impl LoginRequestMutator {
    /// Create a new login request mutator
    pub fn new() -> Self {
        Self
    }
}

impl Mutator<LoginRequest> for LoginRequestMutator {
    fn mutate(&self, request: &mut LoginRequest) {
        let mut rng = rand::thread_rng();
        
        // Choose a random aspect to mutate
        let mutation = rng.gen_range(0..3);
        
        match mutation {
            0 => {
                // Mutate address
                if rng.gen_bool(0.5) {
                    // Remove 0x prefix
                    if request.address.starts_with("0x") {
                        request.address = request.address[2..].to_string();
                    }
                } else {
                    // Add invalid characters
                    request.address = format!("0x{}Z", &request.address[2..]);
                }
            },
            1 => {
                // Mutate signed message
                if rng.gen_bool(0.5) {
                    // Empty message
                    request.signed_message = "".to_string();
                } else {
                    // Add very long message
                    request.signed_message = format!("{}{}", request.signed_message, "X".repeat(1000));
                }
            },
            2 => {
                // Mutate signature
                if rng.gen_bool(0.5) {
                    // Remove 0x prefix
                    if request.signature.starts_with("0x") {
                        request.signature = request.signature[2..].to_string();
                    }
                } else {
                    // Invalid length
                    request.signature = format!("0x{}", "a".repeat(rng.gen_range(10..300)));
                }
            },
            _ => {}
        }
    }
}

/// Mutator for VM creation requests
pub struct VMCreateRequestMutator;

impl VMCreateRequestMutator {
    /// Create a new VM creation request mutator
    pub fn new() -> Self {
        Self
    }
}

impl Mutator<VMCreateRequest> for VMCreateRequestMutator {
    fn mutate(&self, request: &mut VMCreateRequest) {
        let mut rng = rand::thread_rng();
        
        // Choose a random aspect to mutate
        let mutation = rng.gen_range(0..4);
        
        match mutation {
            0 => {
                // Mutate name
                if rng.gen_bool(0.5) {
                    // Empty name
                    request.name = "".to_string();
                } else {
                    // Very long name
                    request.name = format!("vm-{}", "a".repeat(rng.gen_range(100..1000)));
                }
            },
            1 => {
                // Mutate vcpus
                if rng.gen_bool(0.5) {
                    // Set to zero
                    request.vcpus = Some(0);
                } else {
                    // Set to very large number
                    request.vcpus = Some(rng.gen_range(100..255));
                }
            },
            2 => {
                // Mutate memory_mb
                if rng.gen_bool(0.5) {
                    // Set to very small
                    request.memory_mb = Some(rng.gen_range(1..100));
                } else {
                    // Set to very large
                    request.memory_mb = Some(rng.gen_range(1024*1024..u64::MAX/2));
                }
            },
            3 => {
                // Mutate disk_gb
                if rng.gen_bool(0.5) {
                    // Set to very small
                    request.disk_gb = Some(rng.gen_range(1..5));
                } else {
                    // Set to very large
                    request.disk_gb = Some(rng.gen_range(10000..u64::MAX/1024));
                }
            },
            _ => {}
        }
    }
}

/// Mutator for JSON value
pub struct JsonValueMutator;

impl JsonValueMutator {
    /// Create a new JSON value mutator
    pub fn new() -> Self {
        Self
    }
    
    /// Add a random field to a JSON object
    pub fn add_random_field(&self, value: &mut Value) {
        let mut rng = rand::thread_rng();
        
        if let Some(obj) = value.as_object_mut() {
            // Add a random field with a random value
            let field_type = rng.gen_range(0..5);
            let field_name = format!("random_field_{}", rng.gen_range(0..1000));
            
            match field_type {
                0 => obj.insert(field_name, json!(rng.gen::<i32>())),
                1 => obj.insert(field_name, json!(rng.gen::<f64>())),
                2 => obj.insert(field_name, json!(format!("string_{}", rng.gen::<u32>()))),
                3 => obj.insert(field_name, json!(rng.gen::<bool>())),
                _ => obj.insert(field_name, json!(null)),
            };
        }
    }
    
    /// Remove a random field from a JSON object
    pub fn remove_random_field(&self, value: &mut Value) {
        let mut rng = rand::thread_rng();
        
        if let Some(obj) = value.as_object_mut() {
            if !obj.is_empty() {
                let keys: Vec<String> = obj.keys().cloned().collect();
                let idx = rng.gen_range(0..keys.len());
                obj.remove(&keys[idx]);
            }
        }
    }
    
    /// Change the type of a random field in a JSON object
    pub fn change_field_type(&self, value: &mut Value) {
        let mut rng = rand::thread_rng();
        
        if let Some(obj) = value.as_object_mut() {
            if !obj.is_empty() {
                let keys: Vec<String> = obj.keys().cloned().collect();
                let idx = rng.gen_range(0..keys.len());
                let key = &keys[idx];
                
                // Change the type of the field
                let new_type = rng.gen_range(0..5);
                match new_type {
                    0 => obj.insert(key.clone(), json!(rng.gen::<i32>())),
                    1 => obj.insert(key.clone(), json!(rng.gen::<f64>())),
                    2 => obj.insert(key.clone(), json!(format!("string_{}", rng.gen::<u32>()))),
                    3 => obj.insert(key.clone(), json!(rng.gen::<bool>())),
                    _ => obj.insert(key.clone(), json!(null)),
                };
            }
        }
    }
}

impl Mutator<Value> for JsonValueMutator {
    fn mutate(&self, value: &mut Value) {
        let mut rng = rand::thread_rng();
        
        // Choose mutation type
        let mutation = rng.gen_range(0..3);
        
        match mutation {
            0 => self.add_random_field(value),
            1 => self.remove_random_field(value),
            2 => self.change_field_type(value),
            _ => {}
        }
    }
}

/// Mutator for VM list request
pub struct VMListRequestMutator;

impl VMListRequestMutator {
    /// Create a new VM list request mutator
    pub fn new() -> Self {
        Self
    }
}

impl Mutator<VMListRequest> for VMListRequestMutator {
    fn mutate(&self, request: &mut VMListRequest) {
        let mut rng = rand::thread_rng();
        
        // For status field, can set to an invalid status
        if rng.gen_bool(0.7) {
            let invalid_statuses = ["invalid", "pending", "error", "unknown", ""];
            let idx = rng.gen_range(0..invalid_statuses.len());
            request.status = Some(invalid_statuses[idx].to_string());
        } else {
            // Or remove the status
            request.status = None;
        }
    }
}

/// Mutator for Pack build request
pub struct PackBuildRequestMutator;

impl PackBuildRequestMutator {
    /// Create a new Pack build request mutator
    pub fn new() -> Self {
        Self
    }
}

impl Mutator<PackBuildRequest> for PackBuildRequestMutator {
    fn mutate(&self, request: &mut PackBuildRequest) {
        let mut rng = rand::thread_rng();
        
        // Mutate the Formfile
        let mutation = rng.gen_range(0..4);
        
        match mutation {
            0 => {
                // Corrupt a JSON formfile
                if request.formfile_content.contains("{") {
                    let pos = request.formfile_content.find("{").unwrap_or(0);
                    let end = request.formfile_content.rfind("}").unwrap_or(request.formfile_content.len());
                    
                    if pos < end {
                        let mut content = request.formfile_content.clone();
                        let corrupt_pos = rng.gen_range(pos..end);
                        content.remove(corrupt_pos);
                        request.formfile_content = content;
                    }
                }
            },
            1 => {
                // Corrupt a YAML formfile
                if request.formfile_content.contains("name:") {
                    let lines: Vec<&str> = request.formfile_content.lines().collect();
                    if !lines.is_empty() {
                        let idx = rng.gen_range(0..lines.len());
                        let mut new_lines: Vec<String> = lines.iter().map(|s| s.to_string()).collect();
                        
                        // Corrupt indentation
                        if !new_lines[idx].is_empty() {
                            if new_lines[idx].starts_with("  ") {
                                new_lines[idx] = new_lines[idx][1..].to_string(); // Remove one space
                            } else {
                                new_lines[idx] = format!("  {}", new_lines[idx]); // Add spaces
                            }
                        }
                        
                        request.formfile_content = new_lines.join("\n");
                    }
                }
            },
            2 => {
                // Replace with completely invalid content
                request.formfile_content = "This is not a valid Formfile".to_string();
            },
            3 => {
                // Make extremely large
                request.formfile_content = format!("{}\n{}", 
                    request.formfile_content,
                    "# ".repeat(rng.gen_range(1000..10000))
                );
            },
            _ => {}
        }
    }
}

/// Mutator for Pack ship request
pub struct PackShipRequestMutator;

impl PackShipRequestMutator {
    /// Create a new Pack ship request mutator
    pub fn new() -> Self {
        Self
    }
}

impl Mutator<PackShipRequest> for PackShipRequestMutator {
    fn mutate(&self, request: &mut PackShipRequest) {
        let mut rng = rand::thread_rng();
        
        // Choose a random aspect to mutate
        let mutation = rng.gen_range(0..2);
        
        match mutation {
            0 => {
                // Mutate build_id
                if rng.gen_bool(0.5) {
                    // Empty build_id
                    request.build_id = "".to_string();
                } else {
                    // Invalid format
                    request.build_id = format!("not-a-build-id-{}", rng.gen::<u32>());
                }
            },
            1 => {
                // Mutate instance_id
                if rng.gen_bool(0.5) {
                    // Empty instance_id
                    request.instance_id = "".to_string();
                } else {
                    // Invalid format
                    request.instance_id = format!("not-a-vm-id-{}", rng.gen::<u32>());
                }
            },
            _ => {}
        }
    }
} 