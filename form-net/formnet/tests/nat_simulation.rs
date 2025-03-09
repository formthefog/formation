//! NAT simulation tests for relay functionality
//!
//! This module tests the relay functionality in simulated NAT environments
//! to verify that connections work correctly through restricted network scenarios.

mod network_conditions;

use std::collections::{HashMap, HashSet};
use std::net::SocketAddr;
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant};
use std::thread;

use formnet::relay::{
    protocol::{
        RelayNodeInfo, ConnectionRequest, ConnectionResponse, RelayMessage,
        RelayPacket,
    },
    service::RelayConfig,
};
use rand::{Rng, thread_rng};

use network_conditions::SimulatedNetwork;

/// NAT type to simulate
#[derive(Debug, Clone, Copy)]
enum NatType {
    /// Full cone NAT (most permissive)
    FullCone,
    
    /// Restricted cone NAT (allows inbound traffic from hosts previously contacted)
    RestrictedCone,
    
    /// Port-restricted cone NAT (like restricted, but also checks port)
    PortRestrictedCone,
    
    /// Symmetric NAT (most restrictive, uses different mapping for each destination)
    Symmetric,
}

/// Simulates a NAT device in the network
struct NatSimulator {
    /// Original (internal) to mapped (external) address mapping
    internal_to_external: RwLock<HashMap<SocketAddr, SocketAddr>>,
    
    /// For symmetric NAT: (internal addr, destination addr) -> external addr
    symmetric_mappings: RwLock<HashMap<(SocketAddr, SocketAddr), SocketAddr>>,
    
    /// For restricted NAT: external addr -> set of allowed source addresses
    allowed_sources: RwLock<HashMap<SocketAddr, HashSet<SocketAddr>>>,
    
    /// Type of NAT behavior to simulate
    nat_type: NatType,
    
    /// External IP to use for mappings
    external_ip: String,
    
    /// Next available external port
    next_port: Mutex<u16>,

    /// Debug name for this NAT
    name: String,
}

impl NatSimulator {
    /// Create a new NAT simulator
    fn new(nat_type: NatType, external_ip: &str, name: &str) -> Self {
        println!("Creating NAT simulator: {} with type {:?} and external IP {}", name, nat_type, external_ip);
        Self {
            internal_to_external: RwLock::new(HashMap::new()),
            symmetric_mappings: RwLock::new(HashMap::new()),
            allowed_sources: RwLock::new(HashMap::new()),
            nat_type,
            external_ip: external_ip.to_string(),
            next_port: Mutex::new(10000),
            name: name.to_string(),
        }
    }
    
    /// Get the next available external port
    fn get_next_port(&self) -> u16 {
        let mut port = self.next_port.lock().unwrap();
        let result = *port;
        *port += 1;
        result
    }
    
    /// Map an internal address to an external address when sending to a destination
    fn map_outbound(&self, internal_addr: SocketAddr, dest_addr: SocketAddr) -> SocketAddr {
        let result = match self.nat_type {
            NatType::Symmetric => {
                // For symmetric NAT, use a different mapping for each destination
                let mut mappings = self.symmetric_mappings.write().unwrap();
                let key = (internal_addr, dest_addr);
                
                if let Some(existing) = mappings.get(&key) {
                    println!("[{}] Using existing symmetric mapping: {}:{} -> {} (for dest {})", 
                            self.name, internal_addr, dest_addr, existing, dest_addr);
                    *existing
                } else {
                    let external_port = self.get_next_port();
                    let external_addr: SocketAddr = format!("{}:{}", self.external_ip, external_port).parse().unwrap();
                    
                    println!("[{}] Creating new symmetric mapping: {}:{} -> {}", 
                            self.name, internal_addr, dest_addr, external_addr);
                    
                    // Also update the main mapping for convenience
                    let mut main_mapping = self.internal_to_external.write().unwrap();
                    main_mapping.insert(internal_addr, external_addr);
                    
                    mappings.insert(key, external_addr);
                    external_addr
                }
            },
            _ => {
                // For other NAT types, use a consistent mapping
                let mut mapping = self.internal_to_external.write().unwrap();
                
                if let Some(existing) = mapping.get(&internal_addr) {
                    println!("[{}] Using existing mapping: {} -> {}", 
                            self.name, internal_addr, existing);
                    *existing
                } else {
                    let external_port = self.get_next_port();
                    let external_addr = format!("{}:{}", self.external_ip, external_port).parse().unwrap();
                    
                    println!("[{}] Creating new mapping: {} -> {}", 
                            self.name, internal_addr, external_addr);
                    
                    mapping.insert(internal_addr, external_addr);
                    external_addr
                }
            }
        };
        
        // For all NAT types, we need to record that this destination is allowed to send traffic back
        // (this doesn't apply to full cone, but doesn't hurt to do it anyway)
        self.record_outbound_connection(dest_addr, result);
        
        result
    }
    
