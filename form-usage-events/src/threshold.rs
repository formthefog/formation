use crate::errors::UsageEventError;
use crate::events::{UsageEvent, UsageMetrics};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Types of resources that can be monitored
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ResourceType {
    /// CPU usage (percentage or absolute seconds)
    Cpu,
    /// Memory usage (percentage or absolute GB)
    Memory,
    /// Storage usage (percentage or absolute GB)
    Storage,
    /// Network egress (MB)
    NetworkEgress,
    /// Network ingress (MB)
    NetworkIngress,
    /// GPU usage (percentage or absolute seconds)
    Gpu,
}

/// Types of thresholds that can be defined
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ThresholdType {
    /// Absolute value with unit
    Absolute {
        value: f64,
        unit: String,
    },
    /// Percentage value (0-100)
    Percentage {
        value: f64,
    },
}

/// Types of actions to take when a threshold is exceeded
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ActionType {
    /// Log the threshold violation but take no action
    Log,
    /// Send notification via configured channels
    Notify,
    /// Take a predefined action (e.g., throttle)
    Action(String),
}

/// Configuration for a resource threshold
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThresholdConfig {
    /// Unique identifier for this threshold
    pub id: String,
    
    /// Type of resource to monitor
    pub resource_type: ResourceType,
    
    /// Type of threshold (absolute or percentage)
    pub threshold_type: ThresholdType,
    
    /// Action to take when threshold is exceeded
    pub action: ActionType,
    
    /// User ID this threshold applies to (or * for all)
    pub user_id: String,
    
    /// Instance ID this threshold applies to (or * for all)
    pub instance_id: Option<String>,
    
    /// Notification channels for alerts
    pub notification_channels: Vec<String>,
    
    /// Human-readable description of this threshold
    pub description: Option<String>,
}

/// Information about a threshold violation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThresholdViolation {
    /// The threshold configuration that was violated
    pub config: ThresholdConfig,
    
    /// Current value that triggered the violation
    pub current_value: f64,
    
    /// The threshold value that was exceeded
    pub threshold_value: f64,
    
    /// Percentage above/below the threshold
    pub percentage: f64,
    
    /// Timestamp when violation was detected
    pub timestamp: i64,
    
    /// Instance ID where violation occurred
    pub instance_id: String,
    
    /// User ID associated with the instance
    pub user_id: String,
}

/// Manager for threshold configuration and checking
pub struct ThresholdManager {
    /// Current configuration of thresholds
    configs: Arc<RwLock<HashMap<String, ThresholdConfig>>>,
    
    /// Last time configs were loaded
    last_config_load: Arc<RwLock<i64>>,
    
    /// Source for loading configs (file path or API URL)
    config_source: String,
}

impl ThresholdManager {
    /// Create a new threshold manager with the specified config source
    pub fn new(config_source: String) -> Self {
        Self {
            configs: Arc::new(RwLock::new(HashMap::new())),
            last_config_load: Arc::new(RwLock::new(0)),
            config_source,
        }
    }
    
    /// Load configurations from the source
    pub async fn load_configs(&self) -> Result<(), UsageEventError> {
        // For now, we'll just load a hardcoded set of thresholds
        // In a real implementation, this would load from a file or API
        // based on the config_source
        
        // Log that we're loading from the config source
        println!("Loading threshold configurations from source: {}", self.config_source);
        
        // Get write access to the configs
        let mut configs_lock = self.configs.write().await;
        
        // Clear existing configs
        configs_lock.clear();
        
        // Create some example thresholds
        let example_configs = vec![
            ThresholdConfig {
                id: "cpu-high".to_string(),
                resource_type: ResourceType::Cpu,
                threshold_type: ThresholdType::Percentage { value: 80.0 },
                action: ActionType::Notify,
                user_id: "*".to_string(),
                instance_id: None,
                notification_channels: vec!["email".to_string()],
                description: Some("High CPU usage alert".to_string()),
            },
            ThresholdConfig {
                id: "memory-critical".to_string(),
                resource_type: ResourceType::Memory,
                threshold_type: ThresholdType::Percentage { value: 90.0 },
                action: ActionType::Notify,
                user_id: "*".to_string(),
                instance_id: None,
                notification_channels: vec!["email".to_string(), "sms".to_string()],
                description: Some("Critical memory usage alert".to_string()),
            },
            ThresholdConfig {
                id: "storage-warning".to_string(),
                resource_type: ResourceType::Storage,
                threshold_type: ThresholdType::Absolute { 
                    value: 100.0, 
                    unit: "GB".to_string() 
                },
                action: ActionType::Log,
                user_id: "*".to_string(),
                instance_id: None,
                notification_channels: vec!["email".to_string()],
                description: Some("Storage usage warning".to_string()),
            },
        ];
        
        // Add configs to the map
        for config in example_configs {
            configs_lock.insert(config.id.clone(), config);
        }
        
        // Update last load time
        *self.last_config_load.write().await = chrono::Utc::now().timestamp();
        
        Ok(())
    }
    
