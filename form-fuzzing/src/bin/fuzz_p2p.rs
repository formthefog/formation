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
        P2PHarness, P2POperationResult, P2PError, Message, Topic, NodeId
    }
};
use form_fuzzing::generators::p2p::{
    QueueRequest, QueueResponse, FormMQGenerator,
    generate_node_id, generate_topic, generate_failure_reason
};
use form_fuzzing::mutators::p2p::{
    QueueRequestMutator, QueueResponseMutator
};
use form_fuzzing::instrumentation::coverage::{self, init_coverage_tracking};
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
}

impl P2PFuzzer {
    /// Create a new P2P fuzzer
    fn new() -> Self {
        Self {
            harness: P2PHarness::new(),
            request_mutator: QueueRequestMutator::new(),
            response_mutator: QueueResponseMutator::new(),
            stats: FuzzingStats::new(),
            corpus_dir: None,
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
    
    /// Run a single fuzzing iteration with the given strategy
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
        }
    }
    
    /// Fuzz network with valid nodes
    fn fuzz_valid_network(&mut self) {
        // Create 3-5 nodes
        let mut rng = thread_rng();
        let node_count = rng.gen_range(3..6);
        
        // Clear existing nodes
        self.reset_harness();
        
        for i in 0..node_count {
            let node_id = format!("node-{}", i);
            let mut identity = generate_node_id();
            identity.node_id = node_id.clone();
            
            let config = generate_topic();
            
            let result = self.harness.add_node(identity, config);
            self.stats.record(&result);
            
            if let P2POperationResult::Error(e) = &result {
                error!("Failed to create valid node {}: {:?}", node_id, e);
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
        // Create 3-5 nodes, some valid, some invalid
        let mut rng = thread_rng();
        let node_count = rng.gen_range(3..6);
        
        // Clear existing nodes
        self.reset_harness();
        
        for i in 0..node_count {
            let node_id = format!("node-{}", i);
            
            // 50% chance of an invalid node
            let (identity, config) = if rng.gen_bool(0.5) {
                let mut identity = generate_node_id();
                identity.node_id = node_id.clone();
                (identity, generate_topic())
            } else {
                let mut identity = generate_failure_reason();
                identity.node_id = node_id.clone();
                (identity, generate_topic())
            };
            
            let result = self.harness.add_node(identity, config);
            self.stats.record(&result);
            
            if let P2POperationResult::Error(e) = &result {
                debug!("Failed to create node {}: {:?}", node_id, e);
            }
        }
        
        // Try to connect nodes
        if let Some(nodes) = self.get_active_node_ids() {
            if nodes.len() >= 2 {
                for _ in 0..rng.gen_range(1..10) {
                    let from_idx = rng.gen_range(0..nodes.len());
                    let mut to_idx = rng.gen_range(0..nodes.len());
                    // Ensure from != to
                    while to_idx == from_idx {
                        to_idx = rng.gen_range(0..nodes.len());
                    }
                    
                    let result = self.harness.connect_nodes(&nodes[from_idx], &nodes[to_idx]);
                    self.stats.record(&result);
                }
            }
        }
    }
    
    /// Fuzz with valid subscriptions
    fn fuzz_valid_subscription(&mut self) {
        let mut rng = thread_rng();
        
        // Ensure we have some nodes
        if self.get_active_node_ids().map(|n| n.len()).unwrap_or(0) < 2 {
            self.fuzz_valid_network();
        }
        
        if let Some(nodes) = self.get_active_node_ids() {
            // Create 3-10 valid subscriptions
            let sub_count = rng.gen_range(3..11);
            
            for i in 0..sub_count {
                let mut subscription = generate_topic();
                subscription.subscriber_id = nodes.choose(&mut rng).unwrap().clone();
                subscription.subscription_id = format!("sub-{}", i);
                
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
                let mut subscription = generate_topic();
                
                // Set valid subscriber ID to ensure it's only the subscription that's invalid
                if !nodes.is_empty() {
                    subscription.subscriber_id = nodes.choose(&mut rng).unwrap().clone();
                }
                
                subscription.subscription_id = format!("sub-{}", i);
                
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
            
            for i in 0..msg_count {
                let mut message = generate_topic();
                message.header.message_id = format!("msg-{}", Uuid::new_v4());
                
                let node_id = nodes.choose(&mut rng).unwrap();
                
                let result = self.harness.publish(Some(node_id), message);
                self.stats.record(&result);
                
                // Occasionally try to receive messages
                if i % 3 == 0 && !nodes.is_empty() {
                    let sub_id = format!("sub-{}", rng.gen_range(0..3)); // Assuming we have subs from fuzz_valid_subscription
                    let node_id = nodes.choose(&mut rng).unwrap();
                    
                    match self.harness.receive(Some(node_id), &sub_id, 5) {
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
                let message = generate_failure_reason();
                
                let node_id = nodes.choose(&mut rng).unwrap();
                
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
                let mut message = generate_topic();
                message.header.message_id = "duplicate-id".to_string();
                
                let node_id = nodes.choose(&mut rng).unwrap();
                
                // First publish should succeed
                let result1 = self.harness.publish(Some(node_id), message.clone());
                self.stats.record(&result1);
                
                // Second publish should fail with duplicate ID
                let result2 = self.harness.publish(Some(node_id), message);
                self.stats.record(&result2);
                
                if !matches!(result2, P2POperationResult::Error(P2PError::DuplicateMessageId(_))) {
                    warn!("Duplicate message ID not detected");
                }
            }
        }
    }
    
    /// Fuzz message routing across different topologies
    fn fuzz_message_routing(&mut self) {
        let mut rng = thread_rng();
        
        // Create a new network with a specific topology
        self.reset_harness();
        
        // Create 4-8 nodes
        let node_count = rng.gen_range(4..9);
        
        for i in 0..node_count {
            let node_id = format!("node-{}", i);
            let mut identity = generate_node_id();
            identity.node_id = node_id.clone();
            
            let config = generate_topic();
            
            let result = self.harness.add_node(identity, config);
            self.stats.record(&result);
        }
        
        // Choose a topology
        let topologies = ["star", "mesh", "ring", "line"];
        let topology = topologies.choose(&mut rng).unwrap();
        
        let result = self.harness.build_topology(topology);
        self.stats.record(&result);
        
        if let P2POperationResult::Error(e) = &result {
            error!("Failed to build {} topology: {:?}", topology, e);
            return;
        }
        
        info!("Testing message routing with {} topology and {} nodes", topology, node_count);
        
        // Set up subscriptions
        if let Some(nodes) = self.get_active_node_ids() {
            // Create subscriptions on different nodes
            for i in 0..node_count {
                if i >= nodes.len() {
                    break;
                }
                
                let mut subscription = generate_topic();
                subscription.subscriber_id = nodes[i].clone();
                subscription.subscription_id = format!("routing-sub-{}", i);
                
                // Different subscription patterns
                match i % 3 {
                    0 => {
                        // Exact topic
                        subscription.topic_pattern = "test/routing".to_string();
                    },
                    1 => {
                        // Single-level wildcard
                        subscription.topic_pattern = "test/+/data".to_string();
                    },
                    _ => {
                        // Multi-level wildcard
                        subscription.topic_pattern = "test/#".to_string();
                    }
                }
                
                let result = self.harness.subscribe(Some(&nodes[i]), subscription);
                self.stats.record(&result);
            }
            
            // Publish messages that should match different subscriptions
            let topics = [
                "test/routing",
                "test/level1/data",
                "test/level1/level2",
                "unmatched/topic",
            ];
            
            for topic in topics {
                let mut message = generate_topic();
                message.header.message_id = format!("routing-{}", Uuid::new_v4());
                message.header.topic = topic.to_string();
                
                let pub_node = nodes.choose(&mut rng).unwrap();
                
                let result = self.harness.publish(Some(pub_node), message);
                self.stats.record(&result);
                
                // Try to receive on all nodes to check routing
                for i in 0..node_count {
                    if i >= nodes.len() {
                        break;
                    }
                    
                    let sub_id = format!("routing-sub-{}", i);
                    match self.harness.receive(Some(&nodes[i]), &sub_id, 5) {
                        Ok(messages) => {
                            debug!("Node {} received {} messages for topic {}", 
                                  nodes[i], messages.len(), topic);
                        },
                        Err(e) => {
                            debug!("Node {} failed to receive messages: {:?}", nodes[i], e);
                        }
                    }
                }
            }
        }
    }
    
    /// Fuzz topology changes during operation
    fn fuzz_topology_changes(&mut self) {
        let mut rng = thread_rng();
        
        // Create a new network
        self.reset_harness();
        
        // Create 5-8 nodes
        let node_count = rng.gen_range(5..9);
        
        for i in 0..node_count {
            let node_id = format!("node-{}", i);
            let mut identity = generate_node_id();
            identity.node_id = node_id.clone();
            
            let config = generate_topic();
            
            let result = self.harness.add_node(identity, config);
            self.stats.record(&result);
        }
        
        // Start with a mesh topology
        let result = self.harness.build_topology("mesh");
        self.stats.record(&result);
        
        // Set up some subscriptions
        self.fuzz_valid_subscription();
        
        // Publish some initial messages
        self.fuzz_valid_message_publishing();
        
        // Change topology during operation
        if let Some(nodes) = self.get_active_node_ids() {
            // Remove a random node
            if nodes.len() > 3 {
                let node_to_remove = nodes.choose(&mut rng).unwrap();
                let result = self.harness.remove_node(node_to_remove);
                self.stats.record(&result);
                
                info!("Removed node {} during operation", node_to_remove);
            }
            
            // Change to ring topology
            let result = self.harness.build_topology("ring");
            self.stats.record(&result);
            
            info!("Changed topology to ring during operation");
            
            // Publish more messages after topology change
            self.fuzz_valid_message_publishing();
            
            // Add a new node during operation
            let new_node_id = format!("node-new-{}", Uuid::new_v4());
            let mut identity = generate_node_id();
            identity.node_id = new_node_id.clone();
            
            let config = generate_topic();
            
            let result = self.harness.add_node(identity, config);
            self.stats.record(&result);
            
            info!("Added new node {} during operation", new_node_id);
            
            // Connect the new node to an existing node
            if let Some(existing_nodes) = self.get_active_node_ids() {
                if !existing_nodes.is_empty() && existing_nodes.contains(&new_node_id) {
                    let existing = existing_nodes.iter()
                        .filter(|&n| n != &new_node_id)
                        .collect::<Vec<_>>();
                        
                    if !existing.is_empty() {
                        let connect_to = existing.choose(&mut rng).unwrap();
                        let result = self.harness.connect_nodes(&new_node_id, connect_to);
                        self.stats.record(&result);
                        
                        info!("Connected new node {} to {}", new_node_id, connect_to);
                    }
                }
            }
            
            // Publish more messages after adding a node
            self.fuzz_valid_message_publishing();
        }
    }
    
    /// Fuzz high load scenarios
    fn fuzz_high_load(&mut self) {
        let mut rng = thread_rng();
        
        // Create a new network with a mesh topology
        self.reset_harness();
        
        // Create 5-10 nodes
        let node_count = rng.gen_range(5..11);
        
        for i in 0..node_count {
            let node_id = format!("node-{}", i);
            let mut identity = generate_node_id();
            identity.node_id = node_id.clone();
            
            // Use configs with small queue sizes and message sizes
            let mut config = generate_topic();
            config.max_queue_size = rng.gen_range(5..20);
            config.max_message_size = rng.gen_range(1024..4096);
            
            let result = self.harness.add_node(identity, config);
            self.stats.record(&result);
        }
        
        let result = self.harness.build_topology("mesh");
        self.stats.record(&result);
        
        if let Some(nodes) = self.get_active_node_ids() {
            // Create a lot of subscriptions (1-3 per node)
            for node in &nodes {
                let sub_count = rng.gen_range(1..4);
                
                for j in 0..sub_count {
                    let mut subscription = generate_topic();
                    subscription.subscriber_id = node.clone();
                    subscription.subscription_id = format!("high-load-{}-{}", node, j);
                    
                    // Use wildcard topics to increase message routing
                    subscription.topic_pattern = if j % 2 == 0 {
                        "load/#".to_string()
                    } else {
                        "load/+/test".to_string()
                    };
                    
                    let result = self.harness.subscribe(Some(node), subscription);
                    self.stats.record(&result);
                }
            }
            
            // Publish a large number of messages (50-100)
            let msg_count = rng.gen_range(50..101);
            
            for i in 0..msg_count {
                let mut message = generate_topic();
                message.header.message_id = format!("high-load-{}", i);
                
                // Use varying topics that will hit different subscriptions
                message.header.topic = match i % 3 {
                    0 => "load/topic1".to_string(),
                    1 => "load/topic2/test".to_string(),
                    _ => "load/topic3/data".to_string(),
                };
                
                // Use random node for publishing
                let node_id = nodes.choose(&mut rng).unwrap();
                
                let result = self.harness.publish(Some(node_id), message);
                self.stats.record(&result);
                
                // We expect some to be rate limited or fail with queue full
                match &result {
                    P2POperationResult::Error(P2PError::QueueFull) => {
                        // Expected under high load
                    },
                    P2POperationResult::RateLimited => {
                        // Expected under high load
                    },
                    _ => {},
                }
            }
            
            // Check receiving messages after high load
            for node in &nodes {
                let sub_id = format!("high-load-{}-0", node);
                match self.harness.receive(Some(node), &sub_id, 10) {
                    Ok(messages) => {
                        debug!("After high load, node {} received {} messages", 
                              node, messages.len());
                    },
                    Err(e) => {
                        debug!("Node {} failed to receive messages: {:?}", node, e);
                    }
                }
            }
        }
    }
    
    /// Fuzz with high failure rates
    fn fuzz_high_failure_rate(&mut self) {
        let mut rng = thread_rng();
        
        // Create a new network
        self.reset_harness();
        
        // Create 3-5 nodes
        let node_count = rng.gen_range(3..6);
        
        for i in 0..node_count {
            let node_id = format!("node-{}", i);
            let mut identity = generate_node_id();
            identity.node_id = node_id.clone();
            
            let config = generate_topic();
            
            let result = self.harness.add_node(identity, config);
            self.stats.record(&result);
        }
        
        // Set high failure and timeout rates
        self.harness.set_failure_rate(0.7); // 70% failure rate
        self.harness.set_timeout_rate(0.2); // 20% timeout rate
        
        // Set high latency
        self.harness.set_network_latency(2000); // 2 second latency
        
        info!("Testing with high failure rate (70%) and timeout rate (20%)");
        
        // Try to build a topology
        let topologies = ["star", "mesh", "ring", "line"];
        let topology = topologies.choose(&mut rng).unwrap();
        
        let result = self.harness.build_topology(topology);
        self.stats.record(&result);
        
        // Try to set up some subscriptions despite failures
        for _ in 0..10 {
            if let Some(nodes) = self.get_active_node_ids() {
                if nodes.is_empty() {
                    break;
                }
                
                let node_id = nodes.choose(&mut rng).unwrap();
                let subscription = generate_topic();
                
                let result = self.harness.subscribe(Some(node_id), subscription);
                self.stats.record(&result);
            }
        }
        
        // Try to publish messages despite failures
        for _ in 0..20 {
            if let Some(nodes) = self.get_active_node_ids() {
                if nodes.is_empty() {
                    break;
                }
                
                let node_id = nodes.choose(&mut rng).unwrap();
                let message = generate_topic();
                
                let result = self.harness.publish(Some(node_id), message);
                self.stats.record(&result);
            }
        }
        
        // Reset failure rates after test
        self.harness.set_failure_rate(0.05);
        self.harness.set_timeout_rate(0.03);
        self.harness.set_network_latency(50);
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
}

fn main() {
    // Initialize logging
    env_logger::init();
    
    // Initialize coverage tracking
    let coverage_guard = init_coverage_tracking("p2p");
    
    // Initialize fault injection
    let fault_config = FaultConfig::default();
    fault_injection::init_with_config(fault_config);
    
    // Initialize sanitizers
    sanitizer::init();
    
    info!("Starting Form-P2P message queue fuzzer");
    
    // Read configuration from environment variables
    let max_iterations = env::var("FUZZ_ITERATIONS")
        .ok()
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(1000);
        
    let corpus_dir = env::var("FUZZ_CORPUS_DIR").ok();
    
    let seed = env::var("FUZZ_SEED")
        .ok()
        .and_then(|s| s.parse::<u64>().ok());
        
    // Set up RNG with seed if provided
    let rng = if let Some(seed_value) = seed {
        info!("Using seed: {}", seed_value);
        StdRng::seed_from_u64(seed_value)
    } else {
        StdRng::from_rng(thread_rng()).expect("Failed to create RNG")
    };
    
    // Create fuzzer
    let mut fuzzer = P2PFuzzer::new();
    
    if let Some(dir) = corpus_dir {
        fuzzer.set_corpus_dir(&dir);
        info!("Using corpus directory: {}", dir);
    }
    
    // Run fuzzing iterations
    info!("Running {} fuzzing iterations", max_iterations);
    
    for i in 0..max_iterations {
        if i % 100 == 0 {
            info!("Completed {} iterations", i);
        }
        
        // Choose a random strategy
        let strategy = FuzzingStrategy::random();
        debug!("Iteration {}: Using strategy {:?}", i, strategy);
        
        // Run the iteration
        fuzzer.run_iteration(strategy);
    }
    
    // Report statistics
    fuzzer.stats.report();
    
    info!("Fuzzing completed");
} 