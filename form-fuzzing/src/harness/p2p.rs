// form-fuzzing/src/harness/p2p.rs
//! Harness for P2P message queue fuzzing

// Remove the imports from generators p2p since these types are defined in this file
// use crate::generators::p2p::{
//     Message, MessageHeader, NodeIdentity, Subscription, P2PConfig,
//     Priority, DeliveryGuarantee, QoS
// };

use std::collections::{HashMap, HashSet, VecDeque};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use std::net::{IpAddr, SocketAddr};
use rand::{Rng, thread_rng};
use uuid::Uuid;
// Fix the log import to use tracing instead
use tracing::{debug, error, info, warn};
use std::fmt;
use serde::{Serialize, Deserialize};

/// Result of a P2P operation
#[derive(Debug, Clone, PartialEq)]
pub enum P2POperationResult {
    /// Operation succeeded
    Success,
    /// Operation failed with error
    Error(P2PError),
    /// Operation timed out
    Timeout,
    /// Operation rejected due to rate limiting
    RateLimited,
    /// Operation succeeded but with warnings
    SuccessWithWarnings(Vec<String>),
}

/// Error types that can occur in P2P operations
#[derive(Debug, Clone, PartialEq)]
pub enum P2PError {
    /// Node not found
    NodeNotFound(String),
    /// Authentication failed
    AuthenticationFailed,
    /// Authorization failed
    AuthorizationFailed,
    /// Invalid message format
    InvalidMessageFormat(String),
    /// Validation failed
    ValidationFailed(String),
    /// Network error
    NetworkError(String),
    /// Invalid configuration
    InvalidConfiguration(String),
    /// Topic not found
    TopicNotFound(String),
    /// Subscription not found
    SubscriptionNotFound(String),
    /// System overloaded
    SystemOverloaded,
    /// Duplicate message ID
    DuplicateMessageId(String),
    /// Message too large
    MessageTooLarge(usize, usize),
    /// Queue full
    QueueFull,
    /// TTL expired
    TtlExpired,
    /// Unknown error
    Unknown(String),
}

/// Mock P2P node for testing
pub struct MockP2PNode {
    /// Node identity
    pub identity: NodeIdentity,
    /// Node configuration
    pub config: P2PConfig,
    /// Active subscriptions
    subscriptions: HashMap<String, Subscription>,
    /// Connected nodes
    connected_nodes: HashMap<String, NodeIdentity>,
    /// Message deduplication set
    message_ids: HashSet<String>,
    /// Last timestamp for rate limiting
    last_operation_time: HashMap<String, u64>,
    /// Operation count for rate limiting
    operation_count: HashMap<String, usize>,
    /// Random failure rate (0.0 - 1.0)
    failure_rate: f64,
    /// Random timeout rate (0.0 - 1.0)
    timeout_rate: f64,
    /// Network latency simulation (ms)
    network_latency: u64,
}

impl MockP2PNode {
    /// Create a new mock P2P node
    pub fn new(identity: NodeIdentity, config: P2PConfig) -> Self {
        Self {
            identity,
            config,
            subscriptions: HashMap::new(),
            connected_nodes: HashMap::new(),
            message_ids: HashSet::new(),
            last_operation_time: HashMap::new(),
            operation_count: HashMap::new(),
            failure_rate: 0.05, // 5% chance of random failure
            timeout_rate: 0.03, // 3% chance of timeout
            network_latency: 50,  // 50ms network latency
        }
    }
    
    /// Set the random failure rate
    pub fn set_failure_rate(&mut self, rate: f64) {
        self.failure_rate = rate.clamp(0.0, 1.0);
    }
    
    /// Set the random timeout rate
    pub fn set_timeout_rate(&mut self, rate: f64) {
        self.timeout_rate = rate.clamp(0.0, 1.0);
    }
    
    /// Set the network latency
    pub fn set_network_latency(&mut self, latency_ms: u64) {
        self.network_latency = latency_ms;
    }
    