    /// Check if inbound traffic is allowed from the source to the destination
    fn allow_inbound(&self, addr: SocketAddr, src_addr: SocketAddr) -> bool {
        // The key issue is that we're getting an internal address (addr) but need to check
        // it against external mappings. For full cone, we should allow all traffic to
        // any port that has been mapped, regardless of source.
        
        // Find if this internal address has an external mapping
        let internal_to_external = self.internal_to_external.read().unwrap();
        let external_addr = match internal_to_external.get(&addr) {
            Some(ext_addr) => {
                println!("[{}] Found mapping for internal address {} -> {}", 
                         self.name, addr, ext_addr);
                *ext_addr
            },
            None => {
                // No mapping found for this internal address
                println!("[{}] No mapping found for {}", self.name, addr);
                return false;
            }
        };
        
        let result = match self.nat_type {
            NatType::FullCone => {
                // Full cone NAT allows all inbound traffic once the mapping is established
                println!("[{}] Full cone NAT allowing traffic to {} from {}", 
                         self.name, addr, src_addr);
                true
            },
            NatType::RestrictedCone => {
                // Restricted cone NAT only allows traffic from previously contacted hosts
                let allowed = self.allowed_sources.read().unwrap();
                let result = if let Some(sources) = allowed.get(&external_addr) {
                    let exists = sources.contains(&src_addr);
                    println!("[{}] Restricted cone check for {} from {}: {} (sources: {:?})", 
                             self.name, external_addr, src_addr, exists, sources);
                    exists
                } else {
                    println!("[{}] Restricted cone check for {} from {}: false (no mapping)", 
                             self.name, external_addr, src_addr);
                    false
                };
                result
            },
            NatType::PortRestrictedCone => {
                // Port-restricted cone NAT is like restricted but also checks port
                let allowed = self.allowed_sources.read().unwrap();
                let result = if let Some(sources) = allowed.get(&external_addr) {
                    let exists = sources.contains(&src_addr);
                    println!("[{}] Port-restricted cone check for {} from {}: {} (sources: {:?})", 
                             self.name, external_addr, src_addr, exists, sources);
                    exists
                } else {
                    println!("[{}] Port-restricted cone check for {} from {}: false (no mapping)", 
                             self.name, external_addr, src_addr);
                    false
                };
                result
            },
            NatType::Symmetric => {
                // Symmetric NAT only allows traffic matching the specific mapping
                let mappings = self.symmetric_mappings.read().unwrap();
                let result = mappings.iter().any(|(&(internal, dest), &ext)| {
                    let exists = internal == addr && ext == external_addr && dest == src_addr;
                    if exists {
                        println!("[{}] Symmetric check for {} from {}: true (found mapping with dest {})", 
                                 self.name, external_addr, src_addr, dest);
                    }
                    exists
                });
                if !result {
                    println!("[{}] Symmetric check for {} from {}: false (no mapping found)", 
                             self.name, external_addr, src_addr);
                }
                result
            }
        };

        result
    }
    