    /// Check if metrics violate any thresholds
    pub async fn check_thresholds(
        &self, 
        metrics: &UsageMetrics,
        instance_id: &str,
        user_id: &str,
    ) -> Result<Vec<ThresholdViolation>, UsageEventError> {
        let configs = self.configs.read().await;
        let mut violations = Vec::new();
        
        // Iterate through all configs
        for (_, config) in configs.iter() {
            // Check if this config applies to this instance/user
            if (config.user_id == "*" || config.user_id == user_id) &&
               (config.instance_id.is_none() || config.instance_id.as_ref().unwrap() == instance_id) {
                
                // Get the current value for this resource type
                let current_value = match config.resource_type {
                    ResourceType::Cpu => metrics.cpu_percent_avg,
                    ResourceType::Memory => {
                        match &config.threshold_type {
                            ThresholdType::Absolute { .. } => metrics.memory_gb,
                            ThresholdType::Percentage { .. } => metrics.memory_percent,
                        }
                    },
                    ResourceType::Storage => metrics.storage_gb,
                    ResourceType::NetworkEgress => metrics.network_egress_mb,
                    ResourceType::NetworkIngress => metrics.network_ingress_mb,
                    ResourceType::Gpu => metrics.gpu_seconds as f64,
                };
                
                // Get threshold value
                let threshold_value = match &config.threshold_type {
                    ThresholdType::Absolute { value, .. } => *value,
                    ThresholdType::Percentage { value } => *value,
                };
                
                // Check if threshold is exceeded
                if current_value > threshold_value {
                    // Calculate percentage over threshold
                    let percentage = (current_value - threshold_value) / threshold_value * 100.0;
                    
                    // Create violation
                    let violation = ThresholdViolation {
                        config: config.clone(),
                        current_value,
                        threshold_value,
                        percentage,
                        timestamp: chrono::Utc::now().timestamp(),
                        instance_id: instance_id.to_string(),
                        user_id: user_id.to_string(),
                    };
                    
                    violations.push(violation);
                }
            }
        }
        
        Ok(violations)
    }
    
    /// Process threshold violations
    pub async fn process_violations(
        &self,
        violations: Vec<ThresholdViolation>,
    ) -> Result<(), UsageEventError> {
        for violation in violations {
            match violation.config.action {
                ActionType::Log => {
                    // Simply log the violation
                    println!(
                        "THRESHOLD VIOLATION: {} - {} exceeded by {:.2}%",
                        violation.config.id,
                        match violation.config.resource_type {
                            ResourceType::Cpu => "CPU",
                            ResourceType::Memory => "Memory",
                            ResourceType::Storage => "Storage",
                            ResourceType::NetworkEgress => "Network Egress",
                            ResourceType::NetworkIngress => "Network Ingress",
                            ResourceType::Gpu => "GPU",
                        },
                        violation.percentage
                    );
                },
                ActionType::Notify => {
                    // Here we would send notifications via the configured channels
                    // For now, just log it
                    println!(
                        "THRESHOLD NOTIFICATION: {} - {} exceeded by {:.2}% - Would notify via: {:?}",
                        violation.config.id,
                        match violation.config.resource_type {
                            ResourceType::Cpu => "CPU",
                            ResourceType::Memory => "Memory",
                            ResourceType::Storage => "Storage",
                            ResourceType::NetworkEgress => "Network Egress",
                            ResourceType::NetworkIngress => "Network Ingress",
                            ResourceType::Gpu => "GPU",
                        },
                        violation.percentage,
                        violation.config.notification_channels
                    );
                },
                ActionType::Action(ref action) => {
                    // Here we would take the specified action
                    // For now, just log it
                    println!(
                        "THRESHOLD ACTION: {} - {} exceeded by {:.2}% - Would take action: {}",
                        violation.config.id,
                        match violation.config.resource_type {
                            ResourceType::Cpu => "CPU",
                            ResourceType::Memory => "Memory",
                            ResourceType::Storage => "Storage",
                            ResourceType::NetworkEgress => "Network Egress",
                            ResourceType::NetworkIngress => "Network Ingress",
                            ResourceType::Gpu => "GPU",
                        },
                        violation.percentage,
                        action
                    );
                },
            }
        }
        
        Ok(())
    }
    
