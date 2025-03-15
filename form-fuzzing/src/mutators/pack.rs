// form-fuzzing/src/mutators/pack.rs
//! Mutators for Pack Manager and Image Builder fuzzing

use crate::harness::pack::{Formfile, Resources, NetworkConfig, User};
use crate::mutators::Mutator;
use rand::{thread_rng, Rng, seq::SliceRandom};
use std::collections::HashMap;

/// Mutator for Formfiles
pub struct FormfileMutator;

impl FormfileMutator {
    /// Create a new Formfile mutator
    pub fn new() -> Self {
        Self
    }
}

impl Mutator<Formfile> for FormfileMutator {
    fn mutate(&self, formfile: &mut Formfile) {
        let mut rng = rand::thread_rng();
        
        // Choose a random aspect to mutate
        let mutation = rng.gen_range(0..8);
        
        match mutation {
            0 => {
                // Mutate base image
                if rng.gen_bool(0.5) {
                    // Use a potentially invalid base image
                    formfile.base_image = format!("{}:{}", 
                        ["ubuntu", "debian", "alpine", "centos", "nonsense", "invalid"]
                            .choose(&mut rng)
                            .unwrap(),
                        rng.gen_range(1..50).to_string());
                } else {
                    // Empty base image
                    formfile.base_image = "".to_string();
                }
            },
            1 => {
                // Mutate run commands
                if rng.gen_bool(0.5) && !formfile.run_commands.is_empty() {
                    // Remove a command
                    let idx = rng.gen_range(0..formfile.run_commands.len());
                    formfile.run_commands.remove(idx);
                } else {
                    // Add an invalid command
                    formfile.run_commands.push(match rng.gen_range(0..3) {
                        0 => "invalid command".to_string(),
                        1 => "rm -rf /".to_string(), // Dangerous command
                        _ => "#!&%*@".to_string(), // Garbage
                    });
                }
            },
            2 => {
                // Mutate environment variables
                if rng.gen_bool(0.5) && !formfile.env_vars.is_empty() {
                    // Remove an env var
                    let keys: Vec<_> = formfile.env_vars.keys().cloned().collect();
                    let idx = rng.gen_range(0..keys.len());
                    formfile.env_vars.remove(&keys[idx]);
                } else {
                    // Add a problematic env var
                    let key = match rng.gen_range(0..3) {
                        0 => "PATH".to_string(),
                        1 => "LD_PRELOAD".to_string(),
                        _ => "SHELL".to_string(),
                    };
                    let value = match rng.gen_range(0..3) {
                        0 => "/dev/null".to_string(),
                        1 => "/etc/passwd".to_string(),
                        _ => "$(whoami)".to_string(), // Command injection
                    };
                    formfile.env_vars.insert(key, value);
                }
            },
            3 => {
                // Mutate resources
                if let Some(ref mut resources) = formfile.resources {
                    let resource_mutation = rng.gen_range(0..4);
                    match resource_mutation {
                        0 => {
                            // Mutate vCPUs
                            resources.vcpus = match rng.gen_range(0..3) {
                                0 => 0, // Invalid: zero vCPUs
                                1 => 255, // Invalid: too many vCPUs
                                _ => rng.gen_range(100..200), // Unusually high
                            };
                        },
                        1 => {
                            // Mutate memory
                            resources.memory_mb = match rng.gen_range(0..3) {
                                0 => 0, // Invalid: zero memory
                                1 => rng.gen_range(1..32), // Too little memory
                                _ => rng.gen_range(1024 * 1024 * 10..u32::MAX / 2), // Extremely high
                            };
                        },
                        2 => {
                            // Mutate disk
                            resources.disk_gb = match rng.gen_range(0..3) {
                                0 => 0, // Invalid: zero disk
                                1 => rng.gen_range(5000..10000), // Very large disk
                                _ => rng.gen_range(1..3), // Very small disk
                            };
                        },
                        3 => {
                            // Mutate GPU
                            if let Some(ref mut gpu) = resources.gpu {
                                // Turn it into an invalid GPU string
                                *gpu = format!("invalid-gpu-{}", rng.gen::<u16>());
                            } else {
                                // Add a GPU with no model
                                resources.gpu = Some("unknown".to_string());
                            }
                        },
                        _ => {}
                    }
                } else {
                    // Create an invalid resource specification
                    formfile.resources = Some(Resources {
                        vcpus: rng.gen_range(64..255),
                        memory_mb: rng.gen_range(1024 * 1024..u32::MAX / 2),
                        disk_gb: rng.gen_range(1000..5000),
                        gpu: Some("overpowered-gpu".to_string()),
                    });
                }
            },
            4 => {
                // Mutate network
                if let Some(ref mut network) = formfile.network {
                    let network_mutation = rng.gen_range(0..2);
                    match network_mutation {
                        0 => {
                            // Flip formation network joining
                            network.join_formnet = !network.join_formnet;
                        },
                        1 => {
                            // Add invalid external networks
                            let invalid_networks = [
                                "192.168.0.0/8", // Invalid CIDR
                                "public-internet", // Non-specific
                                "internal/network", // Invalid chars
                                "my_home_network", // Non-existent
                                "", // Empty string
                            ];
                            
                            network.external_networks = 
                                (0..rng.gen_range(1..5))
                                    .map(|_| invalid_networks.choose(&mut rng).unwrap().to_string())
                                    .collect();
                        },
                        _ => {}
                    }
                } else {
                    // Create an invalid network config
                    formfile.network = Some(NetworkConfig {
                        join_formnet: true,
                        external_networks: vec!["invalid-network".to_string()],
                    });
                }
            },
            5 => {
                // Mutate exposed ports
                if !formfile.exposed_ports.is_empty() {
                    if rng.gen_bool(0.5) {
                        // Remove a port
                        let idx = rng.gen_range(0..formfile.exposed_ports.len());
                        formfile.exposed_ports.remove(idx);
                    } else {
                        // Modify a port to an invalid or edge case value
                        let idx = rng.gen_range(0..formfile.exposed_ports.len());
                        formfile.exposed_ports[idx] = match rng.gen_range(0..3) {
                            0 => 0, // Invalid: port 0
                            1 => 1, // Reserved port
                            _ => 65535, // Max port
                        };
                    }
                } else {
                    // Add some invalid or edge case ports
                    formfile.exposed_ports = vec![0, 1, 65535]; // Mix of invalid and edge case ports
                }
            },
            6 => {
                // Mutate users
                if !formfile.users.is_empty() {
                    // Choose a user to mutate
                    let idx = rng.gen_range(0..formfile.users.len());
                    let user = &mut formfile.users[idx];
                    
                    // Choose what to mutate
                    let user_mutation = rng.gen_range(0..3);
                    match user_mutation {
                        0 => {
                            // Mutate the username (potentially to an invalid one)
                            user.username = match rng.gen_range(0..3) {
                                0 => "root".to_string(), // Not allowed
                                1 => "".to_string(), // Empty
                                _ => "user-with-invalid-chars!@#$%".to_string(),
                            };
                        },
                        1 => {
                            // Mutate password
                            // Make it an invalid or edge case password
                            user.password = if rng.gen_bool(0.5) {
                                // Very short
                                "x".to_string()
                            } else {
                                // Very long
                                "X".repeat(rng.gen_range(1000..10000))
                            };
                        },
                        2 => {
                            // Toggle sudo
                            user.sudo = !user.sudo;
                        },
                        _ => {}
                    }
                } else {
                    // Add an invalid user
                    formfile.users.push(User {
                        username: "root".to_string(), // Invalid: can't create root
                        password: "password".to_string(),
                        sudo: true,
                        ssh_authorized_keys: vec![],
                    });
                }
            },
            7 => {
                // Mutate entrypoint
                if let Some(ref mut entrypoint) = formfile.entrypoint {
                    // Make it invalid
                    *entrypoint = match rng.gen_range(0..3) {
                        0 => "".to_string(), // Empty
                        1 => "/nonexistent/path".to_string(), // Non-existent
                        _ => "/dev/null".to_string(), // Invalid binary
                    };
                } else {
                    // Add an entrypoint
                    formfile.entrypoint = Some("/bin/false".to_string()); // Will immediately exit
                }
            },
            _ => {}
        }
    }
}

