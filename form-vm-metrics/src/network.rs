use serde::{Serialize, Deserialize};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::error::Error;

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)] 
pub struct NetworkInterfaceMetrics {
    pub name: String,
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub packets_sent: u64,
    pub packets_received: u64,
    pub errors_in: u64,
    pub errors_out: u64,
    pub drops_in: u64,
    pub drops_out: u64,
    pub speed: u64, // in bits per second
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)] 
pub struct NetworkMetrics {
    pub interfaces: Vec<NetworkInterfaceMetrics>,
}

pub fn collect_network_metrics() -> NetworkMetrics {
    #[cfg(target_os = "linux")]
    {
        if let Ok(interfaces) = parse_proc_net_dev() {
            return NetworkMetrics { interfaces };
        }
    }

    // Fallback to sysinfo for non-Linux or if parsing fails
    let networks = sysinfo::Networks::new_with_refreshed_list();
    let interfaces = networks
        .iter()
        .map(|(name, data)| NetworkInterfaceMetrics {
            name: name.clone(),
            bytes_sent: data.total_transmitted(),
            bytes_received: data.total_received(),
            packets_sent: data.total_packets_transmitted(),
            packets_received: data.total_packets_received(),
            errors_in: data.total_errors_on_received(),
            errors_out: data.total_errors_on_transmitted(),
            drops_in: 0,  // Not available in sysinfo
            drops_out: 0, // Not available in sysinfo
            speed: 0,     // Not available in sysinfo
        })
        .collect();
    NetworkMetrics { interfaces }
}

#[cfg(target_os = "linux")]
fn parse_proc_net_dev() -> Result<Vec<NetworkInterfaceMetrics>, Box<dyn Error>> {
    let file = File::open("/proc/net/dev")?;
    let reader = BufReader::new(file);
    let mut interfaces = Vec::new();
    for line in reader.lines().skip(2) { // Skip header lines
        let line = line?;
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 17 {
            let name = parts[0].trim_end_matches(':').to_string();
            let speed = get_interface_speed(&name).unwrap_or(0);
            interfaces.push(NetworkInterfaceMetrics {
                name,
                bytes_received: parts[1].parse()?,
                packets_received: parts[2].parse()?,
                errors_in: parts[3].parse()?,
                drops_in: parts[4].parse()?,
                bytes_sent: parts[9].parse()?,
                packets_sent: parts[10].parse()?,
                errors_out: parts[11].parse()?,
                drops_out: parts[12].parse()?,
                speed,
            });
        }
    }
    Ok(interfaces)
}

#[cfg(target_os = "linux")]
fn get_interface_speed(interface: &str) -> Option<u64> {
    let speed_path = format!("/sys/class/net/{}/speed", interface);
    std::fs::read_to_string(speed_path)
        .ok()?
        .trim()
        .parse::<u64>()
        .ok()
        .map(|speed| speed * 1_000_000) // Mbps to bps
}
