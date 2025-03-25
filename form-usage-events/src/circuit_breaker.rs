use std::sync::Arc;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::time::Duration;
use tokio::sync::RwLock;

/// The possible states of a circuit breaker
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitState {
    /// Circuit is closed and requests flow normally
    Closed,
    /// Circuit is open and requests are immediately rejected
    Open,
    /// Circuit is allowing a limited number of requests to test recovery
    HalfOpen,
}

/// Configuration for the circuit breaker
pub struct CircuitBreakerConfig {
    /// Number of consecutive failures that will trip the circuit
    pub failure_threshold: usize,
    /// Time duration to wait before transitioning from Open to HalfOpen
    pub reset_timeout: Duration,
    /// Maximum number of calls allowed in HalfOpen state
    pub half_open_allowed_calls: usize,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 5,
            reset_timeout: Duration::from_secs(30),
            half_open_allowed_calls: 1,
        }
    }
}

/// Circuit breaker implementation that prevents calls to failing services
pub struct CircuitBreaker {
    state: Arc<RwLock<CircuitState>>,
    failures: Arc<AtomicUsize>,
    successes: Arc<AtomicUsize>,
    last_failure_time: Arc<AtomicU64>,
    config: CircuitBreakerConfig,
}

impl CircuitBreaker {
    /// Create a new circuit breaker with the given configuration
    pub fn new(config: CircuitBreakerConfig) -> Self {
        Self {
            state: Arc::new(RwLock::new(CircuitState::Closed)),
            failures: Arc::new(AtomicUsize::new(0)),
            successes: Arc::new(AtomicUsize::new(0)),
            last_failure_time: Arc::new(AtomicU64::new(0)),
            config,
        }
    }

    /// Check if a request is allowed to proceed
    pub async fn allow_request(&self) -> bool {
        let state = *self.state.read().await;
        println!("allow_request called, current state: {:?}", state);
        
        match state {
            CircuitState::Closed => {
                println!("Circuit is CLOSED, allowing request");
                true
            },
            CircuitState::Open => {
                // Check if enough time has passed to transition to half-open
                let last_failure = self.last_failure_time.load(Ordering::SeqCst);
                
                // Using SystemTime instead of Instant for time calculations
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();
                    
                let elapsed = now.saturating_sub(last_failure);
                
                println!(
                    "Circuit is OPEN, last_failure: {}, now: {}, elapsed: {}, reset_timeout: {}",
                    last_failure, now, elapsed, self.config.reset_timeout.as_secs()
                );
                
                if elapsed >= self.config.reset_timeout.as_secs() {
                    // Transition to half-open
                    println!("Transitioning to HALF-OPEN");
                    *self.state.write().await = CircuitState::HalfOpen;
                    self.successes.store(0, Ordering::SeqCst);
                    true
                } else {
                    println!("Staying OPEN, rejecting request");
                    false
                }
            },
            CircuitState::HalfOpen => {
                // Allow a limited number of calls in half-open state
                let current_calls = self.successes.load(Ordering::SeqCst);
                let allowed = current_calls < self.config.half_open_allowed_calls;
                
                println!(
                    "Circuit is HALF-OPEN, current_calls: {}, allowed_calls: {}, allowing: {}",
                    current_calls, self.config.half_open_allowed_calls, allowed
                );
                
                allowed
            }
        }
    }

    /// Record a successful operation
    pub async fn record_success(&self) {
        let state = *self.state.read().await;
        println!("record_success called, current state: {:?}", state);
        
        match state {
            CircuitState::Closed => {
                // Reset failure count on success in closed state
                println!("Success in CLOSED state, resetting failure count");
                self.failures.store(0, Ordering::SeqCst);
            },
            CircuitState::HalfOpen => {
                // Count successes in half-open state
                let success_count = self.successes.fetch_add(1, Ordering::SeqCst) + 1;
                println!("Success in HALF-OPEN state, count: {}/{}", success_count, self.config.half_open_allowed_calls);
                
                // If we've had enough successes, close the circuit
                if success_count >= self.config.half_open_allowed_calls {
                    println!("Success threshold reached, closing circuit");
                    *self.state.write().await = CircuitState::Closed;
                    self.failures.store(0, Ordering::SeqCst);
                    self.successes.store(0, Ordering::SeqCst);
                }
            },
            CircuitState::Open => {
                // This shouldn't happen, but handle it gracefully
                // by treating it as a success in half-open state
                println!("Unexpected success in OPEN state, transitioning to HALF-OPEN");
                *self.state.write().await = CircuitState::HalfOpen;
                self.successes.store(1, Ordering::SeqCst);
            }
        }
    }