/// Mutator for Resources
pub struct ResourcesMutator;

impl ResourcesMutator {
    /// Create a new resources mutator
    pub fn new() -> Self {
        Self
    }
}

impl Mutator<Resources> for ResourcesMutator {
    fn mutate(&self, resources: &mut Resources) {
        let mut rng = rand::thread_rng();
        
        // Choose a resource aspect to mutate
        let mutation = rng.gen_range(0..4);
        
        match mutation {
            0 => {
                // Mutate vCPUs
                resources.vcpus = match rng.gen_range(0..3) {
                    0 => 0, // Invalid: zero vCPUs
                    1 => 255, // Invalid: too many vCPUs
                    _ => rng.gen_range(100..200), // Unusually high
                };
            },
            1 => {
                // Mutate memory
                resources.memory_mb = match rng.gen_range(0..3) {
                    0 => 0, // Invalid: zero memory
                    1 => rng.gen_range(1..32), // Too little memory
                    _ => rng.gen_range(1024 * 1024 * 10..u32::MAX / 2), // Extremely high
                };
            },
            2 => {
                // Mutate disk
                resources.disk_gb = match rng.gen_range(0..3) {
                    0 => 0, // Invalid: zero disk
                    1 => rng.gen_range(5000..10000), // Very large disk
                    _ => rng.gen_range(1..3), // Very small disk
                };
            },
            3 => {
                // Mutate GPU
                if let Some(ref mut gpu) = resources.gpu {
                    // Turn it into an invalid GPU string
                    *gpu = format!("invalid-gpu-{}", rng.gen::<u16>());
                } else {
                    // Add a GPU
                    resources.gpu = Some("unknown-gpu".to_string());
                }
            },
            _ => {}
        }
    }
}

