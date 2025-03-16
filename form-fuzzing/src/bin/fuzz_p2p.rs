// form-fuzzing/src/bin/fuzz_p2p.rs
//! Fuzzer for the Form-P2P message queue component.
//! 
//! This fuzzer tests the P2P message queue component of the Formation Network,
//! including message publishing, topic subscription, message routing,
//! and various network conditions and failure scenarios.

use form_fuzzing::{
    generators::Generator, 
    mutators::Mutator,
    harness::p2p::{
        P2PHarness, P2POperationResult, P2PError, Message, Topic, NodeId,
        NodeIdentity, P2PConfig, Subscription, QoS
    }
};
use form_fuzzing::generators::p2p::{
    QueueRequest, QueueResponse, FormMQGenerator,
    generate_node_id, generate_topic, generate_failure_reason,
    generate_node_identity, generate_p2p_config, generate_subscription, generate_message
};
use form_fuzzing::mutators::p2p::{
    QueueRequestMutator, QueueResponseMutator
};
use form_fuzzing::instrumentation::coverage;
use form_fuzzing::instrumentation::fault_injection::{self, FaultConfig};
use form_fuzzing::instrumentation::sanitizer;

use std::fs;
use std::path::Path;
use std::collections::HashMap;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use log::{debug, error, info, warn};
use rand::{Rng, thread_rng, seq::SliceRandom};
use rand::rngs::StdRng;
use rand::SeedableRng;
use uuid::Uuid;
use std::env;
use env_logger;

/// Fuzzing strategy
#[derive(Debug, Clone, Copy)]
enum FuzzingStrategy {
    /// Create a network with valid nodes and test normal operations
    ValidNetwork,
    /// Create a network with some invalid nodes and test operations
    MixedNetwork,
    /// Test subscription operations with valid subscriptions
    ValidSubscription,
    /// Test subscription operations with invalid subscriptions
    InvalidSubscription,
    /// Test message publishing with valid messages
    ValidMessagePublishing,
    /// Test message publishing with invalid messages
    InvalidMessagePublishing,
    /// Test message routing across different network topologies
    MessageRouting,
    /// Test network topology changes during operation
    TopologyChanges,
    /// Test high load scenarios
    HighLoad,
    /// Test with extreme failure rates
    HighFailureRate,
    /// Random strategy
    Random,
}

impl FuzzingStrategy {
    /// Get all available strategies
    fn all() -> Vec<FuzzingStrategy> {
        vec![
            FuzzingStrategy::ValidNetwork,
            FuzzingStrategy::MixedNetwork,
            FuzzingStrategy::ValidSubscription,
            FuzzingStrategy::InvalidSubscription,
            FuzzingStrategy::ValidMessagePublishing,
            FuzzingStrategy::InvalidMessagePublishing,
            FuzzingStrategy::MessageRouting,
            FuzzingStrategy::TopologyChanges,
            FuzzingStrategy::HighLoad,
            FuzzingStrategy::HighFailureRate,
            FuzzingStrategy::Random,
        ]
    }
    
    /// Get a random strategy
    fn random() -> Self {
        let strategies = Self::all();
        let mut rng = thread_rng();
        *strategies.choose(&mut rng).unwrap()
    }
}

/// Fuzzing statistics
struct FuzzingStats {
    /// Total operations performed
    operations: usize,
    /// Successful operations
    successes: usize,
    /// Failed operations
    failures: usize,
    /// Timeouts
    timeouts: usize,
    /// Rate limited operations
    rate_limited: usize,
    /// Operations with warnings
    warnings: usize,
    /// Crashes detected
    crashes: usize,
    /// Start time
    start_time: Instant,
}

impl FuzzingStats {
    /// Create new stats
    fn new() -> Self {
        Self {
            operations: 0,
            successes: 0,
            failures: 0,
            timeouts: 0,
            rate_limited: 0,
            warnings: 0,
            crashes: 0,
            start_time: Instant::now(),
        }
    }
    
    /// Record an operation result
    fn record(&mut self, result: &P2POperationResult) {
        self.operations += 1;
        
        match result {
            P2POperationResult::Success => self.successes += 1,
            P2POperationResult::Error(_) => self.failures += 1,
            P2POperationResult::Timeout => self.timeouts += 1,
            P2POperationResult::RateLimited => self.rate_limited += 1,
            P2POperationResult::SuccessWithWarnings(_) => {
                self.successes += 1;
                self.warnings += 1;
            }
        }
    }
    
