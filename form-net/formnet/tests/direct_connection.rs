//! Direct connection tests
//!
//! This module tests direct peer-to-peer connections without relays.

use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use std::thread;

use rand::{Rng, thread_rng};

mod network_conditions;
use network_conditions::SimulatedNetwork;

/// A peer that can establish direct connections
struct DirectPeer {
    /// The peer's address
    addr: SocketAddr,
    
    /// The simulated network
    network: Arc<SimulatedNetwork>,
    
    /// Received messages
    received_messages: Arc<Mutex<Vec<Vec<u8>>>>,
}

impl DirectPeer {
    /// Create a new peer with the given network
    fn new(network: Arc<SimulatedNetwork>) -> Self {
        let addr = create_virtual_addr();
        network.register_endpoint(addr);
        
        DirectPeer {
            addr,
            network,
            received_messages: Arc::new(Mutex::new(Vec::new())),
        }
    }
    
    /// Send data to the target peer
    fn send_data(&self, target: &DirectPeer, data: Vec<u8>) -> Result<(), String> {
        self.network.send_message(self.addr, target.addr, data);
        Ok(())
    }
    
    /// Process any received messages
    fn process_messages(&self) -> Result<(), String> {
        while let Some((data, _src_addr)) = self.network.receive_message(self.addr) {
            // Store the received data
            let mut received = self.received_messages.lock().unwrap();
            received.push(data);
        }
        
        Ok(())
    }
    
    /// Get received messages
    fn get_received_messages(&self) -> Vec<Vec<u8>> {
        let received = self.received_messages.lock().unwrap();
        received.clone()
    }
    
    /// Clear received messages
    fn clear_received_messages(&self) {
        let mut received = self.received_messages.lock().unwrap();
        received.clear();
    }
    
    /// Get the number of messages received
    fn get_message_count(&self) -> usize {
        let received = self.received_messages.lock().unwrap();
        received.len()
    }
}

/// Creates a virtual socket address for testing
fn create_virtual_addr() -> SocketAddr {
    let mut rng = thread_rng();
    let port = rng.gen_range(10000..60000);
    format!("127.0.0.1:{}", port).parse().unwrap()
}

#[test]
fn test_direct_connection_basic() {
    // Create a simulated network with default settings (no latency, no packet loss)
    let network = Arc::new(SimulatedNetwork::new());
    
    // Create two peers
    let peer1 = DirectPeer::new(network.clone());
    let peer2 = DirectPeer::new(network.clone());
    
    // Send a message from peer1 to peer2
    let test_data = b"Hello from peer1".to_vec();
    peer1.send_data(&peer2, test_data.clone()).expect("Send should succeed");
    
    // Process messages at peer2
    peer2.process_messages().expect("Processing should succeed");
    
    // Verify peer2 received the message
    let messages = peer2.get_received_messages();
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0], test_data);
    
    // Send a reply from peer2 to peer1
    let reply_data = b"Hello from peer2".to_vec();
    peer2.send_data(&peer1, reply_data.clone()).expect("Send should succeed");
    
    // Process messages at peer1
    peer1.process_messages().expect("Processing should succeed");
    
    // Verify peer1 received the reply
    let messages = peer1.get_received_messages();
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0], reply_data);
}

#[test]
fn test_direct_connection_with_latency() {
    // Create a simulated network
    let network = Arc::new(SimulatedNetwork::new());
    
    // Create two peers
    let peer1 = DirectPeer::new(network.clone());
    let peer2 = DirectPeer::new(network.clone());
    
    // Set 50ms latency between the peers
    network.set_link_conditions(peer1.addr, peer2.addr, 50, 0, 0);
    network.set_link_conditions(peer2.addr, peer1.addr, 50, 0, 0);
    
    // Send a message from peer1 to peer2
    let test_data = b"Hello with latency".to_vec();
    peer1.send_data(&peer2, test_data.clone()).expect("Send should succeed");
    
    // Immediately try to process (should not receive yet due to latency)
    peer2.process_messages().expect("Processing should succeed");
    assert_eq!(peer2.get_message_count(), 0, "Message should not arrive immediately with latency");
    
    // Wait for latency
    thread::sleep(Duration::from_millis(60));
    
    // Now process again
    peer2.process_messages().expect("Processing should succeed");
    
    // Verify peer2 received the message after latency
    let messages = peer2.get_received_messages();
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0], test_data);
}

