// form-fuzzing/src/harness/vm_metrics.rs
//! Fuzzing harness for VM metrics

use super::FuzzingHarness;
use std::fmt;
use std::sync::{Arc, Mutex};
use form_vm_metrics::{
    system::SystemMetrics,
    cpu::CpuMetrics,
    mem::MemoryMetrics, 
    disk::DiskMetrics,
    gpu::GpuMetrics,
    network::{NetworkMetrics, NetworkInterfaceMetrics},
    load::LoadMetrics
};

/// Response from metrics operations
#[derive(Debug, Clone)]
pub enum MetricsResponse {
    /// Operation succeeded
    Success,
    /// Operation failed with error message
    Error(String),
}

impl fmt::Display for MetricsResponse {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MetricsResponse::Success => write!(f, "Success"),
            MetricsResponse::Error(msg) => write!(f, "Error: {}", msg),
        }
    }
}

/// Mock metrics publisher for testing
pub struct MockMetricsPublisher {
    /// Published metrics
    published: Arc<Mutex<Vec<SystemMetrics>>>,
    /// Whether to fail publishing
    should_fail: bool,
    /// Error message to return when failing
    error_message: Option<String>,
}

impl MockMetricsPublisher {
    /// Create a new mock publisher
    pub fn new() -> Self {
        Self {
            published: Arc::new(Mutex::new(Vec::new())),
            should_fail: false,
            error_message: None,
        }
    }
    
    /// Get all published metrics
    pub fn get_published(&self) -> Vec<SystemMetrics> {
        self.published.lock().unwrap().clone()
    }
    
    /// Set whether the publisher should fail
    pub fn set_error(&mut self, error: Option<String>) {
        self.should_fail = error.is_some();
        self.error_message = error;
    }
    
    /// Publish metrics
    pub fn publish(&self, metrics: SystemMetrics) -> Result<(), String> {
        if self.should_fail {
            Err(self.error_message.clone().unwrap_or_else(|| "Unknown error".to_string()))
        } else {
            let mut published = self.published.lock().unwrap();
            published.push(metrics);
            Ok(())
        }
    }
}

/// Fuzzing harness for VM metrics
pub struct VmMetricsFuzzHarness {
    publisher: MockMetricsPublisher,
}

impl VmMetricsFuzzHarness {
    /// Create a new harness
    pub fn new() -> Self {
        Self {
            publisher: MockMetricsPublisher::new(),
        }
    }
    
    /// Create valid metrics for testing
    pub fn create_valid_metrics(&self) -> SystemMetrics {
        let timestamp = chrono::Utc::now().timestamp();
        let instance_id = Some(uuid::Uuid::new_v4().to_string());
        let account_id = Some(uuid::Uuid::new_v4().to_string());
        
        // Create CPU metrics
        let cpu = CpuMetrics::default();
        
        // Create memory metrics
        let memory = MemoryMetrics::default();
        
        // Create disk metrics
        let disks = vec![DiskMetrics {
            device_name: "/dev/sda1".to_string(),
            reads_completed: 1000,
            reads_merged: 500,
            sectors_read: 2000,
            time_reading: 100,
            writes_completed: 1500,
            writes_merged: 700,
            sectors_written: 3000,
            time_writing: 150,
            io_in_progress: 5,
            time_doing_io: 250,
            weighted_time_doing_io: 300,
        }];
        
        // Create network metrics
        let network = NetworkMetrics {
            interfaces: vec![NetworkInterfaceMetrics {
                name: "eth0".to_string(),
                bytes_sent: 20000,
                bytes_received: 10000,
                packets_sent: 200,
                packets_received: 100,
                errors_in: 0,
                errors_out: 0,
                drops_in: 0,
                drops_out: 0,
                speed: 1000000000, // 1 Gbps
            }],
        };
        
        // Create GPU metrics
        let gpus = vec![GpuMetrics {
            index: 0,
            model: "NVIDIA GeForce RTX 3080".to_string(),
            utilization_bps: 7000, // 70% in basis points
            memory_usage_bps: 6000, // 60% in basis points
            temperature_deci_c: 650, // 65.0°C
            power_draw_deci_w: 2500, // 250.0W
        }];
        
        // Create load metrics
        let load = LoadMetrics {
            load1: 100, // 1.00 load avg
            load5: 150, // 1.50 load avg
            load15: 200, // 2.00 load avg
        };
        
        SystemMetrics {
            timestamp,
            instance_id,
            account_id,
            cpu,
            memory,
            disks,
            network,
            gpus,
            load,
        }
    }
    
