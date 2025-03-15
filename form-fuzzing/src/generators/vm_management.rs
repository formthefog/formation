// form-fuzzing/src/generators/vm_management.rs
//! Generators for VM management related fuzzing

use crate::generators::{BytesGenerator, ArbitraryGenerator, get_u8_or_default, get_u16_or_default, get_u32_or_default, get_string, get_bool};
use rand::{Rng, seq::SliceRandom};

/// VM creation request for fuzzing
#[derive(Debug, Clone)]
pub struct CreateVmRequest {
    pub name: String,
    pub cpu_count: u32,
    pub memory_mb: u32,
    pub disk_gb: u32,
    pub image_id: String,
    pub network_config: NetworkConfig,
    pub tags: Vec<String>,
    pub metadata: Vec<(String, String)>,
}

impl Default for CreateVmRequest {
    fn default() -> Self {
        Self {
            name: "fuzzer-vm".to_string(),
            cpu_count: 1,
            memory_mb: 512,
            disk_gb: 10,
            image_id: "default-image".to_string(),
            network_config: NetworkConfig::default(),
            tags: vec![],
            metadata: vec![],
        }
    }
}

/// Network configuration for a VM
#[derive(Debug, Clone)]
pub struct NetworkConfig {
    pub public_ip: bool,
    pub vpc_id: Option<String>,
    pub subnet_id: Option<String>,
    pub security_groups: Vec<String>,
    pub dns_name: Option<String>,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            public_ip: true,
            vpc_id: None,
            subnet_id: None,
            security_groups: vec![],
            dns_name: None,
        }
    }
}

/// Signature for request authentication
#[derive(Debug, Clone)]
pub struct Signature {
    pub key_id: String,
    pub algorithm: String,
    pub timestamp: u64,
    pub value: Vec<u8>,
}

