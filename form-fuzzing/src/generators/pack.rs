// form-fuzzing/src/generators/pack.rs
//! Generators for Pack Manager and Image Builder fuzzing

use crate::generators::Generator;
use crate::harness::pack::{Formfile, Resources, Network, User, GpuRequirement, BuildStatus, DeploymentStatus};

use rand::{Rng, distributions::Alphanumeric};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

/// Generator for API keys
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

/// Generator for valid Formfiles
pub struct FormfileGenerator {
    /// Include resource specifications
    include_resources: bool,
    /// Include network configuration
    include_network: bool,
    /// Include user configuration
    include_users: bool,
    /// Complexity level (0-3)
    complexity: usize,
}

impl FormfileGenerator {
    /// Create a new Formfile generator with default settings
    pub fn new() -> Self {
        Self {
            include_resources: true,
            include_network: true,
            include_users: true,
            complexity: 2,
        }
    }
    
    /// Set whether to include resource specifications
    pub fn with_resources(mut self, include: bool) -> Self {
        self.include_resources = include;
        self
    }
    
    /// Set whether to include network configuration
    pub fn with_network(mut self, include: bool) -> Self {
        self.include_network = include;
        self
    }
    
    /// Set whether to include user configuration
    pub fn with_users(mut self, include: bool) -> Self {
        self.include_users = include;
        self
    }
    
    /// Set complexity level (0-3)
    pub fn with_complexity(mut self, level: usize) -> Self {
        self.complexity = level.min(3);
        self
    }
    
    /// Generate a random user
    fn generate_user(&self) -> User {
        let mut rng = rand::thread_rng();
        
        let username = generate_random_string(8);
        let password = if rng.gen_bool(0.7) {
            Some(generate_random_string(12))
        } else {
            None
        };
        
        let sudo = if rng.gen_bool(0.5) {
            Some(true)
        } else {
            Some(false)
        };
        
        let ssh_authorized_keys = if rng.gen_bool(0.8) {
            Some(vec![format!("ssh-rsa {}", generate_random_base64(372))])
        } else {
            None
        };
        
        User {
            username,
            password,
            sudo,
            ssh_authorized_keys,
        }
    }
    
    /// Generate random resources
    fn generate_resources(&self) -> Resources {
        let mut rng = rand::thread_rng();
        
        // Decide what to include
        let include_vcpus = rng.gen_bool(0.9);
        let include_memory = rng.gen_bool(0.9);
        let include_disk = rng.gen_bool(0.8);
        let include_gpu = rng.gen_bool(0.3);
        
        // Generate valid values
        let vcpus = if include_vcpus {
            Some(rng.gen_range(1..=16))
        } else {
            None
        };
        
        let memory_mb = if include_memory {
            Some(rng.gen_range(512..=16384))
        } else {
            None
        };
        
        let disk_gb = if include_disk {
            Some(rng.gen_range(10..=500))
        } else {
            None
        };
        
        let gpu = if include_gpu {
            Some(GpuRequirement {
                model: ["nvidia-a100", "nvidia-t4", "nvidia-a10", "amd-mi100"]
                    .choose(&mut rng)
                    .unwrap()
                    .to_string(),
                count: rng.gen_range(1..=4),
            })
        } else {
            None
        };
        
        Resources {
            vcpus,
            memory_mb,
            disk_gb,
            gpu,
        }
    }
    
    /// Generate random network configuration
    fn generate_network(&self) -> Network {
        let mut rng = rand::thread_rng();
        
        let join_formnet = if rng.gen_bool(0.8) {
            Some(true)
        } else {
            Some(false)
        };
        
        let external_networks = if rng.gen_bool(0.4) {
            let count = rng.gen_range(1..=3);
            let networks = (0..count)
                .map(|_| {
                    let names = ["public", "private", "restricted", "internal", "dmz"];
                    names.choose(&mut rng).unwrap().to_string()
                })
                .collect();
            Some(networks)
        } else {
            None
        };
        
        Network {
            join_formnet,
            external_networks,
        }
    }
}

