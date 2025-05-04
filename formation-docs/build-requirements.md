# Service Build Requirements

This document outlines the build and runtime dependencies for each service in the microservices architecture.

## Common System Dependencies

These dependencies are required by multiple services and should be included in the base image:

### Common Build Dependencies
- protobuf-compiler
- libprotobuf-dev
- build-essential
- clang
- llvm
- pkg-config

### Common Runtime Dependencies
- libdbus-1-dev
- libudev-dev
- libfuse-dev
- libseccomp-dev
- cloud-utils
- libnetfilter-queue-dev
- libnl-3-dev
- libnl-route-3-dev
- zlib1g-dev
- libbpf-dev
- liburing-dev
- libssl-dev
- iproute2
- bridge-utils
- ssh
- socat
- libsqlite3-dev (for services that need database access)

## Service-Specific Dependencies

### 1. form-dns

#### Build Dependencies
- Rust toolchain
- protobuf-compiler
- libprotobuf-dev

#### Runtime Dependencies
- libc6
- libssl-dev
- dns-related libraries

#### Configuration Files
- `/usr/local/bin/run-form-dns.sh` (startup script)
- DNS configuration files (likely in `/etc/formation/dns/` or similar)

### 2. form-state

#### Build Dependencies
- Rust toolchain
- protobuf-compiler
- libprotobuf-dev
- libsqlite3-dev

#### Runtime Dependencies
- libc6
- libssl-dev
- libsqlite3
- Database drivers

#### Configuration Files
- `/usr/local/bin/run-form-state.sh` (startup script)
- `/etc/formation/auth/.env.example` (example environment variables)
- `/var/lib/formation/marketplace/openapi.yaml` (API specification)

### 3. vmm-service

#### Build Dependencies
- Rust toolchain
- protobuf-compiler
- libprotobuf-dev
- linux-headers
- qemu-kvm
- libvirt-dev

#### Runtime Dependencies
- libc6
- qemu-kvm
- libvirt
- libssl-dev
- libseccomp-dev
- libnetfilter-queue-dev
- iproute2
- bridge-utils
- ssh
- socat
- libguestfs-tools
- qemu-utils

#### Configuration Files
- `/usr/local/bin/run-vmm-service.sh` (startup script)
- `/var/lib/formation/kernel/hypervisor-fw` (hypervisor firmware)
- VM-related configuration files

### 4. form-broker

#### Build Dependencies
- Rust toolchain
- protobuf-compiler
- libprotobuf-dev

#### Runtime Dependencies
- libc6
- libssl-dev
- Message broker libraries

#### Configuration Files
- Broker configuration (likely in `/etc/formation/broker/` or similar)

### 5. form-pack-manager

#### Build Dependencies
- Rust toolchain
- protobuf-compiler
- libprotobuf-dev

#### Runtime Dependencies
- libc6
- libssl-dev
- Package management libraries

#### Configuration Files
- `/usr/local/bin/run-pack-manager.sh` (startup script)
- Package repository configuration

### 6. formnet

#### Build Dependencies
- Rust toolchain
- protobuf-compiler
- libprotobuf-dev
- libnetfilter-queue-dev
- libnl-3-dev
- libnl-route-3-dev

#### Runtime Dependencies
- libc6
- libssl-dev
- libnetfilter-queue-dev
- libnl-3
- libnl-route-3
- iproute2
- bridge-utils
- iptables/nftables

#### Configuration Files
- `/usr/local/bin/run-formnet.sh` (startup script)
- Network configuration files
- WireGuard configuration

### 7. form-p2p

#### Build Dependencies
- Rust toolchain
- protobuf-compiler
- libprotobuf-dev

#### Runtime Dependencies
- libc6
- libssl-dev
- P2P networking libraries

#### Configuration Files
- `/usr/local/bin/run-form-p2p.sh` (startup script)
- P2P configuration

### 8. mock-server

#### Build Dependencies
- Rust toolchain
- protobuf-compiler
- libprotobuf-dev

#### Runtime Dependencies
- libc6
- libssl-dev
- Development-specific dependencies

#### Configuration Files
- Mock service configuration
- Test data

## Build Process

All services appear to be Rust-based applications that follow similar build patterns:

1. Compile from source using Cargo
2. Place the compiled binary in `/usr/local/bin/`
3. Set up configuration files in appropriate directories
4. Create startup scripts

### Example Build Steps for a Service

```bash
# Common build steps for a Rust service
FROM rust:latest as builder

WORKDIR /build
COPY . .

# Install build dependencies
RUN apt-get update && apt-get install -y \
    protobuf-compiler \
    libprotobuf-dev \
    # Additional service-specific build dependencies

# Build the service
RUN cargo build --release --bin <service-name>

# Runtime stage
FROM ubuntu:22.04

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    # Service-specific runtime dependencies
    
# Copy the binary from builder
COPY --from=builder /build/target/release/<service-name> /usr/local/bin/

# Copy configuration and startup scripts
COPY ./scripts/run-<service-name>.sh /usr/local/bin/
RUN chmod +x /usr/local/bin/run-<service-name>.sh

# Create necessary directories
RUN mkdir -p /var/lib/formation/<service-specific-dirs>

ENTRYPOINT ["/usr/local/bin/run-<service-name>.sh"]
```

## Optimization Strategies

1. **Multi-stage builds** to reduce image size
2. **Shared base images** for common dependencies
3. **Dependency caching** during build
4. **Minimal runtime images** with only necessary dependencies
5. **Service-specific build arguments** to customize builds

## Configuration Management

Service configurations should follow these principles:

1. **Environment variables** for service-specific settings
2. **Configuration files** mounted as volumes
3. **Secrets management** for sensitive data
4. **Runtime reconfiguration** capabilities where applicable

## Next Steps

1. Create a base Dockerfile with common dependencies
2. Develop service-specific Dockerfiles using multi-stage builds
3. Set up proper CI/CD for building and testing each service
4. Implement configuration management strategy 