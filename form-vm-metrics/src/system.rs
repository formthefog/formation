use std::{sync::Arc, time::{SystemTime, UNIX_EPOCH}};
use tokio::sync::Mutex;
use procfs::DiskStat;
use serde::{Serialize, Deserialize};
use sysinfo::System;

use crate::{cpu::{collect_cpu, CpuMetrics}, disk::{collect_disks, DiskMetrics}, gpu::{collect_gpu_metrics, GpuMetrics}, load::{collect_load_metrics, LoadMetrics}, mem::{collect_memory, MemoryMetrics}, network::{collect_network_metrics, NetworkMetrics}};

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SystemMetrics {
    pub timestamp: i64,
    pub cpu: CpuMetrics,
    pub memory: MemoryMetrics,
    pub disks: Vec<DiskMetrics>,
    pub disk_stats: Vec<DiskStat>,
    pub gpus: Vec<GpuMetrics>,
    pub network: NetworkMetrics,
    pub load: LoadMetrics,
}

pub async fn collect_system_metrics(
    system_metrics: Arc<Mutex<SystemMetrics>>,
    disk_stats: Option<Vec<DiskStat>>,
    refresh: u64,
) -> Arc<Mutex<SystemMetrics>> {
    let mut sys = System::new_all();
    sys.refresh_all();

    let cpu = collect_cpu(&mut sys).await;
    let memory = collect_memory(&mut sys).await;
    let (metrics, stats) = collect_disks(disk_stats, refresh).await;
    let network = collect_network_metrics();
    let gpus = collect_gpu_metrics().await.unwrap_or_else(|e| {
        eprintln!("Failed to collect GPU metrics: {}", e);
        Vec::new()
    });
    let load = collect_load_metrics();
    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).expect("Something is seriously wrong with the system").as_secs() as i64;

    let mut guard = system_metrics.lock().await;

    *guard = SystemMetrics {
        timestamp,
        cpu,
        memory,
        disks: metrics,
        disk_stats: stats,
        network,
        gpus,
        load,
    };
    drop(guard);

    system_metrics
}
