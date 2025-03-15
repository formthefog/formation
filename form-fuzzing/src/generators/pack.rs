// form-fuzzing/src/generators/pack.rs
//! Generators for Pack Manager and Image Builder fuzzing

use crate::generators::Generator;
use crate::harness::pack::{Formfile, BuildStatus, DeploymentStatus};

use rand::{Rng, distributions::Alphanumeric, thread_rng, seq::SliceRandom};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

/// API Key Generator
pub struct ApiKeyGenerator;

impl ApiKeyGenerator {
    /// Create a new API key generator
    pub fn new() -> Self {
        Self
    }
}

impl Generator<(String, String)> for ApiKeyGenerator {
    fn generate(&self) -> (String, String) {
        let mut rng = rand::thread_rng();
        
        // Generate user ID (like an Ethereum address)
        let user_id = format!("0x{}", generate_random_hex(40));
        
        // Generate API key
        let api_key = format!("apk_{}", generate_random_string(32));
        
        (user_id, api_key)
    }
}

/// Generator for invalid API keys
pub struct InvalidApiKeyGenerator;

impl InvalidApiKeyGenerator {
    /// Create a new invalid API key generator
    pub fn new() -> Self {
        Self
    }
}

impl Generator<(String, String)> for InvalidApiKeyGenerator {
    fn generate(&self) -> (String, String) {
        let mut rng = rand::thread_rng();
        
        // Generate invalid user ID
        let user_id = if rng.gen_bool(0.5) {
            // Valid format but nonexistent
            format!("0x{}", generate_random_hex(40))
        } else {
            // Invalid format
            generate_random_string(10)
        };
        
        // Generate invalid API key
        let api_key = if rng.gen_bool(0.5) {
            // Valid format but nonexistent
            format!("apk_{}", generate_random_string(32))
        } else {
            // Invalid format
            generate_random_string(10)
        };
        
        (user_id, api_key)
    }
}

/// Generator for build IDs
pub struct BuildIdGenerator;

impl BuildIdGenerator {
    /// Create a new build ID generator
    pub fn new() -> Self {
        Self
    }
}

impl Generator<String> for BuildIdGenerator {
    fn generate(&self) -> String {
        format!("build-{}", uuid::Uuid::new_v4())
    }
}

/// Generator for VM IDs
pub struct VmIdGenerator;

impl VmIdGenerator {
    /// Create a new VM ID generator
    pub fn new() -> Self {
        Self
    }
}

impl Generator<String> for VmIdGenerator {
    fn generate(&self) -> String {
        format!("vm-{}", uuid::Uuid::new_v4())
    }
}

/// Generator for deployment IDs
pub struct DeploymentIdGenerator;

impl DeploymentIdGenerator {
    /// Create a new deployment ID generator
    pub fn new() -> Self {
        Self
    }
}

impl Generator<String> for DeploymentIdGenerator {
    fn generate(&self) -> String {
        format!("deploy-{}", uuid::Uuid::new_v4())
    }
}

/// Valid Formfile Generator
pub struct ValidFormfileGenerator;

impl ValidFormfileGenerator {
    /// Create a new ValidFormfile generator
    pub fn new() -> Self {
        Self
    }
    
    /// Generate a random base image
    pub fn generate_base_image(&self) -> String {
        let mut rng = rand::thread_rng();
        let bases = [
            "alpine:latest",
            "ubuntu:20.04",
            "debian:bullseye",
            "fedora:latest",
            "amazonlinux:2",
            "python:3.9",
            "ruby:3.0",
            "node:16",
            "golang:1.17",
            "rust:1.56",
        ];
        
        bases.choose(&mut rng).unwrap().to_string()
    }
    
    /// Generate random run commands
    pub fn generate_run_commands(&self) -> Vec<String> {
        let mut rng = rand::thread_rng();
        let mut commands = Vec::new();
        
        let command_count = rng.gen_range(1..10);
        
        for _ in 0..command_count {
            let cmd = match rng.gen_range(0..10) {
                0 => "apt-get update && apt-get install -y curl wget".to_string(),
                1 => "pip install requests".to_string(),
                2 => "echo 'hello world' > /app/hello.txt".to_string(),
                3 => "mkdir -p /data".to_string(),
                4 => "apk add --no-cache curl".to_string(),
                5 => "npm install express".to_string(),
                6 => "cargo build --release".to_string(),
                7 => "go build main.go".to_string(),
                8 => "chmod +x /app/start.sh".to_string(),
                _ => "echo 'Custom command'".to_string(),
            };
            
            commands.push(cmd);
        }
        
        commands
    }
    
    /// Generate environment variables
    pub fn generate_env_vars(&self) -> HashMap<String, String> {
        let mut rng = rand::thread_rng();
        let mut env_vars = HashMap::new();
        
        let env_count = rng.gen_range(0..5);
        
        for _ in 0..env_count {
            let key = match rng.gen_range(0..10) {
                0 => "DEBUG".to_string(),
                1 => "LOG_LEVEL".to_string(),
                2 => "APP_ENV".to_string(),
                3 => "API_KEY".to_string(),
                4 => "PORT".to_string(),
                5 => "DATABASE_URL".to_string(),
                6 => "REDIS_URL".to_string(),
                7 => "APP_SECRET".to_string(),
                8 => "NODE_ENV".to_string(),
                _ => "CUSTOM_VAR".to_string(),
            };
            
            let value = match key.as_str() {
                "DEBUG" => "true".to_string(),
                "LOG_LEVEL" => "info".to_string(),
                "APP_ENV" => "production".to_string(),
                "API_KEY" => format!("key_{}", Uuid::new_v4().simple()),
                "PORT" => "8080".to_string(),
                "DATABASE_URL" => "postgres://user:pass@localhost:5432/db".to_string(),
                "REDIS_URL" => "redis://localhost:6379".to_string(),
                "APP_SECRET" => Uuid::new_v4().to_string(),
                "NODE_ENV" => "production".to_string(),
                _ => "value".to_string(),
            };
            
            env_vars.insert(key, value);
        }
        
        env_vars
    }
    
