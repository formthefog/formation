# Form-Net Relay API Documentation

This document provides detailed API documentation for the Form-Net relay system. It covers the key functions, structures, and usage patterns for developers who want to work with the relay API.

## Table of Contents

1. [Core Types](#core-types)
2. [RelayRegistry API](#relayregistry-api)
3. [RelayManager API](#relaymanager-api)
4. [RelayService API](#relayservice-api)
5. [Protocol Message Types](#protocol-message-types)
6. [Configuration Types](#configuration-types)
7. [Utility Functions](#utility-functions)
8. [Integration Patterns](#integration-patterns)

## Core Types

### RelayNodeInfo

```rust
pub struct RelayNodeInfo {
    pub pubkey: [u8; 32],
    pub endpoints: Vec<String>,
    pub region: Option<String>,
    pub capabilities: u32,
    pub load: u8,
    pub latency: Option<u32>,
    pub max_sessions: u32,
    pub protocol_version: u16,
    pub reliability: u8,
    pub last_result_time: Option<u64>,
    pub packet_loss: Option<u8>,
}
```

`RelayNodeInfo` represents information about a relay node and is used throughout the relay system for discovery, selection, and connection.

**Example:**

```rust
// Create relay node info
let mut relay_info = RelayNodeInfo::new(
    pubkey,                                  // Public key
    vec!["203.0.113.10:51820".to_string()], // Endpoints
    1000                                     // Max sessions
);

// Add capabilities
relay_info.add_capability(RELAY_CAP_IPV4);
relay_info.add_capability(RELAY_CAP_HIGH_BANDWIDTH);

// Set region and other properties
let relay_info = relay_info
    .with_region("us-west")
    .with_latency(50)
    .with_load(25);

// Check capabilities
if relay_info.has_capability(RELAY_CAP_IPV6) {
    println!("Relay supports IPv6");
}
```

### RelaySession

```rust
pub struct RelaySession {
    pub id: u64,
    pub initiator_pubkey: [u8; 32],
    pub target_pubkey: [u8; 32],
    pub created_at: SystemTime,
    pub expires_at: SystemTime,
    pub last_activity: Instant,
    // ... other fields
}
```

`RelaySession` represents an active relay session between two peers.

**Example:**

```rust
// Create a new session
let session = RelaySession::new(
    session_id,
    initiator_pubkey,
    target_pubkey
);

// Update session activity
session.update_activity();

// Check if session is expired
if session.is_expired() {
    println!("Session has expired");
}

// Check if session is inactive
if session.is_inactive(Duration::from_secs(60)) {
    println!("Session is inactive");
}

// Record packet forwarding
session.record_initiator_to_target(1024);
```

## RelayRegistry API

### SharedRelayRegistry

```rust
pub struct SharedRelayRegistry {
    inner: Arc<RwLock<RelayRegistry>>,
}
```

`SharedRelayRegistry` is a thread-safe wrapper around `RelayRegistry` for managing known relay nodes.

**Key Methods:**

```rust
// Create a new registry
let registry = SharedRelayRegistry::new();

// Register a relay
registry.register_relay(relay_info)?;

// Find relays by criteria
let relays = registry.find_relays(
    Some("us-west"),     // Region filter
    RELAY_CAP_IPV4,      // Required capabilities
    10                   // Maximum number to return
)?;

// Select the best relay
let best_relay = registry.select_best_relay(
    target_pubkey,
    RELAY_CAP_IPV4,
    Some("us-west")
)?;

// Configure bootstrap relays
let mut bootstrap_config = BootstrapConfig::new();
bootstrap_config.add_relay(endpoint, pubkey_hex, Some(region));
registry.set_bootstrap_config(bootstrap_config)?;

// Refresh from bootstrap
let refreshed_count = registry.refresh_from_bootstrap()?;
```

## RelayManager API

```rust
pub struct RelayManager {
    // ... fields
}
```

`RelayManager` handles relay connections and connection lifecycle.

**Key Methods:**

```rust
// Create a new manager
let relay_manager = RelayManager::new(
    registry,       // SharedRelayRegistry
    local_pubkey    // Your public key
);

// Connect through a relay
let session_id = relay_manager.connect_via_relay(
    &target_pubkey,
    RELAY_CAP_IPV4,
    Some("us-west")
).await?;

// Send data through the relay
relay_manager.send_packet(
    &target_pubkey,  // Destination
    &payload_data    // Data to send
).await?;

// Create a packet receiver
let mut receiver = relay_manager.create_packet_receiver(
    &target_pubkey
)?;

// Receive data (in a loop)
while let Some(data) = receiver.receive()? {
    println!("Received {} bytes", data.len());
}

// Close a session
relay_manager.close_session(session_id)?;

// Cleanup expired sessions
let (closed_sessions, canceled_attempts) = relay_manager.cleanup()?;
```

### PacketReceiver

```rust
pub struct PacketReceiver {
    // ... fields
}
```

`PacketReceiver` is used to receive packets through a relay.

**Key Methods:**

```rust
// Receive data (may return None if no data is available)
let data_opt = receiver.receive()?;

// Close the receiver
receiver.close();

// Check if the receiver is active
if receiver.is_active() {
    println!("Receiver is still active");
}
```

### CacheIntegration

```rust
pub struct CacheIntegration {
    // ... fields
}
```

`CacheIntegration` integrates relay functionality with the connection cache.

**Key Methods:**

```rust
// Create a new cache integration
let cache_integration = CacheIntegration::new(
    interface_name,
    data_directory
);

// Set the relay manager
cache_integration.set_relay_manager(relay_manager);

// Check if a peer needs a relay
if cache_integration.needs_relay(peer_public_key) {
    println!("Peer needs relay connection");
}

// Record a direct connection failure
cache_integration.record_failure(peer_public_key);

// Record a successful relay connection
cache_integration.record_relay_success(
    peer_public_key,
    relay_endpoint,
    relay_pubkey,
    session_id
);
```

## RelayService API

```rust
pub struct RelayService {
    // ... fields
}
```

`RelayService` implements a relay node that can forward packets between peers.

**Key Methods:**

```rust
// Create a new relay service
let config = RelayConfig::new(listen_addr, pubkey);
let mut relay_service = RelayService::new(config);

// Start the service
relay_service.start()?;

// Get node information
let node_info = relay_service.get_node_info();

// Get statistics
let stats = relay_service.get_stats();

// Create a session manually
let session_id = relay_service.create_session(
    initiator_pubkey,
    target_pubkey
)?;

// Extend a session
relay_service.extend_session(
    session_id,
    Duration::from_secs(3600)
)?;

// Remove a session
relay_service.remove_session(session_id)?;

// Get metrics output
let metrics_text = relay_service.metrics();

// Stop the service
relay_service.stop();
```

## Protocol Message Types

### RelayMessage

```rust
pub enum RelayMessage {
    ConnectionRequest(ConnectionRequest),
    ConnectionResponse(ConnectionResponse),
    ForwardPacket(RelayPacket),
    Heartbeat(Heartbeat),
    DiscoveryQuery(DiscoveryQuery),
    DiscoveryResponse(DiscoveryResponse),
    RelayAnnouncement(RelayAnnouncement),
}
```

**Example:**

```rust
// Serialize a message
let request = ConnectionRequest::new(local_pubkey, target_pubkey);
let message = RelayMessage::ConnectionRequest(request);
let data = message.serialize()?;

// Deserialize a message
let received_message = RelayMessage::deserialize(&data)?;

match received_message {
    RelayMessage::ConnectionResponse(response) => {
        if response.is_success() {
            println!("Connection established with session ID: {}", 
                     response.session_id.unwrap());
        }
    },
    RelayMessage::ForwardPacket(packet) => {
        println!("Received forwarded packet: {} bytes", packet.payload.len());
    },
    // ... handle other message types
}
```

### RelayPacket

```rust
pub struct RelayPacket {
    pub header: RelayHeader,
    pub payload: Vec<u8>,
}
```

**Example:**

```rust
// Create a new packet
let packet = RelayPacket::new(
    target_pubkey,  // Destination peer
    session_id,     // Session ID
    payload_data    // Data to send
);

// Serialize
let data = packet.serialize()?;

// Deserialize
let packet = RelayPacket::deserialize(&data)?;
```

## Configuration Types

### RelayConfig

```rust
pub struct RelayConfig {
    pub listen_addr: SocketAddr,
    pub pubkey: [u8; 32],
    pub region: Option<String>,
    pub capabilities: u32,
    pub limits: ResourceLimits,
    // ... other fields
}
```

**Example:**

```rust
// Create basic configuration
let config = RelayConfig::new(listen_addr, pubkey)
    .with_region("us-east")
    .with_capabilities(RELAY_CAP_IPV4 | RELAY_CAP_LOW_LATENCY);

// Configure resource limits
let config = config.with_limits(ResourceLimits {
    max_total_sessions: 1000,
    max_sessions_per_client: 5,
    max_connection_rate: 100,
    // ... other limits
});

// Enable persistence
let config = config.with_persistence("/path/to/config.json");

// Enable adaptive timeouts
let config = config.with_adaptive_timeouts(
    true,                            // Enable adaptive timeouts
    Some(1.5),                       // Multiplier
    Some(5),                         // Min samples
    Some(20),                        // Max samples
    Some(Duration::from_secs(1)),    // Min timeout
    Some(Duration::from_secs(10))    // Max timeout
);

// Enable background discovery
let config = config.with_background_discovery(
    true,                            // Enable background discovery
    Some(Duration::from_secs(300))   // Interval
);
```

### ResourceLimits

```rust
pub struct ResourceLimits {
    pub max_total_sessions: usize,
    pub max_sessions_per_client: usize,
    pub max_connection_rate: usize,
    pub max_connection_rate_per_ip: usize,
    // ... other fields
}
```

## Utility Functions

### Relay Control

```rust
// Check if relay functionality is enabled
if is_relay_enabled() {
    println!("Relay functionality is enabled");
}

// Manually enable/disable relay functionality
set_relay_enabled(true);
```

### NAT Detection

```rust
// Detect NAT type
let nat_difficulty = detect_nat_type();

match nat_difficulty {
    NatDifficulty::Open => println!("Open internet, direct connections likely"),
    NatDifficulty::Simple => println!("Simple NAT, direct connections likely"),
    NatDifficulty::Moderate => println!("Moderate NAT, direct might work"),
    NatDifficulty::Difficult => println!("Difficult NAT, relay recommended"),
    NatDifficulty::Symmetric => println!("Symmetric NAT, relay required"),
    NatDifficulty::Unknown => println!("Unknown NAT type"),
}
```

## Integration Patterns

### Relay-Enabled NAT Traversal

```rust
// Create NAT traversal with relay support
let mut nat_traverse = RelayNatTraverse::new(
    &interface,
    Backend::Userspace,
    &peer_diffs,
    &cache_integration
)?;

// Process until finished
while !nat_traverse.is_finished() {
    // Perform a NAT traversal step with relay support
    nat_traverse.step_with_relay_sync()?;
    
    // Wait before the next attempt
    std::thread::sleep(Duration::from_secs(1));
}
```

### Manual Relay Connection

```rust
// Create a relay manager
let registry = SharedRelayRegistry::new();
let relay_manager = RelayManager::new(registry.clone(), local_pubkey);

// Register a known relay
let relay_info = RelayNodeInfo::new(
    relay_pubkey,
    vec![relay_endpoint.to_string()],
    1000
);
registry.register_relay(relay_info)?;

// Connect to a peer through the relay
let session_id = relay_manager.connect_via_relay(
    &peer_pubkey,
    RELAY_CAP_IPV4,
    None
).await?;

// Send data
relay_manager.send_packet(&peer_pubkey, &data).await?;

// Create a receiver
let mut receiver = relay_manager.create_packet_receiver(&peer_pubkey)?;

// Receive data
while let Some(data) = receiver.receive()? {
    // Process data
}

// Close the session when done
relay_manager.close_session(session_id)?;
```

### Running a Relay Node

```rust
// Generate or load keypair
let relay_pubkey = [0u8; 32]; // Replace with actual key

// Configure the relay service
let listen_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), 51820);
let config = RelayConfig::new(listen_addr, relay_pubkey)
    .with_region("us-west")
    .with_capabilities(RELAY_CAP_IPV4 | RELAY_CAP_HIGH_BANDWIDTH)
    .with_persistence("/var/lib/relay/config.json")
    .with_background_discovery(true, None);

// Create and start the relay service
let mut relay_service = RelayService::new(config);
relay_service.start()?;

// Monitor statistics periodically
loop {
    let stats = relay_service.get_stats();
    println!("Active sessions: {}", stats.active_sessions);
    println!("Bandwidth usage: {} bps", stats.current_bandwidth_bps);
    
    std::thread::sleep(Duration::from_secs(60));
}

// Shutdown when needed
relay_service.stop();
``` 