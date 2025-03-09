# Form-Net Relay System

This directory contains the implementation of Form-Net's decentralized relay system, which enables connections between peers when direct connections aren't possible due to NAT or firewall restrictions. The relay system provides a TURN-like service that allows peers to establish connections through intermediary relay nodes when direct peer-to-peer communication is blocked.

## Architecture Overview

The relay system consists of several components:

- **Protocol (protocol.rs)**: Defines the message formats and serialization for relay communication
- **Discovery (discovery.rs)**: Handles finding, registering, and selecting relay nodes
- **Manager (manager.rs)**: Manages relay connections and handles connection lifecycles
- **Service (service.rs)**: Implements the relay service that forwards packets between peers

## Component Documentation

### Protocol (protocol.rs)

This component defines the message formats and serialization mechanisms for relay communication.

#### Key Structures:

- **RelayHeader**: Contains routing information for a packet, including destination peer ID, session ID, and timestamp
- **RelayPacket**: Combines a header with an encrypted payload for packet forwarding
- **ConnectionRequest/Response**: Messages for establishing relay connections
- **Heartbeat**: Used to maintain active relay sessions
- **DiscoveryQuery/Response**: Messages for finding available relay nodes
- **RelayNodeInfo**: Contains information about a relay node (endpoints, capabilities, etc.)
- **RelayMessage**: Enum that encapsulates all possible message types

#### Capability Flags:

- `RELAY_CAP_IPV4`: Relay supports IPv4
- `RELAY_CAP_IPV6`: Relay supports IPv6
- `RELAY_CAP_TCP_FALLBACK`: Relay supports TCP fallback for UDP-blocked networks
- `RELAY_CAP_HIGH_BANDWIDTH`: Relay offers high bandwidth forwarding
- `RELAY_CAP_LOW_LATENCY`: Relay offers low latency forwarding

### Discovery (discovery.rs)

This component handles finding, registering, and selecting relay nodes.

#### Key Structures:

- **RelayRegistry**: Manages a collection of known relay nodes
- **SharedRelayRegistry**: Thread-safe wrapper around RelayRegistry
- **BootstrapConfig**: Configuration for bootstrap relay nodes
- **BootstrapRelay**: Information about a known bootstrap relay

#### Key Functions:

- **register_relay**: Add a new relay to the registry
- **find_relays**: Find relays matching specific criteria
- **select_best_relay**: Select the most suitable relay for a connection
- **refresh_from_bootstrap**: Update relay information from bootstrap nodes
- **score_relay**: Evaluate a relay's suitability based on various factors

### Manager (manager.rs)

This component manages relay connections and handles the connection lifecycle.

#### Key Structures:

- **RelayManager**: Central manager for relay connections
- **RelaySession**: Tracks an active relay session
- **ConnectionAttempt**: Tracks an ongoing connection attempt
- **PacketReceiver**: Handles receiving packets through a relay
- **LatencyTracker**: Tracks latency measurements for adaptive timeouts
- **CacheIntegration**: Integrates with the connection cache for relay selection

#### Key Functions:

- **connect_via_relay**: Establish a connection through a relay
- **send_packet**: Send data through an established relay connection
- **process_relay_packet**: Process an incoming relay packet
- **get_sessions_needing_heartbeat**: Identify sessions that need heartbeats
- **cleanup**: Clean up expired sessions and connection attempts

### Service (service.rs)

This component implements the relay service that forwards packets between peers.

#### Key Structures:

- **RelayNode/RelayService**: Implements the relay service functionality
- **RelayConfig**: Configuration for a relay node
- **ResourceLimits**: Defines resource limitations for a relay node
- **RelayStats**: Tracks statistics about relay usage and performance
- **RelaySession**: Manages an active forwarding session

#### Key Functions:

- **start**: Start the relay service
- **stop**: Stop the relay service
- **process_packet**: Process an incoming packet
- **process_connection_request**: Handle a new connection request
- **process_heartbeat**: Process a session heartbeat
- **create_session**: Create a new relay session
- **metrics**: Generate metrics for the relay service

## Connection Flow

The typical connection flow through a relay works as follows:

1. **Discovery**: A peer discovers available relay nodes through:
   - Bootstrap configuration
   - Direct queries to known relays
   - Background discovery process

2. **Connection Establishment**:
   - Peer A sends a ConnectionRequest to the relay
   - The relay processes the request and creates a session
   - The relay sends a ConnectionResponse back to Peer A
   - Peer A begins sending heartbeats to maintain the session

3. **Data Forwarding**:
   - Peer A sends data to Peer B through the relay using RelayPackets
   - The relay authenticates and forwards the packets
   - Peer B receives the data and can respond through the same relay

4. **Connection Maintenance**:
   - Peers send regular heartbeats to keep sessions alive
   - The relay tracks session activity and expires inactive sessions
   - Adaptive timeouts adjust based on observed network conditions

## Test Coverage

The relay system has the following test coverage:

### Protocol Tests
- Header flag operations
- Message validation 
- Serialization/deserialization
- Protocol message format verification
- **Simple Discovery Serialization**: Tests proper serialization and deserialization of basic relay messages
  - Verifies ConnectionRequest and ConnectionResponse serialization
  - Ensures message types are correctly preserved during serialization
  - Validates field values are properly maintained after serialization cycle

### Discovery Tests
- Basic registry operations
- Bootstrap configuration
- Relay node selection and scoring
- Registry pruning of stale relays

