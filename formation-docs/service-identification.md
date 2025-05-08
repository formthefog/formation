# Service Identification Analysis

## Identified Services

Based on analysis of existing Dockerfiles, the following services have been identified:

### 1. form-dns
- **Purpose**: DNS service for resolving network names within the system
- **Binary Path**: `/usr/local/bin/form-dns`
- **Startup Script**: `/usr/local/bin/run-form-dns.sh`
- **Exposed Port**: Not explicitly defined, but likely DNS standard port 53/UDP

### 2. form-state
- **Purpose**: State management service that likely maintains configuration and application state
- **Binary Path**: `/usr/local/bin/form-state`
- **Startup Script**: `/usr/local/bin/run-form-state.sh`
- **Exposed Port**: Possibly 3002 (inference from Docker expose directive)

### 3. vmm-service
- **Purpose**: Virtual Machine Manager service for creating and managing VM instances
- **Binary Path**: `/usr/local/bin/vmm-service`
- **Startup Script**: `/usr/local/bin/run-vmm-service.sh`
- **Exposed Port**: Possibly 3003 (inference from Docker expose directive)

### 4. form-broker
- **Purpose**: Message broker service for handling inter-service communication
- **Binary Path**: `/usr/local/bin/form-broker`
- **Startup Script**: Not explicitly defined in provided Dockerfiles
- **Exposed Port**: Unknown

### 5. form-pack-manager
- **Purpose**: Package manager for handling software deployments within the system
- **Binary Path**: `/usr/local/bin/form-pack-manager`
- **Startup Script**: `/usr/local/bin/run-pack-manager.sh`
- **Exposed Port**: Possibly 3004 (inference from Docker expose directive)

### 6. formnet
- **Purpose**: Networking service that manages virtual network configurations
- **Binary Path**: `/usr/local/bin/formnet`
- **Startup Script**: `/usr/local/bin/run-formnet.sh`
- **Exposed Port**: 51820 (likely WireGuard VPN)

### 7. form-p2p
- **Purpose**: Peer-to-peer communication service
- **Binary Path**: `/usr/local/bin/form-p2p`
- **Startup Script**: `/usr/local/bin/run-form-p2p.sh` (only in main Dockerfile, not in minimal)
- **Exposed Port**: Possibly 53333 (inference from Docker expose directive in main Dockerfile)

### 8. mock-server
- **Purpose**: Development-only mock server (likely for testing/development)
- **Binary Path**: `/usr/local/bin/mock-server` (only in Dockerfile.minimal)
- **Startup Script**: Not explicitly defined
- **Exposed Port**: Unknown

## Shared Resources

### Directories
- `/usr/local/bin`: Binaries and scripts
- `/var/lib/formation/formnet`: Formnet related files
- `/var/lib/formation/kernel`: Kernel files including hypervisor firmware
- `/var/lib/formation/vm-images`: VM image storage
- `/var/lib/formation/db`: Database files (only in minimal)
- `/run/form-vm`: Runtime VM files
- `/var/log/formation`: Log files
- `/etc/formation/auth`: Authentication files (only in minimal)
- `/etc/formation/billing`: Billing information (only in minimal)

### Files
- `/var/lib/formation/kernel/hypervisor-fw`: Hypervisor firmware
- `/var/lib/formation/marketplace/openapi.yaml`: API specification (only in minimal)
- `/etc/formation/auth/.env.example`: Configuration example (only in minimal)

## Initial Dependency Analysis

Based on the Dockerfiles and typical microservice architectures:

1. **Dependencies on form-state**:
   - Likely all services depend on this for system configuration

2. **Dependencies on formnet**:
   - form-dns likely depends on network configuration from formnet
   - vmm-service likely requires formnet for VM networking

3. **Dependencies on form-dns**:
   - Most services likely depend on DNS for service discovery

4. **Dependencies on vmm-service**:
   - form-pack-manager likely depends on vmm-service for deploying packages to VMs

## Required Ports

Explicitly exposed ports from Dockerfiles:
- 3002: Likely form-state API port
- 3003: Likely vmm-service API port
- 3004: Likely form-pack-manager API port
- 3005: Purpose unclear (only in main Dockerfile)
- 53333: Likely form-p2p service port (only in main Dockerfile)
- 51820: WireGuard VPN port for formnet

## Next Steps

1. Further investigation needed to confirm exact purposes of each service
2. Verify port assignments and requirements
3. Complete detailed dependency mapping
4. Document configuration requirements for each service 