    /// Record an outbound connection for restricted/port-restricted NAT types
    fn record_outbound_connection(&self, dest_addr: SocketAddr, external_addr: SocketAddr) {
        // For restricted and port-restricted NAT, record that this destination is allowed to send traffic back
        match self.nat_type {
            NatType::RestrictedCone | NatType::PortRestrictedCone | NatType::FullCone | NatType::Symmetric => {
                let mut allowed = self.allowed_sources.write().unwrap();
                let entry = allowed.entry(external_addr).or_insert_with(HashSet::new);
                entry.insert(dest_addr);
                println!("[{}] Recorded outbound connection: {} can receive from {}", 
                        self.name, external_addr, dest_addr);
            },
        }
    }
    
    /// Map an external address back to an internal address
    fn map_inbound(&self, external_addr: SocketAddr) -> Option<SocketAddr> {
        let mapping = self.internal_to_external.read().unwrap();
        let result = mapping.iter()
            .find_map(|(&internal, &external)| {
                if external == external_addr {
                    Some(internal)
                } else {
                    None
                }
            });
            
        if let Some(internal) = result {
            println!("[{}] Mapped inbound address {} -> {}", 
                    self.name, external_addr, internal);
        } else {
            println!("[{}] Failed to map inbound address {}", 
                    self.name, external_addr);
        }
        
        result
    }
    
    /// Print the current NAT state (for debugging)
    fn print_state(&self) {
        println!("\n=== NAT STATE: {} ({:?}) ===", self.name, self.nat_type);
        
        let internal_to_external = self.internal_to_external.read().unwrap();
        println!("Internal to external mappings:");
        for (internal, external) in internal_to_external.iter() {
            println!("  {} -> {}", internal, external);
        }
        
        let symmetric_mappings = self.symmetric_mappings.read().unwrap();
        println!("Symmetric mappings:");
        for (key, value) in symmetric_mappings.iter() {
            println!("  {}:{} -> {}", key.0, key.1, value);
        }
        
        let allowed_sources = self.allowed_sources.read().unwrap();
        println!("Allowed sources:");
        for (external, sources) in allowed_sources.iter() {
            println!("  {} can receive from: {:?}", external, sources);
        }
        println!("====================\n");
    }
}

/// A network with simulated NAT devices
struct NatNetwork {
    /// The underlying simulated network
    network: Arc<SimulatedNetwork>,
    
    /// NAT simulators for each subnet
    nat_simulators: HashMap<String, Arc<NatSimulator>>,
    
    /// Mapping of node address to subnet
    node_subnets: RwLock<HashMap<SocketAddr, String>>,
}

impl NatNetwork {
    /// Create a new NAT network with the given simulated network
    fn new(network: Arc<SimulatedNetwork>) -> Self {
        Self {
            network,
            nat_simulators: HashMap::new(),
            node_subnets: RwLock::new(HashMap::new()),
        }
    }
    
    /// Add a NAT subnet to the network
    fn add_nat_subnet(&mut self, subnet_name: &str, nat_type: NatType, external_ip: &str) {
        let simulator = Arc::new(NatSimulator::new(nat_type, external_ip, subnet_name));
        self.nat_simulators.insert(subnet_name.to_string(), simulator);
    }
    
    /// Register a node on a specific NAT subnet
    fn register_node_on_subnet(&self, addr: SocketAddr, subnet_name: &str) {
        let mut subnets = self.node_subnets.write().unwrap();
        subnets.insert(addr, subnet_name.to_string());
        
        // Also register the node with the underlying network
        self.network.register_endpoint(addr);
        
        println!("Registered node {} on subnet {}", addr, subnet_name);
    }
    
    /// Send a message from one node to another, accounting for NAT
    fn send_message(&self, from: SocketAddr, to: SocketAddr, data: Vec<u8>) {
        println!("\nSending message: {} -> {} (data len: {})", from, to, data.len());
        
        // Get the source subnet
        let subnets = self.node_subnets.read().unwrap();
        let from_subnet = subnets.get(&from).cloned();
        let to_subnet = subnets.get(&to).cloned();
        
        println!("From subnet: {:?}, To subnet: {:?}", from_subnet, to_subnet);
        
        // If sender is behind NAT, translate address
        let (actual_from, actual_to) = match (from_subnet, to_subnet) {
            (Some(from_subnet), Some(to_subnet)) if from_subnet == to_subnet => {
                // Both in the same subnet - direct communication
                println!("Direct communication within subnet: {}", from_subnet);
                (from, to)
            },
            (Some(from_subnet), _) => {
                // Sender is behind NAT
                let nat = &self.nat_simulators[&from_subnet];
                let external_from = nat.map_outbound(from, to);
                
                println!("Translated send: {} -> {} (via NAT {} as {})", 
                         from, to, from_subnet, external_from);
                
                (external_from, to)
            },
            (None, _) => {
                // Sender is not behind NAT
                println!("Sender {} is not behind NAT", from);
                (from, to)
            }
        };
        
        // Send the message with the translated addresses
        self.network.send_message(actual_from, actual_to, data);
    }
    
