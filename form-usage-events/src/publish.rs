use std::time::Duration;
use std::sync::Arc;

use form_p2p::queue::{QueueRequest, QueueResponse, QUEUE_PORT};
use reqwest::Client;
use serde::Serialize;
use tiny_keccak::{Hasher, Sha3};

use crate::{
    events::UsageEvent,
    errors::UsageEventError,
    retry::{RetryConfig, with_retry},
    circuit_breaker::{CircuitBreaker, CircuitBreakerConfig},
    threshold::ThresholdManager,
};

const DEFAULT_TOPIC: &str = "usage_events";
const DEFAULT_ENDPOINT: &str = "127.0.0.1";
const DEFAULT_SUBTOPIC: u8 = 0; // Using 0 for usage events (arbitrary choice)

/// Handles the publishing of usage events to the message queue
#[derive(Clone)]
pub struct EventPublisher {
    client: Client,
    queue_endpoint: String,
    topic: String,
    sub_topic: u8,
    retry_config: RetryConfig,
    circuit_breaker: Option<Arc<CircuitBreaker>>,
    threshold_manager: Option<Arc<ThresholdManager>>,
}

impl EventPublisher {
    /// Creates a new EventPublisher with default configuration
    pub fn new() -> Self {
        Self::with_config(DEFAULT_ENDPOINT.to_string(), QUEUE_PORT, DEFAULT_TOPIC.to_string(), DEFAULT_SUBTOPIC)
    }
    
    /// Creates a new EventPublisher with custom configuration
    pub fn with_config(endpoint: String, port: u16, topic: String, sub_topic: u8) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .unwrap_or_default();
            
        let queue_endpoint = format!("http://{}:{}/queue/write_local", endpoint, port);
        
