// capabilities.rs
use nvml_wrapper::Nvml;
use serde::{Serialize, Deserialize};
use pnet::datalink;

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NodeCapabilities {
    pub cpu_model: String,
    pub cpu_cores: usize,
    pub total_memory: u64,
    pub total_storage: u64,
    pub gpu_models: Vec<GpuInfo>,
    pub network_interfaces: Vec<NetworkCapability>,  // e.g., a struct for NIC details
    pub tpm: Option<TpmInfo>,
    pub sgx: Option<SgxInfo>,
    pub sev: Option<SevInfo>,
    pub virtualization_type: Option<String>,
}

// Optionally, an implementation to gather this info at startup:
impl NodeCapabilities {
    pub fn collect() -> Self {
        let mut sys = sysinfo::System::new_all();
        sys.refresh_all();  // load all system info

        // Collect CPU info
        let cpu_model = sys.cpus().get(0)
            .map(|cpu| cpu.brand().to_string())
            .unwrap_or_default();  // CPU brand as model

        let cpu_cores = sys
            .physical_core_count()
            .unwrap_or_else(|| sys.cpus().len());  // physical cores if available

        // Collect memory and storage info
        let total_memory = sys.total_memory() / (1024 * 1024);    //
        let total_storage = sysinfo::Disks::new_with_refreshed_list()
            .iter()
            .map(|disk| disk.total_space())
            .sum();  // sum of all disk sizes

        // GPU, network, TPM, virtualization can be fetched via other means (placeholder here)
        let gpu_models = detect_gpus(); 
        let network_interfaces = NetworkCapability::collect(); 

        #[cfg(feature = "tpm")]
        let tpm = detect_tpm();
        #[cfg(not(feature = "tpm"))]
        let tpm = None;

        let virtualization_type = String::from("BareMetal");

        Self {
            cpu_model,
            cpu_cores,
            total_memory,
            total_storage,
            gpu_models,
            network_interfaces,
            tpm,
            sgx: None,
            sev: None,
            virtualization_type: Some(virtualization_type),
        }
    }
}

#[derive(Clone, Default, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NetworkCapability {
    pub interface_name: String,
    pub link_speed_mbps: Option<u64>,
    pub max_bandwidth: Option<u64>,
    pub ipv4_addresses: Vec<String>,
    pub ipv6_addresses: Vec<String>,
    pub is_active: bool,
}

impl NetworkCapability {
    /// Collects network details for all interfaces on the system.
    pub fn collect() -> Vec<NetworkCapability> {
        let mut results = Vec::new();

        // Use sysinfo to get the list of network interfaces
        let _sys = sysinfo::System::new_all();

        let interfaces: Vec<String> = sysinfo::Networks::new_with_refreshed_list().keys().cloned().collect();

        // Alternatively, we can get interface list from pnet directly:
        // let interfaces = datalink::interfaces();

        // Use pnet to get detailed info (addresses, flags) for each interface
        let iface_details = datalink::interfaces();

        for iface_name in interfaces {
            // Find the corresponding interface details from pnet by name
            if let Some(iface) = iface_details.iter().find(|iface| iface.name == iface_name) {
                // Collect IPv4 and IPv6 addresses as strings
                let mut ipv4_addrs = Vec::new();
                let mut ipv6_addrs = Vec::new();
                for ipnet in &iface.ips {
                    let ip_addr = ipnet.ip();  // Extract the IpAddr from IpNetwork
                    if ip_addr.is_ipv4() {
                        ipv4_addrs.push(ip_addr.to_string());
                    } else if ip_addr.is_ipv6() {
                        ipv6_addrs.push(ip_addr.to_string());
                    }
                }

                // Determine link speed (Linux-specific)
                let mut link_speed: Option<u64> = None;
                #[cfg(target_os = "linux")] {
                    let speed_path = format!("/sys/class/net/{}/speed", iface.name);
                    if let Ok(speed_str) = std::fs::read_to_string(&speed_path) {
                        if let Ok(speed_val) = speed_str.trim().parse::<u64>() {
                            // The sysfs speed value is in Mbit/s
                            if speed_val != 0 {
                                link_speed = Some(speed_val);
                            }
                        }
                    }
                }

                // Determine max bandwidth (if known, else use link_speed as a proxy)
                let max_bw = link_speed;  // In absence of other data, use current speed

                // Determine if interface is active (up and running)
                let is_active = iface.is_up() && iface.is_running();

                results.push(NetworkCapability {
                    interface_name: iface.name.clone(),
                    link_speed_mbps: link_speed,
                    max_bandwidth: max_bw,
                    ipv4_addresses: ipv4_addrs,
                    ipv6_addresses: ipv6_addrs,
                    is_active,
                });
            } else {
                // If pnet didn't list this interface (unlikely), skip or handle accordingly
            }
        }

