//! Security and edge case tests
//!
//! This module tests various security concerns and edge cases for the networking code.

use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use std::thread;

use rand::{Rng, thread_rng};

mod network_conditions;
use network_conditions::SimulatedNetwork;

/// A peer for testing security and edge cases
struct SecurityPeer {
    /// The peer's address
    addr: SocketAddr,
    
    /// The simulated network
    network: Arc<SimulatedNetwork>,
    
    /// Received messages
    received_messages: Arc<Mutex<Vec<Vec<u8>>>>,
    
    /// Track the number of messages received
    message_count: Arc<Mutex<usize>>,
}

impl SecurityPeer {
    /// Create a new peer with the given network
    fn new(network: Arc<SimulatedNetwork>) -> Self {
        let addr = create_virtual_addr();
        network.register_endpoint(addr);
        
        SecurityPeer {
            addr,
            network,
            received_messages: Arc::new(Mutex::new(Vec::new())),
            message_count: Arc::new(Mutex::new(0)),
        }
    }
    
    /// Send data to the target peer
    fn send_data(&self, target: &SecurityPeer, data: Vec<u8>) -> Result<(), String> {
        self.network.send_message(self.addr, target.addr, data);
        Ok(())
    }
    
    /// Send a large number of messages to the target peer
    fn flood_messages(&self, target: &SecurityPeer, count: usize, size: usize) -> Result<(), String> {
        let data = generate_test_data(size);
        
        for _ in 0..count {
            self.send_data(target, data.clone())?;
        }
        
        Ok(())
    }
    
    /// Process any received messages
    fn process_messages(&self) -> Result<(), String> {
        while let Some((data, _src_addr)) = self.network.receive_message(self.addr) {
            // Store the received data
            {
                let mut received = self.received_messages.lock().unwrap();
                received.push(data);
            }
            
            // Increment message count
            {
                let mut count = self.message_count.lock().unwrap();
                *count += 1;
            }
        }
        
        Ok(())
    }
    
    /// Get message count
    fn get_message_count(&self) -> usize {
        let count = self.message_count.lock().unwrap();
        *count
    }
    
    /// Reset message count
    fn reset_message_count(&self) {
        let mut count = self.message_count.lock().unwrap();
        *count = 0;
        
        let mut received = self.received_messages.lock().unwrap();
        received.clear();
    }
    
    /// Get received messages
    fn get_received_messages(&self) -> Vec<Vec<u8>> {
        let received = self.received_messages.lock().unwrap();
        received.clone()
    }
}

/// Creates a virtual socket address for testing
fn create_virtual_addr() -> SocketAddr {
    let mut rng = thread_rng();
    let port = rng.gen_range(10000..60000);
    format!("127.0.0.1:{}", port).parse().unwrap()
}

/// Generate test data of the specified size
fn generate_test_data(size: usize) -> Vec<u8> {
    let mut data = Vec::with_capacity(size);
    for i in 0..size {
        data.push((i % 256) as u8);
    }
    data
}

#[test]
fn test_message_flood() {
    // Create a simulated network
    let network = Arc::new(SimulatedNetwork::new());
    
    // Create two peers
    let sender = SecurityPeer::new(network.clone());
    let receiver = SecurityPeer::new(network.clone());
    
    // Send a large number of small messages
    let message_count = 1000;
    let message_size = 64; // 64 bytes
    
    let start_time = Instant::now();
    
    // Send a flood of messages
    sender.flood_messages(&receiver, message_count, message_size).expect("Flood should succeed");
    
    // Process all messages
    receiver.process_messages().expect("Processing should succeed");
    
    let elapsed = start_time.elapsed();
    
    // Verify all messages were received (no drops despite the flood)
    assert_eq!(
        receiver.get_message_count(),
        message_count,
        "Receiver should handle message flood without dropping packets"
    );
    
    println!("Processed {} messages ({} bytes) in {:.2?}", 
             message_count, 
             message_count * message_size,
             elapsed);
}

#[test]
fn test_oversized_message() {
    // Create a simulated network
    let network = Arc::new(SimulatedNetwork::new());
    
    // Create two peers
    let sender = SecurityPeer::new(network.clone());
    let receiver = SecurityPeer::new(network.clone());
    
    // Try sending normal message first to establish baseline
    let normal_data = generate_test_data(1024); // 1KB
    sender.send_data(&receiver, normal_data.clone()).expect("Normal send should succeed");
    
    // Process messages
    receiver.process_messages().expect("Processing should succeed");
    assert_eq!(receiver.get_message_count(), 1, "Normal message should be received");
    
    // Reset counters
    receiver.reset_message_count();
    
    // Try sending very large message
    let large_size = 10 * 1024 * 1024; // 10MB (unreasonably large for most UDP packets)
    let large_data = generate_test_data(large_size);
    
    // This may still "succeed" at the send level, but network conditions 
    // should handle oversized messages appropriately
    sender.send_data(&receiver, large_data).expect("Large send operation should complete");
    
    // Process messages
    receiver.process_messages().expect("Processing should succeed");
    
    // We're not asserting specific behavior here, as it depends on the underlying network
    // implementation. In real networks, oversized UDP packets would be fragmented or dropped.
    println!("Received {} messages after attempting to send oversized message", 
             receiver.get_message_count());
}

