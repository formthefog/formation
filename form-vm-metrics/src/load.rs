use serde::{Serialize, Deserialize};
use sysinfo::System;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LoadMetrics {
    pub load1: i64,  // 1-minute load average, scaled to integer (e.g., 1.23 -> 123)
    pub load5: i64,  // 5-minute load average
    pub load15: i64, // 15-minute load average
}

pub fn collect_load_metrics() -> LoadMetrics {
    let load_avg = System::load_average();
    LoadMetrics {
        load1: (load_avg.one * 100.0) as i64,
        load5: (load_avg.five * 100.0) as i64,
        load15: (load_avg.fifteen * 100.0) as i64,
    }
}