        results
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GpuInfo {
    pub vendor: String,
    pub model: Option<String>,
    pub count: u32,
    pub total_memory_bytes: u64,
    pub pci_bus_id: Option<String>,
    pub cuda_enabled: Option<(i32, i32)>,
    pub driver_version: Option<String>
}

fn detect_gpus() -> Vec<GpuInfo> {
    let mut gpus = Vec::new();

    // 1) Attempt NVIDIA detection using NVML
    match Nvml::init() {
        Ok(nvml_lib) => {
            if let Ok(count) = nvml_lib.device_count() {
                for idx in 0..count {
                    if let Ok(device) = nvml_lib.device_by_index(idx) {
                        let name = device.name().unwrap_or_else(|_| "NVIDIA GPU".to_string());
                        
                        // Memory info
                        let total_mem = match device.memory_info() {
                            Ok(mem_info) => mem_info.total,  // bytes
                            Err(_) => 0,
                        };
                        
                        // PCI bus ID
                        let pci_bus_id = match device.pci_info() {
                            Ok(info) => Some(info.bus_id.to_string()),
                            Err(_) => None,
                        };
                        
                        let cuda_enabled = {
                            // CUDA capability
                            let major_minor = if let Ok(cap) = device.cuda_compute_capability() {
                                let major = cap.major;
                                let minor = cap.minor;
                                Some((major, minor))
                            } else {
                                None
                            };
                            let cuda_cap = if let Some((major, minor)) = major_minor {
                                if major > 0 { 
                                    Some((major, minor)) 
                                } else { 
                                    None 
                                }
                            } else {
                                None
                            };

                            cuda_cap
                        };
                        
                        // Driver version
                        let driver_version = nvml_lib.sys_driver_version().ok();

                        gpus.push(GpuInfo {
                            vendor: "NVIDIA".to_string(),
                            model: Some(name),
                            count: 1,
                            total_memory_bytes: total_mem,
                            pci_bus_id,
                            cuda_enabled,
                            driver_version,
                        });
                    }
                }
            }
        },
        Err(_) => {
            // NVML not available or init failed
        }
    }

    // 2) If we already found NVIDIA GPUs, we might skip fallback. Or keep scanning for AMD/Intel GPUs as well.
    //    If you want to detect all GPU types, you can ALWAYS do the fallback as well.

    // Fallback detection for additional GPUs (AMD/Intel or if NVML not used)
    if let Ok(drm_entries) = std::fs::read_dir("/sys/class/drm") {
        for entry in drm_entries.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            // Only match top-level "cardX" devices, skipping "card0-DP-1" or similar
            if name.starts_with("card") && !name.contains('-') {
                let device_dir = entry.path().join("device");
                let vendor_file = device_dir.join("vendor");

                let vendor_id = std::fs::read_to_string(&vendor_file)
                    .unwrap_or_default()
                    .trim()
                    .to_lowercase();

                // If this device is already accounted for by NVML (NVIDIA),
                // we can skip or detect duplicates by comparing PCI bus ID, etc.

                // Map vendor ID to a name
                let (vendor, vname) = match vendor_id.as_str() {
                    "0x10de" => ("NVIDIA".to_string(), "Unknown NVIDIA".to_string()),
                    "0x1002" => ("AMD".to_string(), "Unknown AMD".to_string()),
                    "0x8086" => ("Intel".to_string(), "Unknown Intel".to_string()),
                    _ => ("Unknown".to_string(), "Unknown GPU".to_string()),
                };

                // Optional: read more info from sub-files if needed (e.g., AMD or Intel driver).

                // Check if a GPU with the same PCI bus ID is already in the vector:
                //   let pci_addr = device_dir.join("uevent") or device_dir.join("bus_id") ...
                //   If it matches an existing GpuInfo, skip. Otherwise, add.

                // For now, add an entry with minimal info
                gpus.push(GpuInfo {
                    vendor,
                    model: Some(vname),
                    count: 1,
                    total_memory_bytes: 0,    // Not easily obtained
                    pci_bus_id: None,
                    cuda_enabled: None,
                    driver_version: None,
                });
            }
        }
    }

    gpus
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TpmInfo {
    // Basic TPM details
    pub present: bool,
    // For a TPM 2.0 device, version might be "2.0", etc.
    pub version: Option<String>,
    // Manufacturer name or ID (like "IFX" for Infineon, "INTC" for Intel, etc.)
    pub manufacturer: Option<String>,
    // E.g., from /sys/class/tpm/tpm0/device/description or other queries
    pub description: Option<String>,
    // Could also store firmware version if accessible
    pub firmware_version: Option<String>,
}

#[cfg(feature = "tpm")]
fn detect_tpm() -> Option<TpmInfo> {
    use tss2::*;
    // Try to detect TPM using tss2
    match tss2::Context::new() {
        Ok(_) => Some(TpmInfo {
            present: true,
            version: Some("2.0".to_string()),
            manufacturer: None,
            description: None,
            firmware_version: None,
        }),
        Err(_) => None,
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SgxInfo {
    pub supported: bool, // CPU + BIOS has SGX enabled
    pub driver_loaded: bool, // is the SGX driver loaded (/dev/sgx_enclave or /dev/isgx)
    pub sgx1_enabled: bool,  // indicates SGX1 is available
    pub sgx2_enabled: bool,  // indicates SGX2/EDMM is available
    pub flc_enabled: bool,   // Flexible Launch Control
    pub max_enclave_size: Option<u64>,   // if we can read from CPUID leaf 0x12
    pub epc_sections: Vec<EpcSection>,
    pub misc_select: Option<u32>,        // from CPUID enumerations
    pub attributes: Option<u64>,         // from CPUID enumerations
}

impl SgxInfo {}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EpcSection {
    // typically from CPUID leaf 0x12 sub-leafs describing EPC memory ranges
    pub base_address: u64,
    pub size_bytes: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SevInfo {
    pub supported: bool,          // CPU supports SEV (and it's enabled in BIOS)
    pub sev_es_supported: bool,   // indicates SEV-ES is supported
    pub sev_snp_supported: bool,  // indicates SEV-SNP is supported
    pub firmware_version: Option<String>, // from /dev/sev firmware query
    pub api_major: Option<u8>,    // from the SEV platform status
    pub api_minor: Option<u8>,    // from the SEV platform status
    pub min_api_major: Option<u8>, // from the SEV crate's capabilities
    pub min_api_minor: Option<u8>,
    // Potentially a list of extended features, if the SEV crate provides them
    pub platform_status: Option<String>, // e.g. "Initialized", "Working", etc.
}
