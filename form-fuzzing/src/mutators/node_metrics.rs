// form-fuzzing/src/mutators/node_metrics.rs
//! Mutators for node metrics fuzzing

use crate::mutators::Mutator;
use form_node_metrics::{
    capabilities::NodeCapabilities,
    capacity::NodeCapacity,
    metrics::NodeMetrics,
    NodeMetricsRequest
};
use rand::{Rng, seq::SliceRandom};

/// Mutator for NodeCapabilities
pub struct NodeCapabilitiesMutator;

impl NodeCapabilitiesMutator {
    /// Create a new NodeCapabilitiesMutator
    pub fn new() -> Self {
        Self
    }
}

impl Mutator<NodeCapabilities> for NodeCapabilitiesMutator {
    fn mutate(&self, capabilities: &mut NodeCapabilities) {
        let mut rng = rand::thread_rng();
        
        // Choose a random mutation
        match rng.gen_range(0..6) {
            0 => {
                // Modify CPU model
                let cpu_models = [
                    "Intel(R) Core(TM) i7-9700K", 
                    "AMD Ryzen 9 3900X", 
                    "Intel(R) Xeon(R) Gold 6142M", 
                    "AMD EPYC 7742",
                    ""  // Empty string for potential error case
                ];
                capabilities.cpu_model = cpu_models.choose(&mut rng).unwrap().to_string();
            }
            1 => {
                // Modify CPU cores
                match rng.gen_range(0..3) {
                    0 => capabilities.cpu_cores = 0,  // Zero cores (error case)
                    1 => capabilities.cpu_cores = rng.gen_range(1..128),  // Valid cores
                    _ => capabilities.cpu_cores = 10000,  // Unrealistically high (edge case)
                }
            }
            2 => {
                // Modify total memory
                match rng.gen_range(0..3) {
                    0 => capabilities.total_memory = 0,  // Zero memory (error case)
                    1 => capabilities.total_memory = rng.gen_range(1..1024) * 1024 * 1024,  // Valid memory (MiB to bytes)
                    _ => capabilities.total_memory = u64::MAX / 2,  // Very high memory (edge case)
                }
            }
            3 => {
                // Modify total storage
                match rng.gen_range(0..3) {
                    0 => capabilities.total_storage = 0,  // Zero storage (error case)
                    1 => capabilities.total_storage = rng.gen_range(1..4096) * 1024 * 1024 * 1024,  // Valid storage (GiB to bytes)
                    _ => capabilities.total_storage = u64::MAX / 2,  // Very high storage (edge case)
                }
            }
            4 => {
                // Modify GPU models
                match rng.gen_range(0..3) {
                    0 => capabilities.gpu_models.clear(),  // No GPUs
                    1 => {
                        // Add a single GPU
                        capabilities.gpu_models.clear();
                        capabilities.gpu_models.push(form_node_metrics::capabilities::GpuInfo {
                            vendor: "NVIDIA".to_string(),
                            model: Some("GeForce RTX 3080".to_string()),
                            count: 1,
                            total_memory_bytes: 10 * 1024 * 1024 * 1024,  // 10 GiB
                            pci_bus_id: Some("0000:01:00.0".to_string()),
                            cuda_enabled: Some((8, 6)),  // CUDA 8.6
                            driver_version: Some("460.32.03".to_string()),
                        });
                    }
                    _ => {
                        // Add multiple GPUs (potentially with errors)
                        capabilities.gpu_models.clear();
                        
                        // First GPU (valid)
                        capabilities.gpu_models.push(form_node_metrics::capabilities::GpuInfo {
                            vendor: "NVIDIA".to_string(),
                            model: Some("GeForce RTX 3080".to_string()),
                            count: 1,
                            total_memory_bytes: 10 * 1024 * 1024 * 1024,  // 10 GiB
                            pci_bus_id: Some("0000:01:00.0".to_string()),
                            cuda_enabled: Some((8, 6)),  // CUDA 8.6
                            driver_version: Some("460.32.03".to_string()),
                        });
                        
                        // Second GPU (potentially invalid)
                        capabilities.gpu_models.push(form_node_metrics::capabilities::GpuInfo {
                            vendor: "AMD".to_string(),
                            model: None,  // Missing model
                            count: 0,  // Invalid count
                            total_memory_bytes: 0,  // Invalid memory
                            pci_bus_id: None,
                            cuda_enabled: None,
                            driver_version: None,
                        });
                    }
                }
            }
            5 => {
                // Modify network interfaces
                match rng.gen_range(0..3) {
                    0 => capabilities.network_interfaces.clear(),  // No interfaces
                    1 => {
                        // Add a single interface
                        capabilities.network_interfaces.clear();
                        capabilities.network_interfaces.push(form_node_metrics::capabilities::NetworkCapability {
                            interface_name: "eth0".to_string(),
                            link_speed_mbps: Some(1000),
                            max_bandwidth: Some(1000),
                            ipv4_addresses: vec!["192.168.1.100".to_string()],
                            ipv6_addresses: vec!["fe80::1".to_string()],
                            is_active: true,
                        });
                    }
                    _ => {
                        // Add multiple interfaces (potentially with errors)
                        capabilities.network_interfaces.clear();
                        
                        // First interface (valid)
                        capabilities.network_interfaces.push(form_node_metrics::capabilities::NetworkCapability {
                            interface_name: "eth0".to_string(),
                            link_speed_mbps: Some(1000),
                            max_bandwidth: Some(1000),
                            ipv4_addresses: vec!["192.168.1.100".to_string()],
                            ipv6_addresses: vec!["fe80::1".to_string()],
                            is_active: true,
                        });
                        
                        // Second interface (potentially invalid)
                        capabilities.network_interfaces.push(form_node_metrics::capabilities::NetworkCapability {
                            interface_name: "".to_string(),  // Empty name
                            link_speed_mbps: None,
                            max_bandwidth: None,
                            ipv4_addresses: vec![],  // No IPs
                            ipv6_addresses: vec![],
                            is_active: false,
                        });
                    }
                }
            }
            _ => {}  // No mutation
        }
    }
}

