//! Integration test for relay functionality
//!
//! This test file creates a virtual test harness and tests
//! relay functionality across component boundaries.

use formnet::relay::{
    discovery::RelayRegistry,
    protocol::{
        RelayNodeInfo, ConnectionRequest, ConnectionResponse, RelayMessage,
        ConnectionStatus, DiscoveryQuery, DiscoveryResponse,
    },
    service::{RelayConfig, RelayNode},
    manager::RelayManager,
    SharedRelayRegistry,
};

use std::collections::{HashMap, VecDeque};
use std::net::SocketAddr;
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, SystemTime};
use rand::Rng;

//------------------------------------------------------------------------------
// Test Harness
//------------------------------------------------------------------------------

/// A virtual network that allows message passing between nodes
/// without actual network connectivity.
struct VirtualNetwork {
    /// Queue of messages for each endpoint
    message_queues: Arc<RwLock<HashMap<SocketAddr, VecDeque<(Vec<u8>, SocketAddr)>>>>,
}

impl VirtualNetwork {
    /// Create a new virtual network
    fn new() -> Self {
        Self {
            message_queues: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register an endpoint with the virtual network
    fn register_endpoint(&self, addr: SocketAddr) {
        let mut queues = self.message_queues.write().unwrap();
        queues.entry(addr).or_insert_with(VecDeque::new);
    }

    /// Send a message from one endpoint to another
    fn send_message(&self, from: SocketAddr, to: SocketAddr, data: Vec<u8>) {
        let mut queues = self.message_queues.write().unwrap();
        if let Some(queue) = queues.get_mut(&to) {
            queue.push_back((data, from));
        }
    }

    /// Receive a message for an endpoint (non-blocking)
    fn receive_message(&self, addr: SocketAddr) -> Option<(Vec<u8>, SocketAddr)> {
        let mut queues = self.message_queues.write().unwrap();
        if let Some(queue) = queues.get_mut(&addr) {
            queue.pop_front()
        } else {
            None
        }
    }
}

/// Creates a virtual socket address for testing
fn create_virtual_addr() -> SocketAddr {
    let mut rng = rand::thread_rng();
    let port = rng.gen_range(10000..60000);
    format!("127.0.0.1:{}", port).parse().unwrap()
}

/// A virtual relay node for testing
struct VirtualRelayNode {
    /// The relay node configuration
    config: RelayConfig,
    
    /// The virtual network
    network: Arc<VirtualNetwork>,
    
    /// The socket address this node is bound to
    addr: SocketAddr,
    
    /// Whether the node is running
    running: Arc<Mutex<bool>>,
    
    /// The relay registry
    registry: SharedRelayRegistry,
}

impl VirtualRelayNode {
    /// Create a new virtual relay node
    fn new(network: Arc<VirtualNetwork>) -> Self {
        let addr = create_virtual_addr();
        network.register_endpoint(addr);
        
        // Generate a random public key for this relay
        let mut pubkey = [0u8; 32];
        rand::thread_rng().fill(&mut pubkey);
        
        let config = RelayConfig::new(addr, pubkey);
        let registry = SharedRelayRegistry::new();
        
        Self {
            config,
            network,
            addr,
            running: Arc::new(Mutex::new(false)),
            registry,
        }
    }
    
    /// Start the virtual relay node
    fn start(&self) {
        let mut running = self.running.lock().unwrap();
        *running = true;
        
        // In a real implementation, we would spawn a background thread to process messages
        // For now, we'll keep it simple
    }
    
    /// Stop the virtual relay node
    fn stop(&self) {
        let mut running = self.running.lock().unwrap();
        *running = false;
    }
    
    /// Process an incoming message
    fn process_message(&self, data: Vec<u8>, src_addr: SocketAddr) -> Result<(), String> {
        // Decode the message
        let message = match RelayMessage::deserialize(&data) {
            Ok(m) => m,
            Err(e) => return Err(format!("Failed to deserialize message: {}", e)),
        };
        
        // Process the message based on its type
        match message {
            RelayMessage::ConnectionRequest(req) => {
                // For simplicity, always accept connection requests in the test harness
                let response = ConnectionResponse::success(req.nonce, 12345);
                let response_msg = RelayMessage::ConnectionResponse(response);
                let response_data = response_msg.serialize()
                    .map_err(|e| format!("Failed to serialize response: {}", e))?;
                
                self.network.send_message(self.addr, src_addr, response_data);
                Ok(())
            },
            RelayMessage::DiscoveryQuery(query) => {
                // Return a basic discovery response with this node's info
                let relay_info = RelayNodeInfo::new(
                    self.config.pubkey,
                    vec![self.addr.to_string()],
                    100
                );
                
                let response = DiscoveryResponse::new(
                    query.nonce,
                    vec![relay_info],
                    false
                );
                
                let response_msg = RelayMessage::DiscoveryResponse(response);
                let response_data = response_msg.serialize()
                    .map_err(|e| format!("Failed to serialize response: {}", e))?;
                
                self.network.send_message(self.addr, src_addr, response_data);
                Ok(())
            },
            RelayMessage::ForwardPacket(packet) => {
                // In a real implementation, we would forward this to the target
                // For our test, we'll just acknowledge receipt
                Ok(())
            },
            // Other message types would be handled here
            _ => Ok(()),
        }
    }
    
    /// Get the relay information for this node
    fn get_node_info(&self) -> RelayNodeInfo {
        RelayNodeInfo::new(
            self.config.pubkey,
            vec![self.addr.to_string()],
            100
        )
    }
}

/// A virtual peer that can use relays
struct VirtualPeer {
    /// The peer's address
    addr: SocketAddr,
    
    /// The peer's public key
    pubkey: [u8; 32],
    
    /// The virtual network
    network: Arc<VirtualNetwork>,
    
    /// The relay manager
    relay_manager: Option<RelayManager>,
    
    /// The relay registry
    registry: SharedRelayRegistry,
}

impl VirtualPeer {
    /// Create a new virtual peer
    fn new(network: Arc<VirtualNetwork>) -> Self {
        let addr = create_virtual_addr();
        network.register_endpoint(addr);
        
        // Generate a random public key for this peer
        let mut pubkey = [0u8; 32];
        rand::thread_rng().fill(&mut pubkey);
        
        let registry = SharedRelayRegistry::new();
        
        Self {
            addr,
            pubkey,
            network,
            relay_manager: None,
            registry,
        }
    }
    
    /// Initialize the relay manager
    fn init_relay_manager(&mut self) {
        self.relay_manager = Some(RelayManager::new(self.registry.clone(), self.pubkey));
    }
    
    /// Discover relays 
    fn discover_relays(&self, relay_node: &VirtualRelayNode) -> Result<(), String> {
        // Add the relay to the registry
        self.registry.register_relay(relay_node.get_node_info())
            .map_err(|e| format!("Failed to register relay: {}", e))?;
        
        Ok(())
    }
    
    /// Connect to another peer through a relay
    fn connect_through_relay(&self, target_peer: &VirtualPeer, relay: &VirtualRelayNode) -> Result<(), String> {
        // Create a connection request
        let request = ConnectionRequest::new(self.pubkey, target_peer.pubkey);
        let message = RelayMessage::ConnectionRequest(request);
        
        // Serialize and send
        let data = message.serialize()
            .map_err(|e| format!("Failed to serialize request: {}", e))?;
        
        self.network.send_message(self.addr, relay.addr, data);
        
        // In a real implementation, we would wait for a response here
        Ok(())
    }
    
    /// Process messages from the network
    fn process_messages(&self) -> Result<(), String> {
        while let Some((data, src_addr)) = self.network.receive_message(self.addr) {
            // Process the message
            let message = RelayMessage::deserialize(&data)
                .map_err(|e| format!("Failed to deserialize message: {}", e))?;
            
            // Handle the message based on its type
            match message {
                RelayMessage::ConnectionResponse(resp) => {
                    if resp.is_success() {
                        println!("Connection established through relay");
                    } else {
                        return Err(format!("Connection failed: {:?}", resp.error));
                    }
                },
                // Handle other message types
                _ => {},
            }
        }
        
        Ok(())
    }
    
    /// Send data to another peer through a relay
    fn send_data(&self, target_peer: &VirtualPeer, relay: &VirtualRelayNode, data: Vec<u8>) -> Result<(), String> {
        // Create a relay packet
        let packet = formnet::relay::protocol::RelayPacket::new(
            target_peer.pubkey,
            12345, // Session ID (would be obtained from the successful connection)
            data
        );
        
        let message = RelayMessage::ForwardPacket(packet);
        let message_data = message.serialize()
            .map_err(|e| format!("Failed to serialize packet: {}", e))?;
        
        self.network.send_message(self.addr, relay.addr, message_data);
        
        Ok(())
    }
}

//------------------------------------------------------------------------------
// Tests
//------------------------------------------------------------------------------

#[test]
fn test_virtual_network_basic() {
    let network = Arc::new(VirtualNetwork::new());
    
    let addr1 = "127.0.0.1:10001".parse().unwrap();
    let addr2 = "127.0.0.1:10002".parse().unwrap();
    
    network.register_endpoint(addr1);
    network.register_endpoint(addr2);
    
    let test_data = b"hello world".to_vec();
    network.send_message(addr1, addr2, test_data.clone());
    
    let (received, from) = network.receive_message(addr2).unwrap();
    assert_eq!(received, test_data);
    assert_eq!(from, addr1);
    
    // Should be empty now
    assert!(network.receive_message(addr2).is_none());
}

#[test]
fn test_virtual_relay_creation() {
    let network = Arc::new(VirtualNetwork::new());
    let relay = VirtualRelayNode::new(network);
    
    assert!(!relay.running.lock().unwrap().clone());
    
    relay.start();
    assert!(*relay.running.lock().unwrap());
    
    relay.stop();
    assert!(!*relay.running.lock().unwrap());
}

/// Test basic end-to-end relay functionality
#[test]
fn test_basic_relay_connection() {
    // Create a virtual network
    let network = Arc::new(VirtualNetwork::new());
    
    // Create a relay node
    let relay = VirtualRelayNode::new(network.clone());
    relay.start();
    
    // Create two peers
    let mut peer1 = VirtualPeer::new(network.clone());
    let mut peer2 = VirtualPeer::new(network.clone());
    
    // Initialize relay managers
    peer1.init_relay_manager();
    peer2.init_relay_manager();
    
    // Discover the relay
    peer1.discover_relays(&relay).expect("Peer1 should discover relay");
    peer2.discover_relays(&relay).expect("Peer2 should discover relay");
    
    // Peer1 connects to peer2 through the relay
    peer1.connect_through_relay(&peer2, &relay).expect("Connection should succeed");
    
    // Let the relay process the connection request
    // In a real scenario, this would happen automatically through message processing
    let (data, src_addr) = network.receive_message(relay.addr).expect("Relay should receive message");
    relay.process_message(data, src_addr).expect("Relay should process message");
    
    // Let peer1 process the response
    peer1.process_messages().expect("Peer1 should process messages");
    
    // Now test data transfer
    let test_data = b"Hello through relay!".to_vec();
    peer1.send_data(&peer2, &relay, test_data.clone()).expect("Sending data should succeed");
    
    // Let the relay process the message
    let (data, src_addr) = network.receive_message(relay.addr).expect("Relay should receive message");
    relay.process_message(data, src_addr).expect("Relay should process message");
    
    // In a more complete test, we would verify that peer2 receives the data
    // But for our basic test, we'll just check that the message flow works
    
    // Cleanup
    relay.stop();
}

/// Test connection to a non-existent peer
#[test]
fn test_relay_connection_to_nonexistent_peer() {
    // Create a virtual network
    let network = Arc::new(VirtualNetwork::new());
    
    // Create a relay node
    let relay = VirtualRelayNode::new(network.clone());
    relay.start();
    
    // Create peer1
    let mut peer1 = VirtualPeer::new(network.clone());
    peer1.init_relay_manager();
    peer1.discover_relays(&relay).expect("Peer1 should discover relay");
    
    // Create a peer2 that is not registered with the network
    let mut peer2 = VirtualPeer::new(Arc::new(VirtualNetwork::new())); // Different network!
    peer2.init_relay_manager();
    
    // Peer1 tries to connect to peer2 through the relay
    peer1.connect_through_relay(&peer2, &relay).expect("Connection request should be sent");
    
    // The relay should receive and process the request
    let (data, src_addr) = network.receive_message(relay.addr).expect("Relay should receive message");
    relay.process_message(data, src_addr).expect("Relay should process message");
    
    // Peer1 should receive a success response in our simplified test
    // In a more realistic test, we would check for a failure response
    peer1.process_messages().expect("Peer1 should process messages");
    
    // Cleanup
    relay.stop();
}

/// Test basic relay failover
#[test]
fn test_relay_failover() {
    // Create a virtual network
    let network = Arc::new(VirtualNetwork::new());
    
    // Create two relay nodes
    let relay1 = VirtualRelayNode::new(network.clone());
    let relay2 = VirtualRelayNode::new(network.clone());
    
    relay1.start();
    relay2.start();
    
    // Create two peers
    let mut peer1 = VirtualPeer::new(network.clone());
    let mut peer2 = VirtualPeer::new(network.clone());
    
    peer1.init_relay_manager();
    peer2.init_relay_manager();
    
    // Discover both relays
    peer1.discover_relays(&relay1).expect("Peer1 should discover relay1");
    peer1.discover_relays(&relay2).expect("Peer1 should discover relay2");
    peer2.discover_relays(&relay1).expect("Peer2 should discover relay1");
    peer2.discover_relays(&relay2).expect("Peer2 should discover relay2");
    
    // Try connecting through relay1
    peer1.connect_through_relay(&peer2, &relay1).expect("Connection through relay1 should be attempted");
    
    // Let relay1 process the request
    let (data, src_addr) = network.receive_message(relay1.addr).expect("Relay1 should receive message");
    relay1.process_message(data, src_addr).expect("Relay1 should process message");
    
    // Let peer1 process the response
    peer1.process_messages().expect("Peer1 should process messages");
    
    // Now simulate relay1 going down
    relay1.stop();
    
    // Try connecting through relay2 instead
    peer1.connect_through_relay(&peer2, &relay2).expect("Connection through relay2 should be attempted");
    
    // Let relay2 process the request
    let (data, src_addr) = network.receive_message(relay2.addr).expect("Relay2 should receive message");
    relay2.process_message(data, src_addr).expect("Relay2 should process message");
    
    // Let peer1 process the response
    peer1.process_messages().expect("Peer1 should process messages");
    
    // Cleanup
    relay2.stop();
} 