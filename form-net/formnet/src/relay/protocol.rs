//! Protocol definitions for relay communication
//!
//! This module defines the message formats and serialization
//! for relay-based communication.

use crate::relay::{RelayError, Result};
use bincode::{deserialize, serialize};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::time::{SystemTime, UNIX_EPOCH};

/// The RelayHeader contains routing information for a relayed packet
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RelayHeader {
    /// Public key of the destination peer (in binary format)
    pub dest_peer_id: [u8; 32],
    
    /// Unique identifier for this relay session
    pub session_id: u64,
    
    /// Timestamp (seconds since UNIX epoch) for replay protection
    pub timestamp: u64,
    
    /// Flags for future extensions
    /// - Bit 0: Encrypted header
    /// - Bit 1: Ack required
    /// - Bits 2-7: Reserved
    pub flags: u8,
}

impl RelayHeader {
    /// Create a new relay header for the specified destination
    pub fn new(dest_peer_id: [u8; 32], session_id: u64) -> Self {
        // Get current time as seconds since UNIX epoch
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        
        Self {
            dest_peer_id,
            session_id,
            timestamp,
            flags: 0, // No flags set by default
        }
    }
    
    /// Check if the header has a specific flag set
    pub fn has_flag(&self, flag_bit: u8) -> bool {
        if flag_bit >= 8 {
            return false; // Only 8 bits available
        }
        (self.flags & (1 << flag_bit)) != 0
    }
    
    /// Set a specific flag in the header
    pub fn set_flag(&mut self, flag_bit: u8, value: bool) {
        if flag_bit >= 8 {
            return; // Only 8 bits available
        }
        
        if value {
            self.flags |= 1 << flag_bit;
        } else {
            self.flags &= !(1 << flag_bit);
        }
    }
    
    /// Check if header is valid (not too old, etc.)
    pub fn is_valid(&self) -> bool {
        // Get current time as seconds since UNIX epoch
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        
        // Reject headers more than 30 seconds old (replay protection)
        if now > self.timestamp && now - self.timestamp > 30 {
            return false;
        }
        
        // Reject headers from the future (with 5 second allowance for clock skew)
        if self.timestamp > now && self.timestamp - now > 5 {
            return false;
        }
        
        true
    }
}

/// A relay packet contains a header and payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelayPacket {
    /// Routing information for the packet
    pub header: RelayHeader,
    
    /// Encrypted packet payload (original WireGuard packet)
    pub payload: Vec<u8>,
}

impl RelayPacket {
    /// Create a new relay packet
    pub fn new(dest_peer_id: [u8; 32], session_id: u64, payload: Vec<u8>) -> Self {
        Self {
            header: RelayHeader::new(dest_peer_id, session_id),
            payload,
        }
    }
    
    /// Serialize the packet to binary format
    pub fn serialize(&self) -> Result<Vec<u8>> {
        bincode::serialize(self)
            .map_err(|e| RelayError::Serialization(e))
    }
    
    /// Deserialize from binary format
    pub fn deserialize(data: &[u8]) -> Result<Self> {
        bincode::deserialize(data)
            .map_err(|e| RelayError::Serialization(e))
    }
}

/// Connection request to establish a relay session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionRequest {
    /// Public key of the requesting peer (in binary format)
    pub peer_pubkey: [u8; 32],
    
    /// Public key of the target peer to connect to
    pub target_pubkey: [u8; 32],
    
    /// Timestamp for replay protection
    pub timestamp: u64,
    
    /// Random nonce for request uniqueness
    pub nonce: u64,
    
    /// Optional authentication token
    pub auth_token: Option<Vec<u8>>,
}

impl ConnectionRequest {
    /// Create a new connection request
    pub fn new(peer_pubkey: [u8; 32], target_pubkey: [u8; 32]) -> Self {
        Self {
            peer_pubkey,
            target_pubkey,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            nonce: rand::random::<u64>(), // Random nonce
            auth_token: None,
        }
    }
    