    /// Receive a message at a node, accounting for NAT
    fn receive_message(&self, addr: SocketAddr) -> Option<(Vec<u8>, SocketAddr)> {
        println!("\nAttempting to receive message at: {}", addr);
        
        let subnets = self.node_subnets.read().unwrap();
        let subnet = subnets.get(&addr).cloned();
        
        if let Some(ref s) = subnet {
            println!("Node {} is in subnet {}", addr, s);
        } else {
            println!("Node {} is not behind NAT", addr);
        }
        
        // For nodes not behind NAT, we pass through to the underlying network
        if subnet.is_none() {
            let msg = self.network.receive_message(addr);
            if let Some((data, src_addr)) = &msg {
                println!("Received message at {} from {} (data len: {})", 
                         addr, src_addr, data.len());
            }
            return msg;
        }
        
        let msg = self.network.receive_message(addr);
        
        if let Some((data, src_addr)) = msg {
            println!("Received message at {} from {} (data len: {})", 
                     addr, src_addr, data.len());
            
            // If recipient is behind NAT, check if inbound traffic is allowed
            if let Some(subnet) = subnet {
                let nat = &self.nat_simulators[&subnet];
                
                // Debug: print NAT state
                nat.print_state();
                
                if !nat.allow_inbound(addr, src_addr) {
                    // Blocked by NAT - drop the packet
                    println!("Message blocked by NAT {} for {} from {}", 
                             subnet, addr, src_addr);
                    return None;
                }
                
                // Translate the source address back if needed
                if let Some(original_src) = nat.map_inbound(src_addr) {
                    println!("Translated receive: {} appears as {} to node at {}", 
                             src_addr, original_src, addr);
                    return Some((data, original_src));
                }
            }
            
            Some((data, src_addr))
        } else {
            println!("No message available for {}", addr);
            None
        }
    }
    
    /// Print the state of all NAT simulators
    fn print_all_nat_states(&self) {
        println!("\n=== FULL NETWORK STATE ===");
        for (name, nat) in &self.nat_simulators {
            nat.print_state();
        }
        println!("====================\n");
    }
}

/// Create a test network with simulated NAT environments
fn create_test_nat_network() -> (Arc<SimulatedNetwork>, NatNetwork) {
    let sim_network = Arc::new(SimulatedNetwork::new());
    let mut nat_network = NatNetwork::new(sim_network.clone());
    
    // Add various NAT subnets
    nat_network.add_nat_subnet("full-cone", NatType::FullCone, "203.0.113.1");
    nat_network.add_nat_subnet("restricted", NatType::RestrictedCone, "203.0.113.2");
    nat_network.add_nat_subnet("port-restricted", NatType::PortRestrictedCone, "203.0.113.3");
    nat_network.add_nat_subnet("symmetric", NatType::Symmetric, "203.0.113.4");
    
    (sim_network, nat_network)
}

/// Creates a virtual socket address for testing
fn create_virtual_addr() -> SocketAddr {
    let mut rng = thread_rng();
    let port = rng.gen_range(10000..60000);
    format!("127.0.0.1:{}", port).parse().unwrap()
}