/// Mutator for NodeCapacity
pub struct NodeCapacityMutator;

impl NodeCapacityMutator {
    /// Create a new NodeCapacityMutator
    pub fn new() -> Self {
        Self
    }
}

impl Mutator<NodeCapacity> for NodeCapacityMutator {
    fn mutate(&self, capacity: &mut NodeCapacity) {
        let mut rng = rand::thread_rng();
        
        // Choose a random mutation
        match rng.gen_range(0..8) {
            0 => {
                // Modify CPU cores
                match rng.gen_range(0..3) {
                    0 => capacity.cpu_total_cores = 0,  // Zero cores (error case)
                    1 => capacity.cpu_total_cores = rng.gen_range(1..128),  // Valid cores
                    _ => capacity.cpu_total_cores = 10000,  // Unrealistically high (edge case)
                }
            }
            1 => {
                // Modify available CPU cores
                match rng.gen_range(0..4) {
                    0 => capacity.cpu_available_cores = 0,  // Zero available
                    1 => capacity.cpu_available_cores = rng.gen_range(1..1000),  // Some available
                    2 => capacity.cpu_available_cores = -1,  // Negative (error case)
                    _ => {
                        // Make available cores more than total (error case)
                        if capacity.cpu_total_cores > 0 {
                            capacity.cpu_available_cores = (capacity.cpu_total_cores as i64) * 10;
                        } else {
                            capacity.cpu_available_cores = 1000;
                        }
                    }
                }
            }
            2 => {
                // Modify memory total
                match rng.gen_range(0..3) {
                    0 => capacity.memory_total_bytes = 0,  // Zero memory (error case)
                    1 => capacity.memory_total_bytes = rng.gen_range(1..1024) * 1024 * 1024,  // Valid memory (MiB to bytes)
                    _ => capacity.memory_total_bytes = u64::MAX / 2,  // Very high memory (edge case)
                }
            }
            3 => {
                // Modify memory available
                match rng.gen_range(0..3) {
                    0 => capacity.memory_available_bytes = 0,  // No available memory
                    1 => {
                        // Valid available memory
                        if capacity.memory_total_bytes > 0 {
                            capacity.memory_available_bytes = rng.gen_range(0..capacity.memory_total_bytes);
                        } else {
                            capacity.memory_available_bytes = rng.gen_range(1..1024) * 1024 * 1024;
                        }
                    }
                    _ => {
                        // More available than total (error case)
                        if capacity.memory_total_bytes > 0 {
                            capacity.memory_available_bytes = capacity.memory_total_bytes * 2;
                        } else {
                            capacity.memory_available_bytes = u64::MAX / 2;
                        }
                    }
                }
            }
            4 => {
                // Modify storage total
                match rng.gen_range(0..3) {
                    0 => capacity.storage_total_bytes = 0,  // Zero storage (error case)
                    1 => capacity.storage_total_bytes = rng.gen_range(1..4096) * 1024 * 1024 * 1024,  // Valid storage (GiB to bytes)
                    _ => capacity.storage_total_bytes = u64::MAX / 2,  // Very high storage (edge case)
                }
            }
            5 => {
                // Modify storage available
                match rng.gen_range(0..3) {
                    0 => capacity.storage_available_bytes = 0,  // No available storage
                    1 => {
                        // Valid available storage
                        if capacity.storage_total_bytes > 0 {
                            capacity.storage_available_bytes = rng.gen_range(0..capacity.storage_total_bytes);
                        } else {
                            capacity.storage_available_bytes = rng.gen_range(1..4096) * 1024 * 1024 * 1024;
                        }
                    }
                    _ => {
                        // More available than total (error case)
                        if capacity.storage_total_bytes > 0 {
                            capacity.storage_available_bytes = capacity.storage_total_bytes * 2;
                        } else {
                            capacity.storage_available_bytes = u64::MAX / 2;
                        }
                    }
                }
            }
            6 => {
                // Modify GPU memory total
                match rng.gen_range(0..3) {
                    0 => capacity.gpu_total_memory_bytes = 0,  // Zero GPU memory
                    1 => capacity.gpu_total_memory_bytes = rng.gen_range(1..32) * 1024 * 1024 * 1024,  // Valid GPU memory (GiB to bytes)
                    _ => capacity.gpu_total_memory_bytes = u64::MAX / 2,  // Very high GPU memory
                }
            }
            7 => {
                // Modify GPU memory available
                match rng.gen_range(0..3) {
                    0 => capacity.gpu_available_memory_bytes = 0,  // No available GPU memory
                    1 => {
                        // Valid available GPU memory
                        if capacity.gpu_total_memory_bytes > 0 {
                            capacity.gpu_available_memory_bytes = rng.gen_range(0..capacity.gpu_total_memory_bytes);
                        } else {
                            capacity.gpu_available_memory_bytes = rng.gen_range(1..32) * 1024 * 1024 * 1024;
                        }
                    }
                    _ => {
                        // More available than total (error case)
                        if capacity.gpu_total_memory_bytes > 0 {
                            capacity.gpu_available_memory_bytes = capacity.gpu_total_memory_bytes * 2;
                        } else {
                            capacity.gpu_available_memory_bytes = u64::MAX / 2;
                        }
                    }
                }
            }
            _ => {}  // No mutation
        }
    }
}