    /// Connect to another node
    pub fn connect(&mut self, node: &NodeIdentity) -> P2POperationResult {
        let mut rng = thread_rng();
        
        // Simulate random failures
        if rng.gen_bool(self.failure_rate) {
            return P2POperationResult::Error(P2PError::NetworkError(
                "Random connection failure".to_string()
            ));
        }
        
        // Simulate timeouts
        if rng.gen_bool(self.timeout_rate) {
            return P2POperationResult::Timeout;
        }
        
        // Validate node identity
        if node.id.is_empty() {
            return P2POperationResult::Error(P2PError::ValidationFailed(
                "Node ID cannot be empty".to_string()
            ));
        }
        
        if node.addresses.is_empty() {
            return P2POperationResult::Error(P2PError::ValidationFailed(
                "Node address cannot be empty".to_string()
            ));
        }
        
        // Check if connection limit reached
        if self.connected_nodes.len() >= self.config.max_connections {
            return P2POperationResult::Error(P2PError::SystemOverloaded);
        }
        
        // Add to connected nodes
        self.connected_nodes.insert(node.id.clone(), node.clone());
        
        // Check authentication
        if node.public_key.is_empty() {
            return P2POperationResult::Error(P2PError::AuthenticationFailed);
        }
        
        P2POperationResult::Success
    }
    
    /// Disconnect from a node
    pub fn disconnect(&mut self, node_id: &str) -> P2POperationResult {
        let mut rng = thread_rng();
        
        // Simulate random failures
        if rng.gen::<f64>() < self.failure_rate {
            return P2POperationResult::Error(P2PError::NetworkError(
                "Failed to disconnect".to_string()
            ));
        }
        
        // Check if node exists
        if !self.connected_nodes.contains_key(node_id) {
            return P2POperationResult::Error(P2PError::NodeNotFound(
                node_id.to_string()
            ));
        }
        
        // Remove from connected nodes
        self.connected_nodes.remove(node_id);
        
        P2POperationResult::Success
    }
    
    /// Subscribe to a topic
    pub fn subscribe(&mut self, subscription: Subscription) -> P2POperationResult {
        let mut rng = thread_rng();
        
        // Simulate random failures
        if rng.gen::<f64>() < self.failure_rate {
            return P2POperationResult::Error(P2PError::NetworkError(
                "Failed to subscribe".to_string()
            ));
        }
        
        // Simulate timeouts
        if rng.gen::<f64>() < self.timeout_rate {
            return P2POperationResult::Timeout;
        }
        
        // Validate subscription
        if subscription.id.is_empty() {
            return P2POperationResult::Error(P2PError::ValidationFailed(
                "Subscriber ID cannot be empty".to_string()
            ));
        }
        
        if subscription.topic.is_empty() {
            return P2POperationResult::Error(P2PError::ValidationFailed(
                "Topic pattern cannot be empty".to_string()
            ));
        }
        
        // Validate topic pattern (simple validation)
        if subscription.topic.contains('#') {
            let parts: Vec<&str> = subscription.topic.split('/').collect();
            // Multi-level wildcard must be at the end and alone
            if parts.contains(&"#") && parts.last() != Some(&"#") {
                return P2POperationResult::Error(P2PError::ValidationFailed(
                    "Multi-level wildcard (#) must be at the end of the topic pattern".to_string()
                ));
            }
            
            // If it contains "##" which is invalid
            if subscription.topic.contains("##") {
                return P2POperationResult::Error(P2PError::ValidationFailed(
                    "Invalid multi-level wildcard format (##)".to_string()
                ));
            }
        }
        
        // Perform rate limiting check
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
            
        let op_key = format!("subscribe_{}", subscription.id);
        let last_time = self.last_operation_time.get(&op_key).cloned().unwrap_or(0);
        let count = self.operation_count.get(&op_key).cloned().unwrap_or(0);
        
        // Reset counter if more than 60 seconds have passed
        if now > last_time + 60 {
            self.operation_count.insert(op_key.clone(), 1);
        } else if count > 10 {
            // Rate limit to 10 subscribe operations per minute
            return P2POperationResult::RateLimited;
        } else {
            self.operation_count.insert(op_key.clone(), count + 1);
        }
        
        self.last_operation_time.insert(op_key, now);
        
        // Store subscription
        let subscription_id = if subscription.id.is_empty() {
            Uuid::new_v4().to_string()
        } else {
            subscription.id.clone()
        };
        
        let mut sub = subscription.clone();
        sub.id = subscription_id.clone();
        
        self.subscriptions.insert(subscription_id, sub);
        
        P2POperationResult::Success
    }
    
