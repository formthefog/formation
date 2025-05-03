pub mod wizard;
use net_util::MacAddr;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use super::{VmInstanceConfig, ConsoleType}; 
use vmm::vm_config::{
    ConsoleConfig,
    ConsoleOutputMode,
    CpusConfig, 
    DiskConfig, 
    MemoryConfig, 
    NetConfig, 
    PayloadConfig, 
    RngConfig, 
    VhostMode, 
    VmConfig,
    DeviceConfig,
};

pub fn create_vm_config(config: &VmInstanceConfig) -> VmConfig {

    let disks = vec![DiskConfig {
        // This needs to be a copied disk, raw cannot use backing file
        path: Some(config.rootfs_path.clone()),
        readonly: false,
        direct: true,
        vhost_user: false,
        vhost_socket: None,
        rate_limiter_config: None,
        queue_size: 256,
        num_queues: 1,
        queue_affinity: None,
        id: None,
        rate_limit_group: None,
        pci_segment: 0,
        iommu: false,
        serial: None,
        disable_io_uring: false,  // New field
        disable_aio: false,       // New field
    }];

    let (serial, console) = match config.console_type {
        ConsoleType::Serial => (
            ConsoleConfig {
                file: None,
                mode: ConsoleOutputMode::Socket,
                iommu: false,
                socket: Some(PathBuf::from(&format!("/run/form-vmm/{}-console.sock", &config.name))), 
            },
            ConsoleConfig {
                file: None,
                mode: ConsoleOutputMode::Null,
                iommu: false,
                socket: None
            }
        ),
        ConsoleType::Virtio => (
            ConsoleConfig {
                file: None,
                mode: ConsoleOutputMode::Socket,
                iommu: false,
                socket: Some(PathBuf::from(&format!("/run/form-vmm/{}-console.sock", &config.name))), 
            },
            ConsoleConfig {
                file: None,
                mode: ConsoleOutputMode::Null,
                iommu: false,
                socket: None
            },
        ),
    };

    let net = Some(vec![NetConfig {
        tap: Some(config.tap_device.to_string()),
        ip: config.ip_addr.parse().unwrap(),  // Use our bridge IP as gateway
        mask: "255.255.255.0".parse().unwrap(),
        mac: MacAddr::local_random(),  // Default MAC, can be configured
        host_mac: None,
        mtu: Some(1500),
        iommu: false,
        num_queues: 2,
        queue_size: 256,
        vhost_user: false,
        vhost_socket: None,
        vhost_mode: VhostMode::Client,
        id: Some(format!("net_{}", config.name)),
        fds: None,
        rate_limiter_config: None,
        pci_segment: 0,
        offload_tso: true,
        offload_ufo: true,
        offload_csum: true,
    }]);
    
    // Process GPU devices if any are configured
    let devices = if let Some(gpu_configs) = &config.gpu_devices {
        if !gpu_configs.is_empty() {
            // Create DeviceConfig entries for each GPU
            let mut device_configs = Vec::new();
            
            for gpu_config in gpu_configs {
                for gpu_device in &gpu_config.assigned_devices {
                    // Create a path to the PCI device
                    let pci_path = PathBuf::from("/sys/bus/pci/devices")
                        .join(&gpu_device.pci_address);
                    
                    // Create DeviceConfig for this GPU
                    let device_config = DeviceConfig {
                        path: pci_path,
                        iommu: true, // Enable IOMMU for GPU passthrough
                        id: Some(format!("gpu_{}", gpu_device.pci_address.replace(":", "_"))),
                        pci_segment: 0,
                        // Configure NVIDIA GPUDirect if enabled
                        x_nv_gpudirect_clique: if gpu_device.enable_gpudirect {
                            Some(0) // All GPUs with clique=0 can communicate with each other
                        } else {
                            None
                        },
                    };
                    
                    device_configs.push(device_config);
                }
            }
            
            if !device_configs.is_empty() {
                Some(device_configs)
            } else {
                None
            }
        } else {
            None
        }
    } else {
        None
    };
    
    // Enable IOMMU at the VM level if we have GPU devices
    let enable_iommu = devices.is_some();
    
    VmConfig {
        cpus: CpusConfig {
            boot_vcpus: config.vcpu_count,
            max_vcpus: config.vcpu_count,
            ..CpusConfig::default()
        },
        memory: MemoryConfig {
            size: config.memory_mb << 20, // Convert MB to bytes
            ..MemoryConfig::default()
        },
        payload: Some(PayloadConfig {
            kernel: Some(config.kernel_path.clone()),
            initramfs: None,
            cmdline: None, 
            firmware: None,
        }),
        disks: Some(disks),
        net,
        rng: RngConfig {
            src: config.rng_source.clone().unwrap_or_else(|| "/dev/urandom".to_string()).into(),
            iommu: false,
        },
        balloon: None,
        fs: None,
        pmem: None,
        serial,
        console,
        #[cfg(target_arch = "x86_64")]
        debug_console: vmm::vm_config::DebugConsoleConfig::default(),
        devices, // Use our configured GPU devices
        user_devices: None,
        vdpa: None,
        vsock: None,
        pvpanic: false,
        #[cfg(feature = "pvmemcontrol")]
        pvmemcontrol: None,
        iommu: enable_iommu, // Enable IOMMU when GPU devices are present
        #[cfg(target_arch = "x86_64")]
        sgx_epc: None,
        numa: None,
        watchdog: false,
        #[cfg(feature = "guest_debug")]
        gdb: false,
        platform: None,
        tpm: None,
        preserved_fds: None,
        landlock_enable: false,
        landlock_rules: None,
        rate_limit_groups: None,     // New required field
        pci_segments: None,          // New required field
    }
}