/// Test that peers behind restrictive NATs can connect through relays
#[test]
fn test_relay_nat_traversal() {
    println!("\n=== RUNNING TEST: test_relay_nat_traversal ===\n");
    
    // Create the simulated networks
    let (sim_network, nat_network) = create_test_nat_network();
    
    // Create addresses
    let relay_addr = create_virtual_addr();
    let peer1_addr = create_virtual_addr();
    let peer2_addr = create_virtual_addr();
    
    // Register endpoints with the network
    sim_network.register_endpoint(relay_addr);
    
    // Put peer1 behind a symmetric NAT (most restrictive)
    nat_network.register_node_on_subnet(peer1_addr, "symmetric");
    
    // Put peer2 behind a port-restricted NAT
    nat_network.register_node_on_subnet(peer2_addr, "port-restricted");
    
    // Create relay info
    let mut relay_pubkey = [0u8; 32];
    thread_rng().fill(&mut relay_pubkey);
    
    let relay_info = RelayNodeInfo::new(
        relay_pubkey,
        vec![relay_addr.to_string()],
        100
    );
    
    // Create peer public keys
    let mut peer1_pubkey = [0u8; 32];
    thread_rng().fill(&mut peer1_pubkey);
    
    let mut peer2_pubkey = [0u8; 32];
    thread_rng().fill(&mut peer2_pubkey);
    
    // Simulate peer1 sending a connection request to the relay
    let request = ConnectionRequest::new(peer1_pubkey, peer2_pubkey);
    let message = RelayMessage::ConnectionRequest(request);
    let data = message.serialize().unwrap();
    
    nat_network.send_message(peer1_addr, relay_addr, data);
    
    // Wait for the message to arrive at the relay
    thread::sleep(Duration::from_millis(10));
    
    // The relay should receive it
    let (relay_data, src_addr) = sim_network.receive_message(relay_addr).expect("Relay should receive message");
    
    // Verify message was received
    let relay_message = RelayMessage::deserialize(&relay_data).expect("Should deserialize correctly");
    
    match relay_message {
        RelayMessage::ConnectionRequest(req) => {
            assert_eq!(req.peer_pubkey, peer1_pubkey);
            assert_eq!(req.target_pubkey, peer2_pubkey);
            
            // Simulate relay accepting the connection
            let session_id = 12345;
            let response = ConnectionResponse::success(req.nonce, session_id);
            let response_message = RelayMessage::ConnectionResponse(response);
            let response_data = response_message.serialize().unwrap();
            
            // Relay sends response back to the source address it received from (external NAT address)
            println!("\nRelay sending response to: {}", src_addr);
            sim_network.send_message(relay_addr, src_addr, response_data);
        },
        _ => panic!("Expected ConnectionRequest message"),
    }
    
    // Debug: Print all NAT states before receiving
    nat_network.print_all_nat_states();
    
    // Wait for response to arrive
    thread::sleep(Duration::from_millis(20));
    
    // Peer1 should receive the response from the relay
    let (peer1_data, _) = nat_network.receive_message(peer1_addr).expect("Peer1 should receive response");
    
    // Verify the response
    let peer1_message = RelayMessage::deserialize(&peer1_data).expect("Should deserialize correctly");
    let session_id = match peer1_message {
        RelayMessage::ConnectionResponse(resp) => {
            assert!(resp.is_success());
            resp.session_id.expect("Should have session ID")
        },
        _ => panic!("Expected ConnectionResponse message"),
    };
    
    // Now try to send data from peer1 to peer2 through the relay
    let test_payload = b"Hello through NAT!".to_vec();
    let packet = RelayPacket::new(peer2_pubkey, session_id, test_payload.clone());
    let packet_message = RelayMessage::ForwardPacket(packet);
    let packet_data = packet_message.serialize().unwrap();
    
    // Peer1 sends to relay
    nat_network.send_message(peer1_addr, relay_addr, packet_data);
    
    // Wait for packet to arrive at relay
    thread::sleep(Duration::from_millis(10));
    
    // Relay receives it
    let (relay_packet_data, _) = sim_network.receive_message(relay_addr).expect("Relay should receive packet");
    
    // Verify packet content
    let relay_packet_message = RelayMessage::deserialize(&relay_packet_data).expect("Should deserialize correctly");
    match relay_packet_message {
        RelayMessage::ForwardPacket(packet) => {
            assert_eq!(packet.header.dest_peer_id, peer2_pubkey);
            assert_eq!(packet.header.session_id, session_id);
            assert_eq!(packet.payload, test_payload);
            
            // Simulate relay forwarding to peer2
            sim_network.send_message(relay_addr, peer2_addr, relay_packet_data);
        },
        _ => panic!("Expected ForwardPacket message"),
    }
    
    // Wait for packet to arrive at peer2
    thread::sleep(Duration::from_millis(10));
    
    // Peer2 should receive it
    let (peer2_data, _) = nat_network.receive_message(peer2_addr).expect("Peer2 should receive packet");
    
    // Verify the data
    let peer2_message = RelayMessage::deserialize(&peer2_data).expect("Should deserialize correctly");
    match peer2_message {
        RelayMessage::ForwardPacket(packet) => {
            assert_eq!(packet.payload, test_payload);
        },
        _ => panic!("Expected ForwardPacket message"),
    };
}