#[test]
fn test_malformed_message_handling() {
    // Create a simulated network
    let network = Arc::new(SimulatedNetwork::new());
    
    // Create two peers
    let sender = SecurityPeer::new(network.clone());
    let receiver = SecurityPeer::new(network.clone());
    
    // Send valid message first to establish baseline
    let valid_data = b"Valid message".to_vec();
    sender.send_data(&receiver, valid_data.clone()).expect("Valid send should succeed");
    
    // Process messages
    receiver.process_messages().expect("Processing should succeed");
    assert_eq!(receiver.get_message_count(), 1, "Valid message should be received");
    
    // Reset counters
    receiver.reset_message_count();
    
    // Send a "malformed" message (in this case, empty payload)
    let empty_data = Vec::new();
    sender.send_data(&receiver, empty_data).expect("Empty send operation should complete");
    
    // Process messages
    receiver.process_messages().expect("Processing should handle malformed message");
    
    // Our simple network simulator might still deliver empty messages, but in a real protocol
    // they would likely be rejected by validation logic
    println!("Received {} messages after sending empty message", 
             receiver.get_message_count());
    
    // Reset counters
    receiver.reset_message_count();
    
    // Send message with invalid/random data
    let random_data: Vec<u8> = (0..100).map(|_| rand::random::<u8>()).collect();
    sender.send_data(&receiver, random_data).expect("Random send operation should complete");
    
    // Process messages
    receiver.process_messages().expect("Processing should handle random message");
    
    println!("Received {} messages after sending random data", 
             receiver.get_message_count());
}

#[test]
fn test_rapid_connect_disconnect() {
    // Create a simulated network
    let network = Arc::new(SimulatedNetwork::new());
    
    // Create a receiver peer that will remain connected
    let receiver = SecurityPeer::new(network.clone());
    
    // Number of connect/disconnect cycles to perform
    let cycles = 10;
    
    for i in 0..cycles {
        // Create a new sender that will connect and disconnect
        let sender = SecurityPeer::new(network.clone());
        
        // Send a message to establish connection
        let message = format!("Connection {}", i).into_bytes();
        sender.send_data(&receiver, message).expect("Send should succeed");
        
        // Process the message
        receiver.process_messages().expect("Processing should succeed");
        
        // Verify message was received
        let count_before = receiver.get_message_count();
        
        // Simulate disconnect by removing sender from network
        network.remove_endpoint(sender.addr);
        
        // Try to create another peer at the same address (should not conflict)
        let replacement = SecurityPeer::new(network.clone());
        
        // Send from the replacement
        let message = format!("Replacement {}", i).into_bytes();
        replacement.send_data(&receiver, message).expect("Replacement send should succeed");
        
        // Process the message
        receiver.process_messages().expect("Processing should succeed");
        
        // Verify we received the replacement message
        let count_after = receiver.get_message_count();
        assert!(count_after > count_before, "Should receive message from replacement");
    }
    
    // Overall we should have received at least 2*cycles messages
    assert!(receiver.get_message_count() >= 2 * cycles, 
            "Should handle rapid connect/disconnect cycles");
    
    println!("Successfully handled {} connect/disconnect cycles", cycles);
}

#[test]
fn test_network_jitter_handling() {
    // Create a simulated network
    let network = Arc::new(SimulatedNetwork::new());
    
    // Create two peers
    let sender = SecurityPeer::new(network.clone());
    let receiver = SecurityPeer::new(network.clone());
    
    // Set up high jitter between the peers
    // 50ms base latency with 80% jitter (40ms variation)
    network.set_link_conditions(sender.addr, receiver.addr, 50, 80, 0);
    network.set_link_conditions(receiver.addr, sender.addr, 50, 80, 0);
    
    // Send multiple messages
    let message_count = 50;
    
    for i in 0..message_count {
        let message = format!("Message {}", i).into_bytes();
        sender.send_data(&receiver, message).expect("Send should succeed");
    }
    
    // Wait for all messages to arrive (allowing for jitter)
    // With 50ms base latency and 80% jitter, max latency could be ~90ms
    // Wait 150ms to be safe
    thread::sleep(Duration::from_millis(150));
    
    // Process all messages
    receiver.process_messages().expect("Processing should succeed");
    
    // Verify all messages were received despite jitter
    assert_eq!(
        receiver.get_message_count(),
        message_count,
        "All messages should be received despite network jitter"
    );
    
    println!("Successfully received {} messages with high network jitter", message_count);
} 