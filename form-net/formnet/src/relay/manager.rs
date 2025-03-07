//! Relay connection management
//!
//! This module handles establishing and managing relay connections.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant, SystemTime};

use crate::relay::{
    ConnectionRequest, ConnectionResponse, ConnectionStatus, RelayError, RelayMessage,
    RelayNodeInfo, Result, SharedRelayRegistry
};

/// Default timeout for relay connection attempts
const CONNECTION_TIMEOUT: Duration = Duration::from_secs(10);

/// Default session expiration time
const SESSION_EXPIRATION: Duration = Duration::from_secs(3600); // 1 hour

/// Default heartbeat interval
const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(30);

/// Default heartbeat interval for tests (very short)
#[cfg(test)]
const TEST_HEARTBEAT_INTERVAL: Duration = Duration::from_millis(1);

/// Activity timeout - how long before a session is considered inactive
const ACTIVITY_TIMEOUT: Duration = Duration::from_secs(120);

/// Connection attempt status
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConnectionAttemptStatus {
    /// Connection attempt is in progress
    InProgress,
    /// Connection was established successfully
    Success,
    /// Connection attempt failed
    Failed(String),
    /// Connection attempt timed out
    Timeout,
}

/// Represents a relay connection attempt
#[derive(Debug)]
struct ConnectionAttempt {
    /// Target peer public key
    pub target_pubkey: [u8; 32],
    
    /// Relay node being used for this attempt
    pub relay_info: RelayNodeInfo,
    
    /// When the connection attempt was started
    pub started_at: Instant,
    
    /// Current status of the connection attempt
    pub status: ConnectionAttemptStatus,
    
    /// Session ID if connection was successful
    pub session_id: Option<u64>,
    
    /// Last error message if connection failed
    pub error: Option<String>,
}

/// Represents an active relay session
#[derive(Debug)]
struct RelaySession {
    /// Session ID assigned by the relay
    pub session_id: u64,
    
    /// Remote peer public key
    pub peer_pubkey: [u8; 32],
    
    /// Relay node information
    pub relay_info: RelayNodeInfo,
    
    /// When the session was established
    pub established_at: SystemTime,
    
    /// When the session expires
    pub expires_at: SystemTime,
    
    /// Last activity time
    pub last_activity: Instant,
    
    /// Last heartbeat sent
    pub last_heartbeat: Instant,
    
    /// Number of packets sent through this session
    pub packets_sent: u64,
    
    /// Number of packets received through this session
    pub packets_received: u64,
    
    /// Current sequence number for heartbeats
    pub heartbeat_sequence: u32,
    
    /// Whether the session is marked for cleanup
    pub marked_for_cleanup: bool,
}

/// Manages relay connections and sessions
#[derive(Debug)]
pub struct RelayManager {
    /// Registry of available relay nodes
    relay_registry: SharedRelayRegistry,
    
    /// Currently active sessions
    sessions: RwLock<HashMap<u64, RelaySession>>,
    
    /// Ongoing connection attempts
    connection_attempts: Mutex<Vec<ConnectionAttempt>>,
    
    /// Session ID to peer public key mapping for fast lookups
    session_to_peer: RwLock<HashMap<u64, [u8; 32]>>,
    
    /// Peer public key to session ID mapping for fast lookups
    peer_to_session: RwLock<HashMap<String, u64>>,
    
    /// Our local public key
    local_pubkey: [u8; 32],
}

impl RelayManager {
    /// Create a new relay manager
    pub fn new(relay_registry: SharedRelayRegistry, local_pubkey: [u8; 32]) -> Self {
        Self {
            relay_registry,
            sessions: RwLock::new(HashMap::new()),
            connection_attempts: Mutex::new(Vec::new()),
            session_to_peer: RwLock::new(HashMap::new()),
            peer_to_session: RwLock::new(HashMap::new()),
            local_pubkey,
        }
    }
    
    /// Get the number of active sessions
    pub fn session_count(&self) -> Result<usize> {
        Ok(self.sessions.read().map_err(|_| 
            RelayError::Protocol("Failed to acquire read lock on sessions".into()))?.len())
    }
    
    /// Get the number of ongoing connection attempts
    pub fn connection_attempt_count(&self) -> Result<usize> {
        Ok(self.connection_attempts.lock().map_err(|_| 
            RelayError::Protocol("Failed to acquire lock on connection attempts".into()))?.len())
    }
    
    /// Check if we have an active session with a peer
    pub fn has_active_session(&self, peer_pubkey: &[u8; 32]) -> Result<bool> {
        let peer_key = hex::encode(peer_pubkey);
        let peer_to_session = self.peer_to_session.read().map_err(|_| 
            RelayError::Protocol("Failed to acquire read lock on peer_to_session".into()))?;
        
        Ok(peer_to_session.contains_key(&peer_key))
    }
    
