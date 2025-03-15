// form-fuzzing/src/harness/economic.rs

//! Test harness for Economic Infrastructure testing

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use rand::Rng;

/// ResourceType represents different types of resources that can be metered
#[derive(Debug, Clone, Eq, Hash, PartialEq, Serialize, Deserialize)]
pub enum ResourceType {
    /// CPU usage in percentage (0-100 per core)
    CPU,
    /// Memory usage in MB
    Memory,
    /// Storage usage in GB
    Storage,
    /// Network ingress in MB
    NetworkIn,
    /// Network egress in MB
    NetworkOut,
    /// GPU compute units
    GPU,
}

/// Represents a resource usage measurement
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceUsage {
    /// Type of resource being measured
    pub resource_type: ResourceType,
    /// Value of the measurement in appropriate units
    pub value: f64,
    /// Timestamp of the measurement
    pub timestamp: u64,
    /// VM ID that this measurement is for
    pub vm_id: String,
    /// User ID that owns the VM
    pub user_id: String,
}

/// Represents a threshold configuration for resource usage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceThreshold {
    /// Type of resource being monitored
    pub resource_type: ResourceType,
    /// Value at which a warning should be triggered
    pub warning_threshold: f64,
    /// Value at which a critical alert should be triggered
    pub critical_threshold: f64,
    /// Whether this threshold is enabled
    pub enabled: bool,
}

/// Status for a usage event delivery
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum EventDeliveryStatus {
    /// Event was delivered successfully
    Delivered,
    /// Event delivery failed and will be retried
    Failed,
    /// Circuit breaker is open due to multiple failures
    CircuitBreakerOpen,
    /// Event was dropped due to excessive retries
    Dropped,
}

/// Represents a usage event that is emitted for billing purposes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageEvent {
    /// Unique ID for this event
    pub event_id: String,
    /// VM ID this event relates to
    pub vm_id: String,
    /// User ID that owns the VM
    pub user_id: String,
    /// Map of resource types to usage values
    pub resources: HashMap<ResourceType, f64>,
    /// Timestamp when the event was created
    pub timestamp: u64,
    /// Status of event delivery
    pub delivery_status: EventDeliveryStatus,
    /// Number of delivery attempts
    pub delivery_attempts: u8,
}

/// Represents a threshold violation event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThresholdEvent {
    /// Unique ID for this event
    pub event_id: String,
    /// VM ID this event relates to
    pub vm_id: String,
    /// User ID that owns the VM
    pub user_id: String,
    /// Type of resource that exceeded threshold
    pub resource_type: ResourceType,
    /// Current value of the resource
    pub current_value: f64,
    /// Threshold that was exceeded
    pub threshold_value: f64,
    /// Whether this is a warning or critical threshold
    pub is_critical: bool,
    /// Timestamp when the event was created
    pub timestamp: u64,
}

/// Result of an Economic Infrastructure operation
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum EconomicOperationResult {
    /// Operation succeeded
    Success(serde_json::Value),
    /// Authentication failed
    AuthenticationFailed,
    /// Permission denied
    PermissionDenied,
    /// Resource not found
    ResourceNotFound,
    /// Invalid input
    InvalidInput(String),
    /// Rate limited
    RateLimited,
    /// Operation failed
    OperationFailed(String),
    /// Internal error
    InternalError(String),
    /// Timeout
    Timeout,
}

/// Mock Economic Infrastructure service
pub struct MockEconomicService {
    /// Current resource usage for each VM
    vm_resources: Arc<Mutex<HashMap<String, HashMap<ResourceType, f64>>>>,
    /// History of usage events (limited buffer)
    usage_events: Arc<Mutex<Vec<UsageEvent>>>,
    /// History of threshold events (limited buffer)
    threshold_events: Arc<Mutex<Vec<ThresholdEvent>>>,
    /// Thresholds configured for resources
    thresholds: Arc<Mutex<HashMap<ResourceType, ResourceThreshold>>>,
    /// Registered webhooks for events
    webhooks: Arc<Mutex<Vec<String>>>,
    /// Rate limiting counters
    rate_limits: Arc<Mutex<HashMap<String, usize>>>,
    /// Authentication tokens
    tokens: Arc<Mutex<HashMap<String, String>>>,
    /// API keys
    api_keys: Arc<Mutex<HashMap<String, String>>>,
    /// Counter for generating IDs
    id_counter: Arc<Mutex<u64>>,
    /// Failure rate for simulating random failures
    failure_rate: f64,
    /// Circuit breaker status for event delivery
    circuit_breaker_open: Arc<Mutex<bool>>,
}

