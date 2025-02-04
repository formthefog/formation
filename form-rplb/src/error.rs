use crate::protocol::Protocol;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ProxyError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("TLS error: {0}")]
    Tls(#[from] tokio_rustls::rustls::Error),
    
    #[error("No backend found for domain: {0}")]
    NoBackend(String),
    
    #[error("Protocol mismatch: expected {expected:?}, got {got:?}")]
    ProtocolMismatch {
        expected: Protocol,
        got: Protocol,
    },
    
    #[error("Invalid request: {0}")]
    InvalidRequest(String),
}