### Manager Tests
- **Adaptive Timeout Settings**: Tests that adaptive timeouts correctly adjust based on latency measurements
  - Verifies proper calculation of timeouts based on latency samples
  - Tests minimum and maximum bounds enforcement
  - Validates behavior when enough samples aren't available
  - Confirms correct operation when adaptive timeouts are disabled
  
- **Connection State Management**: Tests relay connection tracking and session management
  - Verifies connection attempt tracking 
  - Tests updating connection status
  - Validates latency recording and registry updates
  - Tests session creation and lookup
  - Verifies session closing and cleanup

### Service Tests
- **Background Discovery**: Tests the background relay discovery functionality
  - Verifies discovery thread starts and stops correctly
  - Confirms the registry is refreshed at the configured interval
  - Tests that relay discovery is properly enabled/disabled through configuration

### Integration and Simulation Tests
- Tests for basic relay connectivity
- Tests for handling network conditions (latency, packet loss, jitter)
- Tests for network partition scenarios
- Tests for relay failover

## Running Tests

The relay unit tests can be run with:

```bash
cargo test -p formnet --lib -- relay::
```

Or to run specific test cases:

```bash
cargo test -p formnet --lib -- relay::manager::tests::test_adaptive_timeout_settings
cargo test -p formnet --lib -- relay::manager::tests::test_relay_connection_state
cargo test -p formnet --lib -- relay::service::tests::test_background_discovery
cargo test -p formnet --lib -- relay::protocol::tests::test_simple_discovery_serialization
```

To run the network simulation tests:

```bash
cargo test -p formnet --test network_simulation_relay
```

## Usage Examples

The relay system is designed to be used automatically by the FormNet networking layer when direct connections fail. Example applications can be found in the `form-net/formnet/examples/` directory:

### Starting a Relay Service

The `start_relay_service.rs` example demonstrates how to start a relay service:

```rust
// Configure the relay service
let listen_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), 51820);
let relay_pubkey = [0u8; 32]; // In real usage, this would be a proper public key
let config = RelayConfig::new(listen_addr, relay_pubkey)
    .with_region("us-west")
    .with_capabilities(formnet::relay::RELAY_CAP_IPV4);

// Create and start the relay service
let mut relay_service = RelayService::new(config);
relay_service.start()?;

// When done, stop the service
relay_service.stop();
```

### Using Relay-Enabled NAT Traversal

The `relay_nat_traverse.rs` example shows how to use relay-enabled NAT traversal:

```rust
// Create the required components
let registry = SharedRelayRegistry::new();
let local_pubkey = [0u8; 32]; // Your local public key
let relay_manager = RelayManager::new(registry, local_pubkey);

// Set up cache integration
let interface = InterfaceName::from_str("wg0").unwrap();
let mut cache_integration = CacheIntegration::new(interface.clone(), data_dir);
cache_integration.set_relay_manager(relay_manager);

// Create NAT traversal with relay support
let mut nat_traverse = RelayNatTraverse::new(
    &interface,
    Backend::Userspace,
    &peer_diffs,
    &cache_integration
)?;

// Perform NAT traversal with automatic relay fallback
while !nat_traverse.is_finished() {
    nat_traverse.step_with_relay_sync()?;
    // Wait before retrying
    std::thread::sleep(Duration::from_secs(1));
}
```

### Managing Relay Sessions

The `test_session.rs` example demonstrates session management:

```rust
// Create a relay session
let session_id = 12345;
let initiator_pubkey = [1u8; 32];
let target_pubkey = [2u8; 32];
let session = RelaySession::new(session_id, initiator_pubkey, target_pubkey);

// Generate authentication tokens
let token = session.generate_auth_token();
let is_valid = session.verify_auth_token(&token);

// Manage session activity
session.update_activity();
let is_inactive = session.is_inactive(Duration::from_secs(60));

// Record packet statistics
session.record_initiator_to_target(1024);
session.record_target_to_initiator(2048);
```

## Configuration

The relay system can be configured in several ways:

### Relay Node Configuration

```rust
// Basic configuration
let config = RelayConfig::new(listen_addr, pubkey)
    .with_region("us-east")
    .with_capabilities(RELAY_CAP_IPV4 | RELAY_CAP_HIGH_BANDWIDTH)
    .with_limits(ResourceLimits {
        max_total_sessions: 1000,
        max_sessions_per_client: 5,
        // ... other limits
    });

// Enable persistence
let config = config.with_persistence("/path/to/config.json");

// Enable background discovery
let config = config.with_background_discovery(true, Some(Duration::from_secs(300)));

// Enable adaptive timeouts
let config = config.with_adaptive_timeouts(
    true,                            // Enable adaptive timeouts
    Some(1.5),                       // Timeout multiplier
    Some(5),                         // Minimum samples
    Some(20),                        // Maximum samples
    Some(Duration::from_secs(1)),    // Minimum timeout
    Some(Duration::from_secs(10))    // Maximum timeout
);
```

### Bootstrap Configuration

```rust
// Create bootstrap configuration
let mut bootstrap_config = BootstrapConfig::new();

// Add relays
bootstrap_config.add_relay(
    "203.0.113.10:51820".to_string(), 
    "AABBCCDDEEFF00112233445566778899AABBCCDDEEFF00112233445566778899".to_string(),
    Some("us-west".to_string())
);

// Configure registry with bootstrap
registry.set_bootstrap_config(bootstrap_config);

// Refresh from bootstrap
registry.refresh_from_bootstrap()?;
``` 