impl MockEconomicService {
    /// Create a new mock economic service
    pub fn new() -> Self {
        // Create default thresholds
        let mut default_thresholds = HashMap::new();
        
        default_thresholds.insert(ResourceType::CPU, ResourceThreshold {
            resource_type: ResourceType::CPU,
            warning_threshold: 80.0,
            critical_threshold: 95.0,
            enabled: true,
        });
        
        default_thresholds.insert(ResourceType::Memory, ResourceThreshold {
            resource_type: ResourceType::Memory,
            warning_threshold: 80.0,
            critical_threshold: 95.0,
            enabled: true,
        });
        
        default_thresholds.insert(ResourceType::Storage, ResourceThreshold {
            resource_type: ResourceType::Storage,
            warning_threshold: 85.0,
            critical_threshold: 95.0,
            enabled: true,
        });
        
        default_thresholds.insert(ResourceType::NetworkIn, ResourceThreshold {
            resource_type: ResourceType::NetworkIn,
            warning_threshold: 800.0, // 800 MB
            critical_threshold: 1000.0, // 1 GB
            enabled: true,
        });
        
        default_thresholds.insert(ResourceType::NetworkOut, ResourceThreshold {
            resource_type: ResourceType::NetworkOut,
            warning_threshold: 800.0, // 800 MB
            critical_threshold: 1000.0, // 1 GB
            enabled: true,
        });
        
        default_thresholds.insert(ResourceType::GPU, ResourceThreshold {
            resource_type: ResourceType::GPU,
            warning_threshold: 80.0,
            critical_threshold: 95.0,
            enabled: true,
        });
        
        Self {
            vm_resources: Arc::new(Mutex::new(HashMap::new())),
            usage_events: Arc::new(Mutex::new(Vec::new())),
            threshold_events: Arc::new(Mutex::new(Vec::new())),
            thresholds: Arc::new(Mutex::new(default_thresholds)),
            webhooks: Arc::new(Mutex::new(Vec::new())),
            rate_limits: Arc::new(Mutex::new(HashMap::new())),
            tokens: Arc::new(Mutex::new(HashMap::new())),
            api_keys: Arc::new(Mutex::new(HashMap::new())),
            id_counter: Arc::new(Mutex::new(0)),
            failure_rate: 0.05,
            circuit_breaker_open: Arc::new(Mutex::new(false)),
        }
    }
    
    /// Set the failure rate for simulating random failures
    pub fn set_failure_rate(&mut self, rate: f64) {
        self.failure_rate = rate;
    }
    
    /// Check if a JWT token is valid
    fn validate_token(&self, token: &str) -> Option<String> {
        let tokens = self.tokens.lock().unwrap();
        tokens.get(token).cloned()
    }
    
    /// Check if an API key is valid
    fn validate_api_key(&self, api_key: &str) -> Option<String> {
        let api_keys = self.api_keys.lock().unwrap();
        api_keys.get(api_key).cloned()
    }
    
    /// Check rate limits for a user/API
    fn check_rate_limit(&self, user_id: &str, max_ops: usize) -> bool {
        let mut rate_limits = self.rate_limits.lock().unwrap();
        let count = rate_limits.entry(user_id.to_string()).or_insert(0);
        *count += 1;
        *count <= max_ops
    }
    
    /// Generate a unique ID with a prefix
    fn generate_id(&self, prefix: &str) -> String {
        let mut counter = self.id_counter.lock().unwrap();
        *counter += 1;
        
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        
        format!("{}-{}-{}", prefix, now, *counter)
    }
    
    /// Register an API key for a user
    pub fn register_api_key(&self, user_id: &str, api_key: &str) -> EconomicOperationResult {
        let mut api_keys = self.api_keys.lock().unwrap();
        api_keys.insert(api_key.to_string(), user_id.to_string());
        
        EconomicOperationResult::Success(serde_json::json!({
            "status": "success",
            "message": "API key registered successfully",
            "api_key": api_key
        }))
    }
    
    /// Register a JWT token for a user
    pub fn register_token(&self, user_id: &str, token: &str) -> EconomicOperationResult {
        let mut tokens = self.tokens.lock().unwrap();
        tokens.insert(token.to_string(), user_id.to_string());
        
        EconomicOperationResult::Success(serde_json::json!({
            "status": "success",
            "message": "Token registered successfully"
        }))
    }
    
