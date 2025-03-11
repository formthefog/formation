use std::{sync::Arc, time::{SystemTime, UNIX_EPOCH}};
use tokio::sync::Mutex;
use serde::{Serialize, Deserialize};
use sysinfo::System;

use crate::{
    cpu::{collect_cpu, CpuMetrics}, 
    disk::{collect_disk_metrics, DiskMetrics}, 
    gpu::{collect_gpu_metrics, GpuMetrics}, 
    load::{collect_load_metrics, LoadMetrics}, 
    mem::{collect_memory, MemoryMetrics}, 
    network::{collect_network_metrics, NetworkMetrics}
};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SystemMetrics {
    pub timestamp: i64,
    pub instance_id: Option<String>,
    pub account_id: Option<String>,
    pub cpu: CpuMetrics,
    pub memory: MemoryMetrics,
    pub disks: Vec<DiskMetrics>,
    pub network: NetworkMetrics,
    pub gpus: Vec<GpuMetrics>,
    pub load: LoadMetrics,
}

pub async fn collect_system_metrics(
    system_metrics: Arc<Mutex<SystemMetrics>>,
) -> Arc<Mutex<SystemMetrics>> {
    let mut sys = System::new_all();
    sys.refresh_all();

    let cpu = collect_cpu(&mut sys).await;
    let memory = collect_memory(&mut sys).await;
    let disks = collect_disk_metrics();
    let network = collect_network_metrics();
    let gpus = collect_gpu_metrics().await.unwrap_or_else(|e| {
        eprintln!("Failed to collect GPU metrics: {}", e);
        Vec::new()
    });
    let load = collect_load_metrics();
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Something is seriously wrong with the system")
        .as_secs() as i64;

    let mut guard = system_metrics.lock().await;
    
    // Preserve the instance_id and account_id fields
    let instance_id = guard.instance_id.clone();
    let account_id = guard.account_id.clone();
    
    *guard = SystemMetrics {
        timestamp,
        instance_id,
        account_id,
        cpu,
        memory,
        disks,
        network,
        gpus,
        load,
    };
    drop(guard);

    system_metrics
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_preserve_instance_and_account_id() {
        let test_instance_id = "test-instance-123";
        let test_account_id = "test-account-456";
        
        // Create metrics with initial instance and account IDs
        let mut initial_metrics = SystemMetrics::default();
        initial_metrics.instance_id = Some(test_instance_id.to_string());
        initial_metrics.account_id = Some(test_account_id.to_string());
        
        let metrics = Arc::new(Mutex::new(initial_metrics));
        
        // Collect new metrics (which should preserve the IDs)
        let updated_metrics = collect_system_metrics(metrics).await;
        
        // Verify the IDs were preserved
        let guard = updated_metrics.lock().await;
        assert_eq!(guard.instance_id.as_deref(), Some(test_instance_id));
        assert_eq!(guard.account_id.as_deref(), Some(test_account_id));
    }
}