    /// Report stats
    fn report(&self) {
        let elapsed = self.start_time.elapsed();
        let elapsed_secs = elapsed.as_secs_f64();
        let ops_per_sec = self.operations as f64 / elapsed_secs;
        
        info!("=== P2P Fuzzing Statistics ===");
        info!("Runtime: {:.2}s", elapsed_secs);
        info!("Total operations: {}", self.operations);
        info!("Operations per second: {:.2}", ops_per_sec);
        info!("Successes: {} ({:.1}%)", 
            self.successes, 
            100.0 * self.successes as f64 / self.operations as f64
        );
        info!("Failures: {} ({:.1}%)", 
            self.failures, 
            100.0 * self.failures as f64 / self.operations as f64
        );
        info!("Timeouts: {} ({:.1}%)", 
            self.timeouts, 
            100.0 * self.timeouts as f64 / self.operations as f64
        );
        info!("Rate limited: {} ({:.1}%)", 
            self.rate_limited, 
            100.0 * self.rate_limited as f64 / self.operations as f64
        );
        info!("Operations with warnings: {} ({:.1}%)", 
            self.warnings, 
            100.0 * self.warnings as f64 / self.operations as f64
        );
        info!("Crashes detected: {}", self.crashes);
    }
}

/// P2P Fuzzer
struct P2PFuzzer {
    /// Harness for testing
    harness: P2PHarness,
    /// QueueRequest mutator
    request_mutator: QueueRequestMutator,
    /// QueueResponse mutator
    response_mutator: QueueResponseMutator,
    /// Statistics
    stats: FuzzingStats,
    /// Corpus directory
    corpus_dir: Option<String>,
    /// Strategy to use
    strategy: FuzzingStrategy,
}

impl P2PFuzzer {
    /// Create a new P2P fuzzer
    fn new(harness: P2PHarness, stats: FuzzingStats, strategy: FuzzingStrategy) -> Self {
        Self {
            harness,
            request_mutator: QueueRequestMutator::new(),
            response_mutator: QueueResponseMutator::new(),
            stats,
            corpus_dir: None,
            strategy,
        }
    }
    
    /// Set corpus directory
    fn set_corpus_dir(&mut self, dir: &str) {
        self.corpus_dir = Some(dir.to_string());
        
        // Ensure directory exists
        if let Some(dir) = &self.corpus_dir {
            fs::create_dir_all(dir).unwrap_or_else(|e| {
                error!("Failed to create corpus directory: {}", e);
            });
        }
    }
    
    /// Run the fuzzer for a specified number of iterations
    fn run(&mut self, iterations: usize) {
        info!("Running {} iterations with strategy {:?}", iterations, self.strategy);
        
        for i in 0..iterations {
            info!("Running iteration {}/{}", i+1, iterations);
            
            match self.strategy {
                FuzzingStrategy::ValidNetwork => self.fuzz_valid_network(),
                FuzzingStrategy::MixedNetwork => self.fuzz_mixed_network(),
                FuzzingStrategy::ValidSubscription => self.fuzz_valid_subscription(),
                FuzzingStrategy::InvalidSubscription => self.fuzz_invalid_subscription(),
                FuzzingStrategy::ValidMessagePublishing => self.fuzz_valid_message_publishing(),
                FuzzingStrategy::InvalidMessagePublishing => self.fuzz_invalid_message_publishing(),
                FuzzingStrategy::MessageRouting => self.fuzz_message_routing(),
                FuzzingStrategy::TopologyChanges => self.fuzz_topology_changes(),
                FuzzingStrategy::HighLoad => self.fuzz_high_load(),
                FuzzingStrategy::HighFailureRate => self.fuzz_high_failure_rate(),
                FuzzingStrategy::Random => {
                    // For random strategy, choose a different strategy each time
                    let random_strategy = FuzzingStrategy::random();
                    info!("Selected random strategy: {:?}", random_strategy);
                    self.run_iteration(random_strategy);
                },
            }
        }
    }
    
    /// Run a single iteration with a specific strategy
    fn run_iteration(&mut self, strategy: FuzzingStrategy) {
        match strategy {
            FuzzingStrategy::ValidNetwork => self.fuzz_valid_network(),
            FuzzingStrategy::MixedNetwork => self.fuzz_mixed_network(),
            FuzzingStrategy::ValidSubscription => self.fuzz_valid_subscription(),
            FuzzingStrategy::InvalidSubscription => self.fuzz_invalid_subscription(),
            FuzzingStrategy::ValidMessagePublishing => self.fuzz_valid_message_publishing(),
            FuzzingStrategy::InvalidMessagePublishing => self.fuzz_invalid_message_publishing(),
            FuzzingStrategy::MessageRouting => self.fuzz_message_routing(),
            FuzzingStrategy::TopologyChanges => self.fuzz_topology_changes(),
            FuzzingStrategy::HighLoad => self.fuzz_high_load(),
            FuzzingStrategy::HighFailureRate => self.fuzz_high_failure_rate(),
            FuzzingStrategy::Random => self.fuzz_random(),
        }
    }
    
