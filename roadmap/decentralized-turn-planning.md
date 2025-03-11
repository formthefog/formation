# Decentralized TURN Server Implementation Plan

## 1. Background and Current Architecture Analysis

The current system uses a combination of direct WireGuard connections with sophisticated NAT traversal techniques:

- **ICE-like connectivity establishment**: Using the `NatTraverse` system to discover and test multiple endpoints.
- **Endpoint classification**: Categorizing endpoints as public, private, loopback, etc.
- **Connection quality metrics**: Measuring latency, packet loss, and stability.
- **Connection caching**: Remembering successful connections for faster reconnection.
- **Parallel endpoint testing**: Testing multiple endpoints simultaneously.

However, there are cases where direct connections cannot be established:
- Symmetric NATs on both sides
- Restrictive corporate firewalls
- Carrier-grade NATs with limited port allocation
- Network configurations that block UDP traffic

## 2. Decentralized TURN Concept

A traditional TURN server is centralized, which creates single points of failure and potential bottlenecks. For a decentralized TURN approach, we need:

1. **Multiple relay nodes**: Distributed throughout the network
2. **Dynamic relay selection**: Based on proximity, load, and reliability
3. **Fallback determination**: Clear criteria for when to use relays
4. **Security**: End-to-end encryption of relayed traffic
5. **Resource management**: Prevent abuse and ensure fair resource allocation

## 3. Architecture Proposal

### 3.1 High-Level Components

```
┌─────────────┐     ┌───────────────┐     ┌─────────────┐
│  Peer A     │◄────┤ Relay Network ├────►│  Peer B     │
│ (behind NAT)│     │               │     │ (behind NAT)│
└─────────────┘     └───────────────┘     └─────────────┘
                            ▲
                            │
                    ┌───────────────┐
                    │ Relay Registry│
                    │ & Discovery   │
                    └───────────────┘
```

### 3.2 Key Components

1. **Relay Node Service**:
   - Runs on peers with public IP addresses
   - Managed quota and bandwidth allocation
   - Connection tracking and statistics
   - Packet forwarding with minimal overhead

2. **Relay Discovery Protocol**:
   - Registry of available relay nodes
   - Health and performance metrics
   - Geographic distribution information
   - Load balancing capabilities

3. **Connection Manager**:
   - Determines when direct connections have failed
   - Selects appropriate relay nodes
   - Establishes and maintains relay connections
   - Monitors connection quality and fails over if needed

4. **Traffic Encapsulation**:
   - WireGuard packets encapsulated for relay transport
   - End-to-end encryption maintained
   - Minimal header overhead
   - Potentially using UDP or falling back to TCP when UDP is blocked

5. **Resource Management**:
   - Bandwidth quotas and fair usage policies
   - Priority system for essential traffic
   - Prevention of relay abuse
   - Quality of service guarantees

## 4. Detailed Design

### 4.1 Relay Node Service

```rust
struct RelayNode {
    // Node identity and authentication
    pub_key: String,
    relay_endpoint: SocketAddr,
    
    // Resources and limits
    max_connections: usize,
    max_bandwidth_per_conn: u32, // in Kbps
    current_connections: HashMap<ConnectionId, RelaySession>,
    
    // Statistics and monitoring
    total_bytes_relayed: u64,
    active_since: SystemTime,
    current_load: f32, // 0.0 to 1.0 representing load
}

struct RelaySession {
    peer_a: RelayPeer,
    peer_b: RelayPeer,
    established: SystemTime,
    bytes_transferred: u64,
    last_activity: SystemTime,
}

struct RelayPeer {
    pub_key: String,
    endpoint: SocketAddr,
    session_key: [u8; 32], // For secure channel to relay
}
```

### 4.2 Relay Discovery Protocol

```rust
struct RelayRegistry {
    known_relays: HashMap<String, RelayNodeInfo>,
    last_updated: SystemTime,
}

struct RelayNodeInfo {
    pub_key: String,
    endpoints: Vec<SocketAddr>,
    region: String, // Geographic region
    latency_map: HashMap<String, u32>, // Region -> avg_latency_ms
    availability: f32, // 0.0 to 1.0
    current_load: f32, // 0.0 to 1.0
    last_seen: SystemTime,
}

// Protocol messages
enum RelayDiscoveryMessage {
    Announce(RelayAnnouncement),
    Query(RelayQuery),
    Response(RelayResponse),
    Heartbeat(RelayHeartbeat),
}
```