    /// Get session ID for a peer if one exists
    pub fn get_session_for_peer(&self, peer_pubkey: &[u8; 32]) -> Result<Option<u64>> {
        let peer_key = hex::encode(peer_pubkey);
        let peer_to_session = self.peer_to_session.read().map_err(|_| 
            RelayError::Protocol("Failed to acquire read lock on peer_to_session".into()))?;
        
        Ok(peer_to_session.get(&peer_key).copied())
    }
    
    /// Get peer public key for a session ID
    pub fn get_peer_for_session(&self, session_id: u64) -> Result<Option<[u8; 32]>> {
        let session_to_peer = self.session_to_peer.read().map_err(|_| 
            RelayError::Protocol("Failed to acquire read lock on session_to_peer".into()))?;
        
        Ok(session_to_peer.get(&session_id).copied())
    }
    
    /// Track a new connection attempt
    pub fn track_connection_attempt(
        &self,
        target_pubkey: [u8; 32],
        relay_info: RelayNodeInfo
    ) -> Result<()> {
        let attempt = ConnectionAttempt {
            target_pubkey,
            relay_info,
            started_at: Instant::now(),
            status: ConnectionAttemptStatus::InProgress,
            session_id: None,
            error: None,
        };
        
        let mut attempts = self.connection_attempts.lock().map_err(|_| 
            RelayError::Protocol("Failed to acquire lock on connection attempts".into()))?;
        
        attempts.push(attempt);
        Ok(())
    }
    
    /// Update connection attempt status
    pub fn update_connection_attempt(
        &self,
        target_pubkey: &[u8; 32],
        status: ConnectionAttemptStatus,
        session_id: Option<u64>
    ) -> Result<()> {
        let mut attempts = self.connection_attempts.lock().map_err(|_| 
            RelayError::Protocol("Failed to acquire lock on connection attempts".into()))?;
        
        // Find the index of the matching attempt
        let mut attempt_index = None;
        for (i, attempt) in attempts.iter().enumerate() {
            if attempt.target_pubkey == *target_pubkey {
                attempt_index = Some(i);
                break;
            }
        }
        
        // If we found a matching attempt, update it
        if let Some(idx) = attempt_index {
            let attempt = &mut attempts[idx];
            attempt.status = status.clone();
            attempt.session_id = session_id;
            
            if let ConnectionAttemptStatus::Failed(error) = &status {
                attempt.error = Some(error.clone());
            }
            
            // If the connection was successful, create a session
            if let Some(sid) = session_id {
                if status == ConnectionAttemptStatus::Success {
                    let relay_info = attempt.relay_info.clone();
                    self.create_session(sid, *target_pubkey, relay_info)?;
                }
            }
            
            // If the attempt is no longer in progress, remove it from the list
            if status != ConnectionAttemptStatus::InProgress {
                attempts.remove(idx);
            }
        }
        
        Ok(())
    }
    
    /// Create a new relay session
    fn create_session(
        &self,
        session_id: u64,
        peer_pubkey: [u8; 32],
        relay_info: RelayNodeInfo
    ) -> Result<()> {
        let now = SystemTime::now();
        let expires_at = now + SESSION_EXPIRATION;
        let current_instant = Instant::now();
        
        let session = RelaySession {
            session_id,
            peer_pubkey,
            relay_info,
            established_at: now,
            expires_at,
            last_activity: current_instant,
            last_heartbeat: current_instant,
            packets_sent: 0,
            packets_received: 0,
            heartbeat_sequence: 0,
            marked_for_cleanup: false,
        };
        
        // Add to sessions map
        {
            let mut sessions = self.sessions.write().map_err(|_| 
                RelayError::Protocol("Failed to acquire write lock on sessions".into()))?;
            sessions.insert(session_id, session);
        }
        
        // Update lookup maps
        {
            let mut session_to_peer = self.session_to_peer.write().map_err(|_| 
                RelayError::Protocol("Failed to acquire write lock on session_to_peer".into()))?;
            session_to_peer.insert(session_id, peer_pubkey);
        }
        
        {
            let mut peer_to_session = self.peer_to_session.write().map_err(|_| 
                RelayError::Protocol("Failed to acquire write lock on peer_to_session".into()))?;
            peer_to_session.insert(hex::encode(&peer_pubkey), session_id);
        }
        
        Ok(())
    }
    
