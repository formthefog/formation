//! Protocol definitions for relay communication
//!
//! This module defines the message formats and serialization
//! for relay-based communication.

use crate::relay::{RelayError, Result};
use bincode;
use serde::{Deserialize, Serialize};
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

/// Relay capabilities flags
pub const RELAY_CAP_IPV4: u32 = 1 << 0;
pub const RELAY_CAP_IPV6: u32 = 1 << 1;
pub const RELAY_CAP_TCP_FALLBACK: u32 = 1 << 2;
pub const RELAY_CAP_HIGH_BANDWIDTH: u32 = 1 << 3;
pub const RELAY_CAP_LOW_LATENCY: u32 = 1 << 4;

/// A query to discover relay nodes in the network
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveryQuery {
    /// Public key of the querying peer
    pub peer_pubkey: [u8; 32],
    
    /// Timestamp for replay protection
    pub timestamp: u64,
    
    /// Random nonce for request uniqueness
    pub nonce: u64,
    
    /// Optional geographic region to filter relays
    pub region: Option<String>,
    
    /// Minimum capabilities required (bitmask)
    pub min_capabilities: u32,
    
    /// Maximum number of relays to return
    pub max_results: u32,
}

impl DiscoveryQuery {
    /// Create a new discovery query
    pub fn new(peer_pubkey: [u8; 32], max_results: u32) -> Self {
        Self {
            peer_pubkey,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            nonce: rand::random::<u64>(),
            region: None,
            min_capabilities: 0, // No specific capabilities required
            max_results,
        }
    }
    
    /// Check if the query is valid
    pub fn is_valid(&self) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        
        // Query should not be older than 30 seconds
        if now > self.timestamp && now - self.timestamp > 30 {
            return false;
        }
        
        // Query should not be from the future
        if self.timestamp > now && self.timestamp - now > 5 {
            return false;
        }
        
        true
    }
    
    /// Set required region for filtering
    pub fn with_region(mut self, region: impl Into<String>) -> Self {
        self.region = Some(region.into());
        self
    }
    
    /// Set minimum capabilities required
    pub fn with_capabilities(mut self, capabilities: u32) -> Self {
        self.min_capabilities = capabilities;
        self
    }
}

/// Information about a relay node
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelayNodeInfo {
    /// Public key of the relay
    pub pubkey: [u8; 32],
    
    /// Endpoints (addresses) where the relay can be reached
    pub endpoints: Vec<String>,
    
    /// Geographic region of the relay
    pub region: Option<String>,
    
    /// Capabilities offered by this relay (bitmask)
    pub capabilities: u32,
    
    /// Current load factor (0-100, where 0 is idle and 100 is fully loaded)
    pub load: u8,
    
    /// Estimated latency in milliseconds
    pub latency: Option<u32>,
    
    /// Maximum concurrent sessions supported
    pub max_sessions: u32,
    
    /// Protocol version supported
    pub protocol_version: u16,
}

impl RelayNodeInfo {
    /// Create new relay node info with minimal information
    pub fn new(pubkey: [u8; 32], endpoints: Vec<String>, max_sessions: u32) -> Self {
        Self {
            pubkey,
            endpoints,
            region: None,
            capabilities: RELAY_CAP_IPV4, // Default to IPv4 only
            load: 0,
            latency: None,
            max_sessions,
            protocol_version: 1, // Current version
        }
    }
    
    /// Check if the relay has a specific capability
    pub fn has_capability(&self, capability: u32) -> bool {
        (self.capabilities & capability) != 0
    }
    
    /// Add a capability to the relay
    pub fn add_capability(&mut self, capability: u32) {
        self.capabilities |= capability;
    }
    
    /// Set the geographic region
    pub fn with_region(mut self, region: impl Into<String>) -> Self {
        self.region = Some(region.into());
        self
    }
    
    /// Set the current load factor
    pub fn with_load(mut self, load: u8) -> Self {
        self.load = std::cmp::min(load, 100);
        self
    }
    
    /// Set the estimated latency
    pub fn with_latency(mut self, latency: u32) -> Self {
        self.latency = Some(latency);
        self
    }
}

/// Response to a discovery query
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveryResponse {
    /// Nonce from the original query
    pub request_nonce: u64,
    
    /// Timestamp for replay protection
    pub timestamp: u64,
    
    /// List of relays matching the query
    pub relays: Vec<RelayNodeInfo>,
    
    /// Whether more relays are available
    pub more_available: bool,
}

impl DiscoveryResponse {
    /// Create a new discovery response
    pub fn new(request_nonce: u64, relays: Vec<RelayNodeInfo>, more_available: bool) -> Self {
        Self {
            request_nonce,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            relays,
            more_available,
        }
    }
}

/// Announcement of a relay's availability
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelayAnnouncement {
    /// Information about the relay
    pub relay_info: RelayNodeInfo,
    
    /// Timestamp for freshness and replay protection
    pub timestamp: u64,
    
    /// Expiration time (seconds since UNIX epoch), or 0 for no expiration
    pub expires: u64,
    
    /// Digital signature of the announcement
    pub signature: Option<Vec<u8>>,
}

impl RelayAnnouncement {
    /// Create a new relay announcement
    pub fn new(relay_info: RelayNodeInfo, ttl_secs: u64) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        
        let expires = if ttl_secs > 0 { now + ttl_secs } else { 0 };
        