    /// Report resource usage for a VM
    pub fn report_resource_usage(&self, token: &str, vm_id: &str, resources: HashMap<ResourceType, f64>) -> EconomicOperationResult {
        // Validate authentication
        let user_id = match self.validate_token(token) {
            Some(id) => id,
            None => return EconomicOperationResult::AuthenticationFailed,
        };
        
        // Check rate limits
        if !self.check_rate_limit(&user_id, 100) {
            return EconomicOperationResult::RateLimited;
        }
        
        // Simulate random failures
        let mut rng = rand::thread_rng();
        if rng.gen::<f64>() < self.failure_rate {
            return EconomicOperationResult::InternalError("Random failure simulated".to_string());
        }
        
        // Get current timestamp
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        
        // Update VM resources
        let mut vm_resources = self.vm_resources.lock().unwrap();
        let vm_resource_map = vm_resources.entry(vm_id.to_string()).or_insert_with(HashMap::new);
        
        for (resource_type, value) in &resources {
            vm_resource_map.insert(resource_type.clone(), *value);
        }
        
        // Create usage event
        let event_id = self.generate_id("usage-event");
        let usage_event = UsageEvent {
            event_id,
            vm_id: vm_id.to_string(),
            user_id: user_id.clone(),
            resources: resources.clone(),
            timestamp: now,
            delivery_status: EventDeliveryStatus::Delivered,
            delivery_attempts: 1,
        };
        
        // Store event in buffer
        let mut usage_events = self.usage_events.lock().unwrap();
        usage_events.push(usage_event.clone());
        
        // Limit buffer size to latest 1000 events
        if usage_events.len() > 1000 {
            usage_events.remove(0);
        }
        
        // Check thresholds
        let threshold_events = self.check_thresholds(&user_id, vm_id, &resources, now);
        
        // Return success with events
        EconomicOperationResult::Success(serde_json::json!({
            "status": "success",
            "message": "Resource usage reported successfully",
            "threshold_violations": threshold_events.len(),
        }))
    }
    
    /// Check if any thresholds are exceeded
    fn check_thresholds(&self, user_id: &str, vm_id: &str, resources: &HashMap<ResourceType, f64>, timestamp: u64) -> Vec<ThresholdEvent> {
        let thresholds = self.thresholds.lock().unwrap();
        let mut threshold_events = Vec::new();
        
        for (resource_type, value) in resources {
            if let Some(threshold) = thresholds.get(resource_type) {
                if !threshold.enabled {
                    continue;
                }
                
                let mut is_critical = false;
                let mut threshold_value = 0.0;
                
                // Check if critical threshold is exceeded
                if *value >= threshold.critical_threshold {
                    is_critical = true;
                    threshold_value = threshold.critical_threshold;
                }
                // Check if warning threshold is exceeded
                else if *value >= threshold.warning_threshold {
                    threshold_value = threshold.warning_threshold;
                }
                // No threshold exceeded
                else {
                    continue;
                }
                
                // Create threshold event
                let event_id = self.generate_id("threshold-event");
                let threshold_event = ThresholdEvent {
                    event_id,
                    vm_id: vm_id.to_string(),
                    user_id: user_id.to_string(),
                    resource_type: resource_type.clone(),
                    current_value: *value,
                    threshold_value,
                    is_critical,
                    timestamp,
                };
                
                // Store event
                let mut threshold_events_buffer = self.threshold_events.lock().unwrap();
                threshold_events_buffer.push(threshold_event.clone());
                
                // Limit buffer size to latest 1000 events
                if threshold_events_buffer.len() > 1000 {
                    threshold_events_buffer.remove(0);
                }
                
                threshold_events.push(threshold_event);
            }
        }
        
        threshold_events
    }
    
    /// Get current resource usage for a VM
    pub fn get_vm_usage(&self, token: &str, vm_id: &str) -> EconomicOperationResult {
        // Validate authentication
        let user_id = match self.validate_token(token) {
            Some(id) => id,
            None => return EconomicOperationResult::AuthenticationFailed,
        };
        
        // Check rate limits
        if !self.check_rate_limit(&user_id, 300) {
            return EconomicOperationResult::RateLimited;
        }
        
        // Get VM resources
        let vm_resources = self.vm_resources.lock().unwrap();
        let vm_resource_map = match vm_resources.get(vm_id) {
            Some(map) => map.clone(),
            None => return EconomicOperationResult::ResourceNotFound,
        };
        
        // Return success with resources
        EconomicOperationResult::Success(serde_json::json!({
            "vm_id": vm_id,
            "resources": vm_resource_map,
            "timestamp": SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        }))
    }
    