    /// Fuzz network with valid nodes
    fn fuzz_valid_network(&mut self) {
        info!("Running valid network test");
        
        let mut rng = thread_rng();
        let node_count = rng.gen_range(2..10);
        
        // Create nodes with valid configuration
        let mut nodes = Vec::new();
        for i in 0..node_count {
            let node_id = format!("node-{}", i);
            let identity = generate_node_identity();  // Use the proper generator
            let config = generate_p2p_config();       // Use the proper generator
            
            let result = self.harness.add_node(identity, config);
            self.stats.record(&result);
            
            if let P2POperationResult::Success = &result {
                nodes.push(node_id);
            }
        }
        
        // Connect nodes in a random topology
        let topologies = ["star", "mesh", "ring", "line"];
        let topology = topologies.choose(&mut rng).unwrap();
        
        let result = self.harness.build_topology(topology);
        self.stats.record(&result);
        
        if let P2POperationResult::Error(e) = &result {
            error!("Failed to build {} topology: {:?}", topology, e);
        } else {
            info!("Built {} topology with {} nodes", topology, node_count);
        }
    }
    
    /// Fuzz network with a mix of valid and invalid nodes
    fn fuzz_mixed_network(&mut self) {
        info!("Running mixed network test");
        
        let mut rng = thread_rng();
        let node_count = rng.gen_range(2..10);
        
        // Create nodes with mixed valid/invalid configurations
        let mut nodes = Vec::new();
        for i in 0..node_count {
            let node_id = format!("node-{}", i);
            
            // Generate either valid or invalid configuration
            let (identity, config) = if rng.gen_bool(0.5) {
                (generate_node_identity(), generate_p2p_config())
            } else {
                // For invalid configs, we'll use valid identity but with extreme values in config
                let mut config = generate_p2p_config();
                // Make configuration invalid with extreme values
                config.max_connections = rng.gen_range(1000..5000);  // Too many connections
                config.timeout_ms = if rng.gen_bool(0.5) { 0 } else { 1_000_000 };  // Invalid timeout
                
                (generate_node_identity(), config)
            };
            
            let result = self.harness.add_node(identity, config);
            self.stats.record(&result);
            
            if let P2POperationResult::Success = &result {
                nodes.push(node_id);
            }
        }
        
        // Create some connections between nodes
        if nodes.len() >= 2 {
            let connection_count = rng.gen_range(1..nodes.len());
            
            for _ in 0..connection_count {
                let from = nodes.choose(&mut rng).unwrap();
                let to = nodes.choose(&mut rng).unwrap();
                
                if from != to {
                    let result = self.harness.connect_nodes(from, to);
                    self.stats.record(&result);
                }
            }
        }
    }
    
    /// Fuzz with valid subscriptions
    fn fuzz_valid_subscription(&mut self) {
        info!("Running valid subscription test");
        
        // Get active nodes from the harness
        let nodes = match self.get_active_node_ids() {
            Some(n) if !n.is_empty() => n,
            _ => {
                // Create some nodes if none exist
                self.fuzz_valid_network();
                self.get_active_node_ids().unwrap_or_default()
            }
        };
        
        if nodes.is_empty() {
            warn!("No active nodes available for subscription test");
            return;
        }
        
        let mut rng = thread_rng();
        let sub_count = rng.gen_range(1..20);
        
        // Create valid subscriptions
        for _i in 0..sub_count {
            let subscription = generate_subscription();
            let node_id = nodes.choose(&mut rng).unwrap();
            
            let result = self.harness.subscribe(Some(node_id), subscription);
            self.stats.record(&result);
        }
        
        // Unsubscribe from some
        if rng.gen_bool(0.3) {
            let unsub_id = format!("sub-{}", rng.gen_range(0..sub_count));
            let node_id = nodes.choose(&mut rng).unwrap();
            
            let result = self.harness.unsubscribe(Some(node_id), &unsub_id);
            self.stats.record(&result);
        }
    }
    