impl Default for Signature {
    fn default() -> Self {
        Self {
            key_id: "default-key".to_string(),
            algorithm: "sha256".to_string(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            value: vec![0; 32],
        }
    }
}

/// Generate a VM creation request from raw bytes
pub fn generate_create_vm_request(data: &[u8]) -> CreateVmRequest {
    if data.is_empty() {
        return CreateVmRequest::default();
    }
    
    // Use the data to generate different aspects of the request
    let vm_type = get_u8_or_default(data, 0, 0) % 5;
    
    // Based on the "type", generate different VM configurations
    let (name, cpu, memory, disk) = match vm_type {
        0 => ("small-vm", 1, 512, 10),
        1 => ("medium-vm", 2, 2048, 20),
        2 => ("large-vm", 4, 4096, 40),
        3 => ("xlarge-vm", 8, 8192, 80),
        _ => ("custom-vm", 
              get_u8_or_default(data, 1, 1) as u32,
              get_u16_or_default(data, 2, 512) as u32,
              get_u8_or_default(data, 4, 10) as u32),
    };
    
    // Generate a unique name with a suffix based on the data
    let name_suffix = if data.len() > 5 { 
        format!("-{}-{}-{}", data[1], data[2], data[3])
    } else {
        "-fuzzer".to_string()
    };
    
    // Build the VM request
    CreateVmRequest {
        name: format!("{}{}", name, name_suffix),
        cpu_count: cpu,
        memory_mb: memory,
        disk_gb: disk,
        image_id: select_image_id(data),
        network_config: generate_network_config(data),
        tags: generate_tags(data),
        metadata: generate_metadata(data),
    }
}

fn select_image_id(data: &[u8]) -> String {
    // Select from common images or generate a custom one
    let common_images = [
        "ubuntu-20.04", "ubuntu-22.04", "debian-11", 
        "fedora-36", "centos-8", "alpine-3.16"
    ];
    
    if data.len() < 6 || data[5] % 5 < 4 {
        // 80% of the time, use a common image
        let idx = get_u8_or_default(data, 5, 0) as usize % common_images.len();
        common_images[idx].to_string()
    } else {
        // 20% of the time, generate a custom image ID
        format!("custom-image-{}", get_u32_or_default(data, 6, 12345))
    }
}

fn generate_network_config(data: &[u8]) -> NetworkConfig {
    if data.len() < 10 {
        return NetworkConfig::default();
    }
    
    let has_vpc = data[7] % 2 == 0;
    let has_subnet = has_vpc && data[8] % 2 == 0;
    let has_groups = data[9] % 3 > 0; // 2/3 chance
    
    NetworkConfig {
        public_ip: data[6] % 3 > 0, // 2/3 chance of public IP
        vpc_id: if has_vpc { 
            Some(format!("vpc-{}", get_u32_or_default(data, 10, 10000)))
        } else { 
            None 
        },
        subnet_id: if has_subnet { 
            Some(format!("subnet-{}", get_u32_or_default(data, 14, 20000)))
        } else { 
            None 
        },
        security_groups: if has_groups {
            let count = 1 + (data[9] % 3); // 1-3 groups
            (0..count).map(|i| {
                let offset = 18 + i as usize * 4;
                format!("sg-{}", get_u32_or_default(data, offset, 30000 + i as u32))
            }).collect()
        } else {
            vec![]
        },
        dns_name: if data.len() > 30 && data[30] % 4 == 0 { // 25% chance
            Some(format!("vm-{}.example.com", get_string(data, 31, 10)))
        } else {
            None
        },
    }
}

fn generate_tags(data: &[u8]) -> Vec<String> {
    if data.len() < 35 {
        return vec![];
    }
    
    let tag_count = data[34] % 5; // 0-4 tags
    
    let common_tags = [
        "production", "staging", "development", "test", 
        "web", "database", "cache", "worker", "batch"
    ];
    
    (0..tag_count).map(|i| {
        let idx = (data.get(35 + i as usize).copied().unwrap_or(0) as usize) % common_tags.len();
        common_tags[idx].to_string()
    }).collect()
}

fn generate_metadata(data: &[u8]) -> Vec<(String, String)> {
    if data.len() < 40 {
        return vec![];
    }
    
    let metadata_count = data[39] % 3; // 0-2 metadata items
    
    let keys = ["created-by", "department", "project", "environment", "owner"];
    let values = ["fuzzer", "engineering", "security", "research", "devops"];
    
    (0..metadata_count).map(|i| {
        let key_idx = (data.get(40 + i as usize).copied().unwrap_or(0) as usize) % keys.len();
        let val_idx = (data.get(40 + i as usize + 3).copied().unwrap_or(0) as usize) % values.len();
        (keys[key_idx].to_string(), values[val_idx].to_string())
    }).collect()
}

/// Generate a valid signature for a request
pub fn generate_signature(data: &[u8], _request: &CreateVmRequest) -> Signature {
    if data.len() < 45 {
        return Signature::default();
    }
    
    // In a real implementation, this would actually sign the request
    // For fuzzing purposes, we'll just generate a plausible-looking signature
    
    Signature {
        key_id: format!("key-{}", get_u32_or_default(data, 45, 1000)),
        algorithm: match data.get(49).copied().unwrap_or(0) % 3 {
            0 => "sha256".to_string(),
            1 => "sha512".to_string(),
            _ => "ed25519".to_string(),
        },
        timestamp: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
        value: data.get(50..82).map(|s| s.to_vec()).unwrap_or_else(|| vec![0; 32]),
    }
}

/// Generate a malformed signature for testing signature verification
pub fn generate_malformed_signature(data: &[u8], request: &CreateVmRequest) -> Signature {
    // Start with a valid signature
    let mut sig = generate_signature(data, request);
    
    // Choose how to malform it based on the data
    if data.is_empty() {
        // Default malformation: empty signature
        sig.value = vec![];
        return sig;
    }
    
    match data[0] % 5 {
        0 => {
            // Invalid key ID
            sig.key_id = "non-existent-key".to_string();
        }
        1 => {
            // Invalid algorithm
            sig.algorithm = "invalid-algo".to_string();
        }
        2 => {
            // Timestamp in the future
            sig.timestamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() + 3600; // 1 hour in the future
        }
        3 => {
            // Timestamp in the past (expired)
            sig.timestamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs().saturating_sub(3600 * 24); // 1 day in the past
        }
        4 => {
            // Modify the signature value slightly
            if !sig.value.is_empty() {
                sig.value[0] ^= 0xFF; // Flip bits in the first byte
            }
        }
        _ => {}
    }
    
    sig
}

/// Generator for arbitrary VM creation requests
pub fn arbitrary_create_vm_request() -> impl rand::distributions::Distribution<CreateVmRequest> {
    use rand::distributions::{Distribution, Standard};
    
    struct CreateVmRequestGenerator;
    
    impl Distribution<CreateVmRequest> for CreateVmRequestGenerator {
        fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> CreateVmRequest {
            // Generate a random VM configuration
            let vm_type = rng.gen_range(0..5);
            
            // Based on the "type", generate different VM configurations
            let (name, cpu, memory, disk) = match vm_type {
                0 => ("small-vm", 1, 512, 10),
                1 => ("medium-vm", 2, 2048, 20),
                2 => ("large-vm", 4, 4096, 40),
                3 => ("xlarge-vm", 8, 8192, 80),
                _ => ("custom-vm", 
                      rng.gen_range(1..17),
                      rng.gen_range(512..16384),
                      rng.gen_range(10..200)),
            };
            
            // Generate a unique name with a random suffix
            let name_suffix = format!("-{}", rng.gen_range(1000..10000));
            
            // Common image IDs
            let common_images = [
                "ubuntu-20.04", "ubuntu-22.04", "debian-11", 
                "fedora-36", "centos-8", "alpine-3.16"
            ];
            
            // Common tags
            let common_tags = [
                "production", "staging", "development", "test", 
                "web", "database", "cache", "worker", "batch"
            ];
            
            // Generate tags
            let tag_count = rng.gen_range(0..5);
            let tags = (0..tag_count)
                .map(|_| common_tags.choose(rng).unwrap().to_string())
                .collect();
            
            // Generate metadata
            let metadata_count = rng.gen_range(0..3);
            let keys = ["created-by", "department", "project", "environment", "owner"];
            let values = ["fuzzer", "engineering", "security", "research", "devops"];
            let metadata = (0..metadata_count)
                .map(|_| {
                    (
                        keys.choose(rng).unwrap().to_string(),
                        values.choose(rng).unwrap().to_string()
                    )
                })
                .collect();
            
            // Build the VM request
            CreateVmRequest {
                name: format!("{}{}", name, name_suffix),
                cpu_count: cpu,
                memory_mb: memory,
                disk_gb: disk,
                image_id: if rng.gen_bool(0.8) {
                    common_images.choose(rng).unwrap().to_string()
                } else {
                    format!("custom-image-{}", rng.gen_range(1000..10000))
                },
                network_config: NetworkConfig {
                    public_ip: rng.gen_bool(0.7),
                    vpc_id: if rng.gen_bool(0.5) {
                        Some(format!("vpc-{}", rng.gen_range(10000..20000)))
                    } else {
                        None
                    },
                    subnet_id: if rng.gen_bool(0.4) {
                        Some(format!("subnet-{}", rng.gen_range(20000..30000)))
                    } else {
                        None
                    },
                    security_groups: if rng.gen_bool(0.6) {
                        let count = rng.gen_range(1..4);
                        (0..count)
                            .map(|i| format!("sg-{}", rng.gen_range(30000..40000) + i))
                            .collect()
                    } else {
                        vec![]
                    },
                    dns_name: if rng.gen_bool(0.3) {
                        Some(format!("vm-{}.example.com", rng.gen_range(1000..10000)))
                    } else {
                        None
                    },
                },
                tags,
                metadata,
            }
        }
    }
    
    CreateVmRequestGenerator
} 