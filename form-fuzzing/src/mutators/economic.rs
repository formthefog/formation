// form-fuzzing/src/mutators/economic.rs

//! Mutators for Economic Infrastructure fuzzing

use crate::harness::economic::{ResourceType, ResourceThreshold};
use crate::generators::economic::{AuthToken, ApiKey, ResourceUsageReport};
use crate::mutators::Mutator;

use std::collections::HashMap;
use rand::{Rng, thread_rng, seq::SliceRandom};

/// Mutator for authentication tokens
pub struct AuthTokenMutator;

impl AuthTokenMutator {
    /// Create a new auth token mutator
    pub fn new() -> Self {
        Self
    }
}

impl Mutator<AuthToken> for AuthTokenMutator {
    fn mutate(&self, token: &mut AuthToken) {
        let mut rng = rand::thread_rng();
        
        // Choose what to mutate
        let mutation_type = rng.gen_range(0..3);
        
        match mutation_type {
            0 => {
                // Mutate user ID
                if token.user_id.starts_with("0x") {
                    // Remove 0x prefix
                    token.user_id = token.user_id[2..].to_string();
                } else {
                    // Add invalid prefix
                    token.user_id = format!("x0{}", token.user_id);
                }
            },
            1 => {
                // Mutate token - truncate it
                if token.token.len() > 10 {
                    token.token = token.token[0..rng.gen_range(5..10)].to_string();
                } else {
                    // Or make it invalid
                    token.token = "invalid".to_string();
                }
            },
            2 => {
                // Make token empty
                token.token = "".to_string();
            },
            _ => {}
        }
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

impl Mutator<ApiKey> for ApiKeyMutator {
    fn mutate(&self, api_key: &mut ApiKey) {
        let mut rng = rand::thread_rng();
        
        // Choose what to mutate
        let mutation_type = rng.gen_range(0..3);
        
        match mutation_type {
            0 => {
                // Mutate user ID
                if api_key.user_id.starts_with("0x") {
                    // Remove 0x prefix
                    api_key.user_id = api_key.user_id[2..].to_string();
                } else {
                    // Add invalid prefix
                    api_key.user_id = format!("x0{}", api_key.user_id);
                }
            },
            1 => {
                // Mutate API key - truncate it
                if api_key.api_key.len() > 10 {
                    api_key.api_key = api_key.api_key[0..rng.gen_range(5..10)].to_string();
                } else {
                    // Or make it invalid
                    api_key.api_key = "invalid".to_string();
                }
            },
            2 => {
                // Make API key empty
                api_key.api_key = "".to_string();
            },
            _ => {}
        }
    }
}

/// Mutator for resource usage reports
pub struct ResourceUsageReportMutator;

impl ResourceUsageReportMutator {
    /// Create a new resource usage report mutator
    pub fn new() -> Self {
        Self
    }
}

impl Mutator<ResourceUsageReport> for ResourceUsageReportMutator {
    fn mutate(&self, report: &mut ResourceUsageReport) {
        let mut rng = rand::thread_rng();
        
        // Choose what to mutate
        let mutation_type = rng.gen_range(0..4);
        
        match mutation_type {
            0 => {
                // Mutate VM ID
                report.vm_id = if rng.gen_bool(0.5) {
                    // Empty VM ID
                    "".to_string()
                } else {
                    // Invalid VM ID format
                    format!("invalid-vm-{}", rng.gen::<u32>())
                };
            },
            1 => {
                // Add extreme resource values
                if let Some(resource_type) = [
                    ResourceType::CPU,
                    ResourceType::Memory,
                    ResourceType::Storage,
                    ResourceType::NetworkIn,
                    ResourceType::NetworkOut,
                    ResourceType::GPU,
                ].choose(&mut rng) {
                    // Choose between negative, zero, extremely large, or NaN values
                    let value_type = rng.gen_range(0..4);
                    
                    let value = match value_type {
                        0 => -1.0 * rng.gen_range(1.0..100.0), // Negative value
                        1 => 0.0, // Zero
                        2 => rng.gen_range(1_000_000.0..f64::MAX / 2.0), // Extremely large
                        _ => f64::NAN, // NaN (not a number)
                    };
                    
                    report.resources.insert(resource_type.clone(), value);
                }
            },
            2 => {
                // Remove a resource
                if !report.resources.is_empty() {
                    let resource_types: Vec<ResourceType> = report.resources.keys().cloned().collect();
                    if let Some(resource_type) = resource_types.choose(&mut rng) {
                        report.resources.remove(resource_type);
                    }
                }
            },
            3 => {
                // Clear all resources (empty report)
                report.resources.clear();
            },
            _ => {}
        }
    }
}

/// Mutator for resource thresholds
pub struct ResourceThresholdMutator;

impl ResourceThresholdMutator {
    /// Create a new resource threshold mutator
    pub fn new() -> Self {
        Self
    }
}

impl Mutator<ResourceThreshold> for ResourceThresholdMutator {
    fn mutate(&self, threshold: &mut ResourceThreshold) {
        let mut rng = rand::thread_rng();
        
        // Choose what to mutate
        let mutation_type = rng.gen_range(0..4);
        
        match mutation_type {
            0 => {
                // Set warning threshold to negative value
                threshold.warning_threshold = -1.0 * rng.gen_range(1.0..100.0);
            },
            1 => {
                // Set critical threshold to negative value
                threshold.critical_threshold = -1.0 * rng.gen_range(1.0..100.0);
            },
            2 => {
                // Set warning threshold higher than critical
                let temp = threshold.warning_threshold;
                threshold.warning_threshold = threshold.critical_threshold;
                threshold.critical_threshold = temp;
                if threshold.warning_threshold == threshold.critical_threshold {
                    threshold.warning_threshold += 10.0;
                }
            },
            3 => {
                // Set extreme values for both thresholds
                threshold.warning_threshold = rng.gen_range(1_000_000.0..10_000_000.0);
                threshold.critical_threshold = rng.gen_range(10_000_000.0..100_000_000.0);
            },
            _ => {}
        }
    }
}

/// Mutator for webhook URLs
pub struct WebhookUrlMutator;

impl WebhookUrlMutator {
    /// Create a new webhook URL mutator
    pub fn new() -> Self {
        Self
    }
}

impl Mutator<String> for WebhookUrlMutator {
    fn mutate(&self, url: &mut String) {
        let mut rng = rand::thread_rng();
        
        // Choose what to mutate
        let mutation_type = rng.gen_range(0..4);
        
        match mutation_type {
            0 => {
                // Remove protocol
                if url.starts_with("http://") {
                    *url = url[7..].to_string();
                } else if url.starts_with("https://") {
                    *url = url[8..].to_string();
                }
            },
            1 => {
                // Change protocol to invalid
                if url.starts_with("http://") {
                    *url = format!("ftp://{}", &url[7..]);
                } else if url.starts_with("https://") {
                    *url = format!("file://{}", &url[8..]);
                } else {
                    *url = format!("invalid://{}", url);
                }
            },
            2 => {
                // Add very long path
                *url = format!("{}{}",
                              url,
                              "/".repeat(50) + &"a".repeat(100));
            },
            3 => {
                // Add invalid characters
                *url = format!("{}{}", url, "!@#$%^&*()?><");
            },
            _ => {}
        }
    }
}

/// Resource usage map mutator
pub struct ResourceMapMutator;

impl ResourceMapMutator {
    /// Create a new resource map mutator
    pub fn new() -> Self {
        Self
    }
}

impl Mutator<HashMap<ResourceType, f64>> for ResourceMapMutator {
    fn mutate(&self, resources: &mut HashMap<ResourceType, f64>) {
        let mut rng = rand::thread_rng();
        
        // Choose what to mutate
        let mutation_type = rng.gen_range(0..3);
        
        match mutation_type {
            0 => {
                // Set a resource to an extreme value
                if !resources.is_empty() {
                    let resource_types: Vec<ResourceType> = resources.keys().cloned().collect();
                    if let Some(resource_type) = resource_types.choose(&mut rng) {
                        let value_type = rng.gen_range(0..3);
                        
                        let value = match value_type {
                            0 => -1.0 * rng.gen_range(1.0..100.0), // Negative value
                            1 => f64::MAX / 2.0, // Extremely large
                            _ => f64::NAN, // NaN (not a number)
                        };
                        
                        resources.insert(resource_type.clone(), value);
                    }
                }
            },
            1 => {
                // Remove all resources
                resources.clear();
            },
            2 => {
                // Add all possible resource types with extreme values
                resources.insert(ResourceType::CPU, f64::MAX / 3.0);
                resources.insert(ResourceType::Memory, f64::MAX / 3.0);
                resources.insert(ResourceType::Storage, f64::MAX / 3.0);
                resources.insert(ResourceType::NetworkIn, f64::MAX / 3.0);
                resources.insert(ResourceType::NetworkOut, f64::MAX / 3.0);
                resources.insert(ResourceType::GPU, f64::MAX / 3.0);
            },
            _ => {}
        }
    }
} 