    /// Unsubscribe from a topic
    pub fn unsubscribe(&mut self, subscription_id: &str) -> P2POperationResult {
        let mut rng = thread_rng();
        
        // Simulate random failures
        if rng.gen::<f64>() < self.failure_rate {
            return P2POperationResult::Error(P2PError::NetworkError(
                "Failed to unsubscribe".to_string()
            ));
        }
        
        // Check if subscription exists
        if !self.subscriptions.contains_key(subscription_id) {
            return P2POperationResult::Error(P2PError::SubscriptionNotFound(
                subscription_id.to_string()
            ));
        }
        
        // Remove subscription
        self.subscriptions.remove(subscription_id);
        
        P2POperationResult::Success
    }
    
    /// Publish a message
    pub fn publish<T: std::cmp::PartialEq>(&mut self, message: Message<T>) -> P2POperationResult {
        let mut rng = thread_rng();
        
        // Simulate random failures
        if rng.gen_bool(self.failure_rate) {
            return P2POperationResult::Error(P2PError::NetworkError(
                "Random network failure".to_string()
            ));
        }
        
        // Simulate timeouts
        if rng.gen_bool(self.timeout_rate) {
            return P2POperationResult::Timeout;
        }
        
        // In a real implementation, would validate message content
        // For this mock harness, we'll just route the message
        self.route_message(&message);

        // Return success
        P2POperationResult::Success
    }
    
    /// Route a message to matching subscriptions
    fn route_message<T>(&mut self, message: &Message<T>) {
        // In a real implementation, this would deliver messages to subscribers
        // For this mock harness, we'll just log the action
        let node_id = &self.identity.id;
        println!("Node {} routing message with content type {}", 
                 node_id, std::any::type_name::<T>());
    }
    
    /// Receive messages for a subscription
    pub fn receive<T: Clone>(&mut self, subscription_id: &str, max_messages: usize) -> Result<Vec<Message<T>>, P2PError> {
        let mut rng = thread_rng();
        
        // Simulate random failures
        if rng.gen_bool(self.failure_rate) {
            return Err(P2PError::NetworkError(
                "Random network failure while receiving messages".to_string()
            ));
        }
        
        // Simulate timeouts
        if rng.gen_bool(self.timeout_rate) {
            return Err(P2PError::NetworkError(
                "Timeout while receiving messages".to_string()
            ));
        }
        
        // Validate subscription_id
        if !self.subscriptions.contains_key(subscription_id) {
            return Err(P2PError::SubscriptionNotFound(subscription_id.to_string()));
        }
        
        // In a real implementation, would return actual messages
        // Here we return an empty vec to indicate success but no messages
        let messages: Vec<Message<T>> = Vec::new();
        Ok(messages)
    }
    
    /// Get status information about the node
    pub fn get_status(&self) -> HashMap<String, String> {
        let mut status = HashMap::new();
        
        status.insert("node_id".to_string(), self.identity.id.clone());
        status.insert("subscriptions".to_string(), self.subscriptions.len().to_string());
        status.insert("connected_nodes".to_string(), self.connected_nodes.len().to_string());
        
        // Since we don't have actual message tracking in this simplified version,
        // we'll just report a placeholder value
        let total_messages: usize = 0;
        status.insert("total_messages".to_string(), total_messages.to_string());
        
        status
    }
    