/// Test that direct connections fail between peers with restrictive NATs
#[test]
fn test_nat_direct_connection_failure() {
    println!("\n=== RUNNING TEST: test_nat_direct_connection_failure ===\n");
    
    // Create the simulated networks
    let (sim_network, nat_network) = create_test_nat_network();
    
    // Create addresses
    let peer1_addr = create_virtual_addr();
    let peer2_addr = create_virtual_addr();
    
    // Put peer1 behind a symmetric NAT
    nat_network.register_node_on_subnet(peer1_addr, "symmetric");
    
    // Put peer2 behind a port-restricted NAT
    nat_network.register_node_on_subnet(peer2_addr, "port-restricted");
    
    // Attempt direct connection from peer1 to peer2
    let test_data = b"Direct connection attempt".to_vec();
    nat_network.send_message(peer1_addr, peer2_addr, test_data.clone());
    
    // Wait a moment to make sure the message has time to arrive if it's going to
    thread::sleep(Duration::from_millis(10));
    
    // Debug: Print all NAT states before receiving
    nat_network.print_all_nat_states();
    
    // Peer2 should NOT receive the message due to NAT restrictions
    let result = nat_network.receive_message(peer2_addr);
    assert!(result.is_none(), "Message should be blocked by NAT");
    
    // Try the other direction as well
    let reverse_data = b"Direct connection from peer2 to peer1".to_vec();
    nat_network.send_message(peer2_addr, peer1_addr, reverse_data.clone());
    
    // Peer1 should also not receive this message
    thread::sleep(Duration::from_millis(10));
    let result = nat_network.receive_message(peer1_addr);
    assert!(result.is_none(), "Message should be blocked by NAT");
}