impl Generator<Formfile> for FormfileGenerator {
    fn generate(&self) -> Formfile {
        let mut rng = rand::thread_rng();
        
        // Generate name
        let name = if rng.gen_bool(0.9) {
            Some(format!("app-{}", generate_random_string(8)))
        } else {
            None
        };
        
        // Generate base image
        let from = if rng.gen_bool(0.99) { // Almost always include a base image
            let images = ["ubuntu:22.04", "ubuntu:20.04", "debian:11", "alpine:3.16"];
            Some(images.choose(&mut rng).unwrap().to_string())
        } else {
            None // Occasionally omit for error testing
        };
        
        // Generate run commands based on complexity
        let run = if self.complexity > 0 && rng.gen_bool(0.9) {
            let count = match self.complexity {
                0 => 0,
                1 => rng.gen_range(1..=2),
                2 => rng.gen_range(2..=5),
                _ => rng.gen_range(5..=10),
            };
            
            if count > 0 {
                let commands = [
                    "apt-get update",
                    "apt-get install -y python3",
                    "apt-get install -y nginx",
                    "apt-get install -y postgresql",
                    "pip install flask",
                    "pip install requests",
                    "mkdir -p /app/data",
                    "chmod 755 /app/data",
                    "echo 'Hello World' > /app/index.html",
                    "systemctl enable nginx",
                ];
                
                Some((0..count)
                    .map(|i| commands[i % commands.len()].to_string())
                    .collect())
            } else {
                None
            }
        } else {
            None
        };
        
        // Generate files to include
        let include = if self.complexity > 0 && rng.gen_bool(0.7) {
            let count = match self.complexity {
                0 => 0,
                1 => rng.gen_range(1..=2),
                2 => rng.gen_range(2..=5),
                _ => rng.gen_range(5..=10),
            };
            
            if count > 0 {
                let files = [
                    "app.py",
                    "requirements.txt",
                    "static/index.html",
                    "templates/base.html",
                    "config.json",
                    "data/seed.sql",
                    "scripts/setup.sh",
                    "Dockerfile",
                    "README.md",
                    ".env",
                ];
                
                Some((0..count)
                    .map(|i| files[i % files.len()].to_string())
                    .collect())
            } else {
                None
            }
        } else {
            None
        };
        
        // Generate environment variables
        let env = if self.complexity > 0 && rng.gen_bool(0.6) {
            let count = match self.complexity {
                0 => 0,
                1 => rng.gen_range(1..=2),
                2 => rng.gen_range(2..=5),
                _ => rng.gen_range(5..=10),
            };
            
            if count > 0 {
                let keys = [
                    "PORT",
                    "DEBUG",
                    "LOG_LEVEL",
                    "DB_HOST",
                    "DB_PORT",
                    "DB_USER",
                    "DB_PASSWORD",
                    "API_KEY",
                    "REDIS_URL",
                    "NODE_ENV",
                ];
                
                let values = [
                    "8080",
                    "false",
                    "info",
                    "localhost",
                    "5432",
                    "postgres",
                    "password123",
                    "a1b2c3d4e5f6",
                    "redis://localhost:6379",
                    "production",
                ];
                
                let mut env_map = HashMap::new();
                for i in 0..count {
                    env_map.insert(
                        keys[i % keys.len()].to_string(),
                        values[i % values.len()].to_string(),
                    );
                }
                
                Some(env_map)
            } else {
                None
            }
        } else {
            None
        };
        
        // Generate exposed ports
        let expose = if rng.gen_bool(0.7) {
            let count = rng.gen_range(1..=3);
            let ports = [80, 443, 8080, 3000, 5000, 8000, 8888, 9000];
            
            Some((0..count)
                .map(|i| ports[i % ports.len()])
                .collect())
        } else {
            None
        };
        
        // Generate entrypoint
        let entrypoint = if rng.gen_bool(0.6) {
            let entrypoints = [
                "python app.py",
                "node server.js",
                "nginx -g 'daemon off;'",
                "/usr/sbin/sshd -D",
                "./run.sh",
            ];
            
            Some(entrypoints.choose(&mut rng).unwrap().to_string())
        } else {
            None
        };
        
        // Generate resources if enabled
        let resources = if self.include_resources {
            Some(self.generate_resources())
        } else {
            None
        };
        
        // Generate network if enabled
        let network = if self.include_network {
            Some(self.generate_network())
        } else {
            None
        };
        
        // Generate working directory
        let workdir = if rng.gen_bool(0.7) {
            let dirs = ["/app", "/srv/www", "/var/www", "/opt/app", "/home/app"];
            Some(dirs.choose(&mut rng).unwrap().to_string())
        } else {
            None
        };
        
        // Generate users if enabled
        let users = if self.include_users && rng.gen_bool(0.7) {
            let count = rng.gen_range(1..=3);
            Some((0..count).map(|_| self.generate_user()).collect())
        } else {
            None
        };
        
        Formfile {
            name,
            from,
            run,
            include,
            env,
            expose,
            entrypoint,
            resources,
            network,
            workdir,
            users,
        }
    }
}