    /// Check thresholds for a usage event
    pub async fn check_event(&self, event: &UsageEvent) -> Result<(), UsageEventError> {
        // Check thresholds for the event
        let violations = self.check_thresholds(
            &event.metrics,
            &event.instance_id,
            &event.user_id,
        ).await?;
        
        // Process any violations
        if !violations.is_empty() {
            self.process_violations(violations).await?;
        }
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::UsagePeriod;
    
    #[tokio::test]
    async fn test_threshold_config_load() {
        let manager = ThresholdManager::new("test".to_string());
        manager.load_configs().await.unwrap();
        
        let configs = manager.configs.read().await;
        assert!(!configs.is_empty());
        assert!(configs.contains_key("cpu-high"));
        assert!(configs.contains_key("memory-critical"));
        assert!(configs.contains_key("storage-warning"));
    }
    
    #[tokio::test]
    async fn test_threshold_violation_detection() {
        let manager = ThresholdManager::new("test".to_string());
        manager.load_configs().await.unwrap();
        
        // Create metrics that exceed CPU threshold
        let metrics = UsageMetrics {
            cpu_seconds: 30,
            cpu_percent_avg: 95.0, // Exceeds 80% threshold
            memory_gb: 4.0,        // Below 90% threshold
            memory_percent: 50.0,
            storage_gb: 10.0,
            network_egress_mb: 100.0,
            network_ingress_mb: 50.0,
            gpu_seconds: 0,
        };
        
        let violations = manager.check_thresholds(
            &metrics,
            "test-instance",
            "test-user",
        ).await.unwrap();
        
        // Should find 1 violation (CPU)
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].config.resource_type, ResourceType::Cpu);
        assert!(violations[0].percentage > 0.0);
    }
    
    #[tokio::test]
    async fn test_multiple_threshold_violations() {
        let manager = ThresholdManager::new("test".to_string());
        manager.load_configs().await.unwrap();
        
        // Create metrics that exceed both CPU and Memory thresholds
        let metrics = UsageMetrics {
            cpu_seconds: 30,
            cpu_percent_avg: 95.0,     // Exceeds 80% threshold
            memory_gb: 10.0,
            memory_percent: 95.0,      // Exceeds 90% threshold
            storage_gb: 200.0,         // Exceeds 100GB threshold
            network_egress_mb: 100.0,
            network_ingress_mb: 50.0,
            gpu_seconds: 0,
        };
        
        let violations = manager.check_thresholds(
            &metrics,
            "test-instance",
            "test-user",
        ).await.unwrap();
        
        // Should find 3 violations (CPU, Memory, and Storage)
        assert_eq!(violations.len(), 3);
        
        // Verify that we have the expected resource types in the violations
        let resource_types: Vec<ResourceType> = violations.iter()
            .map(|v| v.config.resource_type.clone())
            .collect();
            
        assert!(resource_types.contains(&ResourceType::Cpu));
        assert!(resource_types.contains(&ResourceType::Memory));
        assert!(resource_types.contains(&ResourceType::Storage));
    }
    
    #[tokio::test]
    async fn test_event_checking() {
        let manager = ThresholdManager::new("test".to_string());
        manager.load_configs().await.unwrap();
        
        // Create an event with metrics that exceed CPU threshold
        let event = UsageEvent {
            event_type: "resource_usage".to_string(),
            version: "1.0".to_string(),
            timestamp: chrono::Utc::now().timestamp(),
            instance_id: "test-instance".to_string(),
            user_id: "test-user".to_string(),
            org_id: None,
            metrics: UsageMetrics {
                cpu_seconds: 30,
                cpu_percent_avg: 95.0, // Exceeds 80% threshold
                memory_gb: 4.0,        // Below 90% threshold
                memory_percent: 50.0,
                storage_gb: 10.0,
                network_egress_mb: 100.0,
                network_ingress_mb: 50.0,
                gpu_seconds: 0,
            },
            period: UsagePeriod {
                start: chrono::Utc::now().timestamp() - 30,
                end: chrono::Utc::now().timestamp(),
            },
        };
        
        // Should not error
        manager.check_event(&event).await.unwrap();
    }
} 