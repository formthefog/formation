//! ICE-like NAT traversal logic.
//!
//! Doesn't follow the specific ICE protocol, but takes great inspiration from RFC 8445
//! and applies it to a protocol more specific to innernet.

use std::{fmt::Display, time::{Duration, Instant}};

use anyhow::Error;
use futures::future;
use shared::{
    wg::{DeviceExt, PeerInfoExt},
    Endpoint, Peer, PeerDiff,
};
use tokio::time::sleep;
use std::collections::HashSet;
use wireguard_control::{Backend, Device, DeviceUpdate, InterfaceName, Key, PeerConfigBuilder};

pub const STEP_INTERVAL: Duration = Duration::from_secs(1);
pub const PARALLEL_ENDPOINTS: usize = 3; // Test up to 3 endpoints per peer in parallel
const MAX_DIRECT_ATTEMPTS: usize = 3; // Try direct connections for a peer at most 3 times

pub struct NatTraverse<'a, T: Display + Clone + PartialEq> {
    interface: &'a InterfaceName,
    backend: Backend,
    remaining: Vec<Peer<T>>,
}

impl<'a, T: Display + Clone + PartialEq> NatTraverse<'a, T> {
    pub fn new(
        interface: &'a InterfaceName,
        backend: Backend,
        diffs: &[PeerDiff<T>],
    ) -> Result<Self, Error> {
        // Filter out removed peers from diffs list.
        let mut remaining: Vec<_> = diffs.iter().filter_map(|diff| diff.new).cloned().collect();

        for peer in &mut remaining {
            // Limit reported alternative candidates to 30.
            peer.candidates.truncate(30);

            // Remove server-reported endpoint from elsewhere in the list if it existed.
            let endpoint = peer.endpoint.clone();
            peer.candidates
                .retain(|addr| Some(addr) != endpoint.as_ref());

            log::info!("removed server reported endpoint: {:?}", peer.candidates);
            // Add the server-reported endpoint to the beginning of the list. In the event
            // no other endpoints worked, the remaining endpoint in the list will be the one
            // assigned to the peer so it should default to the server-reported endpoint.
            // This is inserted at the beginning of the Vec as candidates are popped from
            // the end as the algorithm progresses.
            if let Some(endpoint) = endpoint {
                peer.candidates.insert(0, endpoint);
            }
        }
        let mut nat_traverse = Self {
            interface,
            backend,
            remaining,
        };

        nat_traverse.refresh_remaining()?;

        Ok(nat_traverse)
    }

    pub fn is_finished(&self) -> bool {
        self.remaining.is_empty()
    }

    pub fn remaining(&self) -> usize {
        self.remaining.len()
    }

    /// Refreshes the current state of candidate traversal attempts, filtering out
    /// the peers that have been exhausted of all endpoint options.
    fn refresh_remaining(&mut self) -> Result<(), Error> {
        let device = Device::get(self.interface, self.backend)?;
        // Remove connected and missing peers
        self.remaining.retain(|peer| {
            if let Some(peer_info) = device.get_peer(&peer.public_key) {
                let recently_connected = peer_info.is_recently_connected();
                if recently_connected {
                    log::info!(
                        "peer {} removed from NAT traverser (connected!).",
                        peer.name
                    );
                }
                !recently_connected
            } else {
                log::info!(
                    "peer {} removed from NAT traverser (no longer on interface).",
                    peer.name
                );
                false
            }
        });

        self.remaining.retain(|peer| !peer.candidates.is_empty());

        Ok(())
    }

