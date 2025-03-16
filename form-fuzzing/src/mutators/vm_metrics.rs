// form-fuzzing/src/mutators/vm_metrics.rs
//! Mutator for VM metrics

use crate::mutators::Mutator;
use form_vm_metrics::{
    system::SystemMetrics,
    disk::DiskMetrics,
    network::{NetworkMetrics, NetworkInterfaceMetrics},
    gpu::GpuMetrics
};
use rand::{Rng, thread_rng};
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

/// Mutator for VM metrics
pub struct VmMetricsMutator;

impl VmMetricsMutator {
    /// Create a new VM metrics mutator
    pub fn new() -> Self {
        Self
    }
}

impl Mutator<SystemMetrics> for VmMetricsMutator {
    fn mutate(&self, metrics: &mut SystemMetrics) {
        let mut rng = thread_rng();
        
        // Randomly decide what to mutate
        let mutation_type = rng.gen_range(0..8);
        
        match mutation_type {
            0 => {
                // Mutate timestamp
                match rng.gen_range(0..3) {
                    0 => {
                        // Set to current time
                        metrics.timestamp = SystemTime::now()
                            .duration_since(UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_secs() as i64;
                    }
                    1 => {
                        // Set to a past time
                        metrics.timestamp = SystemTime::now()
                            .duration_since(UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_secs() as i64 - rng.gen_range(3600..86400); // 1 hour to 1 day ago
                    }
                    _ => {
                        // Set to a very old time
                        metrics.timestamp = SystemTime::now()
                            .duration_since(UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_secs() as i64 - rng.gen_range(86400..31536000); // 1 day to 1 year ago
                    }
                }
            }
            1 => {
                // Mutate instance_id
                match rng.gen_range(0..4) {
                    0 => {
                        // Remove instance ID
                        metrics.instance_id = None;
                    }
                    1 => {
                        // Set to empty string
                        metrics.instance_id = Some("".to_string());
                    }
                    2 => {
                        // Set to valid UUID
                        metrics.instance_id = Some(Uuid::new_v4().to_string());
                    }
                    _ => {
                        // Set to invalid value
                        metrics.instance_id = Some("invalid-instance-id".to_string());
                    }
                }
            }
            2 => {
                // Mutate account_id
                match rng.gen_range(0..4) {
                    0 => {
                        // Remove account ID
                        metrics.account_id = None;
                    }
                    1 => {
                        // Set to empty string
                        metrics.account_id = Some("".to_string());
                    }
                    2 => {
                        // Set to valid UUID
                        metrics.account_id = Some(Uuid::new_v4().to_string());
                    }
                    _ => {
                        // Set to invalid value
                        metrics.account_id = Some("invalid-account-id".to_string());
                    }
                }
            }
            3 => {
                // Mutate disks
                match rng.gen_range(0..5) {
                    0 => {
                        // Clear all disks
                        metrics.disks.clear();
                    }
                    1 => {
                        // Add a new disk
                        metrics.disks.push(DiskMetrics {
                            device_name: format!("/dev/sd{}", ['a', 'b', 'c', 'd', 'e', 'f'][rng.gen_range(0..6)]),
                            reads_completed: rng.gen(),
                            reads_merged: rng.gen(),
                            sectors_read: rng.gen(),
                            time_reading: rng.gen(),
                            writes_completed: rng.gen(),
                            writes_merged: rng.gen(),
                            sectors_written: rng.gen(),
                            time_writing: rng.gen(),
                            io_in_progress: rng.gen_range(0..100),
                            time_doing_io: rng.gen(),
                            weighted_time_doing_io: rng.gen(),
                        });
                    }
                    2 => {
                        // If there are disks, remove one
                        if !metrics.disks.is_empty() {
                            let index = rng.gen_range(0..metrics.disks.len());
                            metrics.disks.remove(index);
                        }
                    }
                    3 => {
                        // If there are disks, modify one
                        if !metrics.disks.is_empty() {
                            let index = rng.gen_range(0..metrics.disks.len());
                            metrics.disks[index].device_name = format!("/dev/sd{}", ['a', 'b', 'c', 'd', 'e', 'f'][rng.gen_range(0..6)]);
                            metrics.disks[index].reads_completed = rng.gen();
                            metrics.disks[index].writes_completed = rng.gen();
                        }
                    }
                    _ => {
                        // Add multiple disks
                        let num_disks = rng.gen_range(2..5);
                        for _ in 0..num_disks {
                            metrics.disks.push(DiskMetrics {
                                device_name: format!("/dev/sd{}", ['a', 'b', 'c', 'd', 'e', 'f'][rng.gen_range(0..6)]),
                                reads_completed: rng.gen(),
                                reads_merged: rng.gen(),
                                sectors_read: rng.gen(),
                                time_reading: rng.gen(),
                                writes_completed: rng.gen(),
                                writes_merged: rng.gen(),
                                sectors_written: rng.gen(),
                                time_writing: rng.gen(),
                                io_in_progress: rng.gen_range(0..100),
                                time_doing_io: rng.gen(),
                                weighted_time_doing_io: rng.gen(),
                            });
                        }
                    }
                }
            }
            4 => {
                // Mutate network
                match rng.gen_range(0..4) {
                    0 => {
                        // Clear all interfaces
                        metrics.network.interfaces.clear();
                    }
                    1 => {
                        // Add a new interface
                        metrics.network.interfaces.push(NetworkInterfaceMetrics {
                            name: format!("eth{}", rng.gen_range(0..5)),
                            bytes_sent: rng.gen(),
                            bytes_received: rng.gen(),
                            packets_sent: rng.gen(),
                            packets_received: rng.gen(),
                            errors_in: rng.gen_range(0..100),
                            errors_out: rng.gen_range(0..100),
                            drops_in: rng.gen_range(0..100),
                            drops_out: rng.gen_range(0..100),
                            speed: rng.gen_range(100000000..10000000000), // 100Mbps to 10Gbps
                        });
                    }
                    2 => {
                        // If there are interfaces, remove one
                        if !metrics.network.interfaces.is_empty() {
                            let index = rng.gen_range(0..metrics.network.interfaces.len());
                            metrics.network.interfaces.remove(index);
                        }
                    }
                    _ => {
                        // If there are interfaces, modify one
                        if !metrics.network.interfaces.is_empty() {
                            let index = rng.gen_range(0..metrics.network.interfaces.len());
                            metrics.network.interfaces[index].bytes_sent = rng.gen();
                            metrics.network.interfaces[index].bytes_received = rng.gen();
                            metrics.network.interfaces[index].packets_sent = rng.gen();
                            metrics.network.interfaces[index].packets_received = rng.gen();
                        }
                    }
                }
            }
            5 => {
                // Mutate GPUs
                match rng.gen_range(0..4) {
                    0 => {
                        // Clear all GPUs
                        metrics.gpus.clear();
                    }
                    1 => {
                        // Add a new GPU
                        metrics.gpus.push(GpuMetrics {
                            index: metrics.gpus.len(),
                            model: format!("GPU Model {}", rng.gen_range(1000..9999)),
                            utilization_bps: rng.gen_range(0..10000),
                            memory_usage_bps: rng.gen_range(0..10000),
                            temperature_deci_c: rng.gen_range(300..900), // 30°C to 90°C
                            power_draw_deci_w: rng.gen_range(500..3000), // 50W to 300W
                        });
                    }
                    2 => {
                        // If there are GPUs, remove one
                        if !metrics.gpus.is_empty() {
                            let index = rng.gen_range(0..metrics.gpus.len());
                            metrics.gpus.remove(index);
                        }
                    }
                    _ => {
                        // If there are GPUs, modify one
                        if !metrics.gpus.is_empty() {
                            let index = rng.gen_range(0..metrics.gpus.len());
                            metrics.gpus[index].utilization_bps = rng.gen_range(0..10000);
                            metrics.gpus[index].memory_usage_bps = rng.gen_range(0..10000);
                            metrics.gpus[index].temperature_deci_c = rng.gen_range(300..900);
                            metrics.gpus[index].power_draw_deci_w = rng.gen_range(500..3000);
                        }
                    }
                }
            }
            6 => {
                // Mutate load
                match rng.gen_range(0..3) {
                    0 => {
                        // Low load
                        metrics.load.load1 = rng.gen_range(0..100);  // 0.00-1.00
                        metrics.load.load5 = rng.gen_range(0..100);  // 0.00-1.00
                        metrics.load.load15 = rng.gen_range(0..100); // 0.00-1.00
                    }
                    1 => {
                        // Medium load
                        metrics.load.load1 = rng.gen_range(100..500);  // 1.00-5.00
                        metrics.load.load5 = rng.gen_range(100..500);  // 1.00-5.00
                        metrics.load.load15 = rng.gen_range(100..500); // 1.00-5.00
                    }
                    _ => {
                        // High load
                        metrics.load.load1 = rng.gen_range(500..2000);  // 5.00-20.00
                        metrics.load.load5 = rng.gen_range(500..2000);  // 5.00-20.00
                        metrics.load.load15 = rng.gen_range(500..2000); // 5.00-20.00
                    }
                }
            }
            _ => {
                // Mutate everything
                // Timestamp
                metrics.timestamp = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs() as i64;
                
                // IDs
                metrics.instance_id = Some(Uuid::new_v4().to_string());
                metrics.account_id = Some(Uuid::new_v4().to_string());
                
                // Disks
                metrics.disks.clear();
                metrics.disks.push(DiskMetrics {
                    device_name: "/dev/sda1".to_string(),
                    reads_completed: rng.gen(),
                    reads_merged: rng.gen(),
                    sectors_read: rng.gen(),
                    time_reading: rng.gen(),
                    writes_completed: rng.gen(),
                    writes_merged: rng.gen(),
                    sectors_written: rng.gen(),
                    time_writing: rng.gen(),
                    io_in_progress: rng.gen_range(0..100),
                    time_doing_io: rng.gen(),
                    weighted_time_doing_io: rng.gen(),
                });
                
                // Network
                metrics.network.interfaces.clear();
                metrics.network.interfaces.push(NetworkInterfaceMetrics {
                    name: "eth0".to_string(),
                    bytes_sent: rng.gen(),
                    bytes_received: rng.gen(),
                    packets_sent: rng.gen(),
                    packets_received: rng.gen(),
                    errors_in: rng.gen_range(0..100),
                    errors_out: rng.gen_range(0..100),
                    drops_in: rng.gen_range(0..100),
                    drops_out: rng.gen_range(0..100),
                    speed: rng.gen_range(100000000..10000000000), // 100Mbps to 10Gbps
                });
                
                // GPUs
                metrics.gpus.clear();
                metrics.gpus.push(GpuMetrics {
                    index: 0,
                    model: "GPU Model 1234".to_string(),
                    utilization_bps: rng.gen_range(0..10000),
                    memory_usage_bps: rng.gen_range(0..10000),
                    temperature_deci_c: rng.gen_range(300..900), // 30°C to 90°C
                    power_draw_deci_w: rng.gen_range(500..3000), // 50W to 300W
                });
                
                // Load
                metrics.load.load1 = rng.gen_range(0..1000);  // 0.00-10.00
                metrics.load.load5 = rng.gen_range(0..1000);  // 0.00-10.00
                metrics.load.load15 = rng.gen_range(0..1000); // 0.00-10.00
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use form_vm_metrics::cpu::CpuMetrics;
    use form_vm_metrics::mem::MemoryMetrics;
    use form_vm_metrics::load::LoadMetrics;
    
    #[test]
    fn test_vm_metrics_mutator() {
        let mutator = VmMetricsMutator::new();
        
        // Create a default metrics object
        let mut metrics = SystemMetrics {
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() as i64,
            instance_id: Some(Uuid::new_v4().to_string()),
            account_id: Some(Uuid::new_v4().to_string()),
            cpu: CpuMetrics::default(),
            memory: MemoryMetrics::default(),
            disks: vec![],
            network: NetworkMetrics { interfaces: vec![] },
            gpus: vec![],
            load: LoadMetrics {
                load1: 0,
                load5: 0,
                load15: 0,
            },
        };
        
        // Apply the mutation
        mutator.mutate(&mut metrics);
        
        // Just verify it doesn't panic
        // The actual mutations are random, so we can't assert specific changes
    }
} 