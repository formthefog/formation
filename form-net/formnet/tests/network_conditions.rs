//! Network conditions simulator for testing relay functionality under various network conditions
//!
//! This module provides tools to simulate realistic network conditions like
//! latency, packet loss, and NAT behaviors in a controlled test environment.

use std::collections::{HashMap, VecDeque};
use std::net::SocketAddr;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};
use rand::{Rng, thread_rng};

/// A network link with configurable conditions
#[derive(Clone)]
struct NetworkLink {
    /// Average latency in milliseconds
    latency_ms: u32,
    
    /// Jitter (randomness) in latency as a percentage of latency (0-100)
    jitter_percent: u8,
    
    /// Packet loss percentage (0-100)
    packet_loss_percent: u8,
    
    /// Whether the link is currently up or down
    is_up: bool,
}

impl NetworkLink {
    /// Create a new network link with default settings
    fn new() -> Self {
        Self {
            latency_ms: 0,
            jitter_percent: 0,
            packet_loss_percent: 0,
            is_up: true,
        }
    }
    
    /// Set the latency for this link
    fn with_latency(mut self, latency_ms: u32, jitter_percent: u8) -> Self {
        self.latency_ms = latency_ms;
        self.jitter_percent = jitter_percent.min(100);
        self
    }
    
    /// Set the packet loss for this link
    fn with_packet_loss(mut self, loss_percent: u8) -> Self {
        self.packet_loss_percent = loss_percent.min(100);
        self
    }
    
    /// Set whether the link is up or down
    fn set_up(&mut self, is_up: bool) {
        self.is_up = is_up;
    }
    
    /// Should this packet be dropped due to simulated loss?
    fn should_drop_packet(&self) -> bool {
        if !self.is_up {
            return true;
        }
        
        if self.packet_loss_percent > 0 {
            let mut rng = thread_rng();
            let roll = rng.gen_range(0..100);
            if roll < self.packet_loss_percent {
                return true;
            }
        }
        
        false
    }
    
    /// Calculate the delay to apply for this packet
    fn calculate_delay(&self) -> Duration {
        if self.latency_ms == 0 {
            return Duration::from_millis(0);
        }
        
        let mut rng = thread_rng();
        let base_delay = self.latency_ms;
        
        // Apply jitter if configured
        let jitter_ms = if self.jitter_percent > 0 {
            let max_jitter = (base_delay as f32 * (self.jitter_percent as f32 / 100.0)) as u32;
            if max_jitter > 0 {
                rng.gen_range(0..max_jitter)
            } else {
                0
            }
        } else {
            0
        };
        
        // Randomly add or subtract jitter (but ensure we don't go negative)
        let final_delay = if rng.gen_bool(0.5) {
            base_delay.saturating_add(jitter_ms)
        } else {
            base_delay.saturating_sub(jitter_ms)
        };
        
        Duration::from_millis(final_delay as u64)
    }
}

/// A virtual network that simulates realistic network conditions
pub struct SimulatedNetwork {
    /// Queue of messages for each endpoint
    message_queues: Arc<RwLock<HashMap<SocketAddr, VecDeque<(Vec<u8>, SocketAddr, Instant)>>>>,
    
    /// Network link conditions between endpoints
    links: Arc<RwLock<HashMap<(SocketAddr, SocketAddr), NetworkLink>>>,
    
    /// Default link settings for newly created links
    default_link: NetworkLink,
}

impl SimulatedNetwork {
    /// Create a new simulated network with default settings
    pub fn new() -> Self {
        Self {
            message_queues: Arc::new(RwLock::new(HashMap::new())),
            links: Arc::new(RwLock::new(HashMap::new())),
            default_link: NetworkLink::new(),
        }
    }
    
    /// Register an endpoint with the network
    pub fn register_endpoint(&self, addr: SocketAddr) {
        let mut queues = self.message_queues.write().unwrap();
        queues.entry(addr).or_insert_with(VecDeque::new);
    }
    
    /// Send a message from one endpoint to another
    pub fn send_message(&self, from: SocketAddr, to: SocketAddr, data: Vec<u8>) {
        // Get link conditions
        let link = {
            let links = self.links.read().unwrap();
            links.get(&(from, to)).cloned().unwrap_or_else(|| self.default_link.clone())
        };
        
        // Check if packet should be dropped
        if link.should_drop_packet() {
            return;
        }
        
        // Calculate when the packet should arrive
        let delay = link.calculate_delay();
        let arrival_time = Instant::now() + delay;
        
        // Queue the message
        let mut queues = self.message_queues.write().unwrap();
        if let Some(queue) = queues.get_mut(&to) {
            queue.push_back((data, from, arrival_time));
        }
    }
    
    /// Receive a message for an endpoint (non-blocking)
    pub fn receive_message(&self, addr: SocketAddr) -> Option<(Vec<u8>, SocketAddr)> {
        let now = Instant::now();
        let mut queues = self.message_queues.write().unwrap();
        
        if let Some(queue) = queues.get_mut(&addr) {
            // Find the first message that has arrived (based on delay)
            let pos = queue.iter().position(|(_, _, arrival_time)| *arrival_time <= now);
            
            if let Some(idx) = pos {
                // Remove and return the message
                let (data, from, _) = queue.remove(idx).unwrap();
                return Some((data, from));
            }
        }
        
        None
    }
    
