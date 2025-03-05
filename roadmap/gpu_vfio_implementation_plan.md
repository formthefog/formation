# GPU Passthrough Implementation Plan for Formation

This document outlines the plan for implementing GPU passthrough via VFIO in the Formation system, allowing virtual machines to directly access and utilize physical GPUs on the host system.

## Background and Context

The Formation system currently provides compute resources via the form-vmm service, which is based on Cloud Hypervisor. To support modern AI and compute-intensive workloads, we need to enable GPU passthrough so that virtual machines can directly access physical GPUs on the host machine.

VFIO (Virtual Function I/O) is a Linux kernel framework that allows safe, non-privileged userspace drivers to directly access hardware devices. Cloud Hypervisor already has VFIO support built in, which we will leverage to implement GPU passthrough.

## Current Architecture Analysis

Based on our review of the code:

1. **VM Instance Configuration** (`VmInstanceConfig` in `form-vmm/vmm-service/src/instance/config.rs`)
   - This struct defines the VM configuration but currently doesn't include any GPU or VFIO device settings

2. **VMM Service** (`form-vmm/vmm-service/src/service/vmm.rs`)
   - Handles various VM lifecycle operations
   - Has methods for creating and managing VMs
   - Includes API methods for adding devices, but doesn't explicitly handle GPU devices

3. **Cloud Hypervisor API** (`form-vmm/vmm/src/api/mod.rs`)
   - Provides the `VmAddDevice` API that allows adding devices to a VM
   - Uses the `DeviceConfig` struct to specify device details

4. **Device Manager** (`form-vmm/vmm/src/device_manager.rs`)
   - Includes methods to add VFIO devices to VMs
   - Has methods like `add_device` and `add_vfio_device` for device passthrough

5. **VFIO Documentation** (`form-vmm/docs/vfio.md`)
   - Provides instructions on how to bind host PCI devices to the VFIO driver
   - Details how to pass these devices to Cloud Hypervisor VMs
   - Includes examples and advanced configuration options for GPU passthrough

## Implementation Plan

### 1. Add GPU Configuration Support

1. **Update `VmInstanceConfig` in `config.rs`**
   - Add fields for GPU device paths and configuration
   - Example:
   ```rust
   pub struct VmInstanceConfig {
       // ... existing fields
       pub gpu_devices: Option<Vec<String>>, // Paths to GPU devices
       pub gpu_options: Option<GpuOptions>,  // Additional GPU configuration options
   }

   pub struct GpuOptions {
       pub enable_nvidia_gpudirect: bool,
       pub nvidia_gpudirect_clique: Option<u8>,
   }
   ```

2. **Update `Config` validation in `config.rs`**
   - Add validation for the GPU device paths
   - Ensure the paths exist and are valid VFIO device paths

### 2. Implement GPU Device Binding

1. **Create a new module `gpu.rs` in `form-vmm/vmm-service/src/`**
   - Implement functions to:
     - Detect available GPUs on the host system
     - Bind/unbind GPUs from their native drivers to the VFIO driver
     - Get VFIO device paths for bound GPUs

2. **Example Implementation for `gpu.rs`**:
   ```rust
   pub fn detect_gpus() -> Result<Vec<GpuDevice>, Error> {
       // Use lspci or sysfs to detect GPUs
   }

   pub fn bind_gpu_to_vfio(pci_address: &str) -> Result<String, Error> {
       // Unbind from native driver
       // Bind to VFIO driver
       // Return the path to the VFIO device
   }

   pub fn unbind_gpu_from_vfio(pci_address: &str) -> Result<(), Error> {
       // Unbind from VFIO driver
       // Bind back to native driver
   }
   ```

### 3. Update VMM Service to Handle GPU Passthrough

1. **Modify `create` method in `vmm.rs`**
   - Add logic to process GPU devices specified in the configuration
   - Bind GPUs to VFIO if not already bound
   - Add the VFIO devices to the VM

2. **Implement a new API endpoint for GPU management**
   - Add a new API endpoint to add/remove GPUs from a running VM
   - This could be an extension of the existing `add_device` functionality

3. **Example implementation for handling GPUs in `create`**:
   ```rust
   // In the create method
   if let Some(gpu_devices) = &config.gpu_devices {
       for gpu_path in gpu_devices {
           let mut device_cfg = DeviceConfig {
               path: PathBuf::from(gpu_path),
               iommu: false,
               id: None,
               pci_segment: 0,
               x_nv_gpudirect_clique: config.gpu_options.as_ref().and_then(|opts| opts.nvidia_gpudirect_clique),
           };
           
           let device_request = VmAddDevice.send(
               self.api_evt.try_clone().map_err(VmmError::EventFdClone)?,
               self.api_sender.clone(),
               device_cfg,
           )?;
       }
   }
   ```

### 4. Update VM Lifecycle Management

1. **Modify `shutdown` and `delete` methods**
   - Add logic to properly release GPU resources when a VM is shutdown or deleted
   - Unbind GPUs from VFIO if configured to do so

2. **Example implementation for cleaning up GPUs**:
   ```rust
   // In the shutdown or delete method
   if let Some(gpu_devices) = &config.gpu_devices {
       for gpu_path in gpu_devices {
           // Extract PCI address from the path
           let pci_address = extract_pci_address(gpu_path)?;
           
           // Unbind GPU from VFIO if needed
           if config.gpu_options.as_ref().map_or(false, |opts| opts.unbind_on_shutdown) {
               gpu::unbind_gpu_from_vfio(&pci_address)?;
           }
       }
   }
   ```

### 5. Add Client-Side Support

1. **Update CLI and API Client**
   - Add command-line options for specifying GPUs
   - Example: `form-vmm-ctl create --name test-vm --gpu /sys/bus/pci/devices/0000:01:00.0/`

2. **Example CLI addition**:
   ```rust
   // In the CLI code
   let gpu_devices = matches.values_of("gpu")
       .map(|v| v.map(|s| s.to_string()).collect::<Vec<String>>());
   
   let gpu_options = GpuOptions {
       enable_nvidia_gpudirect: matches.is_present("nvidia-gpudirect"),
       nvidia_gpudirect_clique: matches.value_of("nvidia-gpudirect-clique").map(|s| s.parse::<u8>().unwrap()),
   };
   
   config.gpu_devices = gpu_devices;
   config.gpu_options = Some(gpu_options);
   ```

### 6. Documentation and Testing

1. **Update documentation**
   - Add a new document explaining GPU passthrough in Formation
   - Include examples and requirements for different GPU types (NVIDIA, AMD, Intel)

2. **Create test cases**
   - Unit tests for the new GPU functionality
   - Integration tests for GPU passthrough
   - Performance tests to validate GPU performance in VMs

## Advanced Features (Future Work)

1. **GPU Sharing and Time-sharing**
   - Allow multiple VMs to share a GPU via time-sharing
   - Implement scheduling and resource allocation

2. **GPU Live Migration Support**
   - Enable live migration of VMs with passthrough GPUs

3. **NVIDIA vGPU Support**
   - Add support for NVIDIA vGPU technology for hardware-accelerated GPU sharing

4. **Multi-GPU Support**
   - Enable configuring multiple GPUs with different types and configurations

## Conclusion

This implementation plan provides a roadmap for adding GPU passthrough support to the Formation system. By leveraging the existing VFIO support in Cloud Hypervisor, we can enable GPU passthrough for compute-intensive workloads, significantly enhancing the Formation system's capabilities for AI and other GPU-accelerated applications. 