# VMM Service API

The Virtual Machine Monitor (VMM) Service API provides endpoints for managing virtual machines (VMs) on the Formation cloud. This API allows for the creation, modification, and management of VM instances.

## API Overview

The VMM Service operates on port 3002 by default and exposes a REST API for VM management. This service forms the core of Formation's virtualization capabilities.

> **Note**: The low-level VMM APIs are accessible only via UNIX sockets by the VMM service itself. The endpoints documented here are the higher-level REST APIs that applications and users interact with.

## Authentication

API requests to the VMM Service require authentication using Ethereum wallet signatures through the `Authorization` header, which should contain a JWT token signed by an authorized account.

## Endpoints

### Health Check

```
GET /health
```

Verifies that the VMM service is running and responsive.

**Response**: 
- 200 OK: `"healthy"`

### Ping

```
POST /ping
```

Verifies that the VMM service can process requests.

**Request Body**:
```json
{
  "message": "ping"
}
```

**Response**: 
- 200 OK:
```json
{
  "message": "pong",
  "timestamp": 1677721600
}
```

### Create VM

```
POST /vm/create
```

Creates a new virtual machine with the specified configuration.

**Request Body**:
```json
{
  "name": "my-instance",
  "owner": "0x1234567890abcdef1234567890abcdef12345678",
  "vm_config": {
    "vcpu_count": 2,
    "mem_size_mib": 2048,
    "disk_size": 10,
    "boot_args": "console=ttyS0 reboot=k panic=1 pci=off",
    "kernel": "vmlinux.bin",
    "rootfs": "rootfs.ext4"
  }
}
```

**Response**:
- 200 OK:
```json
{
  "success": true,
  "vm_id": "build-123456789",
  "message": "VM created successfully"
}
```

- Error:
```json
{
  "success": false,
  "message": "Error creating VM: insufficient resources"
}
```

### Boot VM

```
POST /vm/boot
```

Boots a previously created VM.

**Request Body**:
```json
{
  "build_id": "build-123456789",
  "owner": "0x1234567890abcdef1234567890abcdef12345678"
}
```

**Response**:
- 200 OK:
```json
{
  "success": true,
  "message": "VM boot initiated"
}
```

### Start VM

```
POST /vm/start
```

Starts a stopped VM.

**Request Body**:
```json
{
  "build_id": "build-123456789",
  "owner": "0x1234567890abcdef1234567890abcdef12345678"
}
```

**Response**:
- 200 OK:
```json
{
  "success": true,
  "message": "VM started successfully"
}
```

### Stop VM

```
POST /vm/stop
```

Stops a running VM.

**Request Body**:
```json
{
  "build_id": "build-123456789",
  "owner": "0x1234567890abcdef1234567890abcdef12345678",
  "force": false
}
```

**Response**:
- 200 OK:
```json
{
  "success": true,
  "message": "VM stopped successfully"
}
```

### Reboot VM

```
POST /vm/reboot
```

Reboots a running VM.

**Request Body**:
```json
{
  "build_id": "build-123456789",
  "owner": "0x1234567890abcdef1234567890abcdef12345678"
}
```

**Response**:
- 200 OK:
```json
{
  "success": true,
  "message": "VM rebooted successfully"
}
```

### Delete VM

```
POST /vm/delete
```

Deletes a VM and frees its resources.

**Request Body**:
```json
{
  "build_id": "build-123456789",
  "owner": "0x1234567890abcdef1234567890abcdef12345678"
}
```

**Response**:
- 200 OK:
```json
{
  "success": true,
  "message": "VM deleted successfully"
}
```

### Get VM Information

```
POST /vm/info
```

Retrieves information about a specific VM.

**Request Body**:
```json
{
  "build_id": "build-123456789",
  "owner": "0x1234567890abcdef1234567890abcdef12345678"
}
```

**Response**:
- 200 OK:
```json
{
  "build_id": "build-123456789",
  "name": "my-instance",
  "owner": "0x1234567890abcdef1234567890abcdef12345678",
  "state": "running",
  "resources": {
    "vcpu_count": 2,
    "mem_size_mib": 2048,
    "disk_size": 10
  },
  "ip_address": "192.168.100.5",
  "created_at": 1677721600,
  "updated_at": 1677724200
}
```

### List All VMs

```
POST /vm/list
```

