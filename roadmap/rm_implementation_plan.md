# Remove Commands Implementation Plan

## Overview

The `remove` (or `rm`) commands allow a developer to remove resources from a running VM instance. These commands enable dynamic reduction of VM resources without requiring a restart. The implementation supports three primary resource types:

1. `rm disk` - Detach disk storage from a running VM
2. `rm fs` - Unmount filesystem shares from a running VM
3. `rm device` - Disconnect devices from a running VM

## Workflow

1. Developer identifies a resource that needs to be removed from a running VM
2. Developer executes the appropriate `form manage rm` subcommand with the required parameters
3. The system:
   - Validates the VM instance is in a valid state for modification
   - Verifies the requested resource exists and can be safely removed
   - Uses Cloud Hypervisor's API to detach the resource from the running VM
   - Updates the instance configuration to reflect the removal of the resource

## Implementation Components

### 1. CLI Command Structure

#### Disk Command

```rust
#[derive(Clone, Debug, Args)]
pub struct RemoveDiskCommand {
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
    
    /// ID of the disk to remove (as returned when the disk was added)
    #[clap(long, required = true)]
    pub disk_id: String,
    
    /// Send request via queue instead of direct API call
    #[clap(long)]
    pub queue: bool,
}
```

#### Filesystem Command

```rust
#[derive(Clone, Debug, Args)]
pub struct RemoveFilesystemCommand {
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
    
    /// ID of the filesystem to remove (as returned when the filesystem was added)
    #[clap(long, required = true)]
    pub fs_id: String,
    
    /// Send request via queue instead of direct API call
    #[clap(long)]
    pub queue: bool,
}
```

#### Device Command

```rust
#[derive(Clone, Debug, Args)]
pub struct RemoveDeviceCommand {
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
    
    /// ID of the device to remove (as returned when the device was added)
    #[clap(long, required = true)]
    pub device_id: String,
    
    /// Send request via queue instead of direct API call
    #[clap(long)]
    pub queue: bool,
}
```

### 2. Backend Implementation

#### VMM Service Communication

All remove commands will communicate with the VMM service using the same API endpoint:

- `vm.remove-device` - For removing disks, filesystems, and other devices

The implementation will:
1. Create a `VmRemoveDevice` request with the appropriate ID
2. Sign the request for authentication
3. Send the request to the VMM API or through the message queue
4. Handle and report responses back to the user

#### Request Handling

The implementation will need to:
1. Verify the VM exists and is in a suitable state
2. Confirm the requested device/resource exists
3. Handle any errors that might occur during the removal process
4. Update the VM configuration to reflect the changes

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

### Safety
- Check if the device/resource is in use before removal
- Ensure data integrity for disk detachment
- Handle potentially stuck operations gracefully

### System State
- Update VM configuration after successful removal
- Handle partial failures appropriately
- Maintain consistent state between the CLI's view and the actual VM state

### User Experience
- Provide clear feedback on successful removal
- Offer guidance when errors occur
- Consider dry-run option for testing removal without actually performing it

## Future Enhancements
- Support for batch operations to remove multiple resources at once
- Automatic cleanup of resources after removal
- Resource dependency tracking to prevent unsafe removals 