    /// Validate metrics
    pub fn validate_metrics(&self, metrics: &SystemMetrics) -> Result<(), String> {
        // Validate timestamp
        if metrics.timestamp <= 0 {
            return Err("Timestamp must be positive".to_string());
        }
        
        // Validate IDs (if provided)
        if let Some(id) = &metrics.instance_id {
            if id.is_empty() {
                return Err("Instance ID cannot be empty if provided".to_string());
            }
        }
        
        if let Some(id) = &metrics.account_id {
            if id.is_empty() {
                return Err("Account ID cannot be empty if provided".to_string());
            }
        }
        
        // Validate disks
        if metrics.disks.is_empty() {
            return Err("At least one disk metric is required".to_string());
        }
        
        for disk in &metrics.disks {
            if disk.device_name.is_empty() {
                return Err("Disk device name cannot be empty".to_string());
            }
        }
        
        Ok(())
    }
    
    /// Publish metrics
    pub fn publish_metrics(&self, metrics: SystemMetrics) -> MetricsResponse {
        // Validate the metrics
        if let Err(e) = self.validate_metrics(&metrics) {
            return MetricsResponse::Error(e);
        }
        
        // Publish the metrics
        match self.publisher.publish(metrics) {
            Ok(_) => MetricsResponse::Success,
            Err(e) => MetricsResponse::Error(e),
        }
    }
    
    /// Set whether the publisher should fail
    pub fn set_error(&mut self, error: Option<String>) {
        self.publisher.set_error(error);
    }
    
    /// Get all published metrics
    pub fn get_published_metrics(&self) -> Vec<SystemMetrics> {
        self.publisher.get_published()
    }
}

impl FuzzingHarness for VmMetricsFuzzHarness {
    fn setup(&mut self) {
        // No setup required
    }
    
    fn teardown(&mut self) {
        // No teardown required
    }
    
    fn reset(&mut self) {
        // Clear published metrics
        self.publisher = MockMetricsPublisher::new();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_create_valid_metrics() {
        let harness = VmMetricsFuzzHarness::new();
        let metrics = harness.create_valid_metrics();
        
        // Validate the metrics
        assert!(harness.validate_metrics(&metrics).is_ok());
    }
    
    #[test]
    fn test_publish_metrics() {
        let harness = VmMetricsFuzzHarness::new();
        let metrics = harness.create_valid_metrics();
        
        // Publish the metrics
        let response = harness.publish_metrics(metrics);
        assert!(matches!(response, MetricsResponse::Success));
        
        // Check published metrics
        let published = harness.get_published_metrics();
        assert_eq!(published.len(), 1);
    }
    
    #[test]
    fn test_error_handling() {
        let mut harness = VmMetricsFuzzHarness::new();
        
        // Set error
        harness.set_error(Some("Test error".to_string()));
        
        // Try to publish metrics
        let metrics = harness.create_valid_metrics();
        let response = harness.publish_metrics(metrics);
        
        // Check that it failed
        assert!(matches!(response, MetricsResponse::Error(_)));
    }
    
    #[test]
    fn test_validation() {
        let harness = VmMetricsFuzzHarness::new();
        
        // Create metrics with invalid timestamp
        let mut metrics = harness.create_valid_metrics();
        metrics.timestamp = -1;
        assert!(harness.validate_metrics(&metrics).is_err());
        
        // Create metrics with invalid instance ID
        let mut metrics = harness.create_valid_metrics();
        metrics.instance_id = Some("".to_string());
        assert!(harness.validate_metrics(&metrics).is_err());
        
        // Create metrics with no disks
        let mut metrics = harness.create_valid_metrics();
        metrics.disks.clear();
        assert!(harness.validate_metrics(&metrics).is_err());
    }
} 