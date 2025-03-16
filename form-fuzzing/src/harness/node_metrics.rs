// form-fuzzing/src/harness/node_metrics.rs
//! Harness for fuzzing the node metrics module

use crate::harness::FuzzingHarness;
use form_node_metrics::{
    capabilities::NodeCapabilities,
    capacity::NodeCapacity,
    metrics::NodeMetrics,
    NodeMetricsRequest
};
use std::time::{SystemTime, UNIX_EPOCH};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Response for node metrics operations
#[derive(Debug, Clone)]
pub enum NodeMetricsResponse {
    /// Operation succeeded
    Success,
    /// Operation failed with error message
    Error { error: String },
}

/// Metrics store for the fuzzing harness
struct MetricsStore {
    /// Map of node ID to node capabilities
    capabilities: HashMap<String, NodeCapabilities>,
    /// Map of node ID to node capacity
    capacity: HashMap<String, NodeCapacity>,
    /// Map of node ID to node metrics
    metrics: HashMap<String, NodeMetrics>,
    /// Map of node ID to last heartbeat timestamp
    heartbeats: HashMap<String, i64>,
}

impl MetricsStore {
    /// Create a new metrics store
    fn new() -> Self {
        Self {
            capabilities: HashMap::new(),
            capacity: HashMap::new(),
            metrics: HashMap::new(),
            heartbeats: HashMap::new(),
        }
    }

    /// Clear all metrics
    fn clear(&mut self) {
        self.capabilities.clear();
        self.capacity.clear();
        self.metrics.clear();
        self.heartbeats.clear();
    }
}

/// Harness for fuzzing node metrics operations
pub struct NodeMetricsFuzzHarness {
    metrics_store: Arc<Mutex<MetricsStore>>,
}

impl NodeMetricsFuzzHarness {
    /// Create a new node metrics fuzzing harness
    pub fn new() -> Self {
        Self {
            metrics_store: Arc::new(Mutex::new(MetricsStore::new())),
        }
    }

    /// Generate a current timestamp
    fn generate_timestamp(&self) -> i64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64
    }

    /// Process a node metrics request
    pub fn process_request(&self, request: &NodeMetricsRequest) -> NodeMetricsResponse {
        let mut store = self.metrics_store.lock().unwrap();
        
        match request {
            NodeMetricsRequest::SetInitialMetrics { node_id, node_capabilities, node_capacity } => {
                // Validate the node ID
                if node_id.is_empty() {
                    return NodeMetricsResponse::Error {
                        error: "Node ID cannot be empty".to_string(),
                    };
                }
                
                // Store the initial metrics
                store.capabilities.insert(node_id.clone(), node_capabilities.clone());
                store.capacity.insert(node_id.clone(), node_capacity.clone());
                
                // Store initial heartbeat
                store.heartbeats.insert(node_id.clone(), self.generate_timestamp());
                
                NodeMetricsResponse::Success
            },
            
            NodeMetricsRequest::UpdateMetrics { node_id, node_capacity, node_metrics } => {
                // Validate the node ID
                if node_id.is_empty() {
                    return NodeMetricsResponse::Error {
                        error: "Node ID cannot be empty".to_string(),
                    };
                }
                
                // Check if the node exists
                if !store.capabilities.contains_key(node_id) {
                    return NodeMetricsResponse::Error {
                        error: format!("Node {} not found", node_id),
                    };
                }
                
                // Update the metrics
                store.capacity.insert(node_id.clone(), node_capacity.clone());
                store.metrics.insert(node_id.clone(), node_metrics.clone());
                
                // Update heartbeat
                store.heartbeats.insert(node_id.clone(), self.generate_timestamp());
                
                NodeMetricsResponse::Success
            },
            
            NodeMetricsRequest::Heartbeat { node_id, timestamp } => {
                // Validate the node ID
                if node_id.is_empty() {
                    return NodeMetricsResponse::Error {
                        error: "Node ID cannot be empty".to_string(),
                    };
                }
                
                // Check if the node exists
                if !store.capabilities.contains_key(node_id) {
                    return NodeMetricsResponse::Error {
                        error: format!("Node {} not found", node_id),
                    };
                }
                
                // Validate timestamp
                if *timestamp < 0 {
                    return NodeMetricsResponse::Error {
                        error: "Timestamp cannot be negative".to_string(),
                    };
                }
                
                // Store the heartbeat
                store.heartbeats.insert(node_id.clone(), *timestamp);
                
                NodeMetricsResponse::Success
            },
        }
    }
    
    /// Get metrics for a specific node
    pub fn get_node_metrics(&self, node_id: &str) -> Option<(NodeCapabilities, NodeCapacity, NodeMetrics)> {
        let store = self.metrics_store.lock().unwrap();
        
        if let (Some(capabilities), Some(capacity), Some(metrics)) = (
            store.capabilities.get(node_id),
            store.capacity.get(node_id),
            store.metrics.get(node_id)
        ) {
            Some((capabilities.clone(), capacity.clone(), metrics.clone()))
        } else {
            None
        }
    }
    
    /// Get all node IDs
    pub fn get_all_nodes(&self) -> Vec<String> {
        let store = self.metrics_store.lock().unwrap();
        store.capabilities.keys().cloned().collect()
    }
    
    /// Get last heartbeat for a node
    pub fn get_last_heartbeat(&self, node_id: &str) -> Option<i64> {
        let store = self.metrics_store.lock().unwrap();
        store.heartbeats.get(node_id).cloned()
    }
}

