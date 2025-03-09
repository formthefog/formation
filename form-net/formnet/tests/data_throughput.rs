//! Data throughput tests
//!
//! This module tests the data throughput capabilities under various network conditions
//! to ensure efficient data transfer.

use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Arc;
use std::time::{Duration, Instant};
use std::thread;

use rand::{Rng, thread_rng};

mod network_conditions;
use network_conditions::SimulatedNetwork;

const SMALL_PACKET_SIZE: usize = 512;       // 512 bytes
const MEDIUM_PACKET_SIZE: usize = 8 * 1024; // 8 KB
const LARGE_PACKET_SIZE: usize = 64 * 1024; // 64 KB

/// A simulated peer for data throughput testing
struct ThroughputPeer {
    /// The peer's socket address
    addr: SocketAddr,
    
    /// The simulated network
    network: Arc<SimulatedNetwork>,
    
    /// Buffer for received data
    received_data: Arc<std::sync::Mutex<Vec<Vec<u8>>>>,
    
    /// Total bytes sent
    bytes_sent: Arc<std::sync::Mutex<usize>>,
    
    /// Total bytes received
    bytes_received: Arc<std::sync::Mutex<usize>>,
}

impl ThroughputPeer {
    /// Create a new peer with the given network
    fn new(network: Arc<SimulatedNetwork>) -> Self {
        let addr = create_virtual_addr();
        network.register_endpoint(addr);
        
        ThroughputPeer {
            addr,
            network,
            received_data: Arc::new(std::sync::Mutex::new(Vec::new())),
            bytes_sent: Arc::new(std::sync::Mutex::new(0)),
            bytes_received: Arc::new(std::sync::Mutex::new(0)),
        }
    }
    
    /// Send data to the target peer
    fn send_data(&self, target: &ThroughputPeer, data: Vec<u8>) -> Result<(), String> {
        // Record bytes sent
        {
            let mut bytes = self.bytes_sent.lock().unwrap();
            *bytes += data.len();
        }
        
        self.network.send_message(self.addr, target.addr, data);
        Ok(())
    }
    
    /// Process any received messages
    fn process_messages(&self) -> Result<(), String> {
        while let Some((data, _src_addr)) = self.network.receive_message(self.addr) {
            // Record bytes received
            {
                let mut bytes = self.bytes_received.lock().unwrap();
                *bytes += data.len();
            }
            
            // Store the received data
            let mut received = self.received_data.lock().unwrap();
            received.push(data);
        }
        
        Ok(())
    }
    
    /// Get the number of bytes sent
    fn get_bytes_sent(&self) -> usize {
        let bytes = self.bytes_sent.lock().unwrap();
        *bytes
    }
    
    /// Get the number of bytes received
    fn get_bytes_received(&self) -> usize {
        let bytes = self.bytes_received.lock().unwrap();
        *bytes
    }
    
    /// Clear bytes sent/received counters
    fn reset_counters(&self) {
        {
            let mut bytes_sent = self.bytes_sent.lock().unwrap();
            *bytes_sent = 0;
        }
        {
            let mut bytes_received = self.bytes_received.lock().unwrap();
            *bytes_received = 0;
        }
        {
            let mut received = self.received_data.lock().unwrap();
            received.clear();
        }
    }
    