    /// Clear all data (for testing)
    pub fn clear(&mut self) {
        self.subscriptions.clear();
        self.connected_nodes.clear();
        self.message_ids.clear();
        self.last_operation_time.clear();
        self.operation_count.clear();
    }
}

/// P2P message queue harness
pub struct P2PHarness {
    /// Nodes in the network
    nodes: HashMap<String, MockP2PNode>,
    /// Default node for operations
    default_node_id: Option<String>,
}

impl P2PHarness {
    /// Create a new P2P harness
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            default_node_id: None,
        }
    }
    
    /// Add a node to the network
    pub fn add_node(&mut self, identity: NodeIdentity, config: P2PConfig) -> P2POperationResult {
        if identity.id.is_empty() {
            return P2POperationResult::Error(P2PError::ValidationFailed(
                "Node ID cannot be empty".to_string()
            ));
        }
        
        if self.nodes.contains_key(&identity.id) {
            return P2POperationResult::Error(P2PError::ValidationFailed(
                format!("Node with ID {} already exists", identity.id)
            ));
        }
        
        let node = MockP2PNode::new(identity.clone(), config);
        self.nodes.insert(identity.id.clone(), node);
        
        // Set as default if first node
        if self.default_node_id.is_none() {
            self.default_node_id = Some(identity.id.clone());
        }
        
        P2POperationResult::Success
    }
    
    /// Remove a node from the network
    pub fn remove_node(&mut self, node_id: &str) -> P2POperationResult {
        if !self.nodes.contains_key(node_id) {
            return P2POperationResult::Error(P2PError::NodeNotFound(
                node_id.to_string()
            ));
        }
        
        self.nodes.remove(node_id);
        
        // Update default node if needed
        if self.default_node_id == Some(node_id.to_string()) {
            self.default_node_id = self.nodes.keys().next().cloned();
        }
        
        P2POperationResult::Success
    }
    
    /// Get a mutable reference to a node
    pub fn get_node_mut(&mut self, node_id: &str) -> Option<&mut MockP2PNode> {
        self.nodes.get_mut(node_id)
    }
    
    /// Get a reference to a node
    pub fn get_node(&self, node_id: &str) -> Option<&MockP2PNode> {
        self.nodes.get(node_id)
    }
    
    /// Set the default node for operations
    pub fn set_default_node(&mut self, node_id: &str) -> P2POperationResult {
        if !self.nodes.contains_key(node_id) {
            return P2POperationResult::Error(P2PError::NodeNotFound(
                node_id.to_string()
            ));
        }
        
        self.default_node_id = Some(node_id.to_string());
        P2POperationResult::Success
    }
    
    /// Connect nodes in the network
    pub fn connect_nodes(&mut self, from_node_id: &str, to_node_id: &str) -> P2POperationResult {
        if !self.nodes.contains_key(from_node_id) {
            return P2POperationResult::Error(P2PError::NodeNotFound(
                from_node_id.to_string()
            ));
        }
        
        if !self.nodes.contains_key(to_node_id) {
            return P2POperationResult::Error(P2PError::NodeNotFound(
                to_node_id.to_string()
            ));
        }
        
        // Get the target node identity
        let to_node_identity = self.nodes.get(to_node_id).unwrap().identity.clone();
        
        // Connect from source to target
        if let Some(from_node) = self.nodes.get_mut(from_node_id) {
            return from_node.connect(&to_node_identity);
        }
        
        P2POperationResult::Error(P2PError::Unknown(
            "Failed to get from_node".to_string()
        ))
    }
    
    /// Subscribe to a topic
    pub fn subscribe(&mut self, node_id: Option<&str>, subscription: Subscription) -> P2POperationResult {
        let node_id = node_id.map(|s| s.to_string())
            .or_else(|| self.default_node_id.clone())
            .ok_or_else(|| P2PError::ValidationFailed("No node specified and no default node set".to_string()));
            
        let node_id = match node_id {
            Ok(id) => id,
            Err(e) => return P2POperationResult::Error(e),
        };
        
        if let Some(node) = self.nodes.get_mut(&node_id) {
            return node.subscribe(subscription);
        }
        
        P2POperationResult::Error(P2PError::NodeNotFound(node_id))
    }
    
    /// Unsubscribe from a topic
    pub fn unsubscribe(&mut self, node_id: Option<&str>, subscription_id: &str) -> P2POperationResult {
        let node_id = node_id.map(|s| s.to_string())
            .or_else(|| self.default_node_id.clone())
            .ok_or_else(|| P2PError::ValidationFailed("No node specified and no default node set".to_string()));
            
        let node_id = match node_id {
            Ok(id) => id,
            Err(e) => return P2POperationResult::Error(e),
        };
        
        if let Some(node) = self.nodes.get_mut(&node_id) {
            return node.unsubscribe(subscription_id);
        }
        
        P2POperationResult::Error(P2PError::NodeNotFound(node_id))
    }
    
    /// Publish a message
    pub fn publish<T: std::cmp::PartialEq>(&mut self, node_id: Option<&str>, message: Message<T>) -> P2POperationResult {
        let node_id = node_id.map(|s| s.to_string())
            .or_else(|| self.default_node_id.clone())
            .ok_or_else(|| P2PError::ValidationFailed("No node specified and no default node set".to_string()));
            
        let node_id = match node_id {
            Ok(id) => id,
            Err(e) => return P2POperationResult::Error(e),
        };
        
        if let Some(node) = self.nodes.get_mut(&node_id) {
            return node.publish(message);
        }
        
        P2POperationResult::Error(P2PError::NodeNotFound(node_id))
    }
    
    /// Receive messages for a subscription
    pub fn receive<T: Clone + std::cmp::PartialEq>(&mut self, node_id: Option<&str>, subscription_id: &str, max_messages: usize) 
        -> Result<Vec<Message<T>>, P2PError> {
        
        let node_id = node_id.map(|s| s.to_string())
            .or_else(|| self.default_node_id.clone())
            .ok_or_else(|| P2PError::ValidationFailed("No node specified and no default node set".to_string()))?;
        
        if let Some(node) = self.nodes.get_mut(&node_id) {
            return node.receive(subscription_id, max_messages);
        }
        
        Err(P2PError::NodeNotFound(node_id))
    }
    
    /// Get network status
    pub fn get_network_status(&self) -> HashMap<String, HashMap<String, String>> {
        let mut status = HashMap::new();
        
        for (node_id, node) in &self.nodes {
            status.insert(node_id.clone(), node.get_status());
        }
        
        status
    }
    
    /// Set failure rate for all nodes
    pub fn set_failure_rate(&mut self, rate: f64) {
        for node in self.nodes.values_mut() {
            node.set_failure_rate(rate);
        }
    }
    
    /// Set timeout rate for all nodes
    pub fn set_timeout_rate(&mut self, rate: f64) {
        for node in self.nodes.values_mut() {
            node.set_timeout_rate(rate);
        }
    }
    
    /// Set network latency for all nodes
    pub fn set_network_latency(&mut self, latency_ms: u64) {
        for node in self.nodes.values_mut() {
            node.set_network_latency(latency_ms);
        }
    }
    
    /// Disconnect all nodes (clear connections)
    pub fn disconnect_all(&mut self) {
        for node in self.nodes.values_mut() {
            node.clear();
        }
    }
    
    /// Build a network topology
    /// 
    /// Topology types:
    /// - star: One central node connected to all others
    /// - mesh: Every node connected to every other node
    /// - ring: Each node connected to two neighbors in a ring
    /// - line: Nodes connected in a line
    pub fn build_topology(&mut self, topology: &str) -> P2POperationResult {
        let node_ids: Vec<String> = self.nodes.keys().cloned().collect();
        
        if node_ids.len() < 2 {
            return P2POperationResult::Error(P2PError::ValidationFailed(
                "Need at least 2 nodes to build a topology".to_string()
            ));
        }
        
        // First, clear all connections
        self.disconnect_all();
        
        match topology {
            "star" => {
                // Star topology - first node is central
                let central = &node_ids[0];
                
                for i in 1..node_ids.len() {
                    let result = self.connect_nodes(central, &node_ids[i]);
                    if let P2POperationResult::Error(_) = result {
                        return result;
                    }
                    
                    let result = self.connect_nodes(&node_ids[i], central);
                    if let P2POperationResult::Error(_) = result {
                        return result;
                    }
                }
            },
            "mesh" => {
                // Mesh topology - every node connects to every other node
                for i in 0..node_ids.len() {
                    for j in 0..node_ids.len() {
                        if i != j {
                            let result = self.connect_nodes(&node_ids[i], &node_ids[j]);
                            if let P2POperationResult::Error(_) = result {
                                return result;
                            }
                        }
                    }
                }
            },
            "ring" => {
                // Ring topology - each node connects to next and previous
                for i in 0..node_ids.len() {
                    let next = (i + 1) % node_ids.len();
                    let result = self.connect_nodes(&node_ids[i], &node_ids[next]);
                    if let P2POperationResult::Error(_) = result {
                        return result;
                    }
                    
                    let result = self.connect_nodes(&node_ids[next], &node_ids[i]);
                    if let P2POperationResult::Error(_) = result {
                        return result;
                    }
                }
            },
            "line" => {
                // Line topology - nodes in a line
                for i in 0..node_ids.len() - 1 {
                    let result = self.connect_nodes(&node_ids[i], &node_ids[i + 1]);
                    if let P2POperationResult::Error(_) = result {
                        return result;
                    }
                    
                    let result = self.connect_nodes(&node_ids[i + 1], &node_ids[i]);
                    if let P2POperationResult::Error(_) = result {
                        return result;
                    }
                }
            },
            _ => {
                return P2POperationResult::Error(P2PError::ValidationFailed(
                    format!("Unknown topology type: {}", topology)
                ));
            }
        }
        
        P2POperationResult::Success
    }
}

