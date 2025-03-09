//! Relay-aware NAT traversal
//!
//! This module extends the NAT traversal functionality from the client crate
//! to add relay support for cases where direct connection attempts fail.

use std::fmt::Display;
use std::collections::HashMap;
use anyhow::Error;
use client::nat::NatTraverse;
use log::{info, warn};
use shared::{Peer, PeerDiff};
use wireguard_control::{Backend, InterfaceName};
use hex;

use crate::relay::{RelayNodeInfo, CacheIntegration};

/// Minimum number of direct connection attempts before trying relay
const MIN_DIRECT_ATTEMPTS: usize = 3;

/// RelayNatTraverse wraps the client's NatTraverse to add relay capabilities
pub struct RelayNatTraverse<'a, T: Display + Clone + PartialEq> {
    /// The underlying NatTraverse instance
    nat_traverse: NatTraverse<'a, T>,
    
    /// Connection cache integration
    cache_integration: &'a CacheIntegration,
    
    /// Track how many direct connection attempts have been made for each peer
    direct_attempts: HashMap<String, usize>,
    
    /// Keep track of all peers we're trying to connect to
    all_peers: Vec<Peer<T>>,
    
    /// Track which peers have been successfully connected
    connected_peers: HashMap<String, bool>,
}

impl<'a, T: Display + Clone + PartialEq> RelayNatTraverse<'a, T> {
    /// Create a new RelayNatTraverse instance
    pub fn new(
        interface: &'a InterfaceName,
        backend: Backend,
        diffs: &[PeerDiff<T>],
        cache_integration: &'a CacheIntegration,
    ) -> Result<Self, Error> {
        // Extract all peers from the diffs
        let mut all_peers = Vec::new();
        for diff in diffs {
            if let Some(peer) = diff.new {
                all_peers.push(peer.clone());
            }
        }
        
        // Create the base NAT traversal instance
        let nat_traverse = NatTraverse::new(interface, backend, diffs)?;
        
        Ok(Self {
            nat_traverse,
            cache_integration,
            direct_attempts: HashMap::new(),
            all_peers,
            connected_peers: HashMap::new(),
        })
    }
    
    /// Check if NAT traversal is finished
    pub fn is_finished(&self) -> bool {
        self.nat_traverse.is_finished()
    }
    
    /// Get the number of remaining peers
    pub fn remaining(&self) -> usize {
        self.nat_traverse.remaining()
    }
    
    /// Record connection attempts for each peer
    fn record_attempts(&mut self, peers: &[Peer<T>]) {
        for peer in peers {
            let attempts = self.direct_attempts.entry(peer.public_key.clone()).or_insert(0);
            *attempts += 1;
        }
    }
    
    /// Mark a peer as connected
    fn mark_connected(&mut self, pubkey: &str) {
        self.connected_peers.insert(pubkey.to_string(), true);
    }
    
    /// Perform one NAT traversal step with relay support
    pub async fn step_with_relay(&mut self) -> Result<(), Error> {
        // Check if relay functionality is enabled
        if !crate::relay::is_relay_enabled() {
            // If relay isn't enabled by auto-detection or manual control,
            // just use the standard NAT traversal
            return Ok(self.nat_traverse.step_parallel_sync()?);
        }
        
        // Get the list of remaining peers before attempting direct connections
        let remaining_before = self.nat_traverse.remaining();
        
        // Try direct connections first using parallel step
        self.nat_traverse.step_parallel_sync()?;
        
        // Record connection attempts
        if remaining_before > 0 {
            // Record an attempt for each peer that was remaining before
            let remaining_count = self.nat_traverse.remaining();
            info!("Direct connection attempts: {} before, {} after", remaining_before, remaining_count);
            
            // If some peers were connected during this step, mark them as connected
            if remaining_count < remaining_before {
                // Update our connected peers tracking
                let remaining_peers = self.get_remaining_peers();
                
                // Collect public keys of peers that should be marked as connected
                let to_mark_connected: Vec<String> = self.all_peers.iter()
                    .filter(|peer| {
                        !remaining_peers.iter().any(|p| p.public_key == peer.public_key)
                    })
                    .map(|peer| peer.public_key.clone())
                    .collect();
                
                // Now mark them as connected
                for pubkey in to_mark_connected {
                    self.mark_connected(&pubkey);
                }
            }
            
            // If we still have remaining peers that failed direct connection,
            // try connecting via relays
            if remaining_count > 0 {
                // Get a reference to the remaining peers
                let remaining = self.get_remaining_peers();
                self.record_attempts(&remaining);
                
                // Try relay connections for peers with enough direct connection attempts
                for peer in &remaining {
                    let attempts = self.direct_attempts.get(&peer.public_key).cloned().unwrap_or(0);
                    
                    // Check if we should try relay connection for this peer
                    if attempts >= MIN_DIRECT_ATTEMPTS && 
                       self.cache_integration.should_attempt_relay(&peer.public_key, attempts) {
                        // Get relay candidates for this peer
                        let relays = self.cache_integration.get_relay_candidates(&peer.public_key);
                        if !relays.is_empty() {
                            info!("Found {} relay candidates for peer {}", relays.len(), peer.name);
                            // Try connecting through relays
                            self.try_relay_connections(peer, relays).await?;
                        } else {
                            info!("No relay candidates found for peer {}", peer.name);
                        }
                    } else {
                        info!("Not enough direct connection attempts ({}) for peer {} to try relay", 
                             attempts, peer.name);
                    }
                }
            }
        }
        
        Ok(())
    }
    
