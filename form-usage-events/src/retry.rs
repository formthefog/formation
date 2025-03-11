use std::time::Duration;
use rand::Rng;
use futures::Future;

use crate::errors::UsageEventError;

/// Configuration for retry behavior
#[derive(Clone, Debug)]
pub struct RetryConfig {
    /// Maximum number of retry attempts
    pub max_retries: u32,
    
    /// Initial backoff duration before first retry
    pub initial_backoff: Duration,
    
    /// Maximum backoff duration
    pub max_backoff: Duration,
    
    /// Multiplier for exponential backoff
    pub backoff_multiplier: f64,
    
    /// Factor to apply random jitter (0-1)
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

/// Executes an async operation with retry logic
///
/// # Arguments
/// * `operation` - The async operation to execute
/// * `config` - Retry configuration
///
/// # Returns
/// Result of the operation or the last error encountered
pub async fn with_retry<F, Fut, T>(
    mut operation: F,
    config: &RetryConfig,
) -> Result<T, UsageEventError>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T, UsageEventError>>,
{
    let mut current_retry = 0;
    let mut current_backoff = config.initial_backoff;
    
    loop {
        match operation().await {
            Ok(result) => return Ok(result),
            Err(error) => {
                // If we've reached the maximum retry count, return the error
                if current_retry >= config.max_retries {
                    return Err(error);
                }
                
                // Only retry for network/IO errors, not for client errors
                match &error {
                    UsageEventError::ConnectionError(_) | 
                    UsageEventError::HttpError(_) |
                    UsageEventError::PublishError(_) => {
                        // Proceed with retry
                    },
                    UsageEventError::CircuitBreakerOpen => {
                        // Don't retry if circuit breaker is open
                        return Err(error);
                    },
                    UsageEventError::SerializationError(_) |
                    UsageEventError::Other(_) => {
                        // Don't retry for these errors as they're not likely
                        // to be resolved by retrying
                        return Err(error);
                    },
                }
                
                // Calculate sleep duration with jitter
                let jitter_range = (current_backoff.as_millis() as f64 * config.jitter_factor) as u64;
                let jitter = if jitter_range > 0 {
                    rand::thread_rng().gen_range(0..jitter_range)
                } else {
                    0
                };
                
                let sleep_duration = current_backoff.saturating_add(Duration::from_millis(jitter));
                
                // Sleep before retrying
                tokio::time::sleep(sleep_duration).await;
                
                // Update for next iteration
                current_retry += 1;
                
                // Calculate next backoff with exponential increase
                let next_backoff_millis = current_backoff.as_millis() as f64 * config.backoff_multiplier;
                current_backoff = Duration::from_millis(
                    next_backoff_millis.min(config.max_backoff.as_millis() as f64) as u64
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::Arc;
    
    #[tokio::test]
    async fn test_successful_operation() {
        let config = RetryConfig::default();
        let attempt_counter = Arc::new(AtomicU32::new(0));
        
        let counter = attempt_counter.clone();
        let operation = move || {
            let counter_clone = counter.clone();
            async move {
                counter_clone.fetch_add(1, Ordering::SeqCst);
                Ok::<_, UsageEventError>("success")
            }
        };
        
        let result = with_retry(operation, &config).await;
        
        assert_eq!(result.unwrap(), "success");
        assert_eq!(attempt_counter.load(Ordering::SeqCst), 1);
    }
    
    #[tokio::test]
    async fn test_retry_until_success() {
        let config = RetryConfig {
            max_retries: 3,
            initial_backoff: Duration::from_millis(10),
            max_backoff: Duration::from_millis(100),
            backoff_multiplier: 2.0,
            jitter_factor: 0.1,
        };
        
        let attempt_counter = Arc::new(AtomicU32::new(0));
        
        let counter = attempt_counter.clone();
        let operation = move || {
            let counter_clone = counter.clone();
            async move {
                let attempts = counter_clone.fetch_add(1, Ordering::SeqCst);
                
                // Succeed on the third attempt
                if attempts < 2 {
                    Err(UsageEventError::ConnectionError("temporary error".to_string()))
                } else {
                    Ok::<_, UsageEventError>("success")
                }
            }
        };
        
        let result = with_retry(operation, &config).await;
        
        assert_eq!(result.unwrap(), "success");
        assert_eq!(attempt_counter.load(Ordering::SeqCst), 3);
    }
    
    #[tokio::test]
    async fn test_max_retries_exceeded() {
        let attempt_counter = Arc::new(AtomicU32::new(0));
        
        let counter = attempt_counter.clone();
        let operation = move || {
            let counter_clone = counter.clone();
            async move {
                counter_clone.fetch_add(1, Ordering::SeqCst);
                Err(UsageEventError::ConnectionError("always fails".to_string()))
            }
        };
        
        let config = RetryConfig::default();
        
        let result: Result<String, UsageEventError> = with_retry(operation, &config).await;
        
        assert!(result.is_err());
        
        assert_eq!(attempt_counter.load(Ordering::SeqCst), config.max_retries as u32 + 1);
    }
    
    #[tokio::test]
    async fn test_non_retryable_error() {
        let attempt_counter = Arc::new(AtomicU32::new(0));
        
        let counter = attempt_counter.clone();
        let operation = move || {
            let counter_clone = counter.clone();
            async move {
                counter_clone.fetch_add(1, Ordering::SeqCst);
                Err(UsageEventError::SerializationError(serde_json::Error::io(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "non-retryable error"
                ))))
            }
        };
        
        let config = RetryConfig::default();
        
        let result: Result<String, UsageEventError> = with_retry(operation, &config).await;
        
        assert!(result.is_err());
        
        assert_eq!(attempt_counter.load(Ordering::SeqCst), 1);
    }
} 