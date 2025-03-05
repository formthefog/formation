# Add Commands Implementation Plan

## Overview

The `add` commands allow a developer to add resources to a running VM instance. These commands enable dynamic expansion of VM capabilities without requiring a restart. The implementation supports three primary resource types:

1. `add disk` - Attach new disk storage to a running VM
2. `add fs` - Mount new filesystem shares to a running VM
3. `add device` - Connect new devices to a running VM (e.g., PCI, VFIO)

## Workflow

1. Developer identifies a running VM instance that needs additional resources
2. Developer executes the appropriate `form manage add` subcommand with the required parameters
3. The system:
   - Validates the VM instance is in a valid state for modification
   - Prepares the requested resource (disk, filesystem, or device)
   - Uses Cloud Hypervisor's API to attach the resource to the running VM
   - Updates the instance configuration to reflect the new attached resource

## Implementation Components

### 1. CLI Command Structure

#### Disk Command

```rust
#[derive(Clone, Debug, Args)]
pub struct AddDiskCommand {
    /// The ID of the instance to modify
    #[clap(long, short)]
    pub id: Option<String>,
    
    /// The name of the instance to modify, an alternative to ID
    #[clap(long, short)]
    pub name: Option<String>,
    
    /// Private key file for authentication
    #[clap(long)]
    pub private_key: Option<String>,
    
    /// Keyfile containing the private key
    #[clap(long)]
    pub keyfile: Option<String>,
    
    /// Mnemonic for key derivation
    #[clap(long)]
    pub mnemonic: Option<String>,
    
    /// Path to the disk image file to add
    #[clap(long, required = true)]
    pub path: String,
    
    /// Set disk as read-only
    #[clap(long)]
    pub readonly: bool,
    
    /// Use direct I/O for better performance
    #[clap(long)]
    pub direct: bool,
    
    /// Enable IOMMU for this disk
    #[clap(long)]
    pub iommu: bool,
    
    /// Optional disk identifier
    #[clap(long)]
    pub id: Option<String>,
    
    /// Send request via queue instead of direct API call
    #[clap(long)]
    pub queue: bool,
}
```

#### Filesystem Command

```rust
#[derive(Clone, Debug, Args)]
pub struct AddFilesystemCommand {
    /// The ID of the instance to modify
    #[clap(long, short)]
    pub id: Option<String>,
    
    /// The name of the instance to modify, an alternative to ID
    #[clap(long, short)]
    pub name: Option<String>,
    
    /// Private key file for authentication
    #[clap(long)]
    pub private_key: Option<String>,
    
    /// Keyfile containing the private key
    #[clap(long)]
    pub keyfile: Option<String>,
    
    /// Mnemonic for key derivation
    #[clap(long)]
    pub mnemonic: Option<String>,
    
    /// Path to the directory to share with the VM
    #[clap(long, required = true)]
    pub source: String,
    
    /// Mount tag to identify this filesystem in the guest
    #[clap(long, required = true)]
    pub tag: String,
    
    /// Socket path for the virtiofsd daemon
    #[clap(long)]
    pub socket: Option<String>,
    
    /// Send request via queue instead of direct API call
    #[clap(long)]
    pub queue: bool,
}
```

#### Device Command

```rust
#[derive(Clone, Debug, Args)]
pub struct AddDeviceCommand {
    /// The ID of the instance to modify
    #[clap(long, short)]
    pub id: Option<String>,
    
    /// The name of the instance to modify, an alternative to ID
    #[clap(long, short)]
    pub name: Option<String>,
    
    /// Private key file for authentication
    #[clap(long)]
    pub private_key: Option<String>,
    
    /// Keyfile containing the private key
    #[clap(long)]
    pub keyfile: Option<String>,
    
    /// Mnemonic for key derivation
    #[clap(long)]
    pub mnemonic: Option<String>,
    
    /// Path to the device (for VFIO devices)
    #[clap(long)]
    pub path: Option<String>,
    
    /// ID of the device to add
    #[clap(long)]
    pub device_id: Option<String>,
    
    /// Send request via queue instead of direct API call
    #[clap(long)]
    pub queue: bool,
}
```

### 2. Backend Implementation

#### VMM Service Communication

Each add command will need to communicate with the VMM service using the appropriate API endpoint:

1. `vm.add-disk` - For adding disk devices
2. `vm.add-fs` - For adding filesystem mounts
3. `vm.add-device` - For adding generic devices (incl. VFIO)

The implementation will:
1. Create the appropriate request structure based on command-line parameters
2. Sign the request for authentication
3. Send the request to the VMM API or through the message queue
4. Handle and report responses back to the user

#### Request Types

We'll need to implement or use existing request types for each operation:

1. `DiskConfig` - For disk addition
2. `FsConfig` - For filesystem addition
3. `VmAddDevice` - For device addition

### 3. Implementation Phases

#### Phase 1: Foundation
- Update the command structs with necessary parameters
- Implement basic request validation
- Add placeholder handlers that print "Not yet implemented"

#### Phase 2: Direct API Implementation
- Implement the `handle` methods for each command
- Add proper error handling and user feedback
- Test direct API communication with the VMM service

#### Phase 3: Queue Implementation
- Implement the `handle_queue` methods for each command
- Add proper signing and queue message formatting
- Test queue-based communication

#### Phase 4: Documentation and Testing
- Document usage and examples
- Create integration tests
- Update user documentation

## Considerations

### Security
- Ensure proper authentication for sensitive operations
- Validate all user input before processing
- Consider access controls for different resource types

### Performance
- Use direct I/O when appropriate for disk operations
- Consider optimal parameters for filesystem sharing
- Monitor resource usage after adding new components

### Reliability
- Implement robust error handling
- Provide clear feedback on failures
- Validate system state before and after modifications

## Future Enhancements
- Support for hot-adding CPUs and memory
- Integration with resource monitoring
- Batch operations for adding multiple resources at once 