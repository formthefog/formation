//! Decentralized relay system for NAT traversal
//!
//! This module implements a relay-based fallback for cases where
//! direct WireGuard connections cannot be established due to
//! NAT restrictions or firewall limitations.

// Declare submodules (to be implemented)
pub mod protocol;
pub mod discovery;
pub mod manager;
pub mod service;

// Re-export key structures
pub use protocol::{
    RelayHeader, RelayPacket, RelayMessage, 
    ConnectionRequest, ConnectionResponse, ConnectionStatus, Heartbeat,
    DiscoveryQuery, DiscoveryResponse, RelayNodeInfo, RelayAnnouncement,
    RELAY_CAP_IPV4, RELAY_CAP_IPV6, RELAY_CAP_TCP_FALLBACK, RELAY_CAP_HIGH_BANDWIDTH, RELAY_CAP_LOW_LATENCY
};
// pub use discovery::{RelayRegistry, RelayNodeInfo};
// pub use manager::{RelayManager, RelayConnection};
// pub use service::{RelayService, RelayNode};

/// Error type for relay operations
#[derive(Debug, thiserror::Error)]
pub enum RelayError {
    /// I/O error
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    
    /// Serialization error
    #[error("Serialization error: {0}")]
    Serialization(#[from] bincode::Error),
    
    /// Protocol error
    #[error("Protocol error: {0}")]
    Protocol(String),
    
    /// Authentication error
    #[error("Authentication error: {0}")]
    Authentication(String),
    
    /// Resource limit reached
    #[error("Resource limit reached: {0}")]
    ResourceLimit(String),
}

/// Result type for relay operations
pub type Result<T> = std::result::Result<T, RelayError>; 