    /// Generate a valid Formfile
    pub fn generate_formfile(&self) -> Formfile {
        let base_image = self.generate_base_image();
        let run_commands = self.generate_run_commands();
        let env_vars = self.generate_env_vars();
        
        // Create a basic valid formfile
        Formfile {
            base_image,
            run_commands,
            env_vars,
            resources: None,
            network: None, 
            exposed_ports: Vec::new(),
            users: Vec::new(),
            entrypoint: None,
        }
    }
}

/// Implement Generator trait for ValidFormfileGenerator
impl Generator<Formfile> for ValidFormfileGenerator {
    fn generate(&self) -> Formfile {
        self.generate_formfile()
    }
}

/// Invalid Formfile Generator
pub struct InvalidFormfileGenerator {
    valid_generator: ValidFormfileGenerator,
}

impl InvalidFormfileGenerator {
    /// Create a new InvalidFormfile generator
    pub fn new() -> Self {
        Self {
            valid_generator: ValidFormfileGenerator::new(),
        }
    }
    
    /// Generate an invalid Formfile
    pub fn generate(&self) -> Formfile {
        let mut rng = rand::thread_rng();
        let mut formfile = self.valid_generator.generate_formfile();
        
        // Choose one way to make it invalid
        match rng.gen_range(0..5) {
            0 => {
                // Empty base image
                formfile.base_image = "".to_string();
            },
            1 => {
                // No run commands
                formfile.run_commands = vec![];
            },
            2 => {
                // Invalid resources
                if let Some(resources) = &mut formfile.resources {
                    resources.vcpus = 0;
                }
            },
            3 => {
                // Invalid network configuration
                if let Some(network) = &mut formfile.network {
                    network.external_networks = vec!["invalid/network".to_string()];
                }
            },
            4 => {
                // Risky/dangerous command
                formfile.run_commands = vec!["rm -rf /".to_string()];
            },
            _ => {
                // Default case, make base image invalid
                formfile.base_image = "nonexistent:taggg".to_string();
            }
        }
        
        formfile
    }
}

/// Implement Generator trait for InvalidFormfileGenerator
impl Generator<Formfile> for InvalidFormfileGenerator {
    fn generate(&self) -> Formfile {
        self.generate()
    }
}

/// BuildInfo Generator
pub struct BuildInfoGenerator;

impl BuildInfoGenerator {
    /// Create a new BuildInfo generator
    pub fn new() -> Self {
        Self
    }
    
    /// Generate a build ID and status
    pub fn generate(&self) -> (String, String, BuildStatus) {
        let mut rng = rand::thread_rng();
        
        let build_id = format!("build_{}", Uuid::new_v4().simple());
        let user_id = format!("user_{}", Uuid::new_v4().simple());
        
        let status = match rng.gen_range(0..5) {
            0 => BuildStatus::Queued,
            1 => BuildStatus::InProgress,
            2 => BuildStatus::Completed,
            3 => BuildStatus::Failed,
            _ => BuildStatus::Cancelled,
        };
        
        (build_id, user_id, status)
    }
}

/// DeploymentInfo Generator
pub struct DeploymentInfoGenerator {
    build_info_generator: BuildInfoGenerator,
}

impl DeploymentInfoGenerator {
    /// Create a new DeploymentInfo generator
    pub fn new() -> Self {
        Self {
            build_info_generator: BuildInfoGenerator::new(),
        }
    }
    
    /// Generate a deployment ID and status
    pub fn generate(&self) -> (String, String, String, DeploymentStatus) {
        let mut rng = rand::thread_rng();
        
        let (build_id, user_id, _) = self.build_info_generator.generate();
        let deployment_id = format!("deploy_{}", Uuid::new_v4().simple());
        let vm_id = format!("vm_{}", Uuid::new_v4().simple());
        
        let status = match rng.gen_range(0..5) {
            0 => DeploymentStatus::Queued,
            1 => DeploymentStatus::InProgress,
            2 => DeploymentStatus::Completed,
            3 => DeploymentStatus::Failed,
            _ => DeploymentStatus::Cancelled,
        };
        
        (deployment_id, build_id, vm_id, status)
    }
}

/// Generate a random string of specified length
pub fn generate_random_string(length: usize) -> String {
    let mut rng = rand::thread_rng();
    (0..length)
        .map(|_| rng.sample(Alphanumeric) as char)
        .collect()
}

/// Generate a random hexadecimal string of specified length
pub fn generate_random_hex(length: usize) -> String {
    let mut rng = rand::thread_rng();
    let hex_chars = b"0123456789abcdef";
    (0..length)
        .map(|_| char::from(hex_chars[rng.gen_range(0..16)]))
        .collect()
}

/// Generate a random base64 string of specified length
pub fn generate_random_base64(length: usize) -> String {
    let mut rng = rand::thread_rng();
    let base64_chars = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    (0..length)
        .map(|_| char::from(base64_chars[rng.gen_range(0..64)]))
        .collect()
}