    /// Set link conditions between two endpoints
    pub fn set_link_conditions(
        &self, 
        from: SocketAddr, 
        to: SocketAddr, 
        latency_ms: u32, 
        jitter_percent: u8, 
        packet_loss_percent: u8
    ) {
        let mut links = self.links.write().unwrap();
        let link = links.entry((from, to)).or_insert_with(NetworkLink::new);
        *link = NetworkLink::new()
            .with_latency(latency_ms, jitter_percent)
            .with_packet_loss(packet_loss_percent);
    }
    
    /// Set the link state (up/down) between two endpoints
    pub fn set_link_state(&self, from: SocketAddr, to: SocketAddr, is_up: bool) {
        let mut links = self.links.write().unwrap();
        let link = links.entry((from, to)).or_insert_with(NetworkLink::new);
        link.set_up(is_up);
    }
    
    /// Set default conditions for new links
    pub fn set_default_conditions(
        &mut self, 
        latency_ms: u32, 
        jitter_percent: u8, 
        packet_loss_percent: u8
    ) {
        self.default_link = NetworkLink::new()
            .with_latency(latency_ms, jitter_percent)
            .with_packet_loss(packet_loss_percent);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    
    #[test]
    fn test_network_link_defaults() {
        let link = NetworkLink::new();
        assert_eq!(link.latency_ms, 0);
        assert_eq!(link.jitter_percent, 0);
        assert_eq!(link.packet_loss_percent, 0);
        assert!(link.is_up);
    }
    
    #[test]
    fn test_network_link_configuration() {
        let link = NetworkLink::new()
            .with_latency(100, 10)
            .with_packet_loss(5);
            
        assert_eq!(link.latency_ms, 100);
        assert_eq!(link.jitter_percent, 10);
        assert_eq!(link.packet_loss_percent, 5);
    }
    
    #[test]
    fn test_simulated_network_basic() {
        let network = SimulatedNetwork::new();
        
        let addr1 = "127.0.0.1:10001".parse().unwrap();
        let addr2 = "127.0.0.1:10002".parse().unwrap();
        
        network.register_endpoint(addr1);
        network.register_endpoint(addr2);
        
        // With default settings (no latency), the message should be immediately available
        let test_data = b"hello world".to_vec();
        network.send_message(addr1, addr2, test_data.clone());
        
        let (received, from) = network.receive_message(addr2).unwrap();
        assert_eq!(received, test_data);
        assert_eq!(from, addr1);
        
        // Queue should be empty now
        assert!(network.receive_message(addr2).is_none());
    }
    
    #[test]
    fn test_network_latency() {
        let network = SimulatedNetwork::new();
        
        let addr1 = "127.0.0.1:10001".parse().unwrap();
        let addr2 = "127.0.0.1:10002".parse().unwrap();
        
        network.register_endpoint(addr1);
        network.register_endpoint(addr2);
        
        // Set latency to 50ms
        network.set_link_conditions(addr1, addr2, 50, 0, 0);
        
        // Send a message
        let test_data = b"delayed message".to_vec();
        network.send_message(addr1, addr2, test_data.clone());
        
        // Message shouldn't be available immediately
        assert!(network.receive_message(addr2).is_none());
        
        // Wait for the message to arrive
        thread::sleep(Duration::from_millis(60));
        
        // Now we should be able to receive it
        let (received, from) = network.receive_message(addr2).unwrap();
        assert_eq!(received, test_data);
        assert_eq!(from, addr1);
    }
    
    #[test]
    fn test_packet_loss() {
        let network = SimulatedNetwork::new();
        
        let addr1 = "127.0.0.1:10001".parse().unwrap();
        let addr2 = "127.0.0.1:10002".parse().unwrap();
        
        network.register_endpoint(addr1);
        network.register_endpoint(addr2);
        
        // Set 100% packet loss (all packets will be dropped)
        network.set_link_conditions(addr1, addr2, 0, 0, 100);
        
        // Send a message that should be dropped
        let test_data = b"dropped message".to_vec();
        network.send_message(addr1, addr2, test_data.clone());
        
        // Message should never arrive
        assert!(network.receive_message(addr2).is_none());
        
        // Set link back to normal
        network.set_link_conditions(addr1, addr2, 0, 0, 0);
        
        // Send another message that should go through
        let test_data2 = b"delivered message".to_vec();
        network.send_message(addr1, addr2, test_data2.clone());
        
        // This one should be received
        let (received, from) = network.receive_message(addr2).unwrap();
        assert_eq!(received, test_data2);
        assert_eq!(from, addr1);
    }
    
    #[test]
    fn test_link_up_down() {
        let network = SimulatedNetwork::new();
        
        let addr1 = "127.0.0.1:10001".parse().unwrap();
        let addr2 = "127.0.0.1:10002".parse().unwrap();
        
        network.register_endpoint(addr1);
        network.register_endpoint(addr2);
        
        // Link starts up by default, message should go through
        let test_data = b"message 1".to_vec();
        network.send_message(addr1, addr2, test_data.clone());
        
        let (received, _) = network.receive_message(addr2).unwrap();
        assert_eq!(received, test_data);
        
        // Set link down
        network.set_link_state(addr1, addr2, false);
        
        // Message should not go through when link is down
        let test_data2 = b"message 2".to_vec();
        network.send_message(addr1, addr2, test_data2.clone());
        
        assert!(network.receive_message(addr2).is_none());
        
        // Set link back up
        network.set_link_state(addr1, addr2, true);
        
        // Message should go through again
        let test_data3 = b"message 3".to_vec();
        network.send_message(addr1, addr2, test_data3.clone());
        
        let (received, _) = network.receive_message(addr2).unwrap();
        assert_eq!(received, test_data3);
    }
} 