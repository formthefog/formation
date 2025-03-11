# Form Usage Events API Documentation

## Overview

The Form Usage Events library provides functionality for collecting, processing, and publishing resource usage events. It includes features for thresholding, circuit breaking, and reliable event delivery.

## Components

### 1. Event Structure

#### `UsageEvent` Schema

```rust
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
```

#### `UsageMetrics` Schema

```rust
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
```

#### JSON Format Example

```json
{
  "event_type": "resource_usage",
  "version": "1.0",
  "timestamp": 1626350430,
  "instance_id": "instance-abc123",
  "user_id": "user_123456",
  "org_id": "org_789012",
  "metrics": {
    "cpu_seconds": 30,
    "cpu_percent_avg": 12.5,
    "memory_gb": 4.2,
    "memory_percent": 52.5,
    "storage_gb": 25.7,
    "network_egress_mb": 15.2,
    "network_ingress_mb": 8.7,
    "gpu_seconds": 0
  },
  "period": {
    "start": 1626350400,
    "end": 1626350430
  }
}
```

### 2. Event Publishing

The `EventPublisher` provides methods for publishing usage events to a message queue with reliability features:

#### Configuration

```rust
// Basic configuration
let publisher = EventPublisher::with_config(
    "127.0.0.1".to_string(),    // queue_endpoint
    3003,                       // queue_port
    "usage_events".to_string(), // topic
    0,                          // sub_topic
);

// Add retry configuration
let publisher = publisher.with_retry_config(RetryConfig {
    max_retries: 3,
    initial_backoff: Duration::from_millis(100),
    max_backoff: Duration::from_secs(10),
    backoff_multiplier: 2.0,
    jitter_factor: 0.1,
});

// Add circuit breaker
let publisher = publisher.with_circuit_breaker(CircuitBreakerConfig {
    failure_threshold: 5,
    reset_timeout: Duration::from_secs(60),
    half_open_allowed_calls: 2,
});
```

#### Publishing Events

```rust
// Create an event
let event = UsageEvent::new(
    "instance-abc123".to_string(),
    "user_123456".to_string(),
    Some("org_789012".to_string()),
    metrics,
    period,
);

// Publish with retries and circuit breaking
match publisher.publish(event).await {
    Ok(_) => println!("Event published successfully"),
    Err(e) => eprintln!("Failed to publish event: {}", e),
}
```

### 3. Threshold Detection

The `ThresholdManager` allows configuring and checking resource usage thresholds:

#### Configuration

```rust
// Create a threshold manager
let manager = ThresholdManager::new("config_source".to_string());

// Load configurations
manager.load_configs().await?;

// Integrate with the publisher
let publisher = publisher.with_threshold_manager(Arc::new(manager));
```

#### Threshold Configuration Format

```json
{
  "id": "cpu-high",
  "resource_type": "Cpu",
  "threshold_type": {
    "Percentage": {
      "value": 80.0
    }
  },
  "action": "Notify",
  "user_id": "*",
  "notification_channels": ["email"]
}
```

#### Checking Thresholds

```rust
// Check metrics against thresholds
let violations = manager.check_thresholds(
    &metrics,
    "instance-abc123",
    "user_123456"
).await?;

// Process threshold violations
manager.process_violations(violations).await?;

// Or check an entire event
manager.check_event(&event).await?;
```

## Integration with Metrics Collection

The Form VM Metrics system integrates with this library to:

1. Collect system metrics
2. Convert them to usage events
3. Publish events to a message queue
4. Check metrics against thresholds
5. Emit notification events for threshold violations

## Error Handling

The library provides a comprehensive error type `UsageEventError` that covers various failure scenarios:

```rust
pub enum UsageEventError {
    SerializationError(serde_json::Error),
    PublishError(String),
    CircuitBreakerOpen,
    ConnectionError(String),
    HttpError(reqwest::Error),
    Other(String),
}
```

## Testing

The library includes comprehensive tests for:

- Event serialization/deserialization
- Retry mechanism
- Circuit breaker behavior
- Threshold detection
- Event publishing

## Future Enhancements

Planned enhancements include:

- Batch publishing for improved performance
- Dead-letter queue for unprocessable events
- Enhanced monitoring capabilities
- Persistent storage for threshold configurations 