//! GPU management module for Formation VMM
//!
//! This module provides functionality for detecting, managing, and binding
//! GPUs to VFIO for passthrough to virtual machines.

use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs::{self, File};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use log::{debug, error, info, warn};
use crate::instance::config::{GpuConfig, GpuDeviceInfo};

/// Represents a GPU device on the host system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuDevice {
    /// PCI address of the GPU (e.g., "0000:01:00.0")
    pub pci_address: String,
    /// Vendor ID of the GPU
    pub vendor_id: String,
    /// Device ID of the GPU
    pub device_id: String,
    /// Human-readable name of the GPU (if available)
    pub name: Option<String>,
    /// Current driver being used
    pub current_driver: Option<String>,
    /// Whether the device is bound to VFIO
    pub is_vfio_bound: bool,
    /// IOMMU group of the device
    pub iommu_group: Option<String>,
    /// List of related devices in the same IOMMU group
    pub related_devices: Vec<String>,
    /// GPU model (inferred from name or device info)
    pub model: Option<String>,
    /// Whether this GPU is currently assigned to a VM
    pub assigned: bool,
}

/// GPU vendor identifiers
pub enum GpuVendor {
    /// NVIDIA Corporation (10de)
    Nvidia,
    /// Advanced Micro Devices, Inc. [AMD/ATI] (1002)
    Amd,
    /// Intel Corporation (8086)
    Intel,
    /// Other vendor
    Other(String),
}

impl GpuVendor {
    /// Get the vendor ID as a string
    pub fn vendor_id(&self) -> &str {
        match self {
            GpuVendor::Nvidia => "10de",
            GpuVendor::Amd => "1002",
            GpuVendor::Intel => "8086",
            GpuVendor::Other(id) => id,
        }
    }

    /// Get the vendor name
    pub fn name(&self) -> &str {
        match self {
            GpuVendor::Nvidia => "NVIDIA Corporation",
            GpuVendor::Amd => "Advanced Micro Devices, Inc. [AMD/ATI]",
            GpuVendor::Intel => "Intel Corporation",
            GpuVendor::Other(_) => "Unknown Vendor",
        }
    }

    /// Determine vendor from vendor ID
    pub fn from_vendor_id(vendor_id: &str) -> Self {
        match vendor_id.to_lowercase().as_str() {
            "10de" => GpuVendor::Nvidia,
            "1002" => GpuVendor::Amd,
            "8086" => GpuVendor::Intel,
            other => GpuVendor::Other(other.to_string()),
        }
    }
}

/// GPU allocation manager that tracks which GPUs are allocated to which VMs
#[derive(Default)]
pub struct GpuManager {
    /// Map of GPU PCI addresses to their current allocation status
    allocated_gpus: HashMap<String, bool>,
    /// Map of VM names to their allocated GPU PCI addresses
    vm_gpu_allocations: HashMap<String, HashSet<String>>,
    /// Cache of detected GPUs to avoid rescanning
    gpu_cache: Option<Vec<GpuDevice>>,
}

impl GpuManager {
    /// Create a new GPU manager
    pub fn new() -> Self {
        Self {
            allocated_gpus: HashMap::new(),
            vm_gpu_allocations: HashMap::new(),
            gpu_cache: None,
        }
    }
    