/// Test different NAT types with varying restrictions
#[test]
fn test_nat_type_restrictions() {
    println!("\n=== RUNNING TEST: test_nat_type_restrictions ===\n");
    
    // Create the simulated networks
    let (sim_network, nat_network) = create_test_nat_network();
    
    // Create addresses for testing
    let public_addr = create_virtual_addr();
    let full_cone_addr = create_virtual_addr();
    let restricted_addr = create_virtual_addr();
    let port_restricted_addr = create_virtual_addr();
    let symmetric_addr = create_virtual_addr();
    
    // Register endpoints
    sim_network.register_endpoint(public_addr);
    nat_network.register_node_on_subnet(full_cone_addr, "full-cone");
    nat_network.register_node_on_subnet(restricted_addr, "restricted");
    nat_network.register_node_on_subnet(port_restricted_addr, "port-restricted");
    nat_network.register_node_on_subnet(symmetric_addr, "symmetric");
    
    // For full cone NAT, we need an outbound connection first to establish the mapping
    // Even though we don't need it for the permission, we need it for the address mapping
    let initial_data = b"Initial mapping setup".to_vec();
    nat_network.send_message(full_cone_addr, public_addr, initial_data);
    
    // Process any messages to ensure mappings are created
    thread::sleep(Duration::from_millis(10));
    if let Some(_) = sim_network.receive_message(public_addr) {
        println!("Received initial mapping setup message");
    }
    
    // Debug: Print all NAT states
    nat_network.print_all_nat_states();
    
    // Test 1: Public -> Full Cone (should succeed)
    let test_data = b"Public to Full Cone".to_vec();
    sim_network.send_message(public_addr, full_cone_addr, test_data.clone());
    
    // Wait for the message to propagate
    thread::sleep(Duration::from_millis(20));
    
    let result = nat_network.receive_message(full_cone_addr);
    assert!(result.is_some(), "Full cone NAT should allow unsolicited inbound traffic");
    
    // Test 2: Public -> Restricted (should fail without prior outbound connection)
    let test_data = b"Public to Restricted".to_vec();
    sim_network.send_message(public_addr, restricted_addr, test_data.clone());
    
    thread::sleep(Duration::from_millis(10));
    
    let result = nat_network.receive_message(restricted_addr);
    assert!(result.is_none(), "Restricted NAT should block unsolicited inbound traffic");
    
    // Test 3: Establish outbound connection first, then try inbound
    let outbound_data = b"Restricted to Public".to_vec();
    nat_network.send_message(restricted_addr, public_addr, outbound_data.clone());
    
    thread::sleep(Duration::from_millis(10));
    
    if let Some(_) = sim_network.receive_message(public_addr) {
        println!("Received outbound message from restricted NAT");
    }
    
    // Debug: Print all NAT states
    nat_network.print_all_nat_states();
    
    // Now inbound should work
    let test_data = b"Public to Restricted after outbound".to_vec();
    sim_network.send_message(public_addr, restricted_addr, test_data.clone());
    
    thread::sleep(Duration::from_millis(10));
    
    let result = nat_network.receive_message(restricted_addr);
    assert!(result.is_some(), "Restricted NAT should allow traffic from previously contacted host");
    
    // Test 4: Symmetric NAT with multiple destinations should use different mappings
    let dest1 = create_virtual_addr();
    let dest2 = create_virtual_addr();
    sim_network.register_endpoint(dest1);
    sim_network.register_endpoint(dest2);
    
    let data1 = b"Symmetric to Dest1".to_vec();
    let data2 = b"Symmetric to Dest2".to_vec();
    
    nat_network.send_message(symmetric_addr, dest1, data1.clone());
    nat_network.send_message(symmetric_addr, dest2, data2.clone());
    
    thread::sleep(Duration::from_millis(10));
    
    // Check the received messages have different source addresses
    let (_, src1) = sim_network.receive_message(dest1).unwrap();
    let (_, src2) = sim_network.receive_message(dest2).unwrap();
    
    assert_ne!(src1, src2, "Symmetric NAT should use different mappings for different destinations");
}