    /// Fuzz with invalid subscriptions
    fn fuzz_invalid_subscription(&mut self) {
        let mut rng = thread_rng();
        
        // Ensure we have some nodes
        if self.get_active_node_ids().map(|n| n.len()).unwrap_or(0) < 2 {
            self.fuzz_valid_network();
        }
        
        if let Some(nodes) = self.get_active_node_ids() {
            // Create 3-10 invalid subscriptions
            let sub_count = rng.gen_range(3..11);
            
            for i in 0..sub_count {
                // Create a subscription with invalid characteristics
                let mut subscription = Subscription {
                    id: format!("sub-{}", i),
                    topic: if rng.gen_bool(0.5) { "".to_string() } else { "invalid/#/topic".to_string() },
                    qos: QoS::AtMostOnce,
                };
                
                let node_id = nodes.choose(&mut rng).unwrap();
                
                let result = self.harness.subscribe(Some(node_id), subscription);
                self.stats.record(&result);
                
                // We expect this to fail in some way
                match &result {
                    P2POperationResult::Error(_) | P2POperationResult::SuccessWithWarnings(_) => {
                        // Expected behavior
                    },
                    P2POperationResult::Success => {
                        warn!("Invalid subscription was accepted without warnings");
                    },
                    _ => {},
                }
            }
            
            // Try to unsubscribe from non-existent subscription
            if rng.gen_bool(0.5) {
                let unsub_id = format!("non-existent-{}", Uuid::new_v4());
                let node_id = nodes.choose(&mut rng).unwrap();
                
                let result = self.harness.unsubscribe(Some(node_id), &unsub_id);
                self.stats.record(&result);
                
                // We expect this to fail
                if !matches!(result, P2POperationResult::Error(_)) {
                    warn!("Unsubscribing from non-existent subscription didn't fail");
                }
            }
        }
    }
    
    /// Fuzz with valid message publishing
    fn fuzz_valid_message_publishing(&mut self) {
        let mut rng = thread_rng();
        
        // Ensure we have some nodes and subscriptions
        if self.get_active_node_ids().map(|n| n.len()).unwrap_or(0) < 2 {
            self.fuzz_valid_network();
            self.fuzz_valid_subscription();
        }
        
        if let Some(nodes) = self.get_active_node_ids() {
            // Publish 5-15 valid messages
            let msg_count = rng.gen_range(5..16);
            
            for _i in 0..msg_count {
                let message = generate_message();
                let node_id = nodes.choose(&mut rng).unwrap();
                
                let result = self.harness.publish(Some(node_id), message);
                self.stats.record(&result);
                
                // Occasionally try to receive messages
                if _i % 3 == 0 && !nodes.is_empty() {
                    let sub_id = format!("sub-{}", rng.gen_range(0..3)); // Assuming we have subs from fuzz_valid_subscription
                    let node_id = nodes.choose(&mut rng).unwrap();
                    
                    match self.harness.receive::<String>(Some(node_id), &sub_id, 5) {
                        Ok(messages) => {
                            debug!("Received {} messages", messages.len());
                        },
                        Err(e) => {
                            debug!("Failed to receive messages: {:?}", e);
                        }
                    }
                }
            }
        }
    }
    
    /// Fuzz with invalid message publishing
    fn fuzz_invalid_message_publishing(&mut self) {
        let mut rng = thread_rng();
        
        // Ensure we have some nodes
        if self.get_active_node_ids().map(|n| n.len()).unwrap_or(0) < 2 {
            self.fuzz_valid_network();
        }
        
        if let Some(nodes) = self.get_active_node_ids() {
            // Publish 5-15 invalid messages
            let msg_count = rng.gen_range(5..16);
            
            for _ in 0..msg_count {
                // Create an invalid message (using a string directly instead of proper Message)
                let raw_message = generate_failure_reason();
                
                let node_id = nodes.choose(&mut rng).unwrap();
                
                // Properly create a Message object for publishing
                let message = Message::new(raw_message);
                
                let result = self.harness.publish(Some(node_id), message);
                self.stats.record(&result);
                
                // We expect this to fail in some way
                match &result {
                    P2POperationResult::Error(_) | P2POperationResult::SuccessWithWarnings(_) => {
                        // Expected behavior
                    },
                    P2POperationResult::Success => {
                        warn!("Invalid message was accepted without warnings");
                    },
                    _ => {},
                }
            }
            
            // Also try duplicate message IDs
            if !nodes.is_empty() {
                // Since we can't set header directly, we'll just create a message with a special content
                let message = Message::new("duplicate-id".to_string());
                
                let node_id = nodes.choose(&mut rng).unwrap();
                
                let result = self.harness.publish(Some(node_id), message);
                self.stats.record(&result);
                
                if !matches!(result, P2POperationResult::Error(P2PError::DuplicateMessageId(_))) {
                    warn!("Duplicate message ID not detected");
                }
            }
        }
    }
    