/// Topic identifier
#[derive(Clone, Debug)]
pub struct Topic(pub String);

impl Topic {
    pub fn new(name: &str) -> Self {
        Topic(name.to_string())
    }
}

impl From<String> for Topic {
    fn from(s: String) -> Self {
        Topic(s)
    }
}

/// Node identifier
#[derive(Clone, Debug)]
pub struct NodeId(pub String);

impl NodeId {
    pub fn new(id: &str) -> Self {
        NodeId(id.to_string())
    }
}

impl From<String> for NodeId {
    fn from(s: String) -> Self {
        NodeId(s)
    }
}

#[derive(Debug, Clone)]
pub struct Message<T> {
    pub content: T,
}

impl<T> Message<T> {
    pub fn new(content: T) -> Self {
        Self { content }
    }
}

pub struct BFTQueue<T> {
    _marker: std::marker::PhantomData<T>,
    messages: HashMap<String, Vec<(Message<T>, String)>>,
}

impl<T: Clone> BFTQueue<T> {
    pub fn new() -> Self {
        Self {
            _marker: std::marker::PhantomData,
            messages: HashMap::new(),
        }
    }
    
    pub fn add(&mut self, message: Message<T>, node_id: NodeId) -> String {
        let id = Uuid::new_v4().to_string();
        let node_id_str = node_id.0;
        
        self.messages.entry(node_id_str.clone())
            .or_insert_with(Vec::new)
            .push((message, id.clone()));
            
        id
    }
    