### 4.3 Connection Manager Integration

```rust
impl ConnectionCache {
    // New method to determine if relay is needed
    fn needs_relay(&self, pubkey: &str) -> bool {
        // Check for failed direct connection attempts
        if let Some(cached_endpoints) = self.endpoints.get(pubkey) {
            // If all known endpoints are marked as failed or
            // we've made X failed attempts over Y time period
            let all_failed = cached_endpoints.iter()
                .all(|e| e.status == ConnectionStatus::Failed);
                
            if all_failed && cached_endpoints.len() >= 3 {
                return true;
            }
            
            // Check recent failure patterns
            let recent_failures = cached_endpoints.iter()
                .flat_map(|e| &e.recent_failures)
                .filter(|t| t.elapsed().unwrap() < Duration::from_mins(30))
                .count();
                
            if recent_failures > 10 {
                return true;
            }
        }
        
        false
    }
    
    // Method to select best relay for a peer
    fn select_relay(&self, pubkey: &str) -> Option<RelayNodeInfo> {
        // Get relays from registry
        let relays = RELAY_REGISTRY.get_available_relays();
        
        if relays.is_empty() {
            return None;
        }
        
        // Score relays based on:
        // 1. Proximity to us
        // 2. Proximity to target peer (if known)
        // 3. Current load
        // 4. Historical reliability
        // 5. Available bandwidth
        
        // Return highest scoring relay
        relays.into_iter()
            .map(|r| (r, self.score_relay(r, pubkey)))
            .max_by_key(|(_, score)| *score)
            .map(|(relay, _)| relay)
    }
}
```

### 4.4 Traffic Encapsulation

Relay traffic needs special encapsulation to maintain security while allowing the relay to route packets:

```
┌───────────────────────────────────────┐
│            Relay Header               │
├───────────────────────────────────────┤
│ Dest Peer ID | Session ID | Timestamp │
├───────────────────────────────────────┤
│     Encrypted WireGuard Packet        │
└───────────────────────────────────────┘
```

```rust
struct RelayPacket {
    // Relay routing information (unencrypted)
    header: RelayHeader,
    
    // Original WireGuard packet (encrypted)
    payload: Vec<u8>,
}

struct RelayHeader {
    dest_peer_id: [u8; 32], // Destination peer public key
    session_id: u64,
    timestamp: u64,
    flags: u8, // For future extensions
}
```

## 5. Integration with Existing System

### 5.1 Changes to `NatTraverse`

```rust
impl<'a, T: Display + Clone + PartialEq> NatTraverse<'a, T> {
    // Adding relay-aware step function
    pub fn step_with_relay(&mut self) -> Result<(), Error> {
        // Try direct connection first
        let direct_result = self.step();
        
        // If direct connection failed, try relay
        if direct_result.is_err() || self.remaining.len() > 0 {
            for peer in &self.remaining {
                if CONNECTION_CACHE.needs_relay(&peer.public_key) {
                    if let Some(relay) = CONNECTION_CACHE.select_relay(&peer.public_key) {
                        self.establish_relay_connection(peer, relay)?;
                    }
                }
            }
        }
        
        Ok(())
    }
    
    fn establish_relay_connection(&self, peer: &Peer<T>, relay: RelayNodeInfo) -> Result<(), Error> {
        // Protocol to establish relay connection:
        // 1. Connect to relay
        // 2. Request relay to peer
        // 3. Wait for peer to connect to relay
        // 4. Establish encrypted tunnel through relay
        // ...
    }
}
```

### 5.2 Changes to `fetch.rs`