        Self {
            client,
            queue_endpoint,
            topic,
            sub_topic,
            retry_config: RetryConfig::default(),
            circuit_breaker: None,
            threshold_manager: None,
        }
    }
    
    /// Sets a custom retry configuration
    pub fn with_retry_config(mut self, config: RetryConfig) -> Self {
        self.retry_config = config;
        self
    }
    
    /// Sets a circuit breaker with custom configuration
    pub fn with_circuit_breaker(mut self, config: CircuitBreakerConfig) -> Self {
        self.circuit_breaker = Some(Arc::new(CircuitBreaker::new(config)));
        self
    }
    
    /// Sets a circuit breaker with default configuration
    pub fn with_default_circuit_breaker(mut self) -> Self {
        self.circuit_breaker = Some(Arc::new(CircuitBreaker::new(CircuitBreakerConfig::default())));
        self
    }
    
    /// Sets a threshold manager for checking metrics against thresholds
    pub fn with_threshold_manager(mut self, manager: Arc<ThresholdManager>) -> Self {
        self.threshold_manager = Some(manager);
        self
    }
    
    /// Creates a threshold manager with the given config source and adds it to this publisher
    pub async fn with_new_threshold_manager(mut self, config_source: String) -> Result<Self, UsageEventError> {
        let manager = Arc::new(ThresholdManager::new(config_source));
        
        // Load initial configurations
        manager.load_configs().await?;
        
        self.threshold_manager = Some(manager);
        Ok(self)
    }
    
    /// Publishes a usage event to the message queue with retries
    pub async fn publish(&self, event: UsageEvent) -> Result<(), UsageEventError> {
        // Check thresholds if threshold manager is configured
        if let Some(ref manager) = self.threshold_manager {
            manager.check_event(&event).await?;
        }
        
        // Check circuit breaker state before proceeding
        if let Some(ref cb) = self.circuit_breaker {
            if !cb.allow_request().await {
                return Err(UsageEventError::CircuitBreakerOpen);
            }
        }
        
        let publisher = self.clone();
        let event_clone = event.clone();
        let circuit_breaker = self.circuit_breaker.clone();
        
        let result = with_retry(
            || {
                // Clone these values before moving them into the async block
                let pub_clone = publisher.clone();
                let evt_clone = event_clone.clone();
                
                async move {
                    pub_clone.publish_without_retry(evt_clone).await
                }
            },
            &self.retry_config
        ).await;
        
        // Update circuit breaker based on result
        if let Some(cb) = circuit_breaker {
            match &result {
                Ok(_) => cb.record_success().await,
                Err(_) => cb.record_failure().await,
            }
        }
        
        result
    }
    
    /// Publishes a usage event to the message queue without retries
    async fn publish_without_retry(&self, event: UsageEvent) -> Result<(), UsageEventError> {
        self.publish_message(event).await
    }
    
    /// Internal method to publish a serializable message
    async fn publish_message<T: Serialize + Clone>(&self, message: T) -> Result<(), UsageEventError> {
        // Create topic hash
        let mut hasher = Sha3::v256();
        let mut topic_hash = [0u8; 32];
        hasher.update(self.topic.as_bytes());
        hasher.finalize(&mut topic_hash);
        
        // Create message with sub_topic prefix
        let mut message_code = vec![self.sub_topic];
        message_code.extend(serde_json::to_vec(&message).map_err(UsageEventError::SerializationError)?);
        
        // Create queue request
        let request = QueueRequest::Write { 
            content: message_code, 
            topic: hex::encode(topic_hash) 
        };

        // Send request to queue
        let response = self.client
            .post(&self.queue_endpoint)
            .json(&request)
            .send()
            .await
            .map_err(|e| UsageEventError::ConnectionError(e.to_string()))?
            .json::<QueueResponse>()
            .await
            .map_err(|e| UsageEventError::ConnectionError(e.to_string()))?;
            
        // Handle response
        match response {
            QueueResponse::OpSuccess => Ok(()),
            QueueResponse::Failure { reason } => {
                Err(UsageEventError::PublishError(format!("{reason:?}")))
            },
            _ => Err(UsageEventError::PublishError(
                "Invalid response variant for write_local endpoint".to_string()
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::{UsageMetrics, UsagePeriod};
    use crate::circuit_breaker::CircuitState;
    use std::time::Duration;
    use tokio::time::sleep;
    
    #[tokio::test]
    async fn test_event_publisher_creation() {
        let publisher = EventPublisher::new();
        assert_eq!(publisher.topic, DEFAULT_TOPIC);
        assert_eq!(publisher.sub_topic, DEFAULT_SUBTOPIC);
        assert!(publisher.queue_endpoint.contains(DEFAULT_ENDPOINT));
    }
    
    #[tokio::test]
    async fn test_custom_retry_config() {
        let custom_config = RetryConfig {
            max_retries: 5,
            initial_backoff: Duration::from_millis(200),
            max_backoff: Duration::from_secs(20),
            backoff_multiplier: 3.0,
            jitter_factor: 0.2,
        };
        
        let publisher = EventPublisher::new().with_retry_config(custom_config.clone());
        
        assert_eq!(publisher.retry_config.max_retries, 5);
        assert_eq!(publisher.retry_config.initial_backoff, Duration::from_millis(200));
        assert_eq!(publisher.retry_config.max_backoff, Duration::from_secs(20));
        assert_eq!(publisher.retry_config.backoff_multiplier, 3.0);
        assert_eq!(publisher.retry_config.jitter_factor, 0.2);
    }
    
    struct MockPublisher {
        circuit_breaker: Arc<CircuitBreaker>,
        // Controls whether publish_without_retry should succeed or fail
        should_fail: std::sync::atomic::AtomicBool,
    }
    
    impl MockPublisher {
        fn new(circuit_config: CircuitBreakerConfig) -> Self {
            Self {
                circuit_breaker: Arc::new(CircuitBreaker::new(circuit_config)),
                should_fail: std::sync::atomic::AtomicBool::new(true),
            }
        }
        
        fn set_should_fail(&self, should_fail: bool) {
            self.should_fail.store(should_fail, std::sync::atomic::Ordering::SeqCst);
        }
        
        async fn get_state(&self) -> CircuitState {
            self.circuit_breaker.get_state().await
        }
        
        /// Directly trigger enough failures to open the circuit
        async fn trigger_open_circuit(&self) {
            println!("Triggering enough failures to open circuit");
            
            // Hardcode to 2 failures for test simplicity
            let threshold = 2;
            
            // Record failures directly until threshold is reached
            for i in 0..threshold {
                println!("Recording failure {}/{}", i+1, threshold);
                self.circuit_breaker.record_failure().await;
                
                // Small delay to ensure state updates
                sleep(Duration::from_millis(10)).await;
            }
            
            // Verify circuit is now open
            let state = self.circuit_breaker.get_state().await;
            println!("Circuit state after triggering failures: {:?}", state);
            assert_eq!(state, CircuitState::Open);
        }
        
        async fn publish(&self, _event: UsageEvent) -> Result<(), UsageEventError> {
            println!("\n>> Publishing event...");
            
            // Check current circuit state for diagnostics
            let current_state = self.circuit_breaker.get_state().await;
            println!("Circuit state before request: {:?}", current_state);
            
            // First check if the circuit breaker allows the request
            let request_allowed = self.circuit_breaker.allow_request().await;
            println!("Circuit breaker allow_request result: {}", request_allowed);
            
            if !request_allowed {
                println!("Circuit is open, rejecting request immediately");
                return Err(UsageEventError::CircuitBreakerOpen);
            }
            
            println!("Request allowed, checking if should fail");
            
            // Small delay to ensure state transitions are complete
            sleep(Duration::from_millis(10)).await;
            
            // Get the state again after allow_request might have changed it
            let updated_state = self.circuit_breaker.get_state().await;
            println!("Circuit state after allow_request: {:?}", updated_state);
            
            // Simulate success or failure based on the flag
            let result = if self.should_fail.load(std::sync::atomic::Ordering::SeqCst) {
                println!("Simulating connection failure");
                Err(UsageEventError::ConnectionError("Mock connection error".to_string()))
            } else {
                println!("Simulating success");
                Ok(())
            };
            
            // Update circuit breaker state based on result
            match &result {
                Ok(_) => {
                    println!("Recording success");
                    self.circuit_breaker.record_success().await;
                },
                Err(_) => {
                    println!("Recording failure");
                    self.circuit_breaker.record_failure().await;
                },
            }
            
            // Check state after recording result
            let new_state = self.circuit_breaker.get_state().await;
            println!("Circuit state after recording result: {:?}", new_state);
            
            println!("<< Publishing event complete");
            
            result
        }
    }
    
    #[tokio::test]
    async fn test_circuit_breaker_closed_state() {
        println!("\n=== TEST CIRCUIT BREAKER CLOSED STATE ===");
        
        let mock_publisher = MockPublisher::new(CircuitBreakerConfig {
            failure_threshold: 2,
            reset_timeout: Duration::from_millis(100),
            half_open_allowed_calls: 1,
        });
        
        let event = create_test_event();
        
        // In closed state, requests should succeed if not set to fail
        mock_publisher.set_should_fail(false);
        
        let result = mock_publisher.publish(event.clone()).await;
        assert!(result.is_ok());
        
        // Verify still in closed state
        assert_eq!(mock_publisher.get_state().await, CircuitState::Closed);
    }
    
    #[tokio::test]
    async fn test_circuit_breaker_open_state() {
        println!("\n=== TEST CIRCUIT BREAKER OPEN STATE ===");
        
        let mock_publisher = MockPublisher::new(CircuitBreakerConfig {
            failure_threshold: 2,
            reset_timeout: Duration::from_millis(100),
            half_open_allowed_calls: 1,
        });
        
        let event = create_test_event();
        
        // Open the circuit with manual failures
        println!("Opening circuit with manual failures");
        mock_publisher.trigger_open_circuit().await;
        
        // Ensure we're in Open state
        assert_eq!(mock_publisher.get_state().await, CircuitState::Open);
        
        // Manually create an new instance of MockPublisher just to test the rejection logic
        let mock_publisher = MockPublisher::new(CircuitBreakerConfig {
            failure_threshold: 2,
            reset_timeout: Duration::from_secs(60), // Long timeout
            half_open_allowed_calls: 1,
        });
        
        // Open the circuit
        mock_publisher.trigger_open_circuit().await;
        
        // Immediately try a request - should be rejected
        let result = mock_publisher.publish(event.clone()).await;
        
        // Check that we got a circuit breaker error
        assert!(matches!(result, Err(UsageEventError::CircuitBreakerOpen)));
    }
    
    #[tokio::test]
    async fn test_circuit_breaker_half_open_state() {
        println!("\n=== TEST CIRCUIT BREAKER HALF-OPEN STATE ===");
        
        let mock_publisher = MockPublisher::new(CircuitBreakerConfig {
            failure_threshold: 2,
            reset_timeout: Duration::from_millis(100),
            half_open_allowed_calls: 1,
        });
        
        let event = create_test_event();
        
        // Open the circuit
        mock_publisher.trigger_open_circuit().await;
        
        // Wait for reset timeout to transition to half-open on next request
        println!("Waiting for reset timeout...");
        sleep(Duration::from_millis(150)).await;
        
        // Set to success for the half-open test
        mock_publisher.set_should_fail(false);
        
        // This request should succeed and close the circuit
        let result = mock_publisher.publish(event.clone()).await;
        assert!(result.is_ok());
        
        // Circuit should now be closed
        assert_eq!(mock_publisher.get_state().await, CircuitState::Closed);
    }
    
    #[tokio::test]
    async fn test_threshold_manager_integration() {
        // Create a publisher with a threshold manager
        let publisher = EventPublisher::new()
            .with_new_threshold_manager("test".to_string())
            .await
            .unwrap();
        
        // Create an event that exceeds CPU threshold
        let high_cpu_event = UsageEvent {
            event_type: "resource_usage".to_string(),
            version: "1.0".to_string(),
            timestamp: chrono::Utc::now().timestamp(),
            instance_id: "test-instance".to_string(),
            user_id: "test-user".to_string(),
            org_id: None,
            metrics: UsageMetrics {
                cpu_seconds: 30,
                cpu_percent_avg: 95.0, // Exceeds 90% threshold from default config
                memory_gb: 4.0,
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
        
        // This will fail due to no message queue, but we should see threshold violation output
        let _ = publisher.publish(high_cpu_event).await;
        
        // Create an event below all thresholds
        let low_usage_event = UsageEvent {
            event_type: "resource_usage".to_string(),
            version: "1.0".to_string(),
            timestamp: chrono::Utc::now().timestamp(),
            instance_id: "test-instance".to_string(),
            user_id: "test-user".to_string(),
            org_id: None,
            metrics: UsageMetrics {
                cpu_seconds: 30,
                cpu_percent_avg: 50.0, // Below 90% threshold
                memory_gb: 2.0,        // Below 8GB threshold
                memory_percent: 25.0,
                storage_gb: 5.0,
                network_egress_mb: 50.0,
                network_ingress_mb: 25.0,
                gpu_seconds: 0,
            },
            period: UsagePeriod {
                start: chrono::Utc::now().timestamp() - 30,
                end: chrono::Utc::now().timestamp(),
            },
        };
        
        // Should not trigger threshold violation output
        let _ = publisher.publish(low_usage_event).await;
    }
    
    /// Helper function to create a test event
    fn create_test_event() -> UsageEvent {
        UsageEvent {
            event_type: "resource_usage".to_string(),
            version: "1.0".to_string(),
            timestamp: 1234567890,
            instance_id: "test-instance".to_string(),
            user_id: "test-user".to_string(),
            org_id: Some("test-org".to_string()),
            metrics: UsageMetrics {
                cpu_seconds: 30,
                cpu_percent_avg: 12.5,
                memory_gb: 4.2,
                memory_percent: 52.5,
                storage_gb: 25.7,
                network_egress_mb: 15.2,
                network_ingress_mb: 8.7,
                gpu_seconds: 0,
            },
            period: UsagePeriod {
                start: 1234567800,
                end: 1234567890,
            },
        }
    }
} 