#[test]
fn test_network_partition() {
    // Create a simulated network
    let network = Arc::new(SimulatedNetwork::new());
    
    // Create two peers
    let peer1 = DirectPeer::new(network.clone());
    let peer2 = DirectPeer::new(network.clone());
    
    // Test connectivity works initially
    let test_data1 = b"Before partition".to_vec();
    peer1.send_data(&peer2, test_data1.clone()).expect("Send should succeed");
    peer2.process_messages().expect("Processing should succeed");
    assert_eq!(peer2.get_message_count(), 1);
    assert_eq!(peer2.get_received_messages()[0], test_data1);
    peer2.clear_received_messages();
    
    // Create network partition by setting link down
    network.set_link_state(peer1.addr, peer2.addr, false);
    network.set_link_state(peer2.addr, peer1.addr, false);
    
    // Send message during partition
    let test_data2 = b"During partition".to_vec();
    peer1.send_data(&peer2, test_data2.clone()).expect("Send should succeed");
    peer2.process_messages().expect("Processing should succeed");
    
    // Verify no messages were received during partition
    assert_eq!(peer2.get_message_count(), 0, "No messages should be received during partition");
    
    // Restore network connectivity
    network.set_link_state(peer1.addr, peer2.addr, true);
    network.set_link_state(peer2.addr, peer1.addr, true);
    
    // Send message after partition is resolved
    let test_data3 = b"After partition".to_vec();
    peer1.send_data(&peer2, test_data3.clone()).expect("Send should succeed");
    peer2.process_messages().expect("Processing should succeed");
    
    // Verify message after partition was received
    let messages = peer2.get_received_messages();
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0], test_data3);
}

#[test]
fn test_asymmetric_connection() {
    // Create a simulated network
    let network = Arc::new(SimulatedNetwork::new());
    
    // Create two peers
    let peer1 = DirectPeer::new(network.clone());
    let peer2 = DirectPeer::new(network.clone());
    
    // Set up asymmetric conditions:
    // - peer1 to peer2: high latency (100ms)
    // - peer2 to peer1: low latency (10ms)
    network.set_link_conditions(peer1.addr, peer2.addr, 100, 0, 0);
    network.set_link_conditions(peer2.addr, peer1.addr, 10, 0, 0);
    
    // Send from peer1 to peer2 (high latency direction)
    let test_data1 = b"High latency direction".to_vec();
    peer1.send_data(&peer2, test_data1.clone()).expect("Send should succeed");
    
    // Check immediately (should not be there yet)
    peer2.process_messages().expect("Processing should succeed");
    assert_eq!(peer2.get_message_count(), 0, "Message should not arrive immediately with high latency");
    
    // Wait for high latency path
    thread::sleep(Duration::from_millis(110));
    
    // Now process again
    peer2.process_messages().expect("Processing should succeed");
    
    // Verify peer2 received the message after high latency
    assert_eq!(peer2.get_message_count(), 1);
    assert_eq!(peer2.get_received_messages()[0], test_data1);
    
    // Send from peer2 to peer1 (low latency direction)
    peer2.clear_received_messages();
    peer1.clear_received_messages();
    
    let test_data2 = b"Low latency direction".to_vec();
    peer2.send_data(&peer1, test_data2.clone()).expect("Send should succeed");
    
    // Wait for low latency path
    thread::sleep(Duration::from_millis(20));
    
    // Process at peer1
    peer1.process_messages().expect("Processing should succeed");
    
    // Verify peer1 received the message after low latency
    assert_eq!(peer1.get_message_count(), 1);
    assert_eq!(peer1.get_received_messages()[0], test_data2);
} 