    /// Get recent usage events for a user
    pub fn get_recent_usage_events(&self, token: &str, limit: Option<usize>) -> EconomicOperationResult {
        // Validate authentication
        let user_id = match self.validate_token(token) {
            Some(id) => id,
            None => return EconomicOperationResult::AuthenticationFailed,
        };
        
        // Check rate limits
        if !self.check_rate_limit(&user_id, 100) {
            return EconomicOperationResult::RateLimited;
        }
        
        // Get usage events for user
        let usage_events = self.usage_events.lock().unwrap();
        let user_events: Vec<&UsageEvent> = usage_events.iter()
            .filter(|event| event.user_id == user_id)
            .collect();
        
        // Apply limit
        let limit_val = limit.unwrap_or(100).min(1000);
        let limited_events: Vec<&UsageEvent> = if user_events.clone().len() > limit_val {
            user_events[user_events.len() - limit_val..].to_vec()
        } else {
            user_events.clone()
        };
        
        // Return events
        let total_events = user_events.len();
        let returned_events = limited_events.len();
        
        EconomicOperationResult::Success(serde_json::json!({
            "events": limited_events,
            "total": total_events,
            "returned": returned_events,
        }))
    }
    
    /// Get recent threshold events for a user
    pub fn get_recent_threshold_events(&self, token: &str, limit: Option<usize>, critical_only: Option<bool>) -> EconomicOperationResult {
        // Validate authentication
        let user_id = match self.validate_token(token) {
            Some(id) => id,
            None => return EconomicOperationResult::AuthenticationFailed,
        };
        
        // Check rate limits
        if !self.check_rate_limit(&user_id, 100) {
            return EconomicOperationResult::RateLimited;
        }
        
        // Get threshold events for user
        let threshold_events = self.threshold_events.lock().unwrap();
        let user_events: Vec<&ThresholdEvent> = threshold_events.iter()
            .filter(|event| event.user_id == user_id)
            .filter(|event| {
                if let Some(critical) = critical_only {
                    if critical {
                        event.is_critical
                    } else {
                        true
                    }
                } else {
                    true
                }
            })
            .collect();
        
        // Apply limit
        let limit_val = limit.unwrap_or(100).min(1000);
        let limited_events: Vec<&ThresholdEvent> = if user_events.clone().len() > limit_val {
            user_events[user_events.len() - limit_val..].to_vec()
        } else {
            user_events.clone()
        };
        
        // Return events
        let total_events = user_events.len();
        let returned_events = limited_events.len();
        let is_critical_only = critical_only.unwrap_or(false);
        
        EconomicOperationResult::Success(serde_json::json!({
            "events": limited_events,
            "total": total_events,
            "returned": returned_events,
            "critical_only": is_critical_only,
        }))
    }
    
    /// Register a webhook for events
    pub fn register_webhook(&self, token: &str, webhook_url: &str) -> EconomicOperationResult {
        // Validate authentication
        let user_id = match self.validate_token(token) {
            Some(id) => id,
            None => return EconomicOperationResult::AuthenticationFailed,
        };
        
        // Check rate limits
        if !self.check_rate_limit(&user_id, 50) {
            return EconomicOperationResult::RateLimited;
        }
        
        // Validate webhook URL
        if !webhook_url.starts_with("http://") && !webhook_url.starts_with("https://") {
            return EconomicOperationResult::InvalidInput("Invalid webhook URL format".to_string());
        }
        
        // Register webhook
        let mut webhooks = self.webhooks.lock().unwrap();
        webhooks.push(webhook_url.to_string());
        
        // Return success
        EconomicOperationResult::Success(serde_json::json!({
            "status": "success",
            "message": "Webhook registered successfully",
            "webhook_id": self.generate_id("webhook"),
        }))
    }
    
    /// Update threshold configuration
    pub fn update_threshold(&self, token: &str, resource_type: ResourceType, warning: Option<f64>, critical: Option<f64>, enabled: Option<bool>) -> EconomicOperationResult {
        // Validate authentication
        let user_id = match self.validate_token(token) {
            Some(id) => id,
            None => return EconomicOperationResult::AuthenticationFailed,
        };
        
        // Check rate limits
        if !self.check_rate_limit(&user_id, 50) {
            return EconomicOperationResult::RateLimited;
        }
        
        // Update threshold
        let mut thresholds = self.thresholds.lock().unwrap();
        let threshold = thresholds.entry(resource_type.clone()).or_insert_with(|| {
            ResourceThreshold {
                resource_type: resource_type.clone(),
                warning_threshold: 80.0,
                critical_threshold: 95.0,
                enabled: true,
            }
        });
        
        // Apply updates
        if let Some(warning_val) = warning {
            threshold.warning_threshold = warning_val;
        }
        
        if let Some(critical_val) = critical {
            threshold.critical_threshold = critical_val;
        }
        
        if let Some(enabled_val) = enabled {
            threshold.enabled = enabled_val;
        }
        
        // Return success
        EconomicOperationResult::Success(serde_json::json!({
            "status": "success",
            "message": "Threshold updated successfully",
            "threshold": threshold,
        }))
    }
    
