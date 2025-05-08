# Shared Resources Analysis

This document provides a detailed analysis of shared resources across services in the microservices architecture.

## Shared Directories and Files

### Critical System Directories

| Directory | Purpose | Services Using | Sharing Strategy |
|-----------|---------|----------------|------------------|
| `/usr/local/bin` | Contains service binaries and startup scripts | All services | Read-only volume in all containers |
| `/var/lib/formation/kernel` | Hypervisor firmware and kernel files | vmm-service | Read-only volume for vmm-service |
| `/var/lib/formation/vm-images` | VM image storage | vmm-service, form-pack-manager | Shared volume between related services |
| `/var/lib/formation/formnet` | Network configuration for VMs | formnet, vmm-service | Shared volume with specific permissions |
| `/var/lib/formation/db` | Database storage | form-state and potentially others | Service-specific volume with backup strategy |
| `/run/form-vm` | Runtime VM files | vmm-service | Service-specific temporary volume |
| `/var/log/formation` | Log storage | All services | Shared log volume or central logging service |
| `/etc/formation/auth` | Authentication configuration | All services | Read-only configuration volume |
| `/etc/formation/billing` | Billing information | Potentially marketplace services | Read-only configuration volume |

### Critical Shared Files

| File | Purpose | Services Using | Sharing Strategy |
|------|---------|----------------|------------------|
| `/var/lib/formation/kernel/hypervisor-fw` | Hypervisor firmware | vmm-service | Read-only volume |
| `/var/lib/formation/marketplace/openapi.yaml` | API specification | form-state and admin services | Read-only configuration volume |
| `/etc/formation/auth/.env.example` | Configuration template | All services potentially | Template for environment setup, not directly shared |

## Volume Requirements

### Service-Specific Volumes

| Service | Required Volumes | Purpose | Size Considerations | Persistence |
|---------|------------------|---------|---------------------|-------------|
| vmm-service | `/var/lib/formation/vm-images`<br>`/var/lib/formation/kernel`<br>`/run/form-vm` | VM images and runtime<br>Kernel files<br>Runtime VM files | Large (GB)<br>Small (MB)<br>Medium (MB-GB) | Persistent<br>Persistent<br>Ephemeral |
| form-state | `/var/lib/formation/db` | State database | Medium (MB-GB) | Persistent with backup |
| formnet | `/var/lib/formation/formnet` | Network config | Small (MB) | Persistent |
| form-dns | None specific | - | - | - |
| form-broker | None identified | - | - | - |
| form-pack-manager | `/var/lib/formation/vm-images` | VM packages | Large (GB) | Persistent |
| form-p2p | None identified | - | - | - |
| mock-server | None specific | - | - | - |

### Shared Volumes Strategy

1. **Read-Only System Volumes**:
   - Base container images and binaries
   - Configuration files
   - Hypervisor firmware

2. **Persistent Data Volumes**:
   - Database files
   - VM images
   - Network configurations

3. **Ephemeral Operational Volumes**:
   - Runtime VM files
   - Temporary processing data

## Inter-Service Communication Patterns

### Direct API Communication

| Source Service | Target Service | Communication Type | Purpose |
|----------------|----------------|-------------------|---------|
| All services | form-state | HTTP/REST | Configuration and state management |
| form-pack-manager | vmm-service | HTTP/REST | VM management for package deployment |
| Admin UI | All services | HTTP/REST | Management and monitoring |

### Network-Level Communication

| Source Service | Target Service | Communication Type | Purpose |
|----------------|----------------|-------------------|---------|
| All services | form-dns | DNS (UDP/53) | Service discovery |
| VM instances | formnet | WireGuard (UDP/51820) | Virtual networking |
| formnet | External network | Various | External connectivity |

### Message-Based Communication

| Publisher | Subscribers | Communication Type | Purpose |
|-----------|------------|-------------------|---------|
| All services | form-broker | Message queue | Event notification |
| form-state | All services | Message queue | Configuration updates |
| vmm-service | form-state, form-pack-manager | Message queue | VM state changes |

## Migration Considerations

### Volume Mount Strategy for Docker Compose

```yaml
volumes:
  vm-images:
    driver: local
  kernel-files:
    driver: local
  formnet-config:
    driver: local
  db-storage:
    driver: local
  log-storage:
    driver: local
  auth-config:
    driver: local
```

### Service Volume Assignments

```yaml
services:
  vmm-service:
    volumes:
      - vm-images:/var/lib/formation/vm-images
      - kernel-files:/var/lib/formation/kernel
      - /run/form-vm:/run/form-vm
  
  form-state:
    volumes:
      - db-storage:/var/lib/formation/db
      - auth-config:/etc/formation/auth:ro
  
  # Additional services follow similar pattern
```

### Data Consistency Challenges

1. **Database State**: Ensure form-state database is properly persisted and backed up
2. **VM Image Consistency**: Coordinate access to VM images between vmm-service and form-pack-manager
3. **Network Configuration**: Maintain consistency between formnet and services that depend on it

### Communication Security

1. **Authentication**: All inter-service API calls should require authentication tokens
2. **Encryption**: All communication should be encrypted in transit
3. **Network Isolation**: Use Docker networks to isolate service communication

## Next Steps

1. **Define Docker Volume Configuration**: Create specific volume definitions for docker-compose
2. **Design Service Discovery**: Implement service discovery for reliable inter-service communication
3. **Configure Network Security**: Set up appropriate network security controls for each service
4. **Implement Logging Strategy**: Define a centralized logging approach for all services 