```rust
async fn try_server_nat_traversal(
    interface: &InterfaceName,
    network: NetworkOpts,
    my_ip: String,
    connection_cache: &mut ConnectionCache,
) -> Result<(), Box<dyn std::error::Error>> {
    // Existing code...
    
    // Add relay-based fallback
    if !nat_traverse.is_finished() {
        log::info!("Direct connection attempts failed, trying relay...");
        
        // Try relay-based connection
        if let Err(e) = nat_traverse.step_with_relay() {
            log::warn!("Relay connection also failed: {}", e);
        }
    }
    
    // Rest of existing code...
}
```

### 5.3 New Module: `relay.rs`

This would be a new module implementing the relay functionality:

```rust
pub mod relay {
    use std::{net::SocketAddr, collections::HashMap, time::{SystemTime, Duration}};
    use serde::{Serialize, Deserialize};
    
    // Relay node management
    pub struct RelayManager {
        // State and configuration
    }
    
    impl RelayManager {
        pub fn new() -> Self {
            // Initialize
        }
        
        pub fn start_relay_service(&self) -> Result<(), Error> {
            // Start listening for relay requests
        }
        
        pub fn connect_via_relay(
            &self, 
            peer_pubkey: &str,
            relay: &RelayNodeInfo
        ) -> Result<RelayConnection, Error> {
            // Establish connection through relay
        }
    }
    
    // Relay connection handling
    pub struct RelayConnection {
        // Connection state
    }
    
    impl RelayConnection {
        pub fn send(&self, data: &[u8]) -> Result<(), Error> {
            // Send data through relay
        }
        
        pub fn recv(&self) -> Result<Vec<u8>, Error> {
            // Receive data through relay
        }
        
        pub fn close(self) -> Result<(), Error> {
            // Close relay connection
        }
    }
}
```

## 6. Security Considerations

1. **End-to-End Encryption**:
   - All WireGuard traffic remains encrypted through the relay
   - Relay nodes can only see routing information, not content

2. **Relay Authentication**:
   - Strong relay node identity verification
   - Signed relay announcements and responses

3. **DoS Protection**:
   - Rate limiting for relay requests
   - Connection quotas per peer
   - Blacklisting of abusive peers

4. **Privacy**:
   - Minimal metadata exposure to relays
   - Optional path randomization through multiple relays

## 7. Performance Considerations

1. **Relay Selection Optimization**:
   - Prefer relays with lowest latency path
   - Consider geographic proximity
   - Monitor relay performance and adapt

2. **Bandwidth Efficiency**:
   - Minimal encapsulation overhead
   - Efficient protocol design
   - Optional compression for certain traffic types

3. **Connection Establishment Speed**:
   - Fast relay discovery
   - Parallel relay connection attempts
   - Preemptive relay connection for frequently failing peers

## 8. Implementation Phases

### Phase 1: Basic Relay Functionality
- Implement core relay node service
- Basic relay discovery protocol
- Simple relay selection based on availability
- Integration with existing NAT traversal

### Phase 2: Enhanced Optimization
- Advanced relay selection based on latency, load, etc.
- Relay performance monitoring
- Dynamic failover between relays
- Connection quality-based adaptation

### Phase 3: Full Decentralization
- Complete peer-to-peer relay discovery
- Incentive mechanisms for relay operation
- Advanced security features
- Multi-hop relay capabilities

## 9. Testing Plan

1. **Unit Tests**:
   - Relay protocol message serialization/deserialization
   - Relay selection algorithm
   - Encapsulation/decapsulation performance

2. **Integration Tests**:
   - End-to-end relay connections under various network conditions
   - Failover from direct to relay connections
   - Discovery protocol under network partitions

3. **Performance Tests**:
   - Latency overhead measurement
   - Throughput testing under various conditions
   - Connection establishment time measurements

4. **Chaos Testing**:
   - Random relay failures
   - Network degradation simulation
   - Complex NAT scenarios with multiple layers

## 10. Next Steps

1. Develop detailed protocol specifications for:
   - Relay discovery protocol
   - Relay connection establishment
   - Traffic encapsulation format

2. Create proof-of-concept implementation of basic relay functionality

3. Test relay approach with simulated NAT environments

4. Refine relay selection algorithm based on real-world testing

5. Integrate with existing health check and connection quality metrics 