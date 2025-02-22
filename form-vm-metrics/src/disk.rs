#[cfg(target_os = "linux")]
use procfs::{diskstats, DiskStat};
use serde::{Serialize, Deserialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DiskMetrics {
    pub device_name: String,
    pub reads_completed: u64,
    pub reads_merged: u64,
    pub sectors_read: u64,
    pub time_reading: u64,
    pub writes_completed: u64,
    pub writes_merged: u64,
    pub sectors_written: u64,
    pub time_writing: u64,
    pub io_in_progress: u64,
    pub time_doing_io: u64,
    pub weighted_time_doing_io: u64,
}

pub fn collect_disk_metrics() -> Vec<DiskMetrics> {
    #[cfg(target_os = "linux")]
    {
        match diskstats() {
            Ok(stats) => stats.into_iter().map(|stat| DiskMetrics {
                device_name: stat.name,
                reads_completed: stat.reads_completed,
                reads_merged: stat.reads_merged,
                sectors_read: stat.sectors_read,
                time_reading: stat.time_reading,
                writes_completed: stat.writes_completed,
                writes_merged: stat.writes_merged,
                sectors_written: stat.sectors_written,
                time_writing: stat.time_writing,
                io_in_progress: stat.io_in_progress,
                time_doing_io: stat.time_doing_io,
                weighted_time_doing_io: stat.weighted_time_doing_io,
            }).collect(),
            Err(_) => Vec::new(),
        }
    }

    #[cfg(not(target_os = "linux"))]
    {
        // On non-Linux platforms, return empty metrics for now
        // TODO: Implement macOS disk metrics using IOKit or similar
        Vec::new()
    }
}
