// form-fuzzing/src/generators/mcp.rs

//! Generators for MCP server fuzzing

use crate::generators::Generator;
use rand::{distributions::Alphanumeric, Rng};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::iter;
use std::time::{SystemTime, UNIX_EPOCH};

/// Login request for MCP API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginRequest {
    /// User address (Ethereum wallet)
    pub address: String,
    /// Signed message
    pub signed_message: String,
    /// Signature
    pub signature: String,
}

/// Validates token request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidateTokenRequest {
    /// JWT token
    pub token: String,
}

/// VM creation request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VMCreateRequest {
    /// VM name
    pub name: String,
    /// Number of vCPUs
    pub vcpus: Option<u8>,
    /// Memory size in MB
    pub memory_mb: Option<u64>,
    /// Disk size in GB
    pub disk_gb: Option<u64>,
}

/// VM list request with filters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VMListRequest {
    /// Filter by status
    pub status: Option<String>,
}

/// Pack build request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackBuildRequest {
    /// Formfile content
    pub formfile_content: String,
}

/// Pack ship request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackShipRequest {
    /// Build ID to deploy
    pub build_id: String,
    /// Instance ID to deploy to
    pub instance_id: String,
}

/// Generator for login requests
pub struct LoginRequestGenerator;

impl LoginRequestGenerator {
    /// Create a new login request generator
    pub fn new() -> Self {
        Self
    }
}

impl Generator<LoginRequest> for LoginRequestGenerator {
    fn generate(&self) -> LoginRequest {
        let mut rng = rand::thread_rng();
        
        // Generate a random address (simulating an Ethereum address)
        let address = format!("0x{}", generate_random_hex(40));
        
        // Generate a signed message (usually includes a nonce and timestamp)
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let signed_message = format!("I want to authenticate with the MCP server at timestamp {}", timestamp);
        
        // Generate a valid-looking signature
        let signature = format!("0x{}", generate_random_hex(130));
        
        LoginRequest {
            address,
            signed_message,
            signature,
        }
    }
}

/// Generator for invalid login requests
pub struct InvalidLoginRequestGenerator;

impl InvalidLoginRequestGenerator {
    /// Create a new invalid login request generator
    pub fn new() -> Self {
        Self
    }
}

impl Generator<LoginRequest> for InvalidLoginRequestGenerator {
    fn generate(&self) -> LoginRequest {
        let mut rng = rand::thread_rng();
        
        // Pick a type of invalid request
        let invalid_type = rng.gen_range(0..3);
        
        match invalid_type {
            // Invalid address format
            0 => {
                let address = generate_random_string(20); // Not starting with 0x
                let timestamp = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();
                let signed_message = format!("I want to authenticate with the MCP server at timestamp {}", timestamp);
                let signature = format!("0x{}", generate_random_hex(130));
                
                LoginRequest {
                    address,
                    signed_message,
                    signature,
                }
            },
            // Invalid signature format
            1 => {
                let address = format!("0x{}", generate_random_hex(40));
                let timestamp = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();
                let signed_message = format!("I want to authenticate with the MCP server at timestamp {}", timestamp);
                let signature = generate_random_string(130); // Not starting with 0x
                
                LoginRequest {
                    address,
                    signed_message,
                    signature,
                }
            },
            // Empty signed message
            _ => {
                let address = format!("0x{}", generate_random_hex(40));
                let signed_message = "".to_string();
                let signature = format!("0x{}", generate_random_hex(130));
                
                LoginRequest {
                    address,
                    signed_message,
                    signature,
                }
            }
        }
    }
}

/// Generator for VM creation requests
pub struct VMCreateRequestGenerator;

impl VMCreateRequestGenerator {
    /// Create a new VM creation request generator
    pub fn new() -> Self {
        Self
    }
}

