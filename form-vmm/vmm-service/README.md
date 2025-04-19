# vmm-service

This directory contains the Formation Virtual Machine Manager (VMM) service, which provides VM creation, management, and lifecycle control for the Formation platform.

## Overview

The vmm-service is responsible for:

1. Creating and managing virtual machines
2. Provisioning VM resources (CPU, memory, storage)
3. Configuring VM networking
4. Managing VM lifecycle (start, stop, pause, resume)
5. VM image management and snapshots
6. VM monitoring and health checks

## Building the Service

### Prerequisites

- Rust toolchain (1.58 or newer)
- Docker (if building containerized version)
- Formation base image (for containerized version)
- Virtualization dependencies:
  - qemu-kvm
  - libvirt-dev
  - libseccomp-dev
  - Linux headers for your kernel

### Build Steps

#### Local Build

```bash
# Build the service
cargo build --release --bin vmm-service

# Run tests
cargo test --package vmm-service
```

#### Docker Build

```bash
# From the project root
docker build -t formation/vmm-service:latest -f vmm-service/Dockerfile .

# Or using the Makefile
cd docker
make vmm-service
```

## Configuration

The service can be configured using:

1. Configuration file (default: `/etc/formation/vmm/default.conf`)
2. Environment variables
3. Command line arguments

### Key Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `VMM_CONFIG_PATH` | Path to configuration file | `/etc/formation/vmm/default.conf` |
| `VMM_LOG_LEVEL` | Logging level (debug, info, warn, error) | `info` |
| `VMM_API_PORT` | API port to listen on | `3003` |
| `VMM_STATE_URL` | URL for the state service | `http://form-state:3004` |
| `VMM_KERNEL_PATH` | Path to the hypervisor firmware | `/var/lib/formation/kernel/hypervisor-fw` |
| `VMM_VM_DIR` | Directory for VM runtime files | `/run/form-vm` |
| `VMM_IMAGES_DIR` | Directory for VM images | `/var/lib/formation/vm-images` |
| `WAIT_FOR` | Comma-separated list of services to wait for (host:port format) | `` |

### Configuration File

See `config/default.conf` for a fully documented example configuration file.

## Running the Service

### Directly

```bash
vmm-service --config /path/to/config.conf
```

### Using Docker

```bash
docker run -d \
  --name vmm-service \
  --privileged \
  -p 3003:3003 \
  -v /path/to/config:/etc/formation/vmm \
  -v /path/to/vm-images:/var/lib/formation/vm-images \
  -v /path/to/kernel:/var/lib/formation/kernel \
  -v /run/form-vm:/run/form-vm \
  formation/vmm-service:latest
```

### Dependencies

This service has the following dependencies:

- `form-state` - For storing and retrieving VM configurations
- `formnet` - For VM networking (optional)
- Access to KVM virtualization (`/dev/kvm`)

## API Documentation

The service provides a RESTful API for VM management. Key endpoints include:

- `/health` - Service health check
- `/vms` - List all VMs
- `/vms/{id}` - Get VM details
- `/vms/{id}/start` - Start a VM
- `/vms/{id}/stop` - Stop a VM
- `/vms/{id}/restart` - Restart a VM
- `/vms/{id}/pause` - Pause a VM
- `/vms/{id}/resume` - Resume a VM
- `/images` - List available VM images

## VM Images

The service supports several VM image formats:

- Raw disk images (`.img`)
- QCOW2 images (`.qcow2`)
- Virtual machine disk images (`.vmdk`)
- Custom Formation images (`.form`)

Images should be placed in the `/var/lib/formation/vm-images` directory.

## Testing

### Unit Tests

```bash
cargo test --package vmm-service
```

### Integration Testing

```bash
# Health check
curl http://localhost:3003/health

# List VMs
curl http://localhost:3003/vms
```

## Directories

- `/var/lib/formation/vm-images` - VM disk images
- `/var/lib/formation/kernel` - Hypervisor firmware
- `/run/form-vm` - Runtime VM files (sockets, locks, etc.)
- `/etc/formation/vmm` - Configuration

## Troubleshooting

Common issues:

1. **Unable to access KVM**: Ensure the host has KVM virtualization enabled and `/dev/kvm` is accessible
2. **VM startup failure**: Check kernel firmware path and permissions
3. **Network connectivity issues**: Verify formnet service is running and bridges are configured correctly
4. **Resource allocation failures**: Check host system resources (memory, CPU, disk space)

### Debugging with Elevated Privileges

The vmm-service often requires elevated privileges for VM management. When troubleshooting:

```bash
# Check KVM access
ls -la /dev/kvm

# Verify loaded kernel modules
lsmod | grep kvm

# Check VM process status
ps aux | grep qemu
```

## Security Considerations

The vmm-service requires elevated privileges to manage VMs. In production:

1. Use seccomp profiles to restrict system calls
2. Configure fine-grained capabilities instead of running as root
3. Isolate VM networks from host network
4. Implement resource limits to prevent denial of service 