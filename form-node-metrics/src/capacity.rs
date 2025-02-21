use std::{sync::Arc, time::Duration};

use serde::{Serialize, Deserialize};
use sysinfo::System;
use tokio::{sync::Mutex, time::interval};

#[derive(Debug, Serialize, Deserialize)]
pub struct NodeCapacity {
    pub cpu_total_cores: usize,
    pub cpu_available_cores: f32,   // using f32 to represent fractional available cores or usage%
    pub memory_total_bytes: u64,
    pub memory_available_bytes: u64,
    pub storage_total_bytes: u64,
    pub storage_available_bytes: u64,
    pub gpu_total_memory_bytes: u64,
    pub gpu_available_memory_bytes: u64,
    pub network_total_bandwidth: u64,    // e.g., in bytes/sec if known
    pub network_available_bandwidth: u64,
}

pub fn get_current_capacity() -> NodeCapacity {
    let mut sys = System::new_all();  // initialize and gather all info
    sys.refresh_all();               // ensure data is up-to-date

    // CPU: total cores and available (idle cores = total * (1 - usage%))
    let total_cores = sys.cpus().len() as usize;
    let total_cpu_usage_percent = sys.global_cpu_usage(); // e.g., 0.0 to 100.0
    let available_cores = ((100.0 - total_cpu_usage_percent) / 100.0) * total_cores as f32;

    // Memory: total and available (in bytes)
    let total_mem = sys.total_memory();       // bytes of RAM total&#8203;:contentReference[oaicite:10]{index=10}
    let avail_mem = sys.available_memory();   // bytes of RAM available&#8203;:contentReference[oaicite:11]{index=11}

    // Storage: total and available (sum of all disks)
    let mut total_disk = 0;
    let mut avail_disk = 0;
    for disk in &sysinfo::Disks::new_with_refreshed_list() {
        total_disk += disk.total_space();       // bytes of disk size&#8203;:contentReference[oaicite:12]{index=12}
        avail_disk += disk.available_space();   // bytes of available space&#8203;:contentReference[oaicite:13]{index=13}
    }

    // GPU: (Placeholder, as GPU info may require a different approach)
    let gpu_total = 0;
    let gpu_avail = 0;
    // In future, populate via GPU APIs if available.

    // Network: (Placeholder for bandwidth capacity, if known)
    let net_total = 0;
    let net_avail = 0;
    // Could use sys.networks() to get usage, but capacity is not directly available.

    NodeCapacity {
        cpu_total_cores: total_cores,
        cpu_available_cores: available_cores,
        memory_total_bytes: total_mem,
        memory_available_bytes: avail_mem,
        storage_total_bytes: total_disk,
        storage_available_bytes: avail_disk,
        gpu_total_memory_bytes: gpu_total,
        gpu_available_memory_bytes: gpu_avail,
        network_total_bandwidth: net_total,
        network_available_bandwidth: net_avail,
    }
}

pub async fn start_capacity_monitor(refresh: Duration) -> Arc<Mutex<NodeCapacity>> {
    let capacity = Arc::new(Mutex::new(get_current_capacity()));
    let inner_capacity = capacity.clone();
    tokio::spawn(async move {
        let mut interval = interval(refresh);
        loop {
            interval.tick().await;
            // Here we could log or output the capacity if needed
            println!("Node Capacity: {:?}", inner_capacity);
            // Refresh capacity data
            let mut guard = inner_capacity.lock().await;
            *guard = get_current_capacity();
            drop(guard);
        }
    });

    capacity
}