    /// Allocate GPUs for a VM based on requested configuration
    pub fn allocate_gpus(&mut self, vm_name: &str, gpu_configs: &mut Vec<GpuConfig>) -> Result<()> {
        log::info!("Allocating GPUs for VM {}: {:?}", vm_name, gpu_configs);
        
        // Clone the allocation keys first to avoid borrowing conflicts
        let allocation_keys: Vec<String> = self.allocated_gpus.keys()
            .map(|k| k.clone())
            .collect();
            
        // Get all available GPUs
        let available_gpus = self.get_available_gpus()?;
        
        // Create a new set for storing the allocated GPUs for this VM
        let mut allocated_for_vm = HashSet::new();
        
        // Process each GPU config
        for gpu_config in gpu_configs.iter_mut() {
            let model = &gpu_config.model;
            let count = gpu_config.count;
            
            // Find matching GPUs that are not allocated
            let matching_gpus: Vec<&GpuDevice> = available_gpus.iter()
                .filter(|gpu| {
                    // Match by model and not already allocated or in our pending allocation
                    let model_matches = match &gpu.model {
                        Some(gpu_model) => gpu_model == model,
                        None => false
                    };
                    
                    model_matches && 
                    !allocation_keys.contains(&gpu.pci_address) &&
                    !allocated_for_vm.contains(&gpu.pci_address)
                })
                .collect();
            
            // Check if we have enough GPUs
            if matching_gpus.len() < count as usize {
                return Err(anyhow!(
                    "Not enough available GPUs of model {}. Requested: {}, Available: {}",
                    model, count, matching_gpus.len()
                ));
            }
            
            // Allocate the GPUs
            for i in 0..count as usize {
                let gpu = matching_gpus[i];
                allocated_for_vm.insert(gpu.pci_address.clone());
                
                // Add to the config's assigned devices
                gpu_config.assigned_devices.push(GpuDeviceInfo {
                    pci_address: gpu.pci_address.clone(),
                    iommu_group: gpu.iommu_group.clone(),
                    enable_gpudirect: model == "RTX5090" || model.starts_with("H"), // Enable for NVIDIA and Hopper GPUs
                });
            }
        }
        
        // Record the allocations
        for pci_address in &allocated_for_vm {
            self.allocated_gpus.insert(pci_address.clone(), true);
        }
        
        // Record which VM has these GPUs
        self.vm_gpu_allocations.insert(vm_name.to_string(), allocated_for_vm);
        
        Ok(())
    }
    
    /// Release GPUs allocated to a VM
    pub fn release_gpus(&mut self, vm_name: &str) -> Result<()> {
        if let Some(gpu_addresses) = self.vm_gpu_allocations.remove(vm_name) {
            for address in gpu_addresses {
                self.allocated_gpus.insert(address.clone(), false);
                
                // Attempt to unbind from VFIO
                let _ = unbind_gpu_from_vfio(&address); // Ignore errors as we're just cleaning up
            }
        }
        
        Ok(())
    }
    
    /// Get list of available GPUs, refreshing the cache if needed
    pub fn get_available_gpus(&mut self) -> Result<&Vec<GpuDevice>> {
        if self.gpu_cache.is_none() {
            self.refresh_gpu_cache()?;
        }
        
        Ok(self.gpu_cache.as_ref().unwrap())
    }
    
    /// Force a refresh of the GPU cache
    pub fn refresh_gpu_cache(&mut self) -> Result<()> {
        let gpus = detect_gpus()?;
        
        // Update allocation status for known GPUs
        for gpu in &gpus {
            if !self.allocated_gpus.contains_key(&gpu.pci_address) {
                self.allocated_gpus.insert(gpu.pci_address.clone(), false);
            }
        }
        
        self.gpu_cache = Some(gpus);
        Ok(())
    }
    
    /// Prepare GPUs for a VM (bind to VFIO)
    pub fn prepare_gpus_for_vm(&self, gpu_devices: &[GpuDeviceInfo]) -> Result<Vec<String>> {
        let mut vfio_paths = Vec::new();
        
        for device in gpu_devices {
            // Bind the GPU to VFIO
            let vfio_path = bind_gpu_to_vfio(&device.pci_address)?;
            vfio_paths.push(vfio_path);
        }
        
        Ok(vfio_paths)
    }
}

/// Infer GPU model from device information
fn infer_gpu_model(device: &GpuDevice) -> Option<String> {
    if let Some(name) = &device.name {
        let name_lower = name.to_lowercase();
        
        // NVIDIA models
        if name_lower.contains("rtx 5090") || name_lower.contains("rtx5090") {
            return Some("RTX5090".to_string());
        }
        
        if name_lower.contains("h100") {
            return Some("H100".to_string());
        }
        
        if name_lower.contains("h200") {
            return Some("H200".to_string());
        }
        
        if name_lower.contains("b200") {
            return Some("B200".to_string());
        }
    }
    
    // If no match found, use vendor/device info to make a best guess
    if device.vendor_id == "10de" { // NVIDIA
        return Some("RTX5090".to_string()); // Default to RTX5090 for NVIDIA
    } else if device.vendor_id == "1002" { // AMD
        return Some("B200".to_string()); // Default to B200 for AMD
    }
    
    None
}