    /// Attempt to connect to a peer through relays
    async fn try_relay_connections(&mut self, peer: &Peer<T>, mut relays: Vec<RelayNodeInfo>) -> Result<(), Error> {
        info!("Attempting relay connection for peer {}", peer.name);
        
        // Use get_relay_manager method instead of directly accessing the field
        if let Some(relay_manager) = self.cache_integration.get_relay_manager() {
            // Sort relays by reliability first (higher reliability first)
            relays.sort_by(|a, b| b.reliability.cmp(&a.reliability));
            
            info!("Selected {} relays for peer {}, sorted by reliability", relays.len(), peer.name);
            
            // Convert the string public key to byte array once
            let pubkey_bytes = match hex::decode(&peer.public_key) {
                Ok(bytes) => {
                    if bytes.len() != 32 {
                        warn!("Invalid public key length for {}: {}", peer.name, bytes.len());
                        return Ok(());
                    }
                    let mut array = [0u8; 32];
                    array.copy_from_slice(&bytes);
                    array
                },
                Err(e) => {
                    warn!("Failed to decode public key for {}: {}", peer.name, e);
                    return Ok(());
                }
            };
            
            // Try to connect through each relay until we succeed
            for relay in relays {
                info!("Trying relay {} (reliability: {}) for peer {}", 
                      hex::encode(&relay.pubkey), relay.reliability, peer.name);
                      
                // No need to track connection start time since we can't measure latency directly
                let connection_result = relay_manager.connect_via_relay(
                    &pubkey_bytes,
                    0, // No specific capabilities required
                    relay.region.as_deref()
                ).await;
                
                match connection_result {
                    Ok(session_id) => {
                        // Record success
                        info!("Successfully connected to {} through relay {} (session {})",
                              peer.name, hex::encode(&relay.pubkey), session_id);
                              
                        // Create a relay endpoint for caching
                        if let Some(endpoint) = CacheIntegration::create_relay_endpoint(&relay) {
                            self.cache_integration.record_relay_success(
                                &peer.public_key, 
                                endpoint, 
                                relay.pubkey, 
                                session_id
                            );
                        }
                        
                        // Mark this peer as connected
                        self.mark_connected(&peer.public_key);
                        
                        // Return after first successful connection
                        return Ok(());
                    },
                    Err(e) => {
                        // Record failure
                        warn!("Failed to connect to {} through relay {}: {}",
                              peer.name, hex::encode(&relay.pubkey), e);
                              
                        // Update relay reliability
                        self.cache_integration.record_relay_failure(relay.pubkey);
                        
                        // Continue trying with next relay
                        continue;
                    }
                }
            }
            
            info!("Tried all relays for peer {}, none succeeded", peer.name);
        } else {
            warn!("No relay manager available");
        }
        
        Ok(())
    }
    
    /// Get the remaining peers from NAT traversal
    fn get_remaining_peers(&self) -> Vec<Peer<T>> {
        // Since we can't directly access the remaining field in NatTraverse,
        // we'll derive it by filtering out peers that we've marked as connected
        self.all_peers.iter()
            .filter(|peer| !self.connected_peers.contains_key(&peer.public_key))
            .cloned()
            .collect()
    }
    
    /// Synchronous version of step_with_relay
    pub fn step_with_relay_sync(&mut self) -> Result<(), Error> {
        // Create a runtime for executing the async step_with_relay method
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;
        
        rt.block_on(async {
            self.step_with_relay().await
        })
    }
} 