/// Test multiple peers behind the same NAT connecting through a relay
#[test]
fn test_multiple_peers_same_nat() {
    println!("\n=== RUNNING TEST: test_multiple_peers_same_nat ===\n");
    
    // Create the simulated networks
    let (sim_network, nat_network) = create_test_nat_network();
    
    // Create addresses
    let relay_addr = create_virtual_addr();
    let peer1_addr = create_virtual_addr();
    let peer2_addr = create_virtual_addr();
    let peer3_addr = create_virtual_addr();
    
    // Register endpoints with the network
    sim_network.register_endpoint(relay_addr);
    
    // Put all peers behind the same symmetric NAT (most restrictive)
    nat_network.register_node_on_subnet(peer1_addr, "symmetric");
    nat_network.register_node_on_subnet(peer2_addr, "symmetric");
    nat_network.register_node_on_subnet(peer3_addr, "symmetric");
    
    // Create relay info
    let mut relay_pubkey = [0u8; 32];
    thread_rng().fill(&mut relay_pubkey);
    
    // Create peer public keys
    let mut peer1_pubkey = [0u8; 32];
    thread_rng().fill(&mut peer1_pubkey);
    
    let mut peer2_pubkey = [0u8; 32];
    thread_rng().fill(&mut peer2_pubkey);
    
    let mut peer3_pubkey = [0u8; 32];
    thread_rng().fill(&mut peer3_pubkey);
    
    // Debug: Print all NAT states 
    nat_network.print_all_nat_states();
    
    // Helper function to establish a connection through the relay
    let establish_connection = |from_addr: SocketAddr, from_pubkey: [u8; 32], to_pubkey: [u8; 32]| -> u64 {
        println!("\nEstablishing connection: {} -> relay for target with pubkey {:?}", from_addr, to_pubkey);
        
        // Send connection request
        let request = ConnectionRequest::new(from_pubkey, to_pubkey);
        let message = RelayMessage::ConnectionRequest(request);
        let data = message.serialize().unwrap();
        
        nat_network.send_message(from_addr, relay_addr, data);
        
        // Wait for the message to propagate through NAT
        thread::sleep(Duration::from_millis(20));
        
        // The relay receives it
        let (relay_data, src_addr) = sim_network.receive_message(relay_addr).expect("Relay should receive message");
        let relay_message = RelayMessage::deserialize(&relay_data).unwrap();
        
        let req = match relay_message {
            RelayMessage::ConnectionRequest(req) => req,
            _ => panic!("Expected ConnectionRequest"),
        };
        
        // Relay sends success response
        let session_id = thread_rng().gen::<u64>();
        let response = ConnectionResponse::success(req.nonce, session_id);
        let response_message = RelayMessage::ConnectionResponse(response);
        let response_data = response_message.serialize().unwrap();
        
        println!("Relay sending response to: {}", src_addr);
        sim_network.send_message(relay_addr, src_addr, response_data);
        
        // Wait for the response to propagate back through NAT
        thread::sleep(Duration::from_millis(20));
        
        // Return the session id
        session_id
    };
    
    // Establish connections
    let session1 = establish_connection(peer1_addr, peer1_pubkey, peer2_pubkey);
    
    // Each peer should receive their response
    let receive_response = |addr: SocketAddr| {
        // Debug: Print all NAT states before receiving
        nat_network.print_all_nat_states();
        
        let (data, _) = nat_network.receive_message(addr).expect("Peer should receive response");
        let message = RelayMessage::deserialize(&data).unwrap();
        match message {
            RelayMessage::ConnectionResponse(resp) => {
                assert!(resp.is_success());
            },
            _ => panic!("Expected ConnectionResponse"),
        }
    };
    
    // Verify peer1 received the response
    receive_response(peer1_addr);
    
    // Now establish session2 after first session is complete
    let session2 = establish_connection(peer2_addr, peer2_pubkey, peer3_pubkey);
    receive_response(peer2_addr);
    
    // And session3
    let session3 = establish_connection(peer3_addr, peer3_pubkey, peer1_pubkey);
    receive_response(peer3_addr);
    
    // Now try to send data between the peers through the relay
    let send_data = |from_addr: SocketAddr, to_pubkey: [u8; 32], session_id: u64, payload: Vec<u8>| {
        let packet = RelayPacket::new(to_pubkey, session_id, payload);
        let message = RelayMessage::ForwardPacket(packet);
        let data = message.serialize().unwrap();
        
        nat_network.send_message(from_addr, relay_addr, data);
        
        // Wait for packet to arrive
        thread::sleep(Duration::from_millis(10));
    };
    
    // Peer1 -> Peer2
    let data12 = b"From peer1 to peer2".to_vec();
    send_data(peer1_addr, peer2_pubkey, session1, data12.clone());
    
    // Peer2 -> Peer3
    let data23 = b"From peer2 to peer3".to_vec();
    send_data(peer2_addr, peer3_pubkey, session2, data23.clone());
    
    // Peer3 -> Peer1
    let data31 = b"From peer3 to peer1".to_vec();
    send_data(peer3_addr, peer1_pubkey, session3, data31.clone());
    
    // Relay should receive all packets
    for _ in 0..3 {
        let (data, _) = sim_network.receive_message(relay_addr).expect("Relay should receive packet");
        let message = RelayMessage::deserialize(&data).unwrap();
        match message {
            RelayMessage::ForwardPacket(_) => {
                // In a real implementation, the relay would forward this to the appropriate destination
                // For this test, we're just verifying that the packet reached the relay
            },
            _ => panic!("Expected ForwardPacket"),
        }
    }
    
    // Verify that all peers have established their connections successfully
    assert_ne!(session1, session2);
    assert_ne!(session2, session3);
    assert_ne!(session3, session1);
} 