    /// Fuzz message routing across different topologies
    fn fuzz_message_routing(&mut self) {
        info!("Running message routing test");
        
        let mut rng = thread_rng();
        
        // First create a network with a specific topology
        // Typically a mesh network works best for routing tests
        let node_count = rng.gen_range(3..8);
        
        // Clear existing state
        self.reset_harness();
        
        // Create nodes
        let mut nodes = Vec::new();
        for i in 0..node_count {
            let node_id = format!("node-{}", i);
            let mut identity = generate_node_identity();
            // Update the id field, not node_id
            identity.id = node_id.clone();
            
            // Create a proper P2PConfig object instead of a string
            let config = generate_p2p_config();
            
            let result = self.harness.add_node(identity, config);
            self.stats.record(&result);
            
            if let P2POperationResult::Success = &result {
                nodes.push(node_id);
            }
        }
        
        // Connect the nodes in a mesh network
        for i in 0..nodes.len() {
            for j in i+1..nodes.len() {
                let result = self.harness.connect_nodes(&nodes[i], &nodes[j]);
                self.stats.record(&result);
            }
        }
        
        // Create subscriptions with different patterns
        for i in 0..nodes.len() {
            if i < nodes.len() {
                // Create a subscription with appropriate fields
                let subscription = Subscription {
                    id: format!("routing-sub-{}", i),
                    topic: match i % 3 {
                        0 => "test/routing".to_string(),
                        1 => "test/+/data".to_string(),
                        _ => "test/#".to_string(),
                    },
                    qos: QoS::AtLeastOnce,
                };
                
                let result = self.harness.subscribe(Some(&nodes[i]), subscription);
                self.stats.record(&result);
            }
        }
        
        // Publish messages to different topics
        let topics = [
            "test/routing",
            "test/custom/data",
            "test/events/important",
        ];
        
        for topic in topics {
            // Create a message with the content being the topic
            let message = Message::new(topic.to_string());
            
            let pub_node = nodes.choose(&mut rng).unwrap();
            
            let result = self.harness.publish(Some(pub_node), message);
            self.stats.record(&result);
            
            // Allow some time for routing
            std::thread::sleep(Duration::from_millis(50));
            
            // Try to receive messages on each node
            for node_id in &nodes {
                let sub_id = format!("routing-sub-{}", nodes.iter().position(|n| n == node_id).unwrap_or(0));
                
                match self.harness.receive::<String>(Some(node_id), &sub_id, 5) {
                    Ok(messages) => {
                        if messages.is_empty() {
                            info!("No messages received on node {} for topic {}", node_id, topic);
                        } else {
                            info!("Received {} messages on node {} for topic {}", messages.len(), node_id, topic);
                        }
                    },
                    Err(e) => {
                        info!("Failed to receive messages on node {}: {:?}", node_id, e);
                    }
                }
            }
        }
    }
    