/// Mutator for NodeMetrics
pub struct NodeMetricsMutator;

impl NodeMetricsMutator {
    /// Create a new NodeMetricsMutator
    pub fn new() -> Self {
        Self
    }
}

impl Mutator<NodeMetrics> for NodeMetricsMutator {
    fn mutate(&self, metrics: &mut NodeMetrics) {
        let mut rng = rand::thread_rng();
        
        // Choose a random mutation
        match rng.gen_range(0..6) {
            0 => {
                // Modify load averages
                match rng.gen_range(0..3) {
                    0 => {
                        // Zero load
                        metrics.load_avg_1 = 0;
                        metrics.load_avg_5 = 0;
                        metrics.load_avg_15 = 0;
                    }
                    1 => {
                        // Valid load
                        metrics.load_avg_1 = rng.gen_range(0..10000);  // 0-10.0 (scaled by 1000)
                        metrics.load_avg_5 = rng.gen_range(0..10000);
                        metrics.load_avg_15 = rng.gen_range(0..10000);
                    }
                    _ => {
                        // Invalid load (negative)
                        metrics.load_avg_1 = -1;
                        metrics.load_avg_5 = -1;
                        metrics.load_avg_15 = -1;
                    }
                }
            }
            1 => {
                // Modify process count
                match rng.gen_range(0..3) {
                    0 => metrics.process_count = 0,  // No processes
                    1 => metrics.process_count = rng.gen_range(1..1000),  // Normal range
                    _ => metrics.process_count = 100000,  // Very high (edge case)
                }
            }
            2 => {
                // Modify disk I/O
                match rng.gen_range(0..3) {
                    0 => {
                        // Zero I/O
                        metrics.disk_read_bytes_per_sec = 0;
                        metrics.disk_write_bytes_per_sec = 0;
                    }
                    1 => {
                        // Normal I/O
                        metrics.disk_read_bytes_per_sec = rng.gen_range(1..1024) * 1024 * 1024;  // 1 MB/s to 1 GB/s
                        metrics.disk_write_bytes_per_sec = rng.gen_range(1..1024) * 1024 * 1024;
                    }
                    _ => {
                        // Very high I/O (edge case)
                        metrics.disk_read_bytes_per_sec = u64::MAX / 2;
                        metrics.disk_write_bytes_per_sec = u64::MAX / 2;
                    }
                }
            }
            3 => {
                // Modify network I/O
                match rng.gen_range(0..3) {
                    0 => {
                        // Zero I/O
                        metrics.network_in_bytes_per_sec = 0;
                        metrics.network_out_bytes_per_sec = 0;
                    }
                    1 => {
                        // Normal I/O
                        metrics.network_in_bytes_per_sec = rng.gen_range(1..1024) * 1024 * 1024;  // 1 MB/s to 1 GB/s
                        metrics.network_out_bytes_per_sec = rng.gen_range(1..1024) * 1024 * 1024;
                    }
                    _ => {
                        // Very high I/O (edge case)
                        metrics.network_in_bytes_per_sec = u64::MAX / 2;
                        metrics.network_out_bytes_per_sec = u64::MAX / 2;
                    }
                }
            }
            4 => {
                // Modify temperatures
                match rng.gen_range(0..4) {
                    0 => {
                        // No temperature data
                        metrics.cpu_temperature = None;
                        metrics.gpu_temperature = None;
                    }
                    1 => {
                        // Normal temperatures
                        metrics.cpu_temperature = Some(rng.gen_range(3000..8000));  // 30-80°C (x100)
                        metrics.gpu_temperature = Some(rng.gen_range(3000..9000));  // 30-90°C (x100)
                    }
                    2 => {
                        // High temperatures (edge case)
                        metrics.cpu_temperature = Some(10000);  // 100°C (x100)
                        metrics.gpu_temperature = Some(15000);  // 150°C (x100)
                    }
                    _ => {
                        // Negative temperatures (error case)
                        metrics.cpu_temperature = Some(u32::MAX);
                        metrics.gpu_temperature = Some(u32::MAX);
                    }
                }
            }
            5 => {
                // Modify power usage
                match rng.gen_range(0..3) {
                    0 => metrics.power_usage_watts = None,  // No power data
                    1 => metrics.power_usage_watts = Some(rng.gen_range(50..500)),  // Normal range
                    _ => metrics.power_usage_watts = Some(u32::MAX),  // Very high (edge case)
                }
            }
            _ => {}  // No mutation
        }
    }
}

