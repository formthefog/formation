# Economic Infrastructure: Next Steps

Based on our analysis of the existing codebase and the updated task list, here are the prioritized next steps to implement the Economic Infrastructure backend components.

## Priority 1: Complete the Metrics Collection & Event Schema (Week 1)

### 1.1 Extend SystemMetrics with Instance Information
The `form-vm-metrics` crate already implements excellent resource metrics collection, but needs to be extended to include instance and account information:

```rust
// In form-vm-metrics/src/system.rs
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SystemMetrics {
    pub timestamp: i64,
    pub instance_id: Option<String>,   // <-- Add this
    pub account_id: Option<String>,    // <-- Add this
    pub cpu: CpuMetrics,
    pub memory: MemoryMetrics,
    pub disks: Vec<DiskMetrics>,
    pub network: NetworkMetrics,
    pub gpus: Vec<GpuMetrics>,
    pub load: LoadMetrics,
}
```

### 1.2 Create Usage Event Schema
Create a new crate `form-usage-events` to define the standardized event schema for usage metrics:

```rust
// In form-usage-events/src/events.rs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageEvent {
    pub event_type: String,  // "resource_usage"
    pub version: String,     // "1.0"
    pub timestamp: i64,
    pub instance_id: String,
    pub user_id: String,
    pub org_id: Option<String>,
    pub metrics: UsageMetrics,
    pub period: UsagePeriod,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageMetrics {
    pub cpu_seconds: u64,
    pub cpu_percent_avg: f64,
    pub memory_gb: f64,
    pub memory_percent: f64,
    pub storage_gb: f64,
    pub network_egress_mb: f64,
    pub network_ingress_mb: f64,
    pub gpu_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsagePeriod {
    pub start: i64,
    pub end: i64,
}
```

### 1.3 Create the Basic Event Emission Pipeline
Implement a basic event emission mechanism that converts `SystemMetrics` to `UsageEvent` and publishes to the message queue:

```rust
// In form-usage-events/src/publish.rs
pub struct EventPublisher {
    client: reqwest::Client,
    queue_endpoint: String,
    topic: String,
}

impl EventPublisher {
    pub fn new(queue_endpoint: String, topic: String) -> Self {
        // Implementation
    }
    
    pub async fn publish(&self, event: UsageEvent) -> Result<(), UsageEventError> {
        // Implementation using form-p2p's write_to_queue
    }
}
```

### 1.4 Modify the Collection Loop to Publish Events
Update the metrics collection loop in `form-vm-metrics/src/main.rs` to publish events after each collection:

```rust
let metrics_collection_handle = tokio::spawn(async move {
    let mut interval = interval(Duration::from_secs(30));
    let publisher = EventPublisher::new(queue_endpoint, "usage_events".to_string());
    
    loop {
        interval.tick().await;
        tokio::select! {
            _ = inner_receiver.recv() => { break }
            _ = async {
                // Collect metrics
                let metrics = collect_system_metrics(collector_metrics.clone()).await;
                // Publish event
                if let Err(e) = publisher.publish_metrics(&metrics.lock().await).await {
                    eprintln!("Failed to publish metrics: {}", e);
                }
            } => {}
        }
    }
});
```

## Priority 2: Account Integration & Reliability (Week 2)

### 2.1 Integrate with Account Service
Create a client for the account service to retrieve account information for instances:

```rust
// In form-vm-metrics/src/accounts.rs
pub struct AccountClient {
    client: reqwest::Client,
    base_url: String,
}

impl AccountClient {
    pub fn new(base_url: String) -> Self {
        // Implementation
    }
    
    pub async fn get_account_for_instance(&self, instance_id: &str) -> Result<Option<Account>, String> {
        // Implementation to query the account service
    }
}
```

### 2.2 Add Retry Mechanism
Implement retry logic for event publishing to handle temporary failures:

```rust
// In form-usage-events/src/retry.rs
pub async fn with_retry<F, Fut, T, E>(
    f: F,
    max_retries: u32,
    initial_backoff: Duration,
) -> Result<T, E>
where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = Result<T, E>>,
    E: std::fmt::Debug,
{
    // Implementation of exponential backoff with jitter
}
```

### 2.3 Implement Circuit Breaking
Add circuit breaking to prevent overwhelming the message queue during outages:

```rust
// In form-usage-events/src/circuit_breaker.rs
pub struct CircuitBreaker {
    state: Arc<RwLock<CircuitState>>,
    // Other fields
}

impl CircuitBreaker {
    // Implementation
}
```

## Priority 3: Threshold Detection & Enhanced API (Week 3)

### 3.1 Create Threshold Configuration Types
Define the structs and enums for threshold configuration:

```rust
// In form-usage-events/src/thresholds.rs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ResourceType {
    Cpu,
    Memory,
    Storage,
    NetworkEgress,
    NetworkIngress,
    Gpu,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ThresholdType {
    Absolute { value: f64, unit: String },
    Percentage { value: f64 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThresholdConfig {
    // Fields
}
```

### 3.2 Implement Threshold Checking
Create the logic to check metrics against configured thresholds:

```rust
// In form-vm-metrics/src/thresholds.rs
pub struct MetricsThresholdChecker {
    threshold_client: ThresholdClient,
    event_publisher: EventPublisher,
}

impl MetricsThresholdChecker {
    // Implementation
}
```

### 3.3 Enhance API with Authentication
Add authentication and filtering to the metrics API:

```rust
// In form-vm-metrics/src/main.rs
async fn get_metrics_authenticated(
    headers: HeaderMap,
    State(state): State<Arc<Mutex<SystemMetrics>>>,
) -> Result<Json<SystemMetrics>, StatusCode> {
    // Implementation
}

async fn get_instance_metrics(
    headers: HeaderMap,
    Path(instance_id): Path<String>,
    State(state): State<Arc<Mutex<SystemMetrics>>>,
) -> Result<Json<SystemMetrics>, StatusCode> {
    // Implementation
}
```

## Priority 4: Testing & Documentation (Week 4)

### 4.1 Implement Unit Tests
Create comprehensive tests for the core functionality:

- Metrics collection accuracy
- Event serialization/deserialization
- Threshold detection
- Retry and circuit breaking

### 4.2 Create Integration Tests
Test the end-to-end flow with mock services:

- Event emission to message queue
- Account service integration
- Threshold notifications

### 4.3 Write Documentation
Create comprehensive documentation for the system:

- Architecture overview
- API documentation
- Configuration options
- Integration guide for external systems

## Implementation Approach

1. **Iterative Development**: Implement and test each component incrementally
2. **Start Simple**: Begin with basic functionality, then add reliability features
3. **Reuse Existing Code**: Leverage the existing metrics collection and message queue
4. **Focus on Core Requirements**: Prioritize the event emission pipeline over advanced features

## First Week Action Items

1. Extend `SystemMetrics` with instance and account information
2. Create the `form-usage-events` crate with the event schema
3. Implement basic event conversion and publishing
4. Modify the collection loop to publish events
5. Test the basic pipeline with manual verification

This approach allows us to build on the significant existing code while focusing on the gaps identified in our analysis. 