/// Default paths for VMM Service
pub struct ServicePaths;

impl ServicePaths {
    /// Base path for all VMM service related files
    pub const BASE_DIR: &'static str = "/var/lib/formation";
    /// Path for kernel image(s)
    pub const KERNEL_DIR: &'static str = "kernel"; 
}

/// Global configuration for the VMM service
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Base directory for VM-related files
    pub base_dir: PathBuf,
    /// Network configuration
    pub network: NetworkConfig,
    /// Resource limits
    pub limits: ResourceLimits,
    /// Default VM parameters
    pub default_vm_params: DefaultVmParams,
    /// Address that the FormPackManager API is listening on
    pub pack_manager: String,
    /// API server configuration
    pub api: ApiConfig,
}

/// Configuration for the API server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiConfig {
    /// Port for the API server to listen on
    #[serde(default = "default_api_port")]
    pub port: u16,
    /// Public key for signature verification in hex format (optional)
    pub signature_public_key: Option<String>,
    /// Legacy authentication token (deprecated)
    pub auth_token: Option<String>,
}

fn default_api_port() -> u16 {
    3020
}

impl Default for ApiConfig {
    fn default() -> Self {
        Self {
            port: default_api_port(),
            signature_public_key: None,
            auth_token: None,
        }
    }
}

/// Network configuration for VMs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    /// Bridge interface name
    pub bridge_interface: String,
    /// DHCP range start
    pub dhcp_range_start: String,
    /// DHCP range end
    pub dhcp_range_end: String,
    /// Network Gateway
    pub gateway: String,
    /// Network mask
    pub netmask: String,
    /// DNS Servers
    pub nameservers: Vec<String>,
    /// Domain suffix
    pub domain_suffix: String,
    /// DNS listen addr
    pub dns_listener_addr: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLimits {
    /// Maximum number of concurrent VMs this host can handle
    pub max_vms: usize,
    /// Maximum memory per VM on this host in MB
    pub max_memory_per_vm: u64,
    /// Maximum vCPUs per VM on this host
    pub max_vcpus_per_vm: u8,
    /// Maximum Disk size per VM on this host in GB
    pub max_disk_size_per_vm: u64

}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DefaultVmParams {
    pub memory_mb: u64,
    pub vcpu_count: u8,
    pub disk_size_gb: u64,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            bridge_interface: "vmbr0".to_string(),
            dhcp_range_start: "192.168.122.2".to_string(),
            dhcp_range_end: "192.168.122.254".to_string(),
            gateway: "192.168.122.1".to_string(),
            netmask: "255.255.255.0".to_string(),
            nameservers: vec!["1.1.1.1".to_string(), "8.8.8.8".to_string()],
            dns_listener_addr: "0.0.0.0:53".to_string(),
            domain_suffix: "dev.formation.cloud".to_string() 
        }
    }
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            max_vms: 10,
            max_memory_per_vm: 32768, // 32GB
            max_vcpus_per_vm: 8,
            max_disk_size_per_vm: 1024, // 1TB
        }
    }
}

impl Default for DefaultVmParams {
    fn default() -> Self {
        Self {
            memory_mb: 2048,  // 2GB
            vcpu_count: 2,
            disk_size_gb: 20,
        }
    }
}