impl Generator<VMCreateRequest> for VMCreateRequestGenerator {
    fn generate(&self) -> VMCreateRequest {
        let mut rng = rand::thread_rng();
        
        // Generate a random VM name
        let name = format!("vm-{}", generate_random_string(8));
        
        // Randomly decide whether to include optional fields
        let include_vcpus = rng.gen_bool(0.7);
        let include_memory = rng.gen_bool(0.7);
        let include_disk = rng.gen_bool(0.7);
        
        // Generate values if included
        let vcpus = if include_vcpus {
            Some(rng.gen_range(1..=16))
        } else {
            None
        };
        
        let memory_mb = if include_memory {
            Some(rng.gen_range(1024..=16384))
        } else {
            None
        };
        
        let disk_gb = if include_disk {
            Some(rng.gen_range(10..=1000))
        } else {
            None
        };
        
        VMCreateRequest {
            name,
            vcpus,
            memory_mb,
            disk_gb,
        }
    }
}

/// Generator for invalid VM creation requests
pub struct InvalidVMCreateRequestGenerator;

impl InvalidVMCreateRequestGenerator {
    /// Create a new invalid VM creation request generator
    pub fn new() -> Self {
        Self
    }
}

impl Generator<Value> for InvalidVMCreateRequestGenerator {
    fn generate(&self) -> Value {
        let mut rng = rand::thread_rng();
        
        // Pick a type of invalid request
        let invalid_type = rng.gen_range(0..4);
        
        match invalid_type {
            // Missing required field (name)
            0 => {
                json!({
                    "vcpus": rng.gen_range(1..=16),
                    "memory_mb": rng.gen_range(1024..=16384),
                    "disk_gb": rng.gen_range(10..=1000),
                })
            },
            // Invalid data type for vcpus
            1 => {
                json!({
                    "name": format!("vm-{}", generate_random_string(8)),
                    "vcpus": generate_random_string(4),
                    "memory_mb": rng.gen_range(1024..=16384),
                    "disk_gb": rng.gen_range(10..=1000),
                })
            },
            // Invalid data type for memory_mb
            2 => {
                json!({
                    "name": format!("vm-{}", generate_random_string(8)),
                    "vcpus": rng.gen_range(1..=16),
                    "memory_mb": generate_random_string(4),
                    "disk_gb": rng.gen_range(10..=1000),
                })
            },
            // Invalid data type for disk_gb
            _ => {
                json!({
                    "name": format!("vm-{}", generate_random_string(8)),
                    "vcpus": rng.gen_range(1..=16),
                    "memory_mb": rng.gen_range(1024..=16384),
                    "disk_gb": generate_random_string(4),
                })
            }
        }
    }
}

/// Generator for VM list requests
pub struct VMListRequestGenerator;

impl VMListRequestGenerator {
    /// Create a new VM list request generator
    pub fn new() -> Self {
        Self
    }
}

impl Generator<VMListRequest> for VMListRequestGenerator {
    fn generate(&self) -> VMListRequest {
        let mut rng = rand::thread_rng();
        
        // Decide whether to include a status filter
        let include_status = rng.gen_bool(0.5);
        
        // Generate a status filter if included
        let status = if include_status {
            let statuses = ["creating", "running", "stopped"];
            let idx = rng.gen_range(0..statuses.len());
            Some(statuses[idx].to_string())
        } else {
            None
        };
        
        VMListRequest {
            status,
        }
    }
}

/// Generator for Pack build requests
pub struct PackBuildRequestGenerator;

impl PackBuildRequestGenerator {
    /// Create a new Pack build request generator
    pub fn new() -> Self {
        Self
    }
    
    /// Generate a valid Formfile
    fn generate_formfile(&self) -> String {
        let mut rng = rand::thread_rng();
        
        // Decide which format to use
        let use_yaml = rng.gen_bool(0.5);
        
        if use_yaml {
            format!(
                r#"name: app-{}
version: 1.0.0
base:
  image: ubuntu:22.04
  update: true
  
packages:
  - name: build-essential
  - name: python3
  - name: python3-pip
  
setup:
  - run: pip3 install fastapi uvicorn

app:
  type: service
  command: python3 main.py
  port: 8000
  health_check: /health
"#,
                generate_random_string(6)
            )
        } else {
            format!(
                r#"{{
  "name": "app-{}",
  "version": "1.0.0",
  "base": {{
    "image": "ubuntu:22.04",
    "update": true
  }},
  "packages": [
    {{ "name": "build-essential" }},
    {{ "name": "python3" }},
    {{ "name": "python3-pip" }}
  ],
  "setup": [
    {{ "run": "pip3 install fastapi uvicorn" }}
  ],
  "app": {{
    "type": "service",
    "command": "python3 main.py",
    "port": 8000,
    "health_check": "/health"
  }}
}}"#,
                generate_random_string(6)
            )
        }
    }
}

