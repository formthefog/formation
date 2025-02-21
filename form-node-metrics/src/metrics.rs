use std::{sync::Arc, time::{Duration, Instant}};

use serde::{Serialize, Deserialize};
use sysinfo::{ProcessesToUpdate, System};
use nvml_wrapper::Nvml;
use tokio::{sync::Mutex, time::interval};  // using NVML for GPU metrics (optional feature)

#[derive(Debug, Serialize, Deserialize)]
pub struct NodeMetrics {
    // Load averages (1, 5, 15 minute)
    pub load_avg_1: f64,
    pub load_avg_5: f64,
    pub load_avg_15: f64,

    // System process count
    pub process_count: usize,

    // Throughput metrics (bytes per second)
    pub disk_read_bytes_per_sec: u64,
    pub disk_write_bytes_per_sec: u64,
    pub network_in_bytes_per_sec: u64,
    pub network_out_bytes_per_sec: u64,

    // Temperatures and power usage
    pub cpu_temperature: Option<f32>,   // in °C
    pub gpu_temperature: Option<f32>,   // in °C (if applicable)
    pub power_usage_watts: Option<f32>, // in Watts (if available)
}

pub struct MetricsCollector {
    sys: System,
    nvml: Option<Nvml>,   // NVML handle for GPU info (if available)
    last_update: std::time::Instant,
    interval: Duration
}

impl MetricsCollector {
    /// Create a new MetricsCollector, initializing system info and NVML (if enabled).
    pub fn new(refresh: Duration) -> Self {
        let mut sys = System::new_all();  // initialize and load all info once
        sys.refresh_all();               // initial refresh to populate data
        let nvml = Nvml::init().ok();
        MetricsCollector {
            sys,
            nvml,
            last_update: std::time::Instant::now(),
            interval: refresh
        }
    }

    /// Collect current metrics and return a NodeMetrics struct.
    pub fn collect(&mut self) -> NodeMetrics {
        // Refresh system information. We can refresh specific aspects to avoid overhead:
        self.sys.refresh_cpu_all();      // refresh CPU load, memory, etc.
        self.sys.refresh_processes(ProcessesToUpdate::All, true);   // refresh process list (for count)
        self.sys.refresh_memory();    // refresh network stats

        // Calculate load averages (1, 5, 15 minute)
        let load_avg = System::load_average(); 
        // load_avg.one, .five, .fifteen are f64&#8203;:contentReference[oaicite:8]{index=8}

        // Count processes (number of entries in the process list)
        let process_count = self.sys.processes().len();

        // Disk I/O throughput: sum bytes read/written since last refresh on all disks
        let mut read_bytes = 0;
        let mut written_bytes = 0;
        for disk in &sysinfo::Disks::new_with_refreshed_list() {
            let usage = disk.usage();  
            read_bytes    += usage.read_bytes;     // bytes read since last refresh&#8203;:contentReference[oaicite:9]{index=9}
            written_bytes += usage.written_bytes;  // bytes written since last refresh
        }
        // Convert to per-second by dividing by the interval (if exactly 30s, divide by 30)
        let secs = self.interval.as_secs();
        let disk_read_bps  = if secs > 0 { read_bytes / secs } else { read_bytes };
        let disk_write_bps = if secs > 0 { written_bytes / secs } else { written_bytes };

        // Network throughput: sum bytes received/transmitted since last refresh on all interfaces
        let mut recv_bytes = 0;
        let mut sent_bytes = 0;
        for (_iface, data) in &sysinfo::Networks::new_with_refreshed_list() {
            recv_bytes += data.received();    // bytes received since last refresh&#8203;:contentReference[oaicite:10]{index=10}
            sent_bytes += data.transmitted(); // bytes sent since last refresh
        }
        let net_in_bps  = if secs > 0 { recv_bytes / secs } else { recv_bytes };
        let net_out_bps = if secs > 0 { sent_bytes / secs } else { sent_bytes };

        // CPU temperature: find a component sensor that likely represents CPU (fallback to first component if uncertain)
        let mut cpu_temp = None;
        for component in &sysinfo::Components::new_with_refreshed_list() {  
            if let Some(temp) = component.temperature() {
                let label = component.label().to_lowercase();
                if label.contains("cpu") || label.contains("package") {
                    cpu_temp = Some(temp);
                    break;
                }
            }
        }

        // GPU temperature and power (if NVML was initialized and a GPU is present)
        let mut gpu_temp = None;
        let mut power_watts = None;
        if let Some(nvml) = &self.nvml {
            if let Ok(device) = nvml.device_by_index(0) {
                // Get GPU core temperature (Sensor type: GPU core)
                if let Ok(temp) = device.temperature(nvml_wrapper::enum_wrappers::device::TemperatureSensor::Gpu) {
                    gpu_temp = Some(temp as f32);
                }
                // Get GPU power usage (mW -> convert to Watts)
                if let Ok(usage) = device.power_usage() {
                    power_watts = Some(usage as f32 / 1000.0);
                }
            }
        }

        self.last_update = Instant::now();

        // Build the NodeMetrics struct with collected values
        NodeMetrics {
            load_avg_1:  load_avg.one,
            load_avg_5:  load_avg.five,
            load_avg_15: load_avg.fifteen,
            process_count,
            disk_read_bytes_per_sec:  disk_read_bps,
            disk_write_bytes_per_sec: disk_write_bps,
            network_in_bytes_per_sec:  net_in_bps,
            network_out_bytes_per_sec: net_out_bps,
            cpu_temperature: cpu_temp,
            gpu_temperature: gpu_temp,
            power_usage_watts: power_watts,
        }
    }
}

pub async fn start_metrics_monitor(refresh: Duration) -> Arc<Mutex<NodeMetrics>> {
    let metrics_collector = Arc::new(Mutex::new(MetricsCollector::new(refresh.clone())));

    let mut guard = metrics_collector.lock().await;
    let node_metrics = Arc::new(Mutex::new(guard.collect()));
    drop(guard);
    let inner_collector = metrics_collector.clone();
    let inner_metrics = node_metrics.clone();
    tokio::spawn(async move {
        let mut interval = interval(refresh);
        loop {
            interval.tick().await;
            let mut collector_guard = inner_collector.lock().await;
            let mut metrics_guard = inner_metrics.lock().await;
            *metrics_guard = collector_guard.collect();
            drop(collector_guard);
            drop(metrics_guard);
        }
    });

    node_metrics
}
