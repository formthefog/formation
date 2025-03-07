# Decentralized TURN Implementation Subtasks

This document tracks the implementation progress of the decentralized TURN server for form-net. Each subtask is designed to be small, focused, and independently verifiable.

## Phase 1: Core Protocol and Basic Structure

- [x] **1. Set up relay module structure**
  - Create relay directory structure in formnet/src
  - Create minimal mod.rs to expose the module
  - Add necessary dependencies to Cargo.toml

- [x] **2. Implement core protocol data structures (protocol.rs)**
  - Define RelayHeader and RelayPacket structures
  - Implement basic serialization/deserialization
  - Add unit tests for serialization

- [ ] **3. Implement relay message types (protocol.rs)**
  - Define RelayMessage enum with all message variants
  - Implement ConnectionRequest/Response structures
  - Implement Heartbeat message structure
  - Add message validation helpers

- [ ] **4. Add relay discovery protocol messages (protocol.rs)**
  - Implement DiscoveryQuery/Response structures
  - Implement RelayAnnouncement structure
  - Add unit tests for discovery messages

- [ ] **5. Update CachedEndpoint for relay support (connection_cache.rs)**
  - Add relay-specific fields to CachedEndpoint
  - Ensure backward compatibility with existing cache
  - Add unit tests for CachedEndpoint with relay fields

## Phase 2: Discovery and Registry

- [ ] **6. Implement basic RelayRegistry (discovery.rs)**
  - Create RelayRegistry and RelayNodeInfo structures
  - Implement basic relay registration and lookup
  - Add unit tests for registry operations

- [ ] **7. Add bootstrap relay configuration (discovery.rs)**
  - Implement bootstrap relay list management
  - Add configuration loading/saving for bootstrap relays
  - Add methods to refresh registry from bootstrap nodes

- [ ] **8. Implement relay selection algorithm (discovery.rs)**
  - Create scoring function for relay selection
  - Implement proximity-based selection logic
  - Add filtering based on relay capabilities and load

## Phase 3: Connection Management

- [ ] **9. Create basic RelayManager (manager.rs)**
  - Implement RelayManager structure
  - Add relay connection tracking
  - Implement relay connection lifecycle management

- [ ] **10. Implement relay connection establishment (manager.rs)**
  - Create connect_via_relay method
  - Implement relay handshake protocol
  - Add connection error handling and retry logic

- [ ] **11. Add relay packet forwarding logic (manager.rs)**
  - Implement send_packet method for relay forwarding
  - Add packet receiving and processing
  - Implement session management for active connections

- [ ] **12. Integrate with connection cache (manager.rs)**
  - Implement needs_relay method using connection history
  - Add relay success recording to connection cache
  - Implement relay prioritization based on past successes

## Phase 4: Integration with Existing System

- [ ] **13. Update NAT traversal for relay support (nat.rs)**
  - Add step_with_relay method to NatTraverse
  - Implement mark_connected helper
  - Add error handling for relay connection attempts

- [ ] **14. Update fetch.rs for relay integration**
  - Add relay_manager creation in try_server_nat_traversal
  - Implement relay fallback when direct connection fails
  - Add helper to obtain local public key

- [ ] **15. Add relay connection monitoring (fetch.rs)**
  - Implement relay connection health checking
  - Add relay connection statistics collection
  - Create periodic relay connection refresh logic

## Phase 5: Relay Service Implementation

- [ ] **16. Implement basic RelayNode (service.rs)**
  - Create RelayNode structure with basic properties
  - Implement resource limitation logic
  - Add statistics tracking for node performance

- [ ] **17. Create RelayService (service.rs)**
  - Implement UDP socket handling
  - Add message parsing and routing logic
  - Implement basic packet forwarding

- [ ] **18. Add relay session management (service.rs)**
  - Implement RelaySession structure
  - Add session creation, tracking, and cleanup
  - Implement session authentication and verification

- [ ] **19. Add security features to relay service**
  - Implement denial-of-service protection
  - Add rate limiting for relay requests
  - Implement authentication for relay control messages

## Phase 6: CLI and Configuration

- [ ] **20. Add CLI commands for relay management**
  - Implement ListRelays command
  - Add StartRelay command with configuration options
  - Create AddBootstrapRelay command

- [ ] **21. Implement relay configuration storage**
  - Create configuration file for relay settings
  - Add methods to load/save relay configuration
  - Implement configuration validation

- [ ] **22. Add relay status reporting**
  - Create command to show relay statistics
  - Implement relay performance monitoring
  - Add logging for relay events and status changes

## Phase 7: Testing and Refinement

- [ ] **23. Create comprehensive relay unit tests**
  - Add tests for each relay component
  - Create property-based tests for protocol robustness
  - Implement scenario-based tests for edge cases

- [ ] **24. Implement integration tests**
  - Create test harness for relay network testing
  - Add tests for end-to-end relay scenarios
  - Implement tests for failure cases and recovery

- [ ] **25. Add network simulation tests**
  - Create NAT simulation environment
  - Implement tests for various network conditions
  - Add performance and scaling tests

## Phase 8: Documentation and Deployment

- [ ] **26. Add comprehensive documentation**
  - Document all relay components and APIs
  - Create usage examples and guides
  - Add diagram of relay architecture

- [ ] **27. Prepare for deployment**
  - Finalize configuration options
  - Create deployment documentation
  - Implement monitoring for production use

- [ ] **28. Final system testing**
  - Perform end-to-end testing in real-world scenarios
  - Verify performance metrics
  - Confirm security measures are effective 