/// Network configuration mutator
pub struct NetworkMutator;

impl NetworkMutator {
    /// Create a new network mutator
    pub fn new() -> Self {
        Self
    }
}

impl Mutator<NetworkConfig> for NetworkMutator {
    fn mutate(&self, network: &mut NetworkConfig) {
        let mut rng = rand::thread_rng();
        
        // Choose a network aspect to mutate
        let mutation = rng.gen_range(0..2);
        
        match mutation {
            0 => {
                // Flip formation network joining
                network.join_formnet = !network.join_formnet;
            },
            1 => {
                // Modify external networks
                if !network.external_networks.is_empty() {
                    // Choose whether to remove or add invalid
                    if rng.gen_bool(0.3) {
                        // Remove one
                        let idx = rng.gen_range(0..network.external_networks.len());
                        network.external_networks.remove(idx);
                    } else {
                        // Replace with invalid
                        let invalid_networks = [
                            "192.168.0.0/8", // Invalid CIDR
                            "public-internet", // Non-specific
                            "internal/network", // Invalid chars
                            "my_home_network", // Non-existent
                            "", // Empty string
                        ];
                        
                        network.external_networks = 
                            (0..rng.gen_range(1..5))
                                .map(|_| invalid_networks.choose(&mut rng).unwrap().to_string())
                                .collect();
                    }
                } else {
                    // Add an invalid one
                    network.external_networks.push("invalid/network".to_string());
                }
            },
            _ => {}
        }
    }
}

/// User mutator
pub struct UserMutator;

impl UserMutator {
    /// Create a new user mutator
    pub fn new() -> Self {
        Self
    }
}

