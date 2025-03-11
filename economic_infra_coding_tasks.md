# Economic Infrastructure: Detailed Coding Tasks

This document breaks down the implementation tasks into specific, actionable coding tasks that can be assigned to team members.

## Phase 1: Core Measurement and Event Schema (Week 1)

### Task 1.1: Create `form-usage-events` Crate (2 days)

1. Create new crate with Cargo.toml dependencies
   ```rust
   [package]
   name = "form-usage-events"
   version = "0.1.0"
   edition = "2021"
   
   [dependencies]
   serde = { version = "1.0", features = ["derive"] }
   serde_json = "1.0"
   chrono = "0.4"
   thiserror = "1.0"
   form-p2p = { path = "../form-p2p" }
   ```

2. Define UsageEvent schema in `src/lib.rs`:
   ```rust
   use serde::{Serialize, Deserialize};
   
   pub mod events;
   pub mod publish;
   pub mod errors;
   ```

3. Implement event types in `src/events.rs`:
   ```rust
   use serde::{Serialize, Deserialize};
   
   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub struct UsageEvent {
       // Event metadata
       pub event_type: String,
       pub version: String,
       pub timestamp: i64,
       
       // Identity information
       pub instance_id: String,
       pub user_id: String,
       pub org_id: Option<String>,
       
       // Metrics data
       pub metrics: UsageMetrics,
       
       // Time period
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

4. Create error types in `src/errors.rs`:
   ```rust
   use thiserror::Error;
   
   #[derive(Error, Debug)]
   pub enum UsageEventError {
       #[error("Failed to serialize event: {0}")]
       SerializationError(#[from] serde_json::Error),
       
       #[error("Failed to publish event: {0}")]
       PublishError(String),
       
       #[error("Circuit breaker open")]
       CircuitBreakerOpen,
       
       #[error("Failed to connect to queue: {0}")]
       ConnectionError(String),
   }
   ```

### Task 1.2: Update `form-vm-metrics` to Include Instance Info (1 day)

1. Update `src/system.rs` to include instance information:
   ```rust
   #[derive(Clone, Debug, Default, Serialize, Deserialize)]
   pub struct SystemMetrics {
       pub timestamp: i64,
       pub instance_id: Option<String>,
       pub account_id: Option<String>,
       pub cpu: CpuMetrics,
       pub memory: MemoryMetrics,
       pub disks: Vec<DiskMetrics>,
       pub network: NetworkMetrics,
       pub gpus: Vec<GpuMetrics>,
       pub load: LoadMetrics,
   }
   ```

2. Update `src/main.rs` to include command line parameters for instance ID:
   ```rust
   use clap::Parser;
   
   #[derive(Parser, Debug)]
   struct Args {
       #[clap(long, short)]
       instance_id: Option<String>,
       
       #[clap(long, short)]
       account_id: Option<String>,
   }
   ```

3. Pass these parameters to the SystemMetrics struct in collection logic.

### Task 1.3: Implement Basic Usage Event Emission (3 days)

1. Create `src/publish.rs` in `form-usage-events`:
   ```rust
   use std::time::Duration;
   use crate::{events::UsageEvent, errors::UsageEventError};
   
   pub struct EventPublisher {
       client: reqwest::Client,
       queue_endpoint: String,
       topic: String,
   }
   
   impl EventPublisher {
       pub fn new(queue_endpoint: String, topic: String) -> Self {
           let client = reqwest::Client::builder()
               .timeout(Duration::from_secs(5))
               .build()
               .unwrap_or_default();
           
           Self {
               client,
               queue_endpoint,
               topic,
           }
       }
       
       pub async fn publish(&self, event: UsageEvent) -> Result<(), UsageEventError> {
           // Implementation using form-p2p's write_to_queue
           // ...
       }
   }
   ```

2. Create a new module in `form-vm-metrics/src/events.rs` for converting and publishing:
   ```rust
   use form_usage_events::{events::UsageEvent, publish::EventPublisher};
   use crate::system::SystemMetrics;
   
   pub struct MetricsPublisher {
       publisher: EventPublisher,
   }
   
   impl MetricsPublisher {
       pub fn new(queue_endpoint: String) -> Self {
           let publisher = EventPublisher::new(
               queue_endpoint,
               "usage_events".to_string(),
           );
           
           Self { publisher }
       }
       
       pub async fn publish_metrics(&self, metrics: &SystemMetrics) -> Result<(), String> {
           // Convert SystemMetrics to UsageEvent
           let usage_event = self.metrics_to_event(metrics)?;
           
           // Publish the event
           self.publisher.publish(usage_event).await
               .map_err(|e| format!("Failed to publish event: {}", e))
       }
       
       fn metrics_to_event(&self, metrics: &SystemMetrics) -> Result<UsageEvent, String> {
           // Implementation to convert SystemMetrics to UsageEvent
           // ...
       }
   }
   ```

3. Integrate the event publisher into `form-vm-metrics/src/main.rs`

## Phase 2: Reliability and Integration (Week 2)

### Task 2.1: Implement Retry Mechanism (2 days)

1. Create `src/retry.rs` in `form-usage-events`:
   ```rust
   use std::time::Duration;
   use crate::errors::UsageEventError;
   
   pub struct RetryConfig {
       pub max_retries: u32,
       pub initial_backoff: Duration,
       pub max_backoff: Duration,
       pub backoff_multiplier: f64,
       pub jitter_factor: f64,
   }
   
   impl Default for RetryConfig {
       fn default() -> Self {
           Self {
               max_retries: 3,
               initial_backoff: Duration::from_millis(100),
               max_backoff: Duration::from_secs(10),
               backoff_multiplier: 2.0,
               jitter_factor: 0.1,
           }
       }
   }
   
   pub async fn with_retry<F, Fut, T, E>(
       f: F,
       config: &RetryConfig,
   ) -> Result<T, E>
   where
       F: Fn() -> Fut,
       Fut: std::future::Future<Output = Result<T, E>>,
       E: std::fmt::Debug,
   {
       // Implementation of exponential backoff with jitter
       // ...
   }
   ```

2. Update `EventPublisher` to use retry mechanism:
   ```rust
   impl EventPublisher {
       // ...
       
       pub async fn publish_with_retry(&self, event: UsageEvent, config: &RetryConfig) -> Result<(), UsageEventError> {
           let event_clone = event.clone();
           let publisher = self.clone();
           
           with_retry(
               || async move { publisher.publish(event_clone.clone()).await },
               config,
           ).await
       }
   }
   ```

### Task 2.2: Implement Circuit Breaking (2 days)

1. Create `src/circuit_breaker.rs` in `form-usage-events`:
   ```rust
   use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
   use std::sync::Arc;
   use std::time::{Duration, Instant};
   use tokio::sync::RwLock;
   
   #[derive(Debug, Clone, Copy, PartialEq, Eq)]
   pub enum CircuitState {
       Closed,
       Open,
       HalfOpen,
   }
   
   pub struct CircuitBreakerConfig {
       pub failure_threshold: usize,
       pub reset_timeout: Duration,
       pub half_open_allowed_calls: usize,
   }
   
   pub struct CircuitBreaker {
       state: Arc<RwLock<CircuitState>>,
       failures: Arc<AtomicUsize>,
       last_failure_time: Arc<AtomicU64>,
       config: CircuitBreakerConfig,
   }
   
   impl CircuitBreaker {
       // Implementation of circuit breaker pattern
       // ...
   }
   ```

2. Update `EventPublisher` to use circuit breaker:
   ```rust
   pub struct EventPublisher {
       client: reqwest::Client,
       queue_endpoint: String,
       topic: String,
       circuit_breaker: Option<CircuitBreaker>,
   }
   
   impl EventPublisher {
       // ...
       
       pub fn with_circuit_breaker(mut self, config: CircuitBreakerConfig) -> Self {
           self.circuit_breaker = Some(CircuitBreaker::new(config));
           self
       }
       
       pub async fn publish(&self, event: UsageEvent) -> Result<(), UsageEventError> {
           // Check circuit breaker state first
           if let Some(ref cb) = self.circuit_breaker {
               if !cb.allow_request().await {
                   return Err(UsageEventError::CircuitBreakerOpen);
               }
           }
           
           // Attempt to publish
           let result = self.publish_internal(event).await;
           
           // Record success or failure
           if let Some(ref cb) = self.circuit_breaker {
               match &result {
                   Ok(_) => cb.record_success().await,
                   Err(_) => cb.record_failure().await,
               }
           }
           
           result
       }
       
       async fn publish_internal(&self, event: UsageEvent) -> Result<(), UsageEventError> {
           // Original implementation moved here
           // ...
       }
   }
   ```

### Task 2.3: Integrate with Account Service (2 days)

1. Create a client for the account service in `form-vm-metrics/src/accounts.rs`:
   ```rust
   use serde::{Serialize, Deserialize};
   
   #[derive(Debug, Serialize, Deserialize)]
   pub struct Account {
       pub address: String,
       pub name: Option<String>,
       pub owned_instances: Vec<String>,
       // Additional fields as needed
   }
   
   pub struct AccountClient {
       client: reqwest::Client,
       base_url: String,
   }
   
   impl AccountClient {
       pub fn new(base_url: String) -> Self {
           let client = reqwest::Client::new();
           Self { client, base_url }
       }
       
       pub async fn get_account_for_instance(&self, instance_id: &str) -> Result<Option<Account>, String> {
           // Implementation to query the account service
           // ...
       }
   }
   ```

2. Update metrics collection to include account information:
   ```rust
   pub async fn collect_system_metrics_with_account(
       system_metrics: Arc<Mutex<SystemMetrics>>,
       instance_id: String,
       account_client: &AccountClient,
   ) -> Arc<Mutex<SystemMetrics>> {
       // Get the account info for this instance
       let account_info = match account_client.get_account_for_instance(&instance_id).await {
           Ok(Some(account)) => Some(account.address),
           _ => None,
       };
       
       // Collect metrics as usual
       let metrics = collect_system_metrics(system_metrics).await;
       
       // Update with instance and account info
       let mut guard = metrics.lock().await;
       guard.instance_id = Some(instance_id);
       guard.account_id = account_info;
       drop(guard);
       
       metrics
   }
   ```

## Phase 3: Threshold Detection and API (Week 3)

### Task 3.1: Implement Threshold Configuration (2 days)

1. Create types for threshold configuration in `src/thresholds.rs` in `form-usage-events`:
   ```rust
   use serde::{Serialize, Deserialize};
   
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
   pub enum ActionType {
       Notify,
       Log,
       Alert,
   }
   
   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub struct ThresholdConfig {
       pub id: String,
       pub resource_type: ResourceType,
       pub threshold_type: ThresholdType,
       pub action: ActionType,
       pub user_id: String,
       pub instance_id: Option<String>,
       pub notification_channels: Vec<String>,
   }
   
   pub struct ThresholdManager {
       configs: Vec<ThresholdConfig>,
   }
   
   impl ThresholdManager {
       pub fn new(configs: Vec<ThresholdConfig>) -> Self {
           Self { configs }
       }
       
       pub fn check_thresholds(&self, metrics: &UsageMetrics, instance_id: &str, user_id: &str) -> Vec<ThresholdViolation> {
           // Implementation to check metrics against thresholds
           // ...
       }
   }
   
   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub struct ThresholdViolation {
       pub config: ThresholdConfig,
       pub current_value: f64,
       pub threshold_value: f64,
       pub percentage: f64,
   }
   ```

2. Create an API client for loading threshold configurations:
   ```rust
   pub struct ThresholdClient {
       client: reqwest::Client,
       base_url: String,
   }
   
   impl ThresholdClient {
       pub fn new(base_url: String) -> Self {
           let client = reqwest::Client::new();
           Self { client, base_url }
       }
       
       pub async fn get_thresholds(&self, instance_id: &str, user_id: &str) -> Result<Vec<ThresholdConfig>, String> {
           // Implementation to fetch threshold configs from API
           // ...
       }
   }
   ```

### Task 3.2: Implement Threshold Checking (2 days)

1. Create module in `form-vm-metrics/src/thresholds.rs` to perform threshold checks:
   ```rust
   use form_usage_events::thresholds::{ThresholdClient, ThresholdManager, ThresholdViolation};
   use crate::system::SystemMetrics;
   
   pub struct MetricsThresholdChecker {
       threshold_client: ThresholdClient,
       event_publisher: EventPublisher,
   }
   
   impl MetricsThresholdChecker {
       pub fn new(threshold_api_url: String, event_publisher: EventPublisher) -> Self {
           let threshold_client = ThresholdClient::new(threshold_api_url);
           Self {
               threshold_client,
               event_publisher,
           }
       }
       
       pub async fn check_thresholds(&self, metrics: &SystemMetrics) -> Result<Vec<ThresholdViolation>, String> {
           if metrics.instance_id.is_none() || metrics.account_id.is_none() {
               return Ok(vec![]);
           }
           
           let instance_id = metrics.instance_id.as_ref().unwrap();
           let user_id = metrics.account_id.as_ref().unwrap();
           
           // Get threshold configurations
           let configs = self.threshold_client.get_thresholds(instance_id, user_id).await?;
           
           // Create threshold manager
           let manager = ThresholdManager::new(configs);
           
           // Convert SystemMetrics to UsageMetrics
           let usage_metrics = metrics_to_usage_metrics(metrics)?;
           
           // Check thresholds
           let violations = manager.check_thresholds(&usage_metrics, instance_id, user_id);
           
           // Publish threshold violation events if any
           for violation in &violations {
               self.publish_violation(violation, metrics).await?;
           }
           
           Ok(violations)
       }
       
       async fn publish_violation(&self, violation: &ThresholdViolation, metrics: &SystemMetrics) -> Result<(), String> {
           // Create and publish threshold violation event
           // ...
       }
   }
   ```

### Task 3.3: Enhance API Layer (2 days)

1. Update API with authentication in `form-vm-metrics/src/main.rs`:
   ```rust
   use axum::{
       extract::{Path, State},
       routing::{get, post},
       Json, Router, http::{HeaderMap, StatusCode},
   };
   
   async fn get_metrics_authenticated(
       headers: HeaderMap,
       State(state): State<Arc<Mutex<SystemMetrics>>>,
   ) -> Result<Json<SystemMetrics>, StatusCode> {
       // Check auth token
       let auth_token = headers.get("Authorization")
           .ok_or(StatusCode::UNAUTHORIZED)?
           .to_str()
           .map_err(|_| StatusCode::UNAUTHORIZED)?;
       
       // Verify token (simplified)
       if !verify_token(auth_token) {
           return Err(StatusCode::UNAUTHORIZED);
       }
       
       // Return metrics
       let metrics = state.lock().await.clone();
       Ok(Json(metrics))
   }
   
   async fn get_instance_metrics(
       headers: HeaderMap,
       Path(instance_id): Path<String>,
       State(state): State<Arc<Mutex<SystemMetrics>>>,
   ) -> Result<Json<SystemMetrics>, StatusCode> {
       // Check auth and verify instance access
       // ...
       
       // Return metrics
       let metrics = state.lock().await.clone();
       
       // Verify this metrics is for the requested instance
       if metrics.instance_id.as_ref() != Some(&instance_id) {
           return Err(StatusCode::NOT_FOUND);
       }
       
       Ok(Json(metrics))
   }
   
   pub async fn serve(metrics: Arc<Mutex<SystemMetrics>>) -> Result<(), Box<dyn std::error::Error>> {
       let routes = Router::new()
           .route("/get", get(get_metrics))  // Keep old endpoint for backward compatibility
           .route("/api/v1/metrics", get(get_metrics_authenticated))
           .route("/api/v1/metrics/instances/:instance_id", get(get_instance_metrics))
           .route("/health", get(health_check))
           .with_state(metrics);
   
       // Rest of implementation...
   }
   
   async fn health_check() -> &'static str {
       "healthy"
   }
   ```

## Phase 4: Testing and Validation (Week 4)

### Task 4.1: Unit Tests (3 days)

1. Create tests for metrics collection:
   ```rust
   #[cfg(test)]
   mod tests {
       use super::*;
       
       #[tokio::test]
       async fn test_cpu_metrics_collection() {
           // Test CPU metrics collection
           // ...
       }
       
       #[tokio::test]
       async fn test_memory_metrics_collection() {
           // Test memory metrics collection
           // ...
       }
       
       // Additional tests...
   }
   ```

2. Create tests for event serialization/deserialization:
   ```rust
   #[cfg(test)]
   mod tests {
       use super::*;
       
       #[test]
       fn test_usage_event_serialization() {
           // Create a sample event
           let event = UsageEvent {
               // Initialize with test data
               // ...
           };
           
           // Serialize to JSON
           let json = serde_json::to_string(&event).unwrap();
           
           // Deserialize back
           let deserialized: UsageEvent = serde_json::from_str(&json).unwrap();
           
           // Assert values match
           assert_eq!(event.event_type, deserialized.event_type);
           // Additional assertions...
       }
   }
   ```

3. Create tests for threshold detection:
   ```rust
   #[cfg(test)]
   mod tests {
       use super::*;
       
       #[test]
       fn test_threshold_detection() {
           // Create sample metrics
           let metrics = UsageMetrics {
               // Initialize with test data
               // ...
           };
           
           // Create threshold configurations
           let configs = vec![
               ThresholdConfig {
                   // Initialize with test data
                   // ...
               },
           ];
           
           // Create threshold manager
           let manager = ThresholdManager::new(configs);
           
           // Check thresholds
           let violations = manager.check_thresholds(&metrics, "test-instance", "test-user");
           
           // Assert violations as expected
           assert_eq!(violations.len(), 1);
           // Additional assertions...
       }
   }
   ```

### Task 4.2: Integration Tests (2 days)

1. Create integration tests for event emission:
   ```rust
   #[cfg(test)]
   mod integration_tests {
       use super::*;
       
       #[tokio::test]
       async fn test_event_emission() {
           // Set up mock message queue
           let mock_server = MockServer::start().await;
           
           // Create publisher
           let publisher = EventPublisher::new(mock_server.url(), "test_topic".to_string());
           
           // Create sample event
           let event = UsageEvent {
               // Initialize with test data
               // ...
           };
           
           // Publish event
           let result = publisher.publish(event).await;
           
           // Assert published successfully
           assert!(result.is_ok());
           
           // Verify mock server received the event
           // ...
       }
   }
   ```

2. Create integration tests for threshold notifications:
   ```rust
   #[cfg(test)]
   mod integration_tests {
       use super::*;
       
       #[tokio::test]
       async fn test_threshold_notification() {
           // Set up mock threshold API and event publisher
           // ...
           
           // Create sample metrics that will trigger a threshold
           let metrics = SystemMetrics {
               // Initialize with test data
               // ...
           };
           
           // Create threshold checker
           let checker = MetricsThresholdChecker::new(mock_api_url, mock_publisher);
           
           // Check thresholds
           let violations = checker.check_thresholds(&metrics).await.unwrap();
           
           // Assert violations detected
           assert_eq!(violations.len(), 1);
           
           // Verify notification event was published
           // ...
       }
   }
   ```

### Task 4.3: Performance Tests (1 day)

1. Create performance tests for event emission:
   ```rust
   #[cfg(test)]
   mod performance_tests {
       use super::*;
       use std::time::Instant;
       
       #[tokio::test]
       async fn test_event_emission_performance() {
           // Set up publisher
           // ...
           
           const NUM_EVENTS: usize = 1000;
           
           // Create sample events
           let events: Vec<UsageEvent> = (0..NUM_EVENTS)
               .map(|i| {
                   // Create events
                   // ...
               })
               .collect();
           
           // Measure time to publish events
           let start = Instant::now();
           
           for event in events {
               publisher.publish(event).await.unwrap();
           }
           
           let elapsed = start.elapsed();
           
           println!("Published {} events in {:?}", NUM_EVENTS, elapsed);
           println!("Average time per event: {:?}", elapsed / NUM_EVENTS as u32);
           
           // Assert performance meets requirements
           assert!(elapsed.as_secs_f64() / NUM_EVENTS as f64 < 0.01); // Less than 10ms per event
       }
   }
   ```

2. Create performance tests for threshold checking:
   ```rust
   #[cfg(test)]
   mod performance_tests {
       use super::*;
       
       #[tokio::test]
       async fn test_threshold_checking_performance() {
           // Set up threshold manager with many configurations
           // ...
           
           // Create sample metrics
           // ...
           
           // Measure threshold checking performance
           let start = Instant::now();
           
           for _ in 0..1000 {
               let _ = manager.check_thresholds(&metrics, "test-instance", "test-user");
           }
           
           let elapsed = start.elapsed();
           
           println!("Performed 1000 threshold checks in {:?}", elapsed);
           println!("Average time per check: {:?}", elapsed / 1000);
           
           // Assert performance meets requirements
           assert!(elapsed.as_secs_f64() / 1000.0 < 0.001); // Less than 1ms per check
       }
   }
   ``` 