    /// Check if the request is valid (not too old, etc.)
    pub fn is_valid(&self) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        
        // Request should not be older than 60 seconds
        if now > self.timestamp && now - self.timestamp > 60 {
            return false;
        }
        
        // Request should not be from the future
        if self.timestamp > now && self.timestamp - now > 5 {
            return false;
        }
        
        true
    }
}

/// Response to a connection request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionResponse {
    /// Status of the connection request
    pub status: ConnectionStatus,
    
    /// Session ID assigned by the relay
    pub session_id: Option<u64>,
    
    /// Timestamp for replay protection
    pub timestamp: u64,
    
    /// Nonce from the original request (to prevent replay attacks)
    pub request_nonce: u64,
    
    /// Error message if status is not Success
    pub error: Option<String>,
}

/// Status codes for connection responses
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConnectionStatus {
    /// Connection request was successful
    Success,
    
    /// Connection request was rejected
    Rejected,
    
    /// Target peer is not reachable
    TargetUnreachable,
    
    /// Authentication failed
    AuthFailed,
    
    /// Resource limit reached
    ResourceLimit,
    
    /// Other error
    Error,
}

impl ConnectionResponse {
    /// Create a successful connection response
    pub fn success(request_nonce: u64, session_id: u64) -> Self {
        Self {
            status: ConnectionStatus::Success,
            session_id: Some(session_id),
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            request_nonce,
            error: None,
        }
    }
    
    /// Create an error response
    pub fn error(request_nonce: u64, status: ConnectionStatus, error: impl Into<String>) -> Self {
        Self {
            status,
            session_id: None,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            request_nonce,
            error: Some(error.into()),
        }
    }
    
    /// Check if the response is successful
    pub fn is_success(&self) -> bool {
        self.status == ConnectionStatus::Success && self.session_id.is_some()
    }
}

/// Heartbeat message to keep connection alive
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Heartbeat {
    /// Session ID to keep alive
    pub session_id: u64,
    
    /// Timestamp for replay protection
    pub timestamp: u64,
    
    /// Sequence number to detect dropped heartbeats
    pub sequence: u32,
}

impl Heartbeat {
    /// Create a new heartbeat message
    pub fn new(session_id: u64, sequence: u32) -> Self {
        Self {
            session_id,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            sequence,
        }
    }
}

/// Relay protocol message types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RelayMessage {
    /// Request to establish a relay connection
    ConnectionRequest(ConnectionRequest),
    
    /// Response to a connection request
    ConnectionResponse(ConnectionResponse),
    
    /// Packet to be forwarded
    ForwardPacket(RelayPacket),
    
    /// Keep-alive message
    Heartbeat(Heartbeat),
}

impl RelayMessage {
    /// Serialize the message to binary format
    pub fn serialize(&self) -> Result<Vec<u8>> {
        bincode::serialize(self)
            .map_err(|e| RelayError::Serialization(e))
    }
    
    /// Deserialize from binary format
    pub fn deserialize(data: &[u8]) -> Result<Self> {
        bincode::deserialize(data)
            .map_err(|e| RelayError::Serialization(e))
    }
    
    /// Get a timestamp for the message (used for timeout calculations)
    pub fn timestamp(&self) -> u64 {
        match self {
            RelayMessage::ConnectionRequest(req) => req.timestamp,
            RelayMessage::ConnectionResponse(resp) => resp.timestamp,
            RelayMessage::ForwardPacket(packet) => packet.header.timestamp,
            RelayMessage::Heartbeat(heartbeat) => heartbeat.timestamp,
        }
    }
    
