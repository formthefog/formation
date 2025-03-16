// form-fuzzing/src/generators/economic.rs

//! Generators for Economic Infrastructure fuzzing

use crate::generators::Generator;
use crate::harness::economic::{ResourceType, ResourceThreshold, EventDeliveryStatus};

use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

use rand::{Rng, seq::SliceRandom};
use rand::distributions::{Alphanumeric, Distribution};
use serde::{Deserialize, Serialize};

/// Authentication token for API calls
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthToken {
    /// User ID
    pub user_id: String,
    /// JWT token
    pub token: String,
}

/// API key for system operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKey {
    /// User ID
    pub user_id: String,
    /// API key
    pub api_key: String,
}

/// Resource usage report for a VM
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceUsageReport {
    /// VM ID
    pub vm_id: String,
    /// Resource usage measurements
    pub resources: HashMap<ResourceType, f64>,
}

/// Generator for auth tokens
pub struct AuthTokenGenerator;

impl AuthTokenGenerator {
    /// Create a new auth token generator
    pub fn new() -> Self {
        Self
    }
}

impl Generator<AuthToken> for AuthTokenGenerator {
    fn generate(&self) -> AuthToken {
        let mut rng = rand::thread_rng();
        
        // Generate a random user ID (Ethereum-style address)
        let user_id = format!("0x{}", generate_random_hex(40));
        
        // Generate a random JWT token
        let token = format!("eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.{}",
                           generate_random_string(64));
        
        AuthToken {
            user_id,
            token,
        }
    }
}

/// Generator for API keys
pub struct ApiKeyGenerator;

impl ApiKeyGenerator {
    /// Create a new API key generator
    pub fn new() -> Self {
        Self
    }
}

impl Generator<ApiKey> for ApiKeyGenerator {
    fn generate(&self) -> ApiKey {
        let mut rng = rand::thread_rng();
        
        // Generate a random user ID (Ethereum-style address)
        let user_id = format!("0x{}", generate_random_hex(40));
        
        // Generate a random API key
        let api_key = format!("api_key_{}", generate_random_string(32));
        
        ApiKey {
            user_id,
            api_key,
        }
    }
}

/// Generator for invalid auth tokens
pub struct InvalidAuthTokenGenerator;

impl InvalidAuthTokenGenerator {
    /// Create a new invalid auth token generator
    pub fn new() -> Self {
        Self
    }
}