/// Generator for invalid Formfiles
pub struct InvalidFormfileGenerator;

impl InvalidFormfileGenerator {
    /// Create a new invalid Formfile generator
    pub fn new() -> Self {
        Self
    }
}

impl Generator<Formfile> for InvalidFormfileGenerator {
    fn generate(&self) -> Formfile {
        let mut rng = rand::thread_rng();
        
        // Start with a valid formfile
        let valid_generator = FormfileGenerator::new();
        let mut formfile = valid_generator.generate();
        
        // Choose an invalidation strategy
        let strategy = rng.gen_range(0..5);
        
        match strategy {
            0 => {
                // Missing required base image
                formfile.from = None;
            },
            1 => {
                // Invalid resource specifications
                if let Some(ref mut resources) = formfile.resources {
                    match rng.gen_range(0..4) {
                        0 => {
                            // Invalid vCPU count
                            resources.vcpus = Some(rng.gen_range(100..255));
                        },
                        1 => {
                            // Invalid memory (too small)
                            resources.memory_mb = Some(rng.gen_range(1..32));
                        },
                        2 => {
                            // Invalid disk (too large)
                            resources.disk_gb = Some(rng.gen_range(10000..20000));
                        },
                        3 => {
                            // Invalid GPU count
                            if let Some(ref mut gpu) = resources.gpu {
                                gpu.count = rng.gen_range(20..100);
                            } else {
                                resources.gpu = Some(GpuRequirement {
                                    model: "unknown-gpu".to_string(),
                                    count: rng.gen_range(20..100),
                                });
                            }
                        },
                        _ => {}
                    }
                } else {
                    // Create invalid resources
                    formfile.resources = Some(Resources {
                        vcpus: Some(0),
                        memory_mb: Some(1),
                        disk_gb: Some(0),
                        gpu: None,
                    });
                }
            },
            2 => {
                // Invalid user configuration
                if let Some(ref mut users) = formfile.users {
                    if !users.is_empty() {
                        match rng.gen_range(0..3) {
                            0 => {
                                // Empty username
                                users[0].username = "".to_string();
                            },
                            1 => {
                                // Username "root"
                                users[0].username = "root".to_string();
                            },
                            2 => {
                                // Invalid SSH key
                                users[0].ssh_authorized_keys = Some(vec!["invalid key".to_string()]);
                            },
                            _ => {}
                        }
                    } else {
                        // Add invalid user
                        users.push(User {
                            username: "".to_string(),
                            password: None,
                            sudo: None,
                            ssh_authorized_keys: None,
                        });
                    }
                } else {
                    // Create invalid user list
                    formfile.users = Some(vec![User {
                        username: "root".to_string(),
                        password: Some("password".to_string()),
                        sudo: Some(true),
                        ssh_authorized_keys: None,
                    }]);
                }
            },
            3 => {
                // Invalid network configuration
                if formfile.network.is_none() {
                    formfile.network = Some(Network {
                        join_formnet: Some(true),
                        external_networks: Some(vec!["invalid/network".to_string()]),
                    });
                } else if let Some(ref mut network) = formfile.network {
                    network.external_networks = Some(vec!["invalid/network".to_string()]);
                }
            },
            4 => {
                // Invalid command
                if formfile.run.is_none() {
                    formfile.run = Some(vec!["rm -rf /".to_string()]);
                } else if let Some(ref mut run) = formfile.run {
                    run.push("rm -rf /".to_string());
                }
            },
            _ => {}
        }
        
        formfile
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