    /// Check if the message is valid (not too old, etc.)
    pub fn is_valid(&self) -> bool {
        match self {
            RelayMessage::ConnectionRequest(req) => req.is_valid(),
            RelayMessage::ConnectionResponse(_) => true, // Responses are always considered valid
            RelayMessage::ForwardPacket(packet) => packet.header.is_valid(),
            RelayMessage::Heartbeat(_) => true, // Heartbeats are always considered valid
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_relay_header_flags() {
        let mut header = RelayHeader::new([0; 32], 1);
        
        // Initially no flags should be set
        assert!(!header.has_flag(0));
        assert!(!header.has_flag(1));
        
        // Set and check flags
        header.set_flag(0, true);
        assert!(header.has_flag(0));
        assert!(!header.has_flag(1));
        
        header.set_flag(1, true);
        assert!(header.has_flag(0));
        assert!(header.has_flag(1));
        
        // Unset and check
        header.set_flag(0, false);
        assert!(!header.has_flag(0));
        assert!(header.has_flag(1));
        
        // Test out of bounds flag (should be ignored)
        header.set_flag(10, true);
        assert!(!header.has_flag(10));
    }
    
    #[test]
    fn test_relay_header_validity() {
        let mut header = RelayHeader::new([0; 32], 1);
        
        // A fresh header should be valid
        assert!(header.is_valid());
        
        // Test header from the future
        header.timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() + 10; // 10 seconds in the future
        assert!(!header.is_valid());
        
        // Test old header
        header.timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() - 60; // 60 seconds in the past
        assert!(!header.is_valid());
    }
    
    #[test]
    fn test_relay_packet_serialization() {
        let original_packet = RelayPacket::new(
            [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32],
            12345,
            vec![10, 20, 30, 40, 50],
        );
        
        // Serialize the packet
        let serialized = original_packet.serialize().expect("Failed to serialize packet");
        
        // Deserialize and compare
        let deserialized = RelayPacket::deserialize(&serialized).expect("Failed to deserialize packet");
        
        // Check that the header data matches
        assert_eq!(deserialized.header.dest_peer_id, original_packet.header.dest_peer_id);
        assert_eq!(deserialized.header.session_id, original_packet.header.session_id);
        assert_eq!(deserialized.header.timestamp, original_packet.header.timestamp);
        assert_eq!(deserialized.header.flags, original_packet.header.flags);
        
        // Check that the payload matches
        assert_eq!(deserialized.payload, original_packet.payload);
    }
    
    #[test]
    fn test_connection_request_validity() {
        let request = ConnectionRequest::new([1; 32], [2; 32]);
        
        // A fresh request should be valid
        assert!(request.is_valid());
        
        // Create an old request
        let mut old_request = request.clone();
        old_request.timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() - 120; // 2 minutes in the past
        
        assert!(!old_request.is_valid());
        
        // Create a future request
        let mut future_request = request.clone();
        future_request.timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() + 30; // 30 seconds in the future
        
        assert!(!future_request.is_valid());
    }
    
    #[test]
    fn test_connection_response() {
        let success_response = ConnectionResponse::success(123456, 789);
        assert!(success_response.is_success());
        assert_eq!(success_response.session_id, Some(789));
        assert_eq!(success_response.request_nonce, 123456);
        
        let error_response = ConnectionResponse::error(
            123456, 
            ConnectionStatus::ResourceLimit,
            "Too many connections"
        );
        
        assert!(!error_response.is_success());
        assert_eq!(error_response.session_id, None);
        assert_eq!(error_response.status, ConnectionStatus::ResourceLimit);
        assert_eq!(error_response.error, Some("Too many connections".to_string()));
    }
    
    #[test]
    fn test_relay_message_serialization() {
        // Create a connection request message
        let request = ConnectionRequest::new([3; 32], [4; 32]);
        let message = RelayMessage::ConnectionRequest(request);
        
        // Serialize and deserialize
        let serialized = message.serialize().expect("Failed to serialize message");
        let deserialized = RelayMessage::deserialize(&serialized).expect("Failed to deserialize message");
        
        // Verify the type and timestamp match
        if let RelayMessage::ConnectionRequest(req) = &deserialized {
            assert_eq!(req.peer_pubkey, [3; 32]);
            assert_eq!(req.target_pubkey, [4; 32]);
        } else {
            panic!("Deserialized message has wrong type");
        }
        
        assert_eq!(message.timestamp(), deserialized.timestamp());
    }
} 