Retrieves a list of all VMs available to the authenticated user.

**Response**:
- 200 OK:
```json
[
  {
    "build_id": "build-123456789",
    "name": "my-instance-1",
    "owner": "0x1234567890abcdef1234567890abcdef12345678",
    "state": "running",
    "resources": {
      "vcpu_count": 2,
      "mem_size_mib": 2048,
      "disk_size": 10
    },
    "ip_address": "192.168.100.5",
    "created_at": 1677721600,
    "updated_at": 1677724200
  },
  {
    "build_id": "build-987654321",
    "name": "my-instance-2",
    "owner": "0x1234567890abcdef1234567890abcdef12345678",
    "state": "stopped",
    "resources": {
      "vcpu_count": 4,
      "mem_size_mib": 4096,
      "disk_size": 20
    },
    "ip_address": "192.168.100.6",
    "created_at": 1677721700,
    "updated_at": 1677724300
  }
]
```

### VM Power Button

```
POST /vm/power-button
```

Simulates pressing the power button on a VM, which can trigger a graceful shutdown in the guest OS.

> **Note**: This endpoint is planned but not fully implemented yet.

### Planned Future Endpoints

The following endpoints are planned for future releases:

- `POST /vm/commit`: Create a snapshot of the VM's current state
- `POST /vm/snapshot`: Create a point-in-time snapshot
- `POST /vm/coredump`: Generate a coredump for debugging
- `POST /vm/restore`: Restore VM from a snapshot
- `POST /vm/resize/vcpu`: Adjust the number of vCPUs
- `POST /vm/resize/memory`: Adjust the memory allocation
- `POST /vm/device/add`: Add a device to a VM
- `POST /vm/disk/add`: Add a disk to a VM
- `POST /vm/fs/add`: Add a filesystem to a VM
- `POST /vm/device/remove`: Remove a device from a VM
- `POST /vm/migrate/to`: Initiate VM migration to another node
- `POST /vm/migrate/from`: Receive a migrating VM

## Error Handling

The VMM Service API returns standard HTTP status codes:

- 200: Success
- 400: Bad Request (invalid parameters)
- 401: Unauthorized (authentication failure)
- 403: Forbidden (insufficient permissions)
- 404: Not Found (VM or resource not found)
- 409: Conflict (operation cannot be performed in current state)
- 500: Internal Server Error

Error responses include a JSON object with:
- `success`: false
- `message`: A descriptive error message

## Rate Limiting

The VMM Service implements rate limiting based on the authenticated user:
- Standard users: 100 requests per minute
- Privileged users: 1000 requests per minute

## Example Usage

### Creating and Starting a VM with curl

```bash
# Create VM
curl -X POST https://node.formation.cloud:3002/vm/create \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer <JWT_TOKEN>" \
  -d '{
    "name": "web-server",
    "owner": "0x1234567890abcdef1234567890abcdef12345678",
    "vm_config": {
      "vcpu_count": 2,
      "mem_size_mib": 2048,
      "disk_size": 10,
      "boot_args": "console=ttyS0 reboot=k panic=1 pci=off",
      "kernel": "vmlinux.bin",
      "rootfs": "rootfs.ext4"
    }
  }'

# Boot VM
curl -X POST https://node.formation.cloud:3002/vm/boot \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer <JWT_TOKEN>" \
  -d '{
    "build_id": "build-123456789",
    "owner": "0x1234567890abcdef1234567890abcdef12345678"
  }'
```

### Getting VM Information

```bash
curl -X POST https://node.formation.cloud:3002/vm/info \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer <JWT_TOKEN>" \
  -d '{
    "build_id": "build-123456789",
    "owner": "0x1234567890abcdef1234567890abcdef12345678"
  }'
```

## SDK Integration

The Formation SDK provides wrapper functions for the VMM Service API, simplifying integration into applications:

```javascript
const Formation = require('formation-sdk');

// Initialize the SDK
const formation = new Formation({
  apiKey: 'your-api-key'
});

// Create a VM
const vm = await formation.vm.create({
  name: 'web-server',
  vcpuCount: 2,
  memSizeMib: 2048,
  diskSize: 10
});

// Start the VM
await formation.vm.start(vm.buildId);

// Get VM info
const vmInfo = await formation.vm.getInfo(vm.buildId);
console.log(vmInfo);
``` 