/// Detects all GPU devices on the host system
pub fn detect_gpus() -> Result<Vec<GpuDevice>> {
    let devices_path = Path::new("/sys/bus/pci/devices");
    let vfio_devices_path = Path::new("/dev/vfio");
    
    let mut gpus = Vec::new();
    
    // Check if the directory exists
    if !devices_path.exists() {
        return Err(anyhow!("PCI devices path does not exist: {:?}", devices_path));
    }
    
    // Iterate through all PCI devices
    for entry in fs::read_dir(devices_path)? {
        let entry = entry?;
        let path = entry.path();
        
        // Read class ID to identify GPUs (0x030000 for display controllers)
        let class_path = path.join("class");
        if !class_path.exists() {
            continue;
        }
        
        let class_id = fs::read_to_string(&class_path)?.trim().to_string();
        
        // Check if this is a display controller (GPU)
        // 0x0300xx is for VGA compatible devices
        // 0x0301xx is for XGA compatible devices
        // 0x0302xx is for 3D controllers (e.g., compute GPUs)
        if !class_id.starts_with("0x0300") && !class_id.starts_with("0x0301") && !class_id.starts_with("0x0302") {
            continue;
        }
        
        let pci_address = path.file_name()
            .and_then(|name| name.to_str())
            .map(|name| name.to_string())
            .ok_or_else(|| anyhow!("Invalid PCI device path: {:?}", path))?;
        
        // Read vendor ID
        let vendor_path = path.join("vendor");
        let vendor_id = fs::read_to_string(&vendor_path)?
            .trim()
            .trim_start_matches("0x")
            .to_string();
        
        // Read device ID
        let device_path = path.join("device");
        let device_id = fs::read_to_string(&device_path)?
            .trim()
            .trim_start_matches("0x")
            .to_string();
        
        // Check current driver
        let driver_path = path.join("driver");
        let current_driver = if driver_path.exists() {
            driver_path.read_link().ok()
                .and_then(|link| {
                    // Convert to string then extract the file name
                    link.to_string_lossy().rsplit('/').next().map(|s| s.to_string())
                })
        } else {
            None
        };
        
        // Check if bound to VFIO
        let is_vfio_bound = current_driver.as_ref().map_or(false, |d| d == "vfio-pci");
        
        // Get IOMMU group
        let iommu_group_path = path.join("iommu_group");
        let iommu_group = if iommu_group_path.exists() {
            iommu_group_path.read_link().ok()
                .and_then(|link| {
                    // Convert to string then extract the file name
                    link.to_string_lossy().rsplit('/').next().map(|s| s.to_string())
                })
        } else {
            None
        };
        
        // Get related devices in the same IOMMU group
        let mut related_devices = Vec::new();
        if let Some(group) = &iommu_group {
            let group_path = Path::new("/sys/kernel/iommu_groups")
                .join(group)
                .join("devices");
            
            if group_path.exists() {
                for entry in fs::read_dir(&group_path)? {
                    let entry = entry?;
                    let related_path = entry.path();
                    if let Some(name) = related_path.file_name().and_then(|n| n.to_str()) {
                        if name != pci_address {
                            related_devices.push(name.to_string());
                        }
                    }
                }
            }
        }
        
        // Try to get a human-readable name
        let name = get_device_name(&vendor_id, &device_id);
        
        let mut gpu_device = GpuDevice {
            pci_address,
            vendor_id,
            device_id,
            name,
            current_driver,
            is_vfio_bound,
            iommu_group,
            related_devices,
            model: None,
            assigned: false,
        };
        
        // Infer the GPU model
        gpu_device.model = infer_gpu_model(&gpu_device);
        
        if gpu_device.model.is_some() {
            gpus.push(gpu_device);
        }
    }
    
    Ok(gpus)
}

/// Get a human-readable name for the device from vendor and device IDs
fn get_device_name(vendor_id: &str, device_id: &str) -> Option<String> {
    // Try to use lspci to get the name
    let output = Command::new("lspci")
        .args(["-d", &format!("{}:{}", vendor_id, device_id), "-nn"])
        .output()
        .ok()?;
    
    if output.status.success() {
        let output_str = String::from_utf8_lossy(&output.stdout);
        // Extract the device name from lspci output
        let line = output_str.lines().next()?;
        let mut parts = line.splitn(2, ':');
        parts.next(); // Skip the PCI address
        parts.next().map(|s| s.trim().to_string())
    } else {
        None
    }
}