    /// Fuzz with mixed network topology changes
    fn fuzz_topology_changes(&mut self) {
        info!("Running topology changes test");
        
        // Create a network
        self.fuzz_valid_network();
        
        let mut rng = thread_rng();
        
        if let Some(nodes) = self.get_active_node_ids() {
            if nodes.is_empty() {
                warn!("No active nodes for topology test");
                return;
            }
            
            // Connect random nodes
            let connect_count = rng.gen_range(1..nodes.len().max(2));
            for _ in 0..connect_count {
                if nodes.len() >= 2 {
                    let idx1 = rng.gen_range(0..nodes.len());
                    let mut idx2 = rng.gen_range(0..nodes.len());
                    while idx1 == idx2 {
                        idx2 = rng.gen_range(0..nodes.len());
                    }
                    
                    let result = self.harness.connect_nodes(&nodes[idx1], &nodes[idx2]);
                    self.stats.record(&result);
                }
            }
            
            // Remove some nodes while operations are happening
            if nodes.len() > 2 && rng.gen_bool(0.3) {
                let remove_idx = rng.gen_range(0..nodes.len());
                let node_to_remove = &nodes[remove_idx];
                
                let result = self.harness.remove_node(node_to_remove);
                self.stats.record(&result);
            }
            
            // "Disconnect" some nodes by adding and then removing connections
            if nodes.len() >= 2 && rng.gen_bool(0.4) {
                let idx1 = rng.gen_range(0..nodes.len());
                let mut idx2 = rng.gen_range(0..nodes.len());
                while idx1 == idx2 {
                    idx2 = rng.gen_range(0..nodes.len());
                }
                
                // First connect nodes
                let connect_result = self.harness.connect_nodes(&nodes[idx1], &nodes[idx2]);
                self.stats.record(&connect_result);
                
                // Then simulate disconnection by removing one of the nodes and adding it back
                if rng.gen_bool(0.5) && matches!(connect_result, P2POperationResult::Success) {
                    let temp_result = self.harness.remove_node(&nodes[idx2]);
                    self.stats.record(&temp_result);
                    
                    if let P2POperationResult::Success = temp_result {
                        // Re-add the node with the same ID
                        let identity = NodeIdentity {
                            id: nodes[idx2].clone(),
                            public_key: format!("pk-{}", Uuid::new_v4()),
                            addresses: vec![format!("192.168.1.{}", idx2 + 1)],
                        };
                        
                        let config = P2PConfig {
                            max_connections: rng.gen_range(3..10),
                            timeout_ms: rng.gen_range(50..200),
                            keep_alive_interval_ms: rng.gen_range(500..2000),
                        };
                        
                        let add_result = self.harness.add_node(identity, config);
                        self.stats.record(&add_result);
                    }
                }
            }
            
            // Create some subscriptions to test routing through topology
            let sub_count = rng.gen_range(1..nodes.len().max(3));
            for i in 0..sub_count {
                if i < nodes.len() {
                    let subscription = Subscription {
                        id: format!("routing-sub-{}", i),
                        topic: match rng.gen_range(0..3) {
                            0 => "test/routing".to_string(),
                            1 => "test/+/data".to_string(),
                            _ => "test/#".to_string(),
                        },
                        qos: match rng.gen_range(0..3) {
                            0 => QoS::AtMostOnce,
                            1 => QoS::AtLeastOnce,
                            _ => QoS::ExactlyOnce,
                        },
                    };
                    
                    let result = self.harness.subscribe(Some(&nodes[i]), subscription);
                    self.stats.record(&result);
                }
            }
            
            // Publish messages that should be routed
            let msg_count = rng.gen_range(1..10);
            for _ in 0..msg_count {
                // Select a random topic that should match some subscriptions
                let topic = match rng.gen_range(0..3) {
                    0 => "test/routing",
                    1 => "test/custom/data",
                    _ => "test/events/important",
                };
                
                // Create a message for this topic
                let message = Message::new(format!("Routed message: {}", Uuid::new_v4()));
                
                // Publish from a random node
                let node_idx = rng.gen_range(0..nodes.len());
                let result = self.harness.publish(Some(&nodes[node_idx]), message);
                self.stats.record(&result);
            }
        }
    }
    
    /// Fuzz high load scenarios
    fn fuzz_high_load(&mut self) {
        info!("Running high load test");
        
        // Create a network with nodes that have limited resources
        let mut rng = thread_rng();
        let node_count = rng.gen_range(5..20);
        
        for i in 0..node_count {
            let identity = NodeIdentity {
                id: format!("node-{}", i),
                public_key: format!("pk-{}", Uuid::new_v4()),
                addresses: vec![format!("192.168.1.{}", i+1)],
            };
            
            // Create configs with limited resources
            let config = P2PConfig {
                max_connections: rng.gen_range(3..10),
                timeout_ms: rng.gen_range(50..200),
                keep_alive_interval_ms: rng.gen_range(500..2000),
            };
            
            let result = self.harness.add_node(identity, config);
            self.stats.record(&result);
        }
        
        // Get nodes and connect them in a mesh
        if let Some(nodes) = self.get_active_node_ids() {
            // Connect all nodes to create a full mesh (this should stress connection limits)
            for i in 0..nodes.len() {
                for j in i+1..nodes.len() {
                    let result = self.harness.connect_nodes(&nodes[i], &nodes[j]);
                    self.stats.record(&result);
                }
            }
            
            // Create a large number of subscriptions
            let sub_count = rng.gen_range(50..200);
            let node_count = nodes.len();
            
            for i in 0..sub_count {
                let node_idx = i % node_count;
                let subscription = Subscription {
                    id: format!("highload-sub-{}", i),
                    topic: format!("load/topic/{}", i % 20),
                    qos: QoS::AtLeastOnce, // Use QoS 1 to stress reliability mechanisms
                };
                
                let result = self.harness.subscribe(Some(&nodes[node_idx]), subscription);
                self.stats.record(&result);
            }
            
            // Publish a large number of messages
            let msg_count = rng.gen_range(100..500);
            
            for i in 0..msg_count {
                let node_idx = i % node_count;
                let topic_id = i % 20;
                
                let message = Message::new(format!("Highload message {} for topic {}", i, topic_id));
                
                let result = self.harness.publish(Some(&nodes[node_idx]), message);
                self.stats.record(&result);
                
                // We don't try to receive immediately as that would make the test run very long
                // Just stress test the publish capability
            }
        }
    }
    
