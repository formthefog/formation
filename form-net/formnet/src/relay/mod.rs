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
pub use discovery::{RelayRegistry, SharedRelayRegistry, BootstrapConfig, BootstrapRelay};
pub use manager::{RelayManager, ConnectionAttemptStatus, PacketReceiver};

// Re-export CacheIntegration
pub use manager::CacheIntegration;

use std::sync::atomic::{AtomicBool, Ordering};
use once_cell::sync::Lazy;
use std::net::{UdpSocket, SocketAddr};
use std::time::Duration;
use log::{info, debug};

// Global flag to track if relay functionality should be enabled
static RELAY_ENABLED: Lazy<AtomicBool> = Lazy::new(|| AtomicBool::new(false));

// Global flag to track if we've done automatic detection
static AUTO_DETECTED: Lazy<AtomicBool> = Lazy::new(|| AtomicBool::new(false));

/// Check if relay functionality is enabled
pub fn is_relay_enabled() -> bool {
    if !AUTO_DETECTED.load(Ordering::Relaxed) {
        // Only auto-detect once
        auto_detect_relay_need();
    }
    
    RELAY_ENABLED.load(Ordering::Relaxed)
}

/// Explicitly enable or disable relay functionality
pub fn set_relay_enabled(enabled: bool) {
    info!("Relay functionality {} explicitly", if enabled { "enabled" } else { "disabled" });
    RELAY_ENABLED.store(enabled, Ordering::Relaxed);
    AUTO_DETECTED.store(true, Ordering::Relaxed); // Skip auto-detection
}

/// NAT traversal difficulty level
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum NatDifficulty {
    /// Open internet, no NAT (direct connections likely to work)
    Open,
    /// Simple NAT, should work with direct connections
    Simple,
    /// Moderate NAT, might require NAT traversal but direct connections can work
    Moderate,
    /// Difficult NAT, direct connections less likely to work
    Difficult,
    /// Symmetric NAT, direct connections unlikely to work, relay recommended
    Symmetric,
    /// Unknown NAT type, couldn't determine
    Unknown,
}

/// Auto-detect if relay functionality should be enabled based on network conditions
fn auto_detect_relay_need() {
    info!("Auto-detecting network conditions to determine if relay functionality is needed");
    
    // Mark as auto-detected so we don't repeat this
    AUTO_DETECTED.store(true, Ordering::Relaxed);
    
    // Perform NAT type detection to determine if relay is likely needed
    let should_enable = match detect_nat_type() {
        NatDifficulty::Open | NatDifficulty::Simple => {
            // Easy NAT situation, direct connections likely to work
            debug!("Detected simple NAT configuration, relay functionality not strictly needed");
            false
        },
        NatDifficulty::Moderate => {
            // Moderate NAT, might work with direct connections but keep relay as backup
            debug!("Detected moderate NAT complexity, keeping relay as backup");
            true
        },
        NatDifficulty::Difficult | NatDifficulty::Symmetric => {
            // Difficult NAT situation, relay likely needed
            info!("Detected difficult NAT configuration, relay functionality enabled");
            true
        },
        NatDifficulty::Unknown => {
            // Can't determine, so enable relay to be safe
            debug!("Could not determine NAT type, enabling relay as precaution");
            true
        }
    };
    
    RELAY_ENABLED.store(should_enable, Ordering::Relaxed);
    info!("Relay functionality auto-detection complete: {}", if should_enable { "enabled" } else { "disabled" });
}

/// Detect NAT type to determine if relay functionality is likely to be needed
/// This is a simplified NAT detection implementation
fn detect_nat_type() -> NatDifficulty {
    // Use a list of public STUN servers for testing
    let stun_servers = [
        "stun.l.google.com:19302",
        "stun1.l.google.com:19302",
        "stun2.l.google.com:19302",
        "stun.ekiga.net:3478",
    ];
    
    // Try to determine if we can establish direct UDP connections
    for stun_server in &stun_servers {
        if let Ok(socket) = UdpSocket::bind("0.0.0.0:0") {
            // Set a short timeout
            if socket.set_read_timeout(Some(Duration::from_secs(2))).is_err() {
                continue;
            }
            
            // Try to connect to the STUN server
            if let Ok(addr) = stun_server.parse::<SocketAddr>() {
                if socket.connect(addr).is_ok() {
                    // Send a dummy packet to the STUN server
                    if socket.send(&[0, 1, 0, 8]).is_ok() {
                        // We were able to send a packet to a public STUN server
                        // This suggests we have outbound UDP connectivity
                        let mut buf = [0u8; 32];
                        // Try to receive a response
                        if socket.recv(&mut buf).is_ok() {
                            // We received a response, which suggests our NAT allows incoming traffic
                            // on at least this specific port combination
                            debug!("Successfully contacted STUN server at {}", stun_server);
                            return NatDifficulty::Moderate;
                        } else {
                            // Couldn't receive, which might suggest a symmetric NAT
                            debug!("Could send to but not receive from {}", stun_server);
                            return NatDifficulty::Difficult;
                        }
                    }
                }
            }
        }
    }
    
    // If we couldn't even connect to any STUN servers, assume the worst
    debug!("Could not contact any STUN servers, NAT might be very restrictive");
    NatDifficulty::Symmetric
}

// Errors specific to the relay system
use thiserror::Error;

#[derive(Error, Debug)]
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

pub type Result<T> = std::result::Result<T, RelayError>; 