    pub fn step(&mut self) -> Result<(), Error> {
        self.refresh_remaining()?;

        // Set all peers' endpoints to their next available candidate.
        let candidate_updates = self.remaining.iter_mut().filter_map(|peer| {
            let endpoint = peer.candidates.pop();
            if let Some(endpoint) = &endpoint {
                log::info!("trying endpoint {} for peer {}", endpoint, peer.name);
            }
            set_endpoint(&peer.public_key, endpoint.as_ref())
        });

        let updates: Vec<_> = candidate_updates.collect();

        DeviceUpdate::new()
            .add_peers(&updates)
            .apply(self.interface, self.backend)?;

        let start = Instant::now();
        while start.elapsed() < STEP_INTERVAL {
            self.refresh_remaining()?;

            if self.is_finished() {
                log::info!("NAT traverser is finished!");
                break;
            }
            std::thread::sleep(Duration::from_millis(100));
        }

        Ok(())
    }

    // Add a synchronous version that calls the async method using a runtime
    pub fn step_parallel_sync(&mut self) -> Result<(), Error> {
        // Create a runtime for executing the async step_parallel method
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;
        
        rt.block_on(async {
            self.step_parallel().await
        })
    }

    pub async fn step_parallel(&mut self) -> Result<(), Error> {
        self.refresh_remaining()?;
        
        // Group endpoints by peer to test multiple endpoints per peer in parallel
        let mut peer_updates = Vec::new();
        
        for peer in &mut self.remaining {
            let mut endpoints_to_test = Vec::new();
            
            // Take up to PARALLEL_ENDPOINTS candidates for this peer
            for _ in 0..PARALLEL_ENDPOINTS {
                if let Some(endpoint) = peer.candidates.pop() {
                    log::info!("will try endpoint {} for peer {}", endpoint, peer.name);
                    endpoints_to_test.push(endpoint);
                } else {
                    break;
                }
            }
            
            if !endpoints_to_test.is_empty() {
                peer_updates.push((peer.public_key.clone(), endpoints_to_test));
            }
        }
        
        // Now test all selected endpoints in parallel
        let mut update_futures = Vec::new();
        
        for (pubkey, endpoints) in peer_updates {
            for endpoint in endpoints {
                let pubkey_clone = pubkey.clone();
                // Create a task for each endpoint test
                update_futures.push(tokio::spawn(async move {
                    match endpoint.resolve() {
                        Ok(addr) => {
                            log::info!("successfully resolved endpoint {} for {}", endpoint, pubkey_clone);
                            Some((pubkey_clone, addr, endpoint))
                        },
                        Err(e) => {
                            log::info!("failed to resolve endpoint {} for {}: {}", endpoint, pubkey_clone, e);
                            None
                        }
                    }
                }));
            }
        }
        
        // Wait for all tasks to complete
        let results = future::join_all(update_futures).await;
        
        // Process results and update peers
        let mut updates = Vec::new();
        let mut processed_pubkeys = HashSet::new();
        
        for result in results {
            if let Ok(Some((pubkey, addr, _endpoint))) = result {
                // Only use the first successful endpoint for each peer
                if !processed_pubkeys.contains(&pubkey) {
                    processed_pubkeys.insert(pubkey.clone());
                    match Key::from_base64(&pubkey) {
                        Ok(key) => {
                            updates.push(PeerConfigBuilder::new(&key).set_endpoint(addr));
                        },
                        Err(e) => {
                            log::error!("Invalid public key {}: {}", pubkey, e);
                        }
                    }
                }
            }
        }
        
        // Apply successful updates
        if !updates.is_empty() {
            log::info!("Applying {} endpoint updates in parallel", updates.len());
            DeviceUpdate::new()
                .add_peers(&updates)
                .apply(self.interface, self.backend)?;
        }
        
        // Give connections a chance to establish
        sleep(STEP_INTERVAL).await;
        self.refresh_remaining()?;
        
        Ok(())
    }
}

/// Return a PeerConfigBuilder if an endpoint exists and resolves successfully.
fn set_endpoint(public_key: &str, endpoint: Option<&Endpoint>) -> Option<PeerConfigBuilder> {
    endpoint
        .and_then(|endpoint| endpoint.resolve().ok())
        .map(|addr| {
            PeerConfigBuilder::new(&Key::from_base64(public_key).unwrap()).set_endpoint(addr)
        })
}
