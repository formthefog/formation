use thiserror::Error;

/// Error types for usage event operations
#[derive(Error, Debug)]
pub enum UsageEventError {
    /// Error during event serialization
    #[error("Failed to serialize event: {0}")]
    SerializationError(#[from] serde_json::Error),
    
    /// Error during event publishing to the message queue
    #[error("Failed to publish event: {0}")]
    PublishError(String),
    
    /// Error when the circuit breaker is open
    #[error("Circuit breaker open, not sending request")]
    CircuitBreakerOpen,
    
    /// Error when connecting to the message queue
    #[error("Failed to connect to message queue: {0}")]
    ConnectionError(String),
    
    /// Error during HTTP communication
    #[error("HTTP request failed: {0}")]
    HttpError(#[from] reqwest::Error),
    
    /// Generic error type for other failures
    #[error("Operation failed: {0}")]
    Other(String),
} 