    /// Record a failed operation
    pub async fn record_failure(&self) {
        let state = *self.state.read().await;
        println!("record_failure called, current state: {:?}", state);
        
        // Record the time of this failure using SystemTime
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
            
        self.last_failure_time.store(now, Ordering::SeqCst);
        println!("Stored failure time: {}", now);
        
        match state {
            CircuitState::Closed => {
                // Increment failure counter
                let failures = self.failures.fetch_add(1, Ordering::SeqCst) + 1;
                println!("Failure count in CLOSED state: {}/{}", failures, self.config.failure_threshold);
                
                // If we hit the threshold, open the circuit
                if failures >= self.config.failure_threshold {
                    println!("Threshold reached, opening circuit");
                    *self.state.write().await = CircuitState::Open;
                }
            },
            CircuitState::HalfOpen => {
                // Any failure in half-open state opens the circuit again
                println!("Failure in HALF-OPEN state, opening circuit");
                *self.state.write().await = CircuitState::Open;
                self.successes.store(0, Ordering::SeqCst);
            },
            CircuitState::Open => {
                // Circuit already open, just update the failure time
                println!("Failure in OPEN state, updating failure time");
            }
        }
    }

    /// Get the current state of the circuit breaker
    pub async fn get_state(&self) -> CircuitState {
        *self.state.read().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_initial_state_is_closed() {
        let cb = CircuitBreaker::new(CircuitBreakerConfig::default());
        assert_eq!(cb.get_state().await, CircuitState::Closed);
        assert!(cb.allow_request().await);
    }
    
    #[tokio::test]
    async fn test_opens_after_threshold_failures() {
        let config = CircuitBreakerConfig {
            failure_threshold: 3,
            reset_timeout: Duration::from_secs(5),
            half_open_allowed_calls: 1,
        };
        
        let cb = CircuitBreaker::new(config);
        
        // Record failures up to threshold
        cb.record_failure().await;
        assert_eq!(cb.get_state().await, CircuitState::Closed);
        
        cb.record_failure().await;
        assert_eq!(cb.get_state().await, CircuitState::Closed);
        
        cb.record_failure().await;
        assert_eq!(cb.get_state().await, CircuitState::Open);
        
        // Requests should be rejected
        assert!(!cb.allow_request().await);
    }
    
    #[tokio::test]
    async fn test_transitions_to_half_open_after_timeout() {
        let config = CircuitBreakerConfig {
            failure_threshold: 1,
            reset_timeout: Duration::from_millis(100), // Short timeout for testing
            half_open_allowed_calls: 1,
        };
        
        let cb = CircuitBreaker::new(config);
        
        // Open the circuit
        cb.record_failure().await;
        assert_eq!(cb.get_state().await, CircuitState::Open);
        
        // Wait for timeout
        sleep(Duration::from_millis(150)).await;
        
        // Should transition to half-open and allow one request
        assert!(cb.allow_request().await);
        assert_eq!(cb.get_state().await, CircuitState::HalfOpen);
    }
    
    #[tokio::test]
    async fn test_closes_after_success_in_half_open() {
        let config = CircuitBreakerConfig {
            failure_threshold: 1,
            reset_timeout: Duration::from_millis(100),
            half_open_allowed_calls: 1,
        };
        
        let cb = CircuitBreaker::new(config);
        
        // Open the circuit
        cb.record_failure().await;
        assert_eq!(cb.get_state().await, CircuitState::Open);
        
        // Wait for timeout
        sleep(Duration::from_millis(150)).await;
        
        // Transition to half-open
        assert!(cb.allow_request().await);
        assert_eq!(cb.get_state().await, CircuitState::HalfOpen);
        
        // Record success in half-open state
        cb.record_success().await;
        
        // Should close the circuit
        assert_eq!(cb.get_state().await, CircuitState::Closed);
    }
    
    #[tokio::test]
    async fn test_reopens_on_failure_in_half_open() {
        let config = CircuitBreakerConfig {
            failure_threshold: 1,
            reset_timeout: Duration::from_millis(100),
            half_open_allowed_calls: 2,
        };
        
        let cb = CircuitBreaker::new(config);
        
        // Open the circuit
        cb.record_failure().await;
        assert_eq!(cb.get_state().await, CircuitState::Open);
        
        // Wait for timeout
        sleep(Duration::from_millis(150)).await;
        
        // Transition to half-open
        assert!(cb.allow_request().await);
        assert_eq!(cb.get_state().await, CircuitState::HalfOpen);
        
        // Record failure in half-open state
        cb.record_failure().await;
        
        // Should open the circuit again
        assert_eq!(cb.get_state().await, CircuitState::Open);
    }
} 