    /// Get system health status
    pub fn get_health_status(&self, api_key: &str) -> EconomicOperationResult {
        // Validate API key
        if self.validate_api_key(api_key).is_none() {
            return EconomicOperationResult::AuthenticationFailed;
        }
        
        // Get circuit breaker status
        let circuit_breaker_open = *self.circuit_breaker_open.lock().unwrap();
        
        // Get event counts
        let usage_events_count = self.usage_events.lock().unwrap().len();
        let threshold_events_count = self.threshold_events.lock().unwrap().len();
        
        // Return success with health status
        EconomicOperationResult::Success(serde_json::json!({
            "status": "healthy",
            "components": {
                "event_emission": if circuit_breaker_open { "degraded" } else { "healthy" },
                "metrics_collection": "healthy",
                "threshold_detection": "healthy",
                "api": "healthy",
            },
            "metrics": {
                "usage_events_buffered": usage_events_count,
                "threshold_events_buffered": threshold_events_count,
                "webhooks_registered": self.webhooks.lock().unwrap().len(),
            },
            "timestamp": SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        }))
    }
}

impl Clone for MockEconomicService {
    fn clone(&self) -> Self {
        Self {
            vm_resources: self.vm_resources.clone(),
            usage_events: self.usage_events.clone(),
            threshold_events: self.threshold_events.clone(),
            thresholds: self.thresholds.clone(),
            webhooks: self.webhooks.clone(),
            rate_limits: self.rate_limits.clone(),
            tokens: self.tokens.clone(),
            api_keys: self.api_keys.clone(),
            id_counter: self.id_counter.clone(),
            failure_rate: self.failure_rate,
            circuit_breaker_open: self.circuit_breaker_open.clone(),
        }
    }
}

/// Economic Infrastructure harness for testing
pub struct EconomicHarness {
    /// Economic service
    pub service: MockEconomicService,
}

impl EconomicHarness {
    /// Create a new Economic Infrastructure harness
    pub fn new() -> Self {
        Self {
            service: MockEconomicService::new(),
        }
    }
    
    /// Register an API key for a user
    pub fn register_api_key(&self, user_id: &str, api_key: &str) -> EconomicOperationResult {
        self.service.register_api_key(user_id, api_key)
    }
    
    /// Register a JWT token for a user
    pub fn register_token(&self, user_id: &str, token: &str) -> EconomicOperationResult {
        self.service.register_token(user_id, token)
    }
    
    /// Report resource usage for a VM
    pub fn report_resource_usage(&self, token: &str, vm_id: &str, resources: HashMap<ResourceType, f64>) -> EconomicOperationResult {
        self.service.report_resource_usage(token, vm_id, resources)
    }
    
    /// Get current resource usage for a VM
    pub fn get_vm_usage(&self, token: &str, vm_id: &str) -> EconomicOperationResult {
        self.service.get_vm_usage(token, vm_id)
    }
    
    /// Get recent usage events for a user
    pub fn get_recent_usage_events(&self, token: &str, limit: Option<usize>) -> EconomicOperationResult {
        self.service.get_recent_usage_events(token, limit)
    }
    
    /// Get recent threshold events for a user
    pub fn get_recent_threshold_events(&self, token: &str, limit: Option<usize>, critical_only: Option<bool>) -> EconomicOperationResult {
        self.service.get_recent_threshold_events(token, limit, critical_only)
    }
    
    /// Register a webhook for events
    pub fn register_webhook(&self, token: &str, webhook_url: &str) -> EconomicOperationResult {
        self.service.register_webhook(token, webhook_url)
    }
    
    /// Update threshold configuration
    pub fn update_threshold(&self, token: &str, resource_type: ResourceType, warning: Option<f64>, critical: Option<f64>, enabled: Option<bool>) -> EconomicOperationResult {
        self.service.update_threshold(token, resource_type, warning, critical, enabled)
    }
    
    /// Get system health status
    pub fn get_health_status(&self, api_key: &str) -> EconomicOperationResult {
        self.service.get_health_status(api_key)
    }
} 