impl Generator<PackBuildRequest> for PackBuildRequestGenerator {
    fn generate(&self) -> PackBuildRequest {
        PackBuildRequest {
            formfile_content: self.generate_formfile(),
        }
    }
}

/// Generator for invalid Pack build requests
pub struct InvalidPackBuildRequestGenerator;

impl InvalidPackBuildRequestGenerator {
    /// Create a new invalid Pack build request generator
    pub fn new() -> Self {
        Self
    }
}

impl Generator<PackBuildRequest> for InvalidPackBuildRequestGenerator {
    fn generate(&self) -> PackBuildRequest {
        let mut rng = rand::thread_rng();
        
        // Pick a type of invalid request
        let invalid_type = rng.gen_range(0..3);
        
        match invalid_type {
            // Empty Formfile
            0 => {
                PackBuildRequest {
                    formfile_content: "".to_string(),
                }
            },
            // Invalid JSON syntax
            1 => {
                PackBuildRequest {
                    formfile_content: r#"{
  "name": "invalid-json,
  "version": "1.0.0",
  "base": {
    "image": "ubuntu:22.04",
    "update": true
  }
}"#.to_string(),
                }
            },
            // Invalid YAML syntax
            _ => {
                PackBuildRequest {
                    formfile_content: r#"name: invalid-yaml
version: 1.0.0
base:
  image: "ubuntu:22.04
  update: true
"#.to_string(),
                }
            }
        }
    }
}

/// Generator for Pack ship requests
pub struct PackShipRequestGenerator;

impl PackShipRequestGenerator {
    /// Create a new Pack ship request generator
    pub fn new() -> Self {
        Self
    }
}

impl Generator<PackShipRequest> for PackShipRequestGenerator {
    fn generate(&self) -> PackShipRequest {
        let build_id = format!("build-{}-{}", 
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            generate_random_string(6)
        );
        
        let instance_id = format!("vm-{}-{}", 
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            generate_random_string(6)
        );
        
        PackShipRequest {
            build_id,
            instance_id,
        }
    }
}

/// Generator for invalid Pack ship requests
pub struct InvalidPackShipRequestGenerator;

impl InvalidPackShipRequestGenerator {
    /// Create a new invalid Pack ship request generator
    pub fn new() -> Self {
        Self
    }
}

impl Generator<Value> for InvalidPackShipRequestGenerator {
    fn generate(&self) -> Value {
        let mut rng = rand::thread_rng();
        
        // Pick a type of invalid request
        let invalid_type = rng.gen_range(0..3);
        
        match invalid_type {
            // Missing build_id
            0 => {
                json!({
                    "instance_id": format!("vm-{}-{}", 
                        SystemTime::now()
                            .duration_since(UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_secs(),
                        generate_random_string(6)
                    )
                })
            },
            // Missing instance_id
            1 => {
                json!({
                    "build_id": format!("build-{}-{}", 
                        SystemTime::now()
                            .duration_since(UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_secs(),
                        generate_random_string(6)
                    )
                })
            },
            // Empty fields
            _ => {
                json!({
                    "build_id": "",
                    "instance_id": ""
                })
            }
        }
    }
}

/// Generate a random string of given length
fn generate_random_string(length: usize) -> String {
    let mut rng = rand::thread_rng();
    
    iter::repeat(())
        .map(|()| rng.sample(Alphanumeric) as char)
        .take(length)
        .collect()
}

/// Generate a random hexadecimal string of given length
fn generate_random_hex(length: usize) -> String {
    let mut rng = rand::thread_rng();
    
    iter::repeat(())
        .map(|()| {
            let idx = rng.gen_range(0..16);
            char::from_digit(idx, 16).unwrap()
        })
        .take(length)
        .collect()
} 