use procfs::{diskstats, DiskStat};
use serde::{Serialize, Deserialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)] 
pub struct DiskMetrics {
    device: String,
    total_space: i64,
    available_space: i64,
    read_bytes_per_sec: i64,
    write_bytes_per_sec: i64,
    read_iops: i64,
    write_iops: i64,
}

pub async fn collect_disks(prev: Option<Vec<DiskStat>>, interval: u64) -> (Vec<DiskMetrics>, Vec<DiskStat>) {
    let mut disks = vec![];
    for disk in &sysinfo::Disks::new_with_refreshed_list() {
        disks.push(DiskMetrics {
            device: disk.name().to_string_lossy().to_string(),
            total_space: disk.total_space() as i64,
            available_space: disk.available_space() as i64,
            read_bytes_per_sec: (disk.usage().read_bytes / interval) as i64, 
            write_bytes_per_sec: (disk.usage().written_bytes / interval) as i64,
            read_iops: 0,
            write_iops: 0,
        });
    }

    let stats = if let Ok(stats) = diskstats() {
        for disk in &stats {
            if let Some(disk_metrics) = disks.iter_mut().find(|d| d.device == disk.name) {
                if let Some(prev) = &prev {
                    if let Some(prev_metrics) = prev.iter().find(|d| d.name == disk.name) {
                        let rps = (((disk.sectors_read - prev_metrics.sectors_read) * 512) / interval) as i64;
                        let wps = (((disk.sectors_written - prev_metrics.sectors_written) * 512) / interval) as i64;

                        disk_metrics.write_iops = wps;
                        disk_metrics.read_iops = rps;
                    }
                } else {
                    let rps = (disk.sectors_read * 512 / interval) as i64;
                    let wps = (disk.sectors_written * 512 / interval) as i64;
                    disk_metrics.write_iops = wps;
                    disk_metrics.read_iops = rps;
                }
            }
        }
        stats
    } else {
        vec![]
    };

    (disks, stats)
}