    /// Fuzz with extreme failure rates
    fn fuzz_high_failure_rate(&mut self) {
        info!("Running high failure rate test");
        
        // Use thread_rng
        let mut rng = thread_rng();
        
        // First create a network
        let node_count = rng.gen_range(3..6);
        
        // Clear existing network
        self.reset_harness();
        
        // Set high failure rate
        self.harness.set_failure_rate(0.7); // 70% failure rate
        
        // Create nodes
        let mut nodes = Vec::new();
        for i in 0..node_count {
            let node_id = format!("node-{}", i);
            let mut identity = NodeIdentity {
                id: node_id.clone(),
                public_key: format!("pk-{}", Uuid::new_v4()),
                addresses: vec![format!("192.168.1.{}", i+1)],
            };
            
            let config = P2PConfig {
                max_connections: rng.gen_range(3..10),
                timeout_ms: rng.gen_range(50..200),
                keep_alive_interval_ms: rng.gen_range(500..2000),
            };
            
            let result = self.harness.add_node(identity, config);
            self.stats.record(&result);
            
            if let P2POperationResult::Success = &result {
                nodes.push(node_id);
            }
        }
        
        // Attempt operations that will likely fail
        for _ in 0..20 {
            if nodes.is_empty() {
                break;
            }
            
            let operation = rng.gen_range(0..5);
            match operation {
                0 => if nodes.len() >= 2 {
                    // Connect nodes
                    let idx1 = rng.gen_range(0..nodes.len());
                    let mut idx2 = rng.gen_range(0..nodes.len());
                    while idx1 == idx2 {
                        idx2 = rng.gen_range(0..nodes.len());
                    }
                    
                    info!("Attempting to connect nodes {} and {}", nodes[idx1], nodes[idx2]);
                    let result = self.harness.connect_nodes(&nodes[idx1], &nodes[idx2]);
                    self.stats.record(&result);
                },
                1 => if !nodes.is_empty() {
                    // Subscribe
                    let idx = rng.gen_range(0..nodes.len());
                    let subscription = Subscription {
                        id: format!("sub-{}", Uuid::new_v4()),
                        topic: generate_topic(),
                        qos: QoS::AtLeastOnce,
                    };
                    
                    info!("Attempting to subscribe with node {}", nodes[idx]);
                    let result = self.harness.subscribe(Some(&nodes[idx]), subscription);
                    self.stats.record(&result);
                },
                2 => if !nodes.is_empty() {
                    // Publish
                    let idx = rng.gen_range(0..nodes.len());
                    let message = generate_message();
                    
                    info!("Attempting to publish with node {}", nodes[idx]);
                    let result = self.harness.publish(Some(&nodes[idx]), message);
                    self.stats.record(&result);
                },
                3 => if !nodes.is_empty() {
                    // Remove node
                    let idx = rng.gen_range(0..nodes.len());
                    info!("Attempting to remove node {}", nodes[idx]);
                    let result = self.harness.remove_node(&nodes[idx]);
                    self.stats.record(&result);
                    
                    if let P2POperationResult::Success = &result {
                        nodes.remove(idx);
                    }
                },
                _ => {
                    // Add new node
                    let node_id = format!("node-new-{}", Uuid::new_v4());
                    let identity = NodeIdentity {
                        id: node_id.clone(),
                        public_key: format!("pk-{}", Uuid::new_v4()),
                        addresses: vec![format!("192.168.1.{}", nodes.len() + 1)],
                    };
                    
                    let config = P2PConfig {
                        max_connections: rng.gen_range(3..10),
                        timeout_ms: rng.gen_range(50..200),
                        keep_alive_interval_ms: rng.gen_range(500..2000),
                    };
                    
                    info!("Attempting to add new node {}", node_id);
                    let result = self.harness.add_node(identity, config);
                    self.stats.record(&result);
                    
                    if let P2POperationResult::Success = &result {
                        nodes.push(node_id);
                    }
                }
            }
        }
        
        // Reset failure rate for other tests
        self.harness.set_failure_rate(0.05);
    }
    
    /// Save an interesting input to corpus
    fn save_to_corpus(&self, data: &[u8], prefix: &str) {
        if let Some(dir) = &self.corpus_dir {
            let filename = format!("{}/{}-{}.bin", dir, prefix, Uuid::new_v4());
            fs::write(&filename, data).unwrap_or_else(|e| {
                error!("Failed to write corpus file {}: {}", filename, e);
            });
        }
    }
    