/// Mutator for NodeMetricsRequest
pub struct NodeMetricsRequestMutator {
    pub capabilities_mutator: NodeCapabilitiesMutator,
    pub capacity_mutator: NodeCapacityMutator,
    pub metrics_mutator: NodeMetricsMutator,
}

impl NodeMetricsRequestMutator {
    /// Create a new NodeMetricsRequestMutator
    pub fn new() -> Self {
        Self {
            capabilities_mutator: NodeCapabilitiesMutator::new(),
            capacity_mutator: NodeCapacityMutator::new(),
            metrics_mutator: NodeMetricsMutator::new(),
        }
    }
}

impl Mutator<NodeMetricsRequest> for NodeMetricsRequestMutator {
    fn mutate(&self, request: &mut NodeMetricsRequest) {
        let mut rng = rand::thread_rng();
        
        // First, potentially change the request type
        if rng.gen_bool(0.05) {  // 5% chance to change type
            let new_type = rng.gen_range(0..3);
            let node_id = match request {
                NodeMetricsRequest::SetInitialMetrics { node_id, .. } => node_id.clone(),
                NodeMetricsRequest::UpdateMetrics { node_id, .. } => node_id.clone(),
                NodeMetricsRequest::Heartbeat { node_id, .. } => node_id.clone(),
            };
            
            *request = match new_type {
                0 => NodeMetricsRequest::SetInitialMetrics {
                    node_id,
                    node_capabilities: NodeCapabilities::default(),
                    node_capacity: NodeCapacity::default(),
                },
                1 => NodeMetricsRequest::UpdateMetrics {
                    node_id,
                    node_capacity: NodeCapacity::default(),
                    node_metrics: NodeMetrics::default(),
                },
                _ => NodeMetricsRequest::Heartbeat {
                    node_id,
                    timestamp: chrono::Utc::now().timestamp(),
                },
            };
        }
        
        // Now mutate the node_id
        match request {
            NodeMetricsRequest::SetInitialMetrics { node_id, .. } |
            NodeMetricsRequest::UpdateMetrics { node_id, .. } |
            NodeMetricsRequest::Heartbeat { node_id, .. } => {
                if rng.gen_bool(0.1) {  // 10% chance to mutate node_id
                    match rng.gen_range(0..5) {
                        0 => *node_id = "".to_string(),  // Empty node_id (error case)
                        1 => *node_id = "node-".to_string() + &rng.gen_range(1..1000).to_string(),  // Valid node_id
                        2 => *node_id = "X".repeat(rng.gen_range(1000..10000)),  // Very long node_id (edge case)
                        3 => *node_id = format!("invalid-id-{}-with-special-chars-!@#$%^&*()", rng.gen_range(1..1000)),  // Special characters
                        _ => *node_id = "node_".to_string() + &uuid::Uuid::new_v4().to_string(),  // UUID-based node_id
                    }
                }
            }
        }
        
        // Now mutate the specific fields based on request type
        match request {
            NodeMetricsRequest::SetInitialMetrics { node_capabilities, node_capacity, .. } => {
                // First, potentially mutate capabilities
                if rng.gen_bool(0.3) {  // 30% chance
                    self.capabilities_mutator.mutate(node_capabilities);
                }
                
                // Now, potentially mutate capacity
                if rng.gen_bool(0.3) {  // 30% chance
                    self.capacity_mutator.mutate(node_capacity);
                }
            }
            NodeMetricsRequest::UpdateMetrics { node_capacity, node_metrics, .. } => {
                // First, potentially mutate capacity
                if rng.gen_bool(0.3) {  // 30% chance
                    self.capacity_mutator.mutate(node_capacity);
                }
                
                // Now, potentially mutate metrics
                if rng.gen_bool(0.3) {  // 30% chance
                    self.metrics_mutator.mutate(node_metrics);
                }
            }
            NodeMetricsRequest::Heartbeat { timestamp, .. } => {
                // Potentially mutate timestamp
                if rng.gen_bool(0.3) {  // 30% chance
                    match rng.gen_range(0..4) {
                        0 => *timestamp = 0,  // Zero timestamp
                        1 => *timestamp = -1,  // Negative timestamp (error case)
                        2 => *timestamp = i64::MAX,  // Very far future
                        _ => *timestamp = chrono::Utc::now().timestamp() - rng.gen_range(0..10000000),  // Past timestamp
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_node_capabilities_mutator() {
        let mutator = NodeCapabilitiesMutator::new();
        let mut capabilities = NodeCapabilities::default();
        mutator.mutate(&mut capabilities);
        // Just verifying it runs without panicking
    }
    
    #[test]
    fn test_node_capacity_mutator() {
        let mutator = NodeCapacityMutator::new();
        let mut capacity = NodeCapacity::default();
        mutator.mutate(&mut capacity);
        // Just verifying it runs without panicking
    }
    
    #[test]
    fn test_node_metrics_mutator() {
        let mutator = NodeMetricsMutator::new();
        let mut metrics = NodeMetrics::default();
        mutator.mutate(&mut metrics);
        // Just verifying it runs without panicking
    }
    
    #[test]
    fn test_node_metrics_request_mutator() {
        let mutator = NodeMetricsRequestMutator::new();
        
        // Test SetInitialMetrics
        let mut request = NodeMetricsRequest::SetInitialMetrics {
            node_id: "test-node".to_string(),
            node_capabilities: NodeCapabilities::default(),
            node_capacity: NodeCapacity::default(),
        };
        mutator.mutate(&mut request);
        
        // Test UpdateMetrics
        let mut request = NodeMetricsRequest::UpdateMetrics {
            node_id: "test-node".to_string(),
            node_capacity: NodeCapacity::default(),
            node_metrics: NodeMetrics::default(),
        };
        mutator.mutate(&mut request);
        
        // Test Heartbeat
        let mut request = NodeMetricsRequest::Heartbeat {
            node_id: "test-node".to_string(),
            timestamp: 12345,
        };
        mutator.mutate(&mut request);
        
        // Just verifying they run without panicking
    }
} 