/// Binds a GPU to the VFIO driver for passthrough
pub fn bind_gpu_to_vfio(pci_address: &str) -> Result<String> {
    info!("Binding GPU {} to VFIO driver", pci_address);
    
    // Check if VFIO is loaded and supported
    ensure_vfio_support()?;
    
    // Get the full device path
    let device_path = PathBuf::from("/sys/bus/pci/devices").join(pci_address);
    if !device_path.exists() {
        return Err(anyhow!("Device {} not found", pci_address));
    }
    
    // Get vendor and device IDs
    let vendor_id = fs::read_to_string(device_path.join("vendor"))?
        .trim()
        .trim_start_matches("0x")
        .to_string();
    
    let device_id = fs::read_to_string(device_path.join("device"))?
        .trim()
        .trim_start_matches("0x")
        .to_string();
    
    // Check current driver
    let driver_path = device_path.join("driver");
    if driver_path.exists() {
        // Check if already bound to vfio-pci
        if let Ok(link) = driver_path.read_link() {
            if let Some(driver_name) = link.file_name() {
                if driver_name == "vfio-pci" {
                    info!("Device {} is already bound to vfio-pci", pci_address);
                    
                    // Get IOMMU group
                    let iommu_group = fs::read_link(device_path.join("iommu_group"))?
                        .file_name()
                        .and_then(|name| name.to_str())
                        .ok_or_else(|| anyhow!("Cannot determine IOMMU group"))?
                        .to_string();
                    
                    return Ok(format!("/dev/vfio/{}", iommu_group));
                }
                
                // Unbind from current driver
                info!("Unbinding from current driver: {:?}", driver_name);
                let unbind_path = driver_path.join("unbind");
                fs::write(&unbind_path, pci_address)
                    .with_context(|| format!("Failed to unbind {} from driver", pci_address))?;
            }
        }
    }
    
    // Bind to vfio-pci
    info!("Binding {} to vfio-pci", pci_address);
    
    // Check if the device ID is already known to the vfio-pci driver
    let vfio_new_id_path = PathBuf::from("/sys/bus/pci/drivers/vfio-pci/new_id");
    
    // Write vendor and device ID to new_id
    let new_id = format!("{} {}", vendor_id, device_id);
    match fs::write(&vfio_new_id_path, &new_id) {
        Ok(_) => debug!("Registered new device ID: {}", new_id),
        Err(e) => {
            // If the error is EEXIST, the device ID is already registered, which is fine
            if e.kind() != io::ErrorKind::AlreadyExists {
                warn!("Could not write to new_id: {}. Trying direct bind.", e);
            }
        }
    }
    
    // Check if the device is now bound to vfio-pci
    let driver_path = device_path.join("driver");
    if !driver_path.exists() || driver_path.read_link()?.file_name() != Some("vfio-pci".as_ref()) {
        // Try direct bind if the device is not yet bound
        let bind_path = PathBuf::from("/sys/bus/pci/drivers/vfio-pci/bind");
        fs::write(&bind_path, pci_address)
            .with_context(|| format!("Failed to bind {} to vfio-pci", pci_address))?;
    }
    
    // Get IOMMU group
    let iommu_group = fs::read_link(device_path.join("iommu_group"))?
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| anyhow!("Cannot determine IOMMU group"))?
        .to_string();
    
    let vfio_path = format!("/dev/vfio/{}", iommu_group);
    
    // Check if the VFIO device exists
    if !Path::new(&vfio_path).exists() {
        return Err(anyhow!("VFIO device {} does not exist after binding", vfio_path));
    }
    
    info!("Successfully bound GPU {} to VFIO, path: {}", pci_address, vfio_path);
    
    Ok(vfio_path)
}

/// Unbinds a GPU from the VFIO driver
pub fn unbind_gpu_from_vfio(pci_address: &str) -> Result<()> {
    info!("Unbinding GPU {} from VFIO driver", pci_address);
    
    // Get the full device path
    let device_path = PathBuf::from("/sys/bus/pci/devices").join(pci_address);
    if !device_path.exists() {
        return Err(anyhow!("Device {} not found", pci_address));
    }
    
    // Check current driver
    let driver_path = device_path.join("driver");
    if !driver_path.exists() {
        return Err(anyhow!("Device {} is not bound to any driver", pci_address));
    }
    
    let driver_link = driver_path.read_link()?;
    let current_driver = driver_link
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| anyhow!("Cannot determine current driver"))?
        .to_string();
    
    if current_driver != "vfio-pci" {
        warn!("Device {} is not bound to vfio-pci, but to {}", pci_address, current_driver);
        return Ok(());
    }
    
    // Unbind from vfio-pci
    let unbind_path = driver_path.join("unbind");
    fs::write(&unbind_path, pci_address)
        .with_context(|| format!("Failed to unbind {} from vfio-pci", pci_address))?;
    
    info!("Successfully unbound GPU {} from VFIO", pci_address);
    
    Ok(())
}