    /// Get list of active node IDs in the harness
    fn get_active_node_ids(&self) -> Option<Vec<String>> {
        let status = self.harness.get_network_status();
        
        if status.is_empty() {
            return None;
        }
        
        Some(status.keys().cloned().collect())
    }
    
    /// Reset the harness to a clean state
    fn reset_harness(&mut self) {
        self.harness = P2PHarness::new();
    }

    /// Reset a node's identity
    fn reset_identity(&mut self) {
        let mut rng = thread_rng();
        
        if let Some(nodes) = self.get_active_node_ids() {
            if !nodes.is_empty() {
                let node_id = nodes.choose(&mut rng).unwrap();
                
                info!("Resetting identity for node {}", node_id);
                
                // Instead of using update_identity, we'll remove the node and add it back
                // with the same ID but different details
                
                // First remove the node
                let remove_result = self.harness.remove_node(node_id);
                self.stats.record(&remove_result);
                
                if let P2POperationResult::Success = remove_result {
                    // Create a new identity with the same node ID
                    let identity = NodeIdentity {
                        id: node_id.clone(),
                        public_key: format!("pk-reset-{}", Uuid::new_v4()),
                        addresses: vec![format!("192.168.{}.{}", 
                                          rng.gen_range(1..255), 
                                          rng.gen_range(1..255))],
                    };
                    
                    // Create a config
                    let config = P2PConfig {
                        max_connections: rng.gen_range(5..20),
                        timeout_ms: rng.gen_range(100..1000),
                        keep_alive_interval_ms: rng.gen_range(1000..5000),
                    };
                    
                    // Add the node back
                    let add_result = self.harness.add_node(identity, config);
                    self.stats.record(&add_result);
                }
            }
        }
    }

    /// Add an invalid node
    fn add_invalid_node(&mut self) {
        let node_id = generate_node_id();
        
        info!("Adding invalid node {}", node_id);
        
        // Create a node identity but with invalid properties
        let mut identity = NodeIdentity {
            id: node_id.clone(),
            public_key: "".to_string(), // Invalid empty public key
            addresses: vec![], // Invalid empty addresses
        };
        
        // Create an invalid config
        let config = P2PConfig {
            max_connections: 0, // Invalid zero max connections
            timeout_ms: 0,     // Invalid zero timeout
            keep_alive_interval_ms: 0, // Invalid zero keep alive
        };
        
        let result = self.harness.add_node(identity, config);
        self.stats.record(&result);
        
        // We expect this to fail
        match &result {
            P2POperationResult::Error(_) => {
                // Expected behavior
            },
            _ => {
                warn!("Invalid node was accepted");
            }
        }
    }

    /// Fuzz a random strategy
    fn fuzz_random(&mut self) {
        let strategy = FuzzingStrategy::random();
        self.run_iteration(strategy);
    }

    /// Print the statistics
    fn print_stats(&self) {
        info!("Fuzzing statistics:");
        info!("----------------");
        self.stats.report();
    }
}

fn main() {
    // Initialize logging
    env_logger::init();
    info!("Starting P2P fuzzing");
    
    // Parse arguments
    let mut args = std::env::args().skip(1);
    let iteration_count = args.next()
        .map(|s| s.parse::<usize>().unwrap_or(10))
        .unwrap_or(10);
    
    // Create the fuzzer
    let strategy = if args.len() > 0 {
        match args.next().unwrap().as_str() {
            "valid_network" => FuzzingStrategy::ValidNetwork,
            "mixed_network" => FuzzingStrategy::MixedNetwork,
            "subscription" => FuzzingStrategy::ValidSubscription,
            "message" => FuzzingStrategy::ValidMessagePublishing,
            "topology" => FuzzingStrategy::TopologyChanges,
            "high_load" => FuzzingStrategy::HighLoad,
            _ => FuzzingStrategy::Random,
        }
    } else {
        FuzzingStrategy::Random
    };
    
    // Create a statistics tracker
    let stats = FuzzingStats::new();
    
    // Create the harness 
    let mut harness = P2PHarness::new();
    
    // Configure fault injection parameters directly on the harness
    harness.set_failure_rate(0.05);  // 5% chance of random failure
    harness.set_timeout_rate(0.03);  // 3% chance of timeout
    
    // Create and run the fuzzer
    let mut fuzzer = P2PFuzzer::new(harness, stats, strategy);
    fuzzer.run(iteration_count);
    
    // Print statistics
    fuzzer.print_stats();
    
    info!("P2P fuzzing complete");
} 