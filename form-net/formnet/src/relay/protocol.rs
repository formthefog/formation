//! Protocol definitions for relay communication
//!
//! This module defines the message formats and serialization
//! for relay-based communication.

use crate::relay::{RelayError, Result};
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
} 