    pub fn get_messages(&self, node_id: &NodeId) -> Vec<(Message<T>, String)> {
        self.messages.get(&node_id.0)
            .map(|msgs| msgs.clone())
            .unwrap_or_default()
    }
    
    pub fn clear(&mut self) {
        self.messages.clear();
    }
}

#[derive(Clone, Debug)]
pub struct TopicQueue<T> {
    _marker: std::marker::PhantomData<T>,
    topics: HashMap<String, Vec<T>>,
}

impl<T: Clone> TopicQueue<T> {
    pub fn new() -> Self {
        Self {
            _marker: std::marker::PhantomData,
            topics: HashMap::new(),
        }
    }
    
    pub fn apply_op(&mut self, topic: Topic, op: String, content: Option<T>) -> Result<(), String> {
        let topic_str = topic.0;
        
        match op.as_str() {
            "ADD" => {
                if let Some(item) = content {
                    self.topics.entry(topic_str)
                        .or_insert_with(Vec::new)
                        .push(item);
                    Ok(())
                } else {
                    Err("Content required for ADD operation".to_string())
                }
            },
            "CLEAR" => {
                self.topics.remove(&topic_str);
                Ok(())
            },
            "COUNT" => {
                // Just return success - in a real implementation this would return the count
                Ok(())
            },
            _ => Err(format!("Unknown operation: {}", op)),
        }
    }
    