impl FuzzingHarness for NodeMetricsFuzzHarness {
    fn setup(&mut self) {
        // No special setup needed, initialization is done in constructor
    }
    
    fn teardown(&mut self) {
        // No special teardown needed
    }
    
    fn reset(&mut self) {
        let mut store = self.metrics_store.lock().unwrap();
        store.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_initial_metrics() {
        let harness = NodeMetricsFuzzHarness::new();
        
        // Create node ID
        let node_id = "test-node-1".to_string();
        
        // Create test objects
        let capabilities = NodeCapabilities::default();
        let capacity = NodeCapacity::default();
        
        // Create request
        let request = NodeMetricsRequest::SetInitialMetrics {
            node_id: node_id.clone(),
            node_capabilities: capabilities.clone(),
            node_capacity: capacity.clone(),
        };
        
        // Process the request
        let response = harness.process_request(&request);
        
        // Verify success
        match response {
            NodeMetricsResponse::Success => {},
            _ => panic!("Expected success response"),
        }
        
        // Verify the node is in the list
        let nodes = harness.get_all_nodes();
        assert!(nodes.contains(&node_id));
        
        // Verify the heartbeat was recorded
        let heartbeat = harness.get_last_heartbeat(&node_id);
        assert!(heartbeat.is_some());
    }
    
    #[test]
    fn test_update_metrics() {
        let harness = NodeMetricsFuzzHarness::new();
        
        // Create node ID
        let node_id = "test-node-2".to_string();
        
        // Create test objects
        let capabilities = NodeCapabilities::default();
        let capacity = NodeCapacity::default();
        
        // Create initial request
        let initial_request = NodeMetricsRequest::SetInitialMetrics {
            node_id: node_id.clone(),
            node_capabilities: capabilities.clone(),
            node_capacity: capacity.clone(),
        };
        
        // Process the initial request
        harness.process_request(&initial_request);
        
        // Create updated metrics
        let updated_metrics = NodeMetrics::default();
        
        // Create update request
        let update_request = NodeMetricsRequest::UpdateMetrics {
            node_id: node_id.clone(),
            node_capacity: capacity.clone(),
            node_metrics: updated_metrics.clone(),
        };
        
        // Process the update request
        let response = harness.process_request(&update_request);
        
        // Verify success
        match response {
            NodeMetricsResponse::Success => {},
            _ => panic!("Expected success response"),
        }
    }
    
    #[test]
    fn test_heartbeat() {
        let harness = NodeMetricsFuzzHarness::new();
        
        // Create node ID
        let node_id = "test-node-3".to_string();
        
        // Create test objects
        let capabilities = NodeCapabilities::default();
        let capacity = NodeCapacity::default();
        
        // Create initial request
        let initial_request = NodeMetricsRequest::SetInitialMetrics {
            node_id: node_id.clone(),
            node_capabilities: capabilities.clone(),
            node_capacity: capacity.clone(),
        };
        
        // Process the initial request
        harness.process_request(&initial_request);
        
        // Create heartbeat request
        let heartbeat_request = NodeMetricsRequest::Heartbeat {
            node_id: node_id.clone(),
            timestamp: 12345,
        };
        
        // Process the heartbeat request
        let response = harness.process_request(&heartbeat_request);
        
        // Verify success
        match response {
            NodeMetricsResponse::Success => {},
            _ => panic!("Expected success response"),
        }
        
        // Verify the heartbeat was recorded
        let heartbeat = harness.get_last_heartbeat(&node_id);
        assert_eq!(heartbeat, Some(12345));
    }
    
    #[test]
    fn test_error_handling() {
        let harness = NodeMetricsFuzzHarness::new();
        
        // Test empty node ID
        let request = NodeMetricsRequest::SetInitialMetrics {
            node_id: "".to_string(),
            node_capabilities: NodeCapabilities::default(),
            node_capacity: NodeCapacity::default(),
        };
        
        let response = harness.process_request(&request);
        
        match response {
            NodeMetricsResponse::Error { error } => {
                assert!(error.contains("empty"));
            },
            _ => panic!("Expected error response"),
        }
        
        // Test update for non-existent node
        let update_request = NodeMetricsRequest::UpdateMetrics {
            node_id: "non-existent".to_string(),
            node_capacity: NodeCapacity::default(),
            node_metrics: NodeMetrics::default(),
        };
        
        let response = harness.process_request(&update_request);
        
        match response {
            NodeMetricsResponse::Error { error } => {
                assert!(error.contains("not found"));
            },
            _ => panic!("Expected error response"),
        }
        
        // Test negative timestamp
        let node_id = "test-node-4".to_string();
        let init_request = NodeMetricsRequest::SetInitialMetrics {
            node_id: node_id.clone(),
            node_capabilities: NodeCapabilities::default(),
            node_capacity: NodeCapacity::default(),
        };
        
        harness.process_request(&init_request);
        
        let heartbeat_request = NodeMetricsRequest::Heartbeat {
            node_id,
            timestamp: -1,
        };
        
        let response = harness.process_request(&heartbeat_request);
        
        match response {
            NodeMetricsResponse::Error { error } => {
                assert!(error.contains("negative"));
            },
            _ => panic!("Expected error response"),
        }
    }
} 