    /// Close a relay session
    pub fn close_session(&self, session_id: u64) -> Result<bool> {
        // Remove from sessions map
        let session_removed = {
            let mut sessions = self.sessions.write().map_err(|_| 
                RelayError::Protocol("Failed to acquire write lock on sessions".into()))?;
            sessions.remove(&session_id).is_some()
        };
        
        if session_removed {
            // Get peer pubkey for this session
            let peer_pubkey = {
                let mut session_to_peer = self.session_to_peer.write().map_err(|_| 
                    RelayError::Protocol("Failed to acquire write lock on session_to_peer".into()))?;
                session_to_peer.remove(&session_id)
            };
            
            // If we found the peer pubkey, remove it from peer_to_session map
            if let Some(pubkey) = peer_pubkey {
                let mut peer_to_session = self.peer_to_session.write().map_err(|_| 
                    RelayError::Protocol("Failed to acquire write lock on peer_to_session".into()))?;
                peer_to_session.remove(&hex::encode(&pubkey));
            }
        }
        
        Ok(session_removed)
    }
    
    /// Clean up expired sessions and timed out connection attempts
    pub fn cleanup(&self) -> Result<(usize, usize)> {
        let now = SystemTime::now();
        let current_instant = Instant::now();
        
        // Find expired or inactive sessions
        let expired_sessions: Vec<u64> = {
            let sessions = self.sessions.read().map_err(|_| 
                RelayError::Protocol("Failed to acquire read lock on sessions".into()))?;
            
            sessions.iter()
                .filter(|(_, session)| {
                    // Check if session has expired or has been inactive for too long
                    let expired = now > session.expires_at;
                    let inactive = current_instant.duration_since(session.last_activity) > ACTIVITY_TIMEOUT;
                    let marked = session.marked_for_cleanup;
                    
                    expired || inactive || marked
                })
                .map(|(session_id, _)| *session_id)
                .collect()
        };
        
        // Close expired sessions
        let mut closed_count = 0;
        for session_id in expired_sessions {
            if self.close_session(session_id)? {
                closed_count += 1;
            }
        }
        
        // Clean up timed out connection attempts
        let mut attempts = self.connection_attempts.lock().map_err(|_| 
            RelayError::Protocol("Failed to acquire lock on connection attempts".into()))?;
        
        let before_len = attempts.len();
        
        // Remove completed or timed out attempts
        attempts.retain(|attempt| {
            let in_progress = attempt.status == ConnectionAttemptStatus::InProgress;
            let not_timed_out = current_instant.duration_since(attempt.started_at) < CONNECTION_TIMEOUT;
            
            in_progress && not_timed_out
        });
        
        let removed_attempts = before_len - attempts.len();
        
        Ok((closed_count, removed_attempts))
    }
    
    /// Mark a session as active
    pub fn mark_session_active(&self, session_id: u64) -> Result<bool> {
        let mut sessions = self.sessions.write().map_err(|_| 
            RelayError::Protocol("Failed to acquire write lock on sessions".into()))?;
        
        if let Some(session) = sessions.get_mut(&session_id) {
            session.last_activity = Instant::now();
            return Ok(true);
        }
        
        Ok(false)
    }
    
    /// Update session statistics
    pub fn record_packet_sent(&self, session_id: u64) -> Result<bool> {
        let mut sessions = self.sessions.write().map_err(|_| 
            RelayError::Protocol("Failed to acquire write lock on sessions".into()))?;
        
        if let Some(session) = sessions.get_mut(&session_id) {
            session.packets_sent += 1;
            session.last_activity = Instant::now();
            return Ok(true);
        }
        
        Ok(false)
    }
    
    /// Record a received packet
    pub fn record_packet_received(&self, session_id: u64) -> Result<bool> {
        let mut sessions = self.sessions.write().map_err(|_| 
            RelayError::Protocol("Failed to acquire write lock on sessions".into()))?;
        
        if let Some(session) = sessions.get_mut(&session_id) {
            session.packets_received += 1;
            session.last_activity = Instant::now();
            return Ok(true);
        }
        
        Ok(false)
    }
    
    /// Get a list of sessions that need heartbeats
    pub fn get_sessions_needing_heartbeat(&self) -> Result<Vec<(u64, RelayNodeInfo)>> {
        let current_instant = Instant::now();
        let sessions = self.sessions.read().map_err(|_| 
            RelayError::Protocol("Failed to acquire read lock on sessions".into()))?;
        
        let mut results = Vec::new();
        
        for (session_id, session) in sessions.iter() {
            let time_since_heartbeat = current_instant.duration_since(session.last_heartbeat);
            
            #[cfg(test)]
            let interval = TEST_HEARTBEAT_INTERVAL;
            
            #[cfg(not(test))]
            let interval = HEARTBEAT_INTERVAL;
            
            if time_since_heartbeat >= interval {
                results.push((*session_id, session.relay_info.clone()));
            }
        }
        
        Ok(results)
    }
    