        Self {
            relay_info,
            timestamp: now,
            expires,
            signature: None,
        }
    }
    
    /// Check if the announcement is valid and not expired
    pub fn is_valid(&self) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        
        // Check if expired
        if self.expires > 0 && now > self.expires {
            return false;
        }
        
        // Announcement should not be older than 24 hours
        if now > self.timestamp && now - self.timestamp > 86400 {
            return false;
        }
        
        // Announcement should not be from the future
        if self.timestamp > now && self.timestamp - now > 300 {
            return false;
        }
        
        true
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
    
    /// Query to discover relays
    DiscoveryQuery(DiscoveryQuery),
    
    /// Response to a discovery query
    DiscoveryResponse(DiscoveryResponse),
    
    /// Announcement of relay availability
    RelayAnnouncement(RelayAnnouncement),
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
            RelayMessage::DiscoveryQuery(query) => query.timestamp,
            RelayMessage::DiscoveryResponse(resp) => resp.timestamp,
            RelayMessage::RelayAnnouncement(announcement) => announcement.timestamp,
        }
    }
    
    /// Check if the message is valid (not too old, etc.)
    pub fn is_valid(&self) -> bool {
        match self {
            RelayMessage::ConnectionRequest(req) => req.is_valid(),
            RelayMessage::ConnectionResponse(_) => true, // Responses are always considered valid
            RelayMessage::ForwardPacket(packet) => packet.header.is_valid(),
            RelayMessage::Heartbeat(_) => true, // Heartbeats are always considered valid
            RelayMessage::DiscoveryQuery(query) => query.is_valid(),
            RelayMessage::DiscoveryResponse(_) => true, // Responses are always considered valid
            RelayMessage::RelayAnnouncement(announcement) => announcement.is_valid(),
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
    fn test_discovery_query_validity() {
        let query = DiscoveryQuery::new([5; 32], 10);
        
        // A fresh query should be valid
        assert!(query.is_valid());
        
        // Create an old query
        let mut old_query = query.clone();
        old_query.timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() - 60; // 1 minute in the past
        
        assert!(!old_query.is_valid());
        
        // Query with region and capabilities
        let query_with_region = query.clone().with_region("us-west");
        assert_eq!(query_with_region.region, Some("us-west".to_string()));
        
        let query_with_caps = query.with_capabilities(RELAY_CAP_IPV4 | RELAY_CAP_LOW_LATENCY);
        assert_eq!(query_with_caps.min_capabilities, RELAY_CAP_IPV4 | RELAY_CAP_LOW_LATENCY);
    }
    
    #[test]
    fn test_relay_node_info() {
        let mut relay_info = RelayNodeInfo::new(
            [6; 32], 
            vec!["1.2.3.4:12345".to_string(), "example.com:8000".to_string()],
            100
        );
        
        // Test default values
        assert_eq!(relay_info.pubkey, [6; 32]);
        assert_eq!(relay_info.endpoints.len(), 2);
        assert_eq!(relay_info.capabilities, RELAY_CAP_IPV4);
        assert_eq!(relay_info.max_sessions, 100);
        
        // Test capability methods
        assert!(relay_info.has_capability(RELAY_CAP_IPV4));
        assert!(!relay_info.has_capability(RELAY_CAP_IPV6));
        
        relay_info.add_capability(RELAY_CAP_IPV6);
        assert!(relay_info.has_capability(RELAY_CAP_IPV6));
        
        // Test builder methods
        let relay_info = relay_info
            .with_region("eu-central")
            .with_load(75)
            .with_latency(20);
        
        assert_eq!(relay_info.region, Some("eu-central".to_string()));
        assert_eq!(relay_info.load, 75);
        assert_eq!(relay_info.latency, Some(20));
    }
    
    #[test]
    fn test_relay_announcement_validity() {
        let relay_info = RelayNodeInfo::new(
            [7; 32],
            vec!["192.168.1.1:8000".to_string()],
            50
        );
        
        // Create an announcement that expires in 1 hour
        let announcement = RelayAnnouncement::new(relay_info, 3600);
        
        // A fresh announcement should be valid
        assert!(announcement.is_valid());
        
        // Test expired announcement
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        let mut expired_announcement = announcement.clone();
        expired_announcement.expires = now - 100; // Expired 100 seconds ago
        
        assert!(!expired_announcement.is_valid());
        
        // Test old announcement
        let mut old_announcement = announcement;
        old_announcement.timestamp = now - 100000; // ~27 hours ago
        old_announcement.expires = 0; // No expiration
        
        assert!(!old_announcement.is_valid());
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
    
    #[test]
    fn test_discovery_message_serialization() {
        // Create relay node info
        let relay_info = RelayNodeInfo::new(
            [8; 32],
            vec!["10.0.0.1:9000".to_string()],
            25
        ).with_region("us-east");
        
        // Create a discovery response
        let response = DiscoveryResponse::new(
            12345678,
            vec![relay_info],
            false
        );
        
        // Create a relay message from it
        let message = RelayMessage::DiscoveryResponse(response);
        
        // Serialize and deserialize
        let serialized = message.serialize().expect("Failed to serialize message");
        let deserialized = RelayMessage::deserialize(&serialized).expect("Failed to deserialize message");
        
        // Verify the type and contents match
        if let RelayMessage::DiscoveryResponse(resp) = &deserialized {
            assert_eq!(resp.request_nonce, 12345678);
            assert_eq!(resp.relays.len(), 1);
            assert_eq!(resp.relays[0].pubkey, [8; 32]);
            assert_eq!(resp.relays[0].region, Some("us-east".to_string()));
        } else {
            panic!("Deserialized message has wrong type");
        }
    }
} 