/// Ensures that VFIO support is available on the system
fn ensure_vfio_support() -> Result<()> {
    // Check if VFIO modules are loaded
    let vfio_loaded = Path::new("/sys/module/vfio").exists();
    let vfio_pci_loaded = Path::new("/sys/module/vfio_pci").exists();
    let vfio_iommu_type1_loaded = Path::new("/sys/module/vfio_iommu_type1").exists();
    
    if !vfio_loaded || !vfio_pci_loaded || !vfio_iommu_type1_loaded {
        // Try to load the modules
        info!("Loading VFIO kernel modules");
        
        if !vfio_iommu_type1_loaded {
            if let Err(e) = Command::new("modprobe").arg("vfio_iommu_type1").output() {
                return Err(anyhow!("Failed to load vfio_iommu_type1 module: {}", e));
            }
            
            // Enable unsafe interrupts for compatibility
            let unsafe_interrupts_path = Path::new("/sys/module/vfio_iommu_type1/parameters/allow_unsafe_interrupts");
            if unsafe_interrupts_path.exists() {
                fs::write(unsafe_interrupts_path, "1")
                    .context("Failed to enable unsafe interrupts for VFIO")?;
            }
        }
        
        if !vfio_pci_loaded {
            if let Err(e) = Command::new("modprobe").arg("vfio_pci").output() {
                return Err(anyhow!("Failed to load vfio_pci module: {}", e));
            }
        }
        
        if !vfio_loaded {
            if let Err(e) = Command::new("modprobe").arg("vfio").output() {
                return Err(anyhow!("Failed to load vfio module: {}", e));
            }
        }
    }
    
    // Check if IOMMU is enabled
    let dmar_enabled = fs::read_to_string("/proc/cmdline")?
        .contains("intel_iommu=on") || 
        fs::read_to_string("/proc/cmdline")?
        .contains("amd_iommu=on");
    
    if !dmar_enabled {
        warn!("IOMMU does not appear to be enabled. GPU passthrough may not work correctly.");
        warn!("To enable IOMMU, add 'intel_iommu=on' or 'amd_iommu=on' to your kernel cmdline.");
    }
    
    // Check if /dev/vfio exists
    if !Path::new("/dev/vfio").exists() {
        return Err(anyhow!("/dev/vfio directory does not exist. VFIO is not properly set up."));
    }
    
    Ok(())
}

/// Extract PCI address from a VFIO device path
pub fn extract_pci_address(path: &str) -> Result<String> {
    // Handle paths in the format /sys/bus/pci/devices/0000:01:00.0/
    if path.contains("/sys/bus/pci/devices/") {
        let parts: Vec<&str> = path.split('/').collect();
        for (i, part) in parts.iter().enumerate() {
            if *part == "devices" && i + 1 < parts.len() {
                return Ok(parts[i + 1].to_string());
            }
        }
    }
    
    Err(anyhow!("Could not extract PCI address from path: {}", path))
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_extract_pci_address() {
        let path = "/sys/bus/pci/devices/0000:01:00.0/";
        assert_eq!(extract_pci_address(path).unwrap(), "0000:01:00.0");
        
        let path = "/sys/bus/pci/devices/0000:01:00.0";
        assert_eq!(extract_pci_address(path).unwrap(), "0000:01:00.0");
    }
    
    #[test]
    fn test_gpu_vendor() {
        assert_eq!(GpuVendor::from_vendor_id("10de").vendor_id(), "10de");
        assert_eq!(GpuVendor::from_vendor_id("10DE").vendor_id(), "10de");
        assert_eq!(GpuVendor::from_vendor_id("1002").vendor_id(), "1002");
        assert_eq!(GpuVendor::from_vendor_id("8086").vendor_id(), "8086");
    }
} 