impl Mutator<User> for UserMutator {
    fn mutate(&self, user: &mut User) {
        let mut rng = rand::thread_rng();
        
        // Choose a user aspect to mutate
        let mutation = rng.gen_range(0..4);
        
        match mutation {
            0 => {
                // Mutate the username (potentially to an invalid one)
                user.username = match rng.gen_range(0..3) {
                    0 => "root".to_string(), // Not allowed
                    1 => "".to_string(), // Empty
                    _ => "user-with-invalid-chars!@#$%".to_string(),
                };
            },
            1 => {
                // Mutate password
                // Make it an invalid or edge case password
                user.password = if rng.gen_bool(0.5) {
                    // Very short
                    "x".to_string()
                } else {
                    // Very long
                    "X".repeat(rng.gen_range(1000..10000))
                };
            },
            2 => {
                // Toggle sudo
                user.sudo = !user.sudo;
            },
            3 => {
                // Mutate ssh keys
                if !user.ssh_authorized_keys.is_empty() {
                    if rng.gen_bool(0.5) {
                        // Remove a key
                        let idx = rng.gen_range(0..user.ssh_authorized_keys.len());
                        user.ssh_authorized_keys.remove(idx);
                    } else {
                        // Replace with invalid
                        user.ssh_authorized_keys = vec![
                            "not a valid ssh key".to_string(),
                            "ssh-rsa with no content".to_string(),
                        ];
                    }
                } else {
                    // Add invalid keys
                    user.ssh_authorized_keys.push("not a valid key".to_string());
                }
            },
            _ => {}
        }
    }
}

/// Mutator for build IDs
pub struct BuildIdMutator;

impl BuildIdMutator {
    /// Create a new build ID mutator
    pub fn new() -> Self {
        Self
    }
}

impl Mutator<String> for BuildIdMutator {
    fn mutate(&self, build_id: &mut String) {
        let mut rng = rand::thread_rng();
        
        *build_id = match rng.gen_range(0..4) {
            0 => "".to_string(), // Empty
            1 => "invalid-format".to_string(), // Invalid format
            2 => "build-".to_string(), // Incomplete
            _ => format!("build-{}", uuid::Uuid::new_v4()), // Valid but different
        };
    }
}

/// Mutator for VM IDs
pub struct VmIdMutator;

impl VmIdMutator {
    /// Create a new VM ID mutator
    pub fn new() -> Self {
        Self
    }
}

impl Mutator<String> for VmIdMutator {
    fn mutate(&self, vm_id: &mut String) {
        let mut rng = rand::thread_rng();
        
        *vm_id = match rng.gen_range(0..4) {
            0 => "".to_string(), // Empty
            1 => "invalid-format".to_string(), // Invalid format
            2 => "vm-".to_string(), // Incomplete
            _ => format!("vm-{}", uuid::Uuid::new_v4()), // Valid but different
        };
    }
}

/// Mutator for deployment IDs
pub struct DeploymentIdMutator;

impl DeploymentIdMutator {
    /// Create a new deployment ID mutator
    pub fn new() -> Self {
        Self
    }
}

impl Mutator<String> for DeploymentIdMutator {
    fn mutate(&self, deployment_id: &mut String) {
        let mut rng = rand::thread_rng();
        
        *deployment_id = match rng.gen_range(0..4) {
            0 => "".to_string(), // Empty
            1 => "invalid-format".to_string(), // Invalid format
            2 => "deploy-".to_string(), // Incomplete
            _ => format!("deploy-{}", uuid::Uuid::new_v4()), // Valid but different
        };
    }
}

/// Mutator for API keys
pub struct ApiKeyMutator;

impl ApiKeyMutator {
    /// Create a new API key mutator
    pub fn new() -> Self {
        Self
    }
}

impl Mutator<(String, String)> for ApiKeyMutator {
    fn mutate(&self, key_pair: &mut (String, String)) {
        let mut rng = rand::thread_rng();
        
        // Choose whether to mutate user ID or API key
        if rng.gen_bool(0.5) {
            // Mutate user ID
            key_pair.0 = match rng.gen_range(0..4) {
                0 => "".to_string(), // Empty
                1 => "invalid-format".to_string(), // Invalid format
                2 => "0x".to_string(), // Incomplete
                _ => format!("0x{}", "1234567890abcdef".repeat(3)), // Valid but different
            };
        } else {
            // Mutate API key
            key_pair.1 = match rng.gen_range(0..4) {
                0 => "".to_string(), // Empty
                1 => "invalid-format".to_string(), // Invalid format
                2 => "apk_".to_string(), // Incomplete
                _ => format!("apk_{}", "X".repeat(32)), // Valid but different
            };
        }
    }
} 
