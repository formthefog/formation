use serde::{Serialize, Deserialize};

/// Represents a resource usage event for billing and monitoring purposes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageEvent {
    /// Type of event, always "resource_usage" for these events
    pub event_type: String,
    
    /// Schema version for forward compatibility
    pub version: String,
    
    /// Unix timestamp when the event was created
    pub timestamp: i64,
    
    /// Identifier for the instance being monitored
    pub instance_id: String,
    
    /// Identifier for the account that owns the instance
    pub user_id: String,
    
    /// Optional organization identifier for the account
    pub org_id: Option<String>,
    
    /// Resource usage metrics for the period
    pub metrics: UsageMetrics,
    
    /// Time period the metrics cover
    pub period: UsagePeriod,
}

/// Contains the actual resource usage metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageMetrics {
    /// CPU usage in seconds during the period
    pub cpu_seconds: u64,
    
    /// Average CPU usage percentage during the period
    pub cpu_percent_avg: f64,
    
    /// Memory usage in GB
    pub memory_gb: f64,
    
    /// Memory usage as a percentage of allocated memory
    pub memory_percent: f64,
    
    /// Storage usage in GB
    pub storage_gb: f64,
    
    /// Network egress in MB
    pub network_egress_mb: f64,
    
    /// Network ingress in MB
    pub network_ingress_mb: f64,
    
    /// GPU usage in seconds (0 if no GPU is used)
    pub gpu_seconds: u64,
}

/// Represents the time period that the usage metrics cover
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsagePeriod {
    /// Unix timestamp for the start of the period
    pub start: i64,
    
    /// Unix timestamp for the end of the period
    pub end: i64,
}

impl UsageEvent {
    /// Creates a new UsageEvent with the current timestamp
    pub fn new(
        instance_id: String,
        user_id: String,
        org_id: Option<String>,
        metrics: UsageMetrics,
        period: UsagePeriod,
    ) -> Self {
        // Get current timestamp
        let timestamp = chrono::Utc::now().timestamp();
        
        Self {
            event_type: "resource_usage".to_string(),
            version: "1.0".to_string(),
            timestamp,
            instance_id,
            user_id,
            org_id,
            metrics,
            period,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_usage_event_serialization() {
        // Create sample metrics
        let metrics = UsageMetrics {
            cpu_seconds: 30,
            cpu_percent_avg: 12.5,
            memory_gb: 4.2,
            memory_percent: 52.5,
            storage_gb: 25.7,
            network_egress_mb: 15.2,
            network_ingress_mb: 8.7,
            gpu_seconds: 0,
        };
        
        // Create sample period
        let period = UsagePeriod {
            start: 1626350400, // 2021-07-15T12:00:00Z
            end: 1626350430,   // 2021-07-15T12:00:30Z
        };
        
        // Create sample event
        let event = UsageEvent {
            event_type: "resource_usage".to_string(),
            version: "1.0".to_string(),
            timestamp: 1626350435, // 2021-07-15T12:00:35Z
            instance_id: "test-instance-123".to_string(),
            user_id: "test-user-456".to_string(),
            org_id: Some("test-org-789".to_string()),
            metrics: metrics.clone(),
            period: period.clone(),
        };
        
        // Serialize to JSON
        let json = serde_json::to_string(&event).unwrap();
        
        // Deserialize back
        let deserialized: UsageEvent = serde_json::from_str(&json).unwrap();
        
        // Assert values match
        assert_eq!(event.event_type, deserialized.event_type);
        assert_eq!(event.version, deserialized.version);
        assert_eq!(event.timestamp, deserialized.timestamp);
        assert_eq!(event.instance_id, deserialized.instance_id);
        assert_eq!(event.user_id, deserialized.user_id);
        assert_eq!(event.org_id, deserialized.org_id);
        assert_eq!(event.metrics.cpu_seconds, deserialized.metrics.cpu_seconds);
        assert_eq!(event.metrics.cpu_percent_avg, deserialized.metrics.cpu_percent_avg);
        assert_eq!(event.metrics.memory_gb, deserialized.metrics.memory_gb);
        assert_eq!(event.metrics.memory_percent, deserialized.metrics.memory_percent);
        assert_eq!(event.metrics.storage_gb, deserialized.metrics.storage_gb);
        assert_eq!(event.metrics.network_egress_mb, deserialized.metrics.network_egress_mb);
        assert_eq!(event.metrics.network_ingress_mb, deserialized.metrics.network_ingress_mb);
        assert_eq!(event.metrics.gpu_seconds, deserialized.metrics.gpu_seconds);
        assert_eq!(event.period.start, deserialized.period.start);
        assert_eq!(event.period.end, deserialized.period.end);
        
        // Test the new() constructor
        let constructed_event = UsageEvent::new(
            "test-instance-123".to_string(),
            "test-user-456".to_string(),
            Some("test-org-789".to_string()),
            metrics,
            period,
        );
        
        assert_eq!(constructed_event.event_type, "resource_usage");
        assert_eq!(constructed_event.version, "1.0");
        assert_eq!(constructed_event.instance_id, "test-instance-123");
        assert_eq!(constructed_event.user_id, "test-user-456");
        assert_eq!(constructed_event.org_id, Some("test-org-789".to_string()));
    }
} 