    pub fn get_messages(&self, topic: &Topic) -> Vec<T> {
        self.topics.get(&topic.0)
            .map(|messages| messages.clone())
            .unwrap_or_default()
    }
}

#[derive(Clone, Debug)]
pub enum QueueRequest {
    Operation {
        topic: Topic,
        content: Vec<u8>,
        node_id: NodeId,
    },
    Write {
        topic: Topic,
        content: Vec<u8>,
    }
}

#[derive(Clone, Debug)]
pub enum QueueResponse {
    Ok,
    Content(Vec<u8>),
    Message(String),
    Failure(String)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_p2p_harness() {
        let mut harness = P2PHarness::new();
        
        // Add nodes
        let node1 = NodeIdentity {
            id: "node1".to_string(),
            public_key: "pk1".to_string(),
            addresses: vec!["127.0.0.1:8000".to_string()],
        };
        
        let node2 = NodeIdentity {
            id: "node2".to_string(),
            public_key: "pk2".to_string(),
            addresses: vec!["127.0.0.1:8001".to_string()],
        };
        
        let config = P2PConfig {
            max_connections: 100,
            timeout_ms: 5000,
            keep_alive_interval_ms: 30000,
        };
        
        harness.add_node(node1.clone(), config.clone());
        harness.add_node(node2.clone(), config.clone());
        
        // Connect nodes
        harness.connect_nodes("node1", "node2");
        
        // Create subscription
        let sub = Subscription {
            id: "sub1".to_string(),
            topic: "test/topic".to_string(),
            qos: QoS::AtLeastOnce,
        };
        
        harness.subscribe(Some("node1"), sub);
        
        // Publish message
        let message = Message::new(vec![1, 2, 3, 4]);
        
        let result = harness.publish(Some("node2"), message);
        
        // Assert success
        assert!(matches!(result, P2POperationResult::Success));
    }
}

// Add required struct definitions for tests to compile
#[derive(Clone, Debug)]
pub struct NodeIdentity {
    pub id: String,
    pub public_key: String,
    pub addresses: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct P2PConfig {
    pub max_connections: usize,
    pub timeout_ms: u64,
    pub keep_alive_interval_ms: u64,
}

#[derive(Clone, Debug)]
pub struct Subscription {
    pub id: String,
    pub topic: String,
    pub qos: QoS,
}

#[derive(Clone, Debug)]
pub enum QoS {
    AtMostOnce,
    AtLeastOnce,
    ExactlyOnce,
} 