    /// Update the heartbeat timestamp for a session
    pub fn update_heartbeat(&self, session_id: u64) -> Result<u32> {
        let mut sessions = self.sessions.write().map_err(|_| 
            RelayError::Protocol("Failed to acquire write lock on sessions".into()))?;
        
        if let Some(session) = sessions.get_mut(&session_id) {
            session.last_heartbeat = Instant::now();
            session.heartbeat_sequence += 1;
            return Ok(session.heartbeat_sequence);
        }
        
        Err(RelayError::Protocol(format!("Session {} not found", session_id)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::relay::{RELAY_CAP_IPV4, RELAY_CAP_IPV6};
    
    fn create_test_relay(id: u8) -> RelayNodeInfo {
        let mut pubkey = [0u8; 32];
        pubkey[0] = id;
        
        let mut relay = RelayNodeInfo::new(
            pubkey,
            vec![format!("192.168.1.{}:8080", id)],
            10
        );
        
        relay.capabilities = RELAY_CAP_IPV4 | RELAY_CAP_IPV6;
        relay.region = Some(format!("region-{}", id));
        relay.load = 10 * id as u8;
        
        relay
    }
    
    fn create_test_pubkey(id: u8) -> [u8; 32] {
        let mut pubkey = [0u8; 32];
        pubkey[0] = id;
        pubkey
    }
    
    #[test]
    fn test_relay_manager_basic() {
        // Create a registry
        let registry = SharedRelayRegistry::new();
        
        // Create a manager
        let local_pubkey = create_test_pubkey(99);
        let manager = RelayManager::new(registry, local_pubkey);
        
        // Initially, there should be no sessions or attempts
        assert_eq!(manager.session_count().unwrap(), 0);
        assert_eq!(manager.connection_attempt_count().unwrap(), 0);
        
        // Track a connection attempt
        let target_pubkey = create_test_pubkey(1);
        let relay_info = create_test_relay(2);
        manager.track_connection_attempt(target_pubkey, relay_info.clone()).unwrap();
        
        // Now there should be one attempt
        let attempt_count = manager.connection_attempt_count().unwrap();
        println!("Attempt count after tracking: {}", attempt_count);
        assert_eq!(attempt_count, 1);
        
        // Update the attempt to success and create a session
        manager.update_connection_attempt(
            &target_pubkey,
            ConnectionAttemptStatus::Success,
            Some(12345)
        ).unwrap();
        
        // Check connection attempts after update
        let attempt_count = manager.connection_attempt_count().unwrap();
        println!("Attempt count after update: {}", attempt_count);
        // The attempt should be removed immediately upon successful update
        assert_eq!(attempt_count, 0);
        
        // Now there should be one session
        assert_eq!(manager.session_count().unwrap(), 1);
        
        // We should have a session for the peer
        assert!(manager.has_active_session(&target_pubkey).unwrap());
        assert_eq!(manager.get_session_for_peer(&target_pubkey).unwrap(), Some(12345));
        assert_eq!(manager.get_peer_for_session(12345).unwrap(), Some(target_pubkey));
        
        // Record some activity
        assert!(manager.mark_session_active(12345).unwrap());
        assert!(manager.record_packet_sent(12345).unwrap());
        assert!(manager.record_packet_received(12345).unwrap());
        
        // Sleep a tiny bit to allow the test heartbeat interval to pass
        std::thread::sleep(std::time::Duration::from_millis(5));
        
        // Test heartbeat tracking
        let sessions_needing_heartbeat = manager.get_sessions_needing_heartbeat().unwrap();
        println!("Sessions needing heartbeat: {}", sessions_needing_heartbeat.len());
        assert_eq!(sessions_needing_heartbeat.len(), 1);
        
        if !sessions_needing_heartbeat.is_empty() {
            println!("First session ID: {}", sessions_needing_heartbeat[0].0);
            assert_eq!(sessions_needing_heartbeat[0].0, 12345);
        }
        
        let sequence = manager.update_heartbeat(12345).unwrap();
        println!("Heartbeat sequence: {}", sequence);
        assert_eq!(sequence, 1);
        
        // Check connection attempts before cleanup
        let attempt_count = manager.connection_attempt_count().unwrap();
        println!("Attempt count before cleanup: {}", attempt_count);
        
        // Clean up should not affect our active session yet
        let (closed, removed) = manager.cleanup().unwrap();
        println!("Cleanup results - closed: {}, removed: {}", closed, removed);
        assert_eq!(closed, 0);
        assert_eq!(removed, 0); // No attempts to remove as they're already gone
        
        // Check connection attempts after cleanup
        let attempt_count = manager.connection_attempt_count().unwrap();
        println!("Attempt count after cleanup: {}", attempt_count);
        
        // Close the session
        assert!(manager.close_session(12345).unwrap());
        
        // Session should be gone
        assert_eq!(manager.session_count().unwrap(), 0);
        assert!(!manager.has_active_session(&target_pubkey).unwrap());
    }
} 