    /// Get the number of messages received
    fn get_message_count(&self) -> usize {
        let received = self.received_data.lock().unwrap();
        received.len()
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

/// Calculate throughput in MB/s
fn calculate_throughput(bytes: usize, duration: Duration) -> f64 {
    let mb = bytes as f64 / (1024.0 * 1024.0);
    let seconds = duration.as_secs_f64();
    mb / seconds
}

#[test]
fn test_data_throughput_optimal_conditions() {
    // Create a simulated network with default settings (no latency, no packet loss)
    let network = Arc::new(SimulatedNetwork::new());
    
    // Create two peers
    let sender = ThroughputPeer::new(network.clone());
    let receiver = ThroughputPeer::new(network.clone());
    
    // Test small packets (512 bytes)
    let small_packets_count = 1000;
    let test_data = generate_test_data(SMALL_PACKET_SIZE);
    
    let start_time = Instant::now();
    
    for _ in 0..small_packets_count {
        sender.send_data(&receiver, test_data.clone()).expect("Send should succeed");
        
        // Process the message at the receiver immediately 
        // (in optimal conditions there's no delay)
        receiver.process_messages().expect("Processing should succeed");
    }
    
    let elapsed = start_time.elapsed();
    
    // Verify all packets were received
    assert_eq!(
        receiver.get_message_count(), 
        small_packets_count,
        "Receiver should have received all small packets"
    );
    
    let total_bytes = SMALL_PACKET_SIZE * small_packets_count;
    let throughput = calculate_throughput(total_bytes, elapsed);
    
    println!(
        "Small packet throughput (optimal): {:.2} MB/s ({} bytes in {:.2} seconds)", 
        throughput, 
        total_bytes, 
        elapsed.as_secs_f64()
    );
    
    // Test large packets (64 KB)
    sender.reset_counters();
    receiver.reset_counters();
    
    let large_packets_count = 100;
    let test_data = generate_test_data(LARGE_PACKET_SIZE);
    
    let start_time = Instant::now();
    
    for _ in 0..large_packets_count {
        sender.send_data(&receiver, test_data.clone()).expect("Send should succeed");
        receiver.process_messages().expect("Processing should succeed");
    }
    
    let elapsed = start_time.elapsed();
    
    // Verify all packets were received
    assert_eq!(
        receiver.get_message_count(), 
        large_packets_count,
        "Receiver should have received all large packets"
    );
    
    let total_bytes = LARGE_PACKET_SIZE * large_packets_count;
    let throughput = calculate_throughput(total_bytes, elapsed);
    
    println!(
        "Large packet throughput (optimal): {:.2} MB/s ({} bytes in {:.2} seconds)", 
        throughput, 
        total_bytes, 
        elapsed.as_secs_f64()
    );
}

#[test]
fn test_data_throughput_with_latency() {
    // Create a simulated network
    let network = Arc::new(SimulatedNetwork::new());
    
    // Create two peers
    let sender = ThroughputPeer::new(network.clone());
    let receiver = ThroughputPeer::new(network.clone());
    
    // Add 50ms latency between the peers
    network.set_link_conditions(sender.addr, receiver.addr, 50, 0, 0);
    network.set_link_conditions(receiver.addr, sender.addr, 50, 0, 0);
    
    // Test medium packets (8 KB) with latency
    let medium_packets_count = 100;
    let test_data = generate_test_data(MEDIUM_PACKET_SIZE);
    
    let start_time = Instant::now();
    
    // Send packets in batches to minimize latency impact
    for _ in 0..medium_packets_count {
        sender.send_data(&receiver, test_data.clone()).expect("Send should succeed");
    }
    
    // Wait for all packets to arrive with latency
    let wait_time = Duration::from_millis(100); // Allow extra time for latency
    thread::sleep(wait_time);
    
    // Process all messages
    receiver.process_messages().expect("Processing should succeed");
    
    let elapsed = start_time.elapsed() - wait_time; // Adjust for the extra wait time
    
    // Verify all packets were received
    assert_eq!(
        receiver.get_message_count(), 
        medium_packets_count,
        "Receiver should have received all medium packets"
    );
    
    let total_bytes = MEDIUM_PACKET_SIZE * medium_packets_count;
    let throughput = calculate_throughput(total_bytes, elapsed);
    
    println!(
        "Medium packet throughput (with 50ms latency): {:.2} MB/s ({} bytes in {:.2} seconds)", 
        throughput, 
        total_bytes, 
        elapsed.as_secs_f64()
    );
    
    // The latency should primarily affect small packets, not throughput of large transfers
    // For comparison, test large packets with latency
    sender.reset_counters();
    receiver.reset_counters();
    
    let large_packets_count = 20;
    let test_data = generate_test_data(LARGE_PACKET_SIZE);
    
    let start_time = Instant::now();
    
    for _ in 0..large_packets_count {
        sender.send_data(&receiver, test_data.clone()).expect("Send should succeed");
    }
    
    // Wait for all packets to arrive with latency
    thread::sleep(wait_time);
    
    // Process all messages
    receiver.process_messages().expect("Processing should succeed");
    
    let elapsed = start_time.elapsed() - wait_time;
    
    // Verify all packets were received
    assert_eq!(
        receiver.get_message_count(), 
        large_packets_count,
        "Receiver should have received all large packets with latency"
    );
    
    let total_bytes = LARGE_PACKET_SIZE * large_packets_count;
    let throughput = calculate_throughput(total_bytes, elapsed);
    
    println!(
        "Large packet throughput (with 50ms latency): {:.2} MB/s ({} bytes in {:.2} seconds)", 
        throughput, 
        total_bytes, 
        elapsed.as_secs_f64()
    );
}

#[test]
fn test_data_throughput_with_packet_loss() {
    // Create a simulated network
    let network = Arc::new(SimulatedNetwork::new());
    
    // Create two peers
    let sender = ThroughputPeer::new(network.clone());
    let receiver = ThroughputPeer::new(network.clone());
    
    // Add 10% packet loss between the peers
    network.set_link_conditions(sender.addr, receiver.addr, 0, 0, 10);
    network.set_link_conditions(receiver.addr, sender.addr, 0, 0, 10);
    
    // Test medium packets with packet loss
    let medium_packets_count = 100;
    let test_data = generate_test_data(MEDIUM_PACKET_SIZE);
    
    let start_time = Instant::now();
    
    // Send all packets
    for _ in 0..medium_packets_count {
        sender.send_data(&receiver, test_data.clone()).expect("Send should succeed");
    }
    
    // Give some time for packets to arrive
    thread::sleep(Duration::from_millis(50));
    
    // Process received messages
    receiver.process_messages().expect("Processing should succeed");
    
    let elapsed = start_time.elapsed();
    
    // With 10% packet loss, we expect approximately 90% of packets to be received
    let received_count = receiver.get_message_count();
    let expected_min = (medium_packets_count as f64 * 0.8) as usize; // Allow for some statistical variation
    
    println!(
        "Received {} out of {} packets with 10% packet loss", 
        received_count, 
        medium_packets_count
    );
    
    assert!(
        received_count >= expected_min,
        "Should receive at least 80% of packets with 10% packet loss"
    );
    
    let total_bytes_sent = MEDIUM_PACKET_SIZE * medium_packets_count;
    let total_bytes_received = receiver.get_bytes_received();
    let throughput = calculate_throughput(total_bytes_received, elapsed);
    
    println!(
        "Effective throughput with packet loss: {:.2} MB/s ({}/{} bytes in {:.2} seconds)", 
        throughput, 
        total_bytes_received,
        total_bytes_sent,
        elapsed.as_secs_f64()
    );
    
    // Calculate packet loss percentage
    let packet_loss_pct = 100.0 * (1.0 - (received_count as f64 / medium_packets_count as f64));
    println!("Measured packet loss: {:.1}%", packet_loss_pct);
} 