impl Generator<AuthToken> for InvalidAuthTokenGenerator {
    fn generate(&self) -> AuthToken {
        let mut rng = rand::thread_rng();
        
        // Generate a random user ID (Ethereum-style address)
        let user_id = format!("0x{}", generate_random_hex(40));
        
        // Generate an invalid JWT token (missing parts, invalid format, etc.)
        let token_type = rng.gen_range(0..4);
        let token = match token_type {
            0 => "invalid_token".to_string(),
            1 => format!("eyJ{}", generate_random_string(10)), // Incomplete JWT
            2 => generate_random_string(50), // Random string
            _ => "".to_string(), // Empty string
        };
        
        AuthToken {
            user_id,
            token,
        }
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

impl Generator<ApiKey> for InvalidApiKeyGenerator {
    fn generate(&self) -> ApiKey {
        let mut rng = rand::thread_rng();
        
        // Generate a random user ID (Ethereum-style address)
        let user_id = format!("0x{}", generate_random_hex(40));
        
        // Generate an invalid API key
        let api_key_type = rng.gen_range(0..3);
        let api_key = match api_key_type {
            0 => "invalid_key".to_string(),
            1 => generate_random_string(5), // Too short
            _ => "".to_string(), // Empty string
        };
        
        ApiKey {
            user_id,
            api_key,
        }
    }
}

/// Generator for VM IDs
pub struct VMIdGenerator;

impl VMIdGenerator {
    /// Create a new VM ID generator
    pub fn new() -> Self {
        Self
    }
}

impl Generator<String> for VMIdGenerator {
    fn generate(&self) -> String {
        let mut rng = rand::thread_rng();
        
        // Generate a timestamp for uniqueness
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        
        // Generate a random VM ID
        format!("vm-{}-{}", now, generate_random_string(8))
    }
}

/// Generator for resource usage reports
pub struct ResourceUsageReportGenerator {
    /// Minimum number of resource types to include
    pub min_resources: usize,
    /// Maximum number of resource types to include
    pub max_resources: usize,
}

impl ResourceUsageReportGenerator {
    /// Create a new resource usage report generator
    pub fn new() -> Self {
        Self {
            min_resources: 1,
            max_resources: 6,
        }
    }
    
    /// Set minimum and maximum number of resource types to include
    pub fn with_resource_range(mut self, min: usize, max: usize) -> Self {
        self.min_resources = min;
        self.max_resources = max;
        self
    }
}

impl Generator<ResourceUsageReport> for ResourceUsageReportGenerator {
    fn generate(&self) -> ResourceUsageReport {
        let mut rng = rand::thread_rng();
        
        // Generate a random VM ID
        let vm_id_generator = VMIdGenerator::new();
        let vm_id = vm_id_generator.generate();
        
        // Generate resource usage map
        let mut resources = HashMap::new();
        
        // All possible resource types
        let all_resource_types = vec![
            ResourceType::CPU,
            ResourceType::Memory,
            ResourceType::Storage,
            ResourceType::NetworkIn,
            ResourceType::NetworkOut,
            ResourceType::GPU,
        ];
        
        // Decide how many resource types to include
        let num_resources = rng.gen_range(self.min_resources..=self.max_resources);
        
        // Select random resource types
        let selected_types: Vec<ResourceType> = all_resource_types
            .choose_multiple(&mut rng, num_resources)
            .cloned()
            .collect();
        
        // Generate values for each selected resource type
        for resource_type in selected_types {
            let value = match resource_type {
                ResourceType::CPU => rng.gen_range(0.0..100.0),
                ResourceType::Memory => rng.gen_range(0.0..32768.0), // Up to 32GB
                ResourceType::Storage => rng.gen_range(0.0..1000.0), // Up to 1TB
                ResourceType::NetworkIn => rng.gen_range(0.0..2000.0), // Up to 2GB
                ResourceType::NetworkOut => rng.gen_range(0.0..2000.0), // Up to 2GB
                ResourceType::GPU => rng.gen_range(0.0..100.0),
            };
            
            resources.insert(resource_type, value);
        }
        
        ResourceUsageReport {
            vm_id,
            resources,
        }
    }
}

/// Generator for high resource usage reports (above warning thresholds)
pub struct HighResourceUsageReportGenerator;

impl HighResourceUsageReportGenerator {
    /// Create a new high resource usage report generator
    pub fn new() -> Self {
        Self
    }
}

impl Generator<ResourceUsageReport> for HighResourceUsageReportGenerator {
    fn generate(&self) -> ResourceUsageReport {
        let mut rng = rand::thread_rng();
        
        // Generate a random VM ID
        let vm_id_generator = VMIdGenerator::new();
        let vm_id = vm_id_generator.generate();
        
        // Generate resource usage map with high values
        let mut resources = HashMap::new();
        
        // Add CPU usage (above warning threshold)
        resources.insert(ResourceType::CPU, rng.gen_range(80.0..95.0)); // Between warning and critical
        
        // Decide whether to add more resources
        if rng.gen_bool(0.8) {
            resources.insert(ResourceType::Memory, rng.gen_range(80.0..95.0)); // Between warning and critical
        }
        
        if rng.gen_bool(0.6) {
            resources.insert(ResourceType::Storage, rng.gen_range(85.0..95.0)); // Between warning and critical
        }
        
        if rng.gen_bool(0.4) {
            resources.insert(ResourceType::NetworkIn, rng.gen_range(800.0..1000.0)); // Between warning and critical
        }
        
        if rng.gen_bool(0.4) {
            resources.insert(ResourceType::NetworkOut, rng.gen_range(800.0..1000.0)); // Between warning and critical
        }
        
        if rng.gen_bool(0.3) {
            resources.insert(ResourceType::GPU, rng.gen_range(80.0..95.0)); // Between warning and critical
        }
        
        ResourceUsageReport {
            vm_id,
            resources,
        }
    }
}

/// Generator for critical resource usage reports (above critical thresholds)
pub struct CriticalResourceUsageReportGenerator;

impl CriticalResourceUsageReportGenerator {
    /// Create a new critical resource usage report generator
    pub fn new() -> Self {
        Self
    }
}

impl Generator<ResourceUsageReport> for CriticalResourceUsageReportGenerator {
    fn generate(&self) -> ResourceUsageReport {
        let mut rng = rand::thread_rng();
        
        // Generate a random VM ID
        let vm_id_generator = VMIdGenerator::new();
        let vm_id = vm_id_generator.generate();
        
        // Generate resource usage map with critical values
        let mut resources = HashMap::new();
        
        // Add CPU usage (above critical threshold)
        resources.insert(ResourceType::CPU, rng.gen_range(95.0..100.0)); // Above critical
        
        // Decide whether to add more resources
        if rng.gen_bool(0.8) {
            resources.insert(ResourceType::Memory, rng.gen_range(95.0..100.0)); // Above critical
        }
        
        if rng.gen_bool(0.6) {
            resources.insert(ResourceType::Storage, rng.gen_range(95.0..100.0)); // Above critical
        }
        
        if rng.gen_bool(0.4) {
            resources.insert(ResourceType::NetworkIn, rng.gen_range(1000.0..2000.0)); // Above critical
        }
        
        if rng.gen_bool(0.4) {
            resources.insert(ResourceType::NetworkOut, rng.gen_range(1000.0..2000.0)); // Above critical
        }
        
        if rng.gen_bool(0.3) {
            resources.insert(ResourceType::GPU, rng.gen_range(95.0..100.0)); // Above critical
        }
        
        ResourceUsageReport {
            vm_id,
            resources,
        }
    }
}

/// Generator for resource thresholds
pub struct ResourceThresholdGenerator;

impl ResourceThresholdGenerator {
    /// Create a new resource threshold generator
    pub fn new() -> Self {
        Self
    }
}

impl Generator<(ResourceType, ResourceThreshold)> for ResourceThresholdGenerator {
    fn generate(&self) -> (ResourceType, ResourceThreshold) {
        let mut rng = rand::thread_rng();
        
        // Select a random resource type
        let resource_types = vec![
            ResourceType::CPU,
            ResourceType::Memory,
            ResourceType::Storage,
            ResourceType::NetworkIn,
            ResourceType::NetworkOut,
            ResourceType::GPU,
        ];
        
        let resource_type = resource_types.choose(&mut rng).unwrap().clone();
        
        // Generate warning and critical thresholds
        let (warning, critical) = match resource_type {
            ResourceType::CPU | ResourceType::Memory | ResourceType::GPU => {
                let warning = rng.gen_range(50.0..80.0);
                let critical = rng.gen_range(warning + 10.0..95.0);
                (warning, critical)
            },
            ResourceType::Storage => {
                let warning = rng.gen_range(60.0..85.0);
                let critical = rng.gen_range(warning + 5.0..95.0);
                (warning, critical)
            },
            ResourceType::NetworkIn | ResourceType::NetworkOut => {
                let warning = rng.gen_range(500.0..800.0);
                let critical = rng.gen_range(warning + 100.0..1500.0);
                (warning, critical)
            },
        };
        
        // Decide if threshold is enabled
        let enabled = rng.gen_bool(0.9); // 90% chance of being enabled
        
        let threshold = ResourceThreshold {
            resource_type: resource_type.clone(),
            warning_threshold: warning,
            critical_threshold: critical,
            enabled,
        };
        
        (resource_type, threshold)
    }
}

/// Generator for webhook URLs
pub struct WebhookUrlGenerator;

impl WebhookUrlGenerator {
    /// Create a new webhook URL generator
    pub fn new() -> Self {
        Self
    }
}

impl Generator<String> for WebhookUrlGenerator {
    fn generate(&self) -> String {
        let mut rng = rand::thread_rng();
        
        // Generate a random domain and path
        let domain = format!("{}.example.com", generate_random_string(8));
        let path = format!("/webhooks/{}", generate_random_string(10));
        
        // Use HTTPS most of the time
        let protocol = if rng.gen_bool(0.8) {
            "https"
        } else {
            "http"
        };
        
        // Generate the full URL
        format!("{}://{}{}", protocol, domain, path)
    }
}

/// Generator for invalid webhook URLs
pub struct InvalidWebhookUrlGenerator;

impl InvalidWebhookUrlGenerator {
    /// Create a new invalid webhook URL generator
    pub fn new() -> Self {
        Self
    }
}

impl Generator<String> for InvalidWebhookUrlGenerator {
    fn generate(&self) -> String {
        let mut rng = rand::thread_rng();
        
        // Generate various types of invalid URLs
        let url_type = rng.gen_range(0..4);
        
        match url_type {
            0 => generate_random_string(20), // Random string, not a URL
            1 => format!("ftp://{}.example.com/webhook", generate_random_string(8)), // Wrong protocol
            2 => format!("/{}", generate_random_string(10)), // Missing protocol and domain
            _ => "".to_string(), // Empty string
        }
    }
}

/// Helper function to generate a random string of given length
fn generate_random_string(length: usize) -> String {
    let mut rng = rand::thread_rng();
    
    (0..length)
        .map(|_| rng.sample(Alphanumeric) as char)
        .collect()
}

/// Helper function to generate a random hexadecimal string
fn generate_random_hex(length: usize) -> String {
    let mut rng = rand::thread_rng();
    
    (0..length)
        .map(|_| {
            let hex_digit = rng.gen_range(0..16);
            let c = match hex_digit {
                0..=9 => (b'0' + hex_digit as u8) as char,
                _ => (b'a' + (hex_digit - 10) as u8) as char,
            };
            c
        })
        .collect()
} 