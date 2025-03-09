//! Protocol definitions for relay communication
//!
//! This module defines the message formats and serialization
//! for relay-based communication.

use crate::relay::{RelayError, Result};
use serde_json;
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
        serde_json::to_vec(self)
            .map_err(|e| RelayError::Protocol(format!("Serialization error: {}", e)))
    }
    
    /// Deserialize from binary format
    pub fn deserialize(data: &[u8]) -> Result<Self> {
        serde_json::from_slice(data)
            .map_err(|e| RelayError::Protocol(format!("Deserialization error: {}", e)))
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub latency: Option<u32>,
    
    /// Maximum concurrent sessions supported
    pub max_sessions: u32,
    
    /// Protocol version supported
    pub protocol_version: u16,
    
    /// Connection success rate (0-100, where 100 is perfect reliability)
    #[serde(default)]
    pub reliability: u8,
    
    /// Last connection success/failure timestamp
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_result_time: Option<u64>,
    
    /// Last measured packet loss percentage (0-100)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub packet_loss: Option<u8>,
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
            reliability: 100, // Start with perfect reliability until proven otherwise
            last_result_time: None,
            packet_loss: None,
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
    
    /// Update relay reliability based on connection success or failure
    pub fn update_reliability(&mut self, success: bool) {
        // Get current timestamp
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
            
        // Record the time of this result
        self.last_result_time = Some(now);
        
        // Update reliability with exponential weighted average
        // (more weight to recent results, but don't change too drastically)
        let current = self.reliability as f32;
        let new_value = if success { 100.0 } else { 0.0 };
        
        // Use 80/20 weighted average (80% existing, 20% new result)
        let updated = (current * 0.8) + (new_value * 0.2);
        self.reliability = updated.round() as u8;
    }
    
    /// Update packet loss information
    pub fn update_packet_loss(&mut self, loss_percentage: u8) {
        // Ensure the value is in valid range
        let loss = loss_percentage.min(100);
        
        // If we already have a packet loss value, use weighted average
        if let Some(current_loss) = self.packet_loss {
            // 70/30 weighted average (70% existing, 30% new measurement)
            let updated = (current_loss as f32 * 0.7) + (loss as f32 * 0.3);
            self.packet_loss = Some(updated.round() as u8);
        } else {
            // First measurement
            self.packet_loss = Some(loss);
        }
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
        serde_json::to_vec(self)
            .map_err(|e| RelayError::Protocol(format!("Serialization error: {}", e)))
    }
    
    /// Deserialize from binary format
    pub fn deserialize(data: &[u8]) -> Result<Self> {
        serde_json::from_slice(data)
            .map_err(|e| RelayError::Protocol(format!("Deserialization error: {}", e)))
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
        // Create a header
        let header = RelayHeader::new([1; 32], 12345);
        
        // Create a packet
        let payload = vec![0, 1, 2, 3, 4, 5];
        let packet = RelayPacket {
            header,
            payload: payload.clone(),
        };
        
        // Serialize and deserialize
        let serialized = serde_json::to_vec(&packet).expect("Failed to serialize packet");
        let deserialized: RelayPacket = serde_json::from_slice(&serialized).expect("Failed to deserialize packet");
        
        // Check the fields
        assert_eq!(deserialized.header.dest_peer_id, [1; 32]);
        assert_eq!(deserialized.header.session_id, 12345);
        assert_eq!(deserialized.payload, payload);
        
        // Test serialization through the helper methods
        let serialized2 = packet.serialize().expect("Failed to serialize packet using method");
        let deserialized2 = RelayPacket::deserialize(&serialized2).expect("Failed to deserialize packet using method");
        
        assert_eq!(deserialized2.header.session_id, 12345);
        assert_eq!(deserialized2.payload, payload);
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
        // Test valid announcement
        let relay_info = RelayNodeInfo::new([7; 32], vec!["192.168.1.1:8080".to_string()], 50);
        let announcement = RelayAnnouncement::new(relay_info, 3600);
        assert!(announcement.is_valid());
        
        // Test announcement with expired TTL
        let mut old_announcement = RelayAnnouncement::new(
            RelayNodeInfo::new([7; 32], vec!["192.168.1.1:8080".to_string()], 50),
            0
        );
        old_announcement.timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() - 7200; // 2 hours ago
        old_announcement.expires = old_announcement.timestamp + 3600; // expired 1 hour ago
        assert!(!old_announcement.is_valid());
    }
    
    #[test]
    fn test_relay_message_serialization() {
        // Create a connection request message
        let request = ConnectionRequest::new([3; 32], [4; 32]);
        let message = RelayMessage::ConnectionRequest(request);
        
        // Serialize and deserialize
        let serialized = serde_json::to_vec(&message).expect("Failed to serialize message");
        let deserialized: RelayMessage = serde_json::from_slice(&serialized).expect("Failed to deserialize message");
        
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
        // Test individual message components instead of the full message hierarchy
        
        // 1. Test RelayNodeInfo serialization first
        let relay_info = RelayNodeInfo::new(
            [8; 32],
            vec!["10.0.0.1:9000".to_string()],
            25
        ).with_region("us-east");
        
        println!("Original relay_info: {:?}", relay_info);
        let serialized_info = serde_json::to_vec(&relay_info).expect("Failed to serialize RelayNodeInfo");
        println!("Serialized info size: {} bytes", serialized_info.len());
        let deserialized_info: RelayNodeInfo = serde_json::from_slice(&serialized_info).expect("Failed to deserialize RelayNodeInfo");
        println!("Deserialized relay_info: {:?}", deserialized_info);
        
        assert_eq!(deserialized_info.pubkey, [8; 32]);
        assert_eq!(deserialized_info.endpoints, vec!["10.0.0.1:9000".to_string()]);
        assert_eq!(deserialized_info.region, Some("us-east".to_string()));
        assert_eq!(deserialized_info.max_sessions, 25);
        
        // 2. Test DiscoveryResponse serialization
        let response = DiscoveryResponse::new(
            12345678,
            vec![relay_info],
            false
        );
        
        println!("Original response: {:?}", response);
        let serialized_response = serde_json::to_vec(&response).expect("Failed to serialize DiscoveryResponse");
        println!("Serialized response size: {} bytes", serialized_response.len());
        let deserialized_response: DiscoveryResponse = serde_json::from_slice(&serialized_response).expect("Failed to deserialize DiscoveryResponse");
        println!("Deserialized response: {:?}", deserialized_response);
        
        assert_eq!(deserialized_response.request_nonce, 12345678);
        assert_eq!(deserialized_response.relays.len(), 1);
        assert_eq!(deserialized_response.relays[0].pubkey, [8; 32]);
        assert_eq!(deserialized_response.relays[0].region, Some("us-east".to_string()));
        
        // 3. Now test RelayMessage serialization with the DiscoveryResponse
        let message = RelayMessage::DiscoveryResponse(response);
        
        println!("Original message: {:?}", message);
        let serialized_message = serde_json::to_vec(&message).expect("Failed to serialize RelayMessage");
        println!("Serialized message size: {} bytes", serialized_message.len());
        let deserialized_message: RelayMessage = match serde_json::from_slice(&serialized_message) {
            Ok(msg) => {
                println!("Deserialization successful!");
                msg
            },
            Err(e) => {
                println!("Deserialization failed: {:?}", e);
                panic!("Failed to deserialize RelayMessage: {}", e);
            }
        };
        
        // Check that we got the right message type and contents
        match deserialized_message {
            RelayMessage::DiscoveryResponse(resp) => {
                assert_eq!(resp.request_nonce, 12345678);
                assert_eq!(resp.relays.len(), 1);
                assert_eq!(resp.relays[0].pubkey, [8; 32]);
            },
            _ => panic!("Wrong message type after deserialization"),
        }
        
        // 4. Test ConnectionRequest which is a different message type
        let req = ConnectionRequest::new([1; 32], [2; 32]);
        let serialized_req = serde_json::to_vec(&req).expect("Failed to serialize ConnectionRequest");
        let deserialized_req: ConnectionRequest = serde_json::from_slice(&serialized_req).expect("Failed to deserialize ConnectionRequest");
        
        assert_eq!(deserialized_req.peer_pubkey, [1; 32]);
        assert_eq!(deserialized_req.target_pubkey, [2; 32]);
    }

    // Test for simpler message types
    #[test]
    fn test_simple_discovery_serialization() {
        // Test ConnectionRequest/Response which are simpler message types
        let req = ConnectionRequest::new([1; 32], [2; 32]);
        let serialized_req = serde_json::to_vec(&req).expect("Failed to serialize ConnectionRequest");
        let deserialized_req: ConnectionRequest = serde_json::from_slice(&serialized_req).expect("Failed to deserialize ConnectionRequest");
        
        assert_eq!(deserialized_req.peer_pubkey, [1; 32]);
        assert_eq!(deserialized_req.target_pubkey, [2; 32]);
        
        let resp = ConnectionResponse::success(12345, 67890);
        let serialized_resp = serde_json::to_vec(&resp).expect("Failed to serialize ConnectionResponse");
        let deserialized_resp: ConnectionResponse = serde_json::from_slice(&serialized_resp).expect("Failed to deserialize ConnectionResponse");
        
        assert_eq!(deserialized_resp.request_nonce, 12345);
        assert_eq!(deserialized_resp.session_id, Some(67890));
        assert!(deserialized_resp.is_success());
    }
} 
