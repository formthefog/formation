# Dockerfile for vmm-service

# Build stage
FROM rust:1.65-slim as builder

WORKDIR /build

# Install build dependencies
RUN apt-get update && \
    apt-get install -y \
    pkg-config \
    libssl-dev \
    libvirt-dev \
    libseccomp-dev \
    libnetfilter-queue-dev \
    libnl-3-dev \
    libnl-route-3-dev \
    linux-headers-amd64 \
    && apt-get clean && \
    rm -rf /var/lib/apt/lists/*

# Copy only files needed for dependency resolution first (for better caching)
COPY Cargo.toml Cargo.lock ./
COPY .cargo ./.cargo
COPY vmm-service/ ./vmm-service/
COPY form-vmm/ ./form-vmm/

# Build dependencies to cache them
RUN mkdir -p src && \
    echo "fn main() {}" > src/main.rs && \
    cargo build --release --bin vmm-service && \
    rm -rf src

# Copy the rest of the source code
COPY . .

# Build the service
RUN cargo build --release --bin vmm-service

# Runtime stage
FROM formation/base:1.0

ARG SERVICE_VERSION=0.1.0

# Add labels
LABEL maintainer="Formation Platform Team"
LABEL service="vmm-service"
LABEL version="${SERVICE_VERSION}"

# Install vm-specific runtime dependencies
RUN apt-get update -y && \
    apt-get install -y \
    qemu-kvm \
    libvirt-daemon-system \
    libvirt-clients \
    bridge-utils \
    iproute2 \
    iptables \
    libseccomp2 \
    libnetfilter-queue1 \
    libnl-3-200 \
    libnl-route-3-200 \
    socat \
    ssh \
    qemu-utils \
    libguestfs-tools \
    && apt-get clean && \
    rm -rf /var/lib/apt/lists/*

# Create VM-specific directories
RUN mkdir -p /var/lib/formation/vm-images && \
    mkdir -p /var/lib/formation/kernel && \
    mkdir -p /run/form-vm && \
    mkdir -p /etc/formation/vmm && \
    chown -R formation:formation /var/lib/formation/vm-images && \
    chown -R formation:formation /var/lib/formation/kernel && \
    chmod 755 /run/form-vm

# Copy hypervisor firmware
COPY ./artifacts/hypervisor-fw /var/lib/formation/kernel/hypervisor-fw

# Copy the binary from builder
COPY --from=builder /build/target/release/vmm-service /usr/local/bin/

# Copy service-specific startup script
COPY ./scripts/run-vmm-service.sh /usr/local/bin/
RUN chmod +x /usr/local/bin/run-vmm-service.sh

# Copy default configuration
COPY ./vmm-service/config/default.conf /etc/formation/vmm/

# Expose API port
EXPOSE 3003

# Set up volumes for persistent storage
VOLUME ["/var/lib/formation/vm-images"]
VOLUME ["/var/lib/formation/kernel"]
VOLUME ["/run/form-vm"]

# Environment variables
ENV VMM_CONFIG_PATH=/etc/formation/vmm/default.conf \
    VMM_LOG_LEVEL=info \
    VMM_API_PORT=3003 \
    VMM_STATE_URL=http://form-state:3004 \
    VMM_KERNEL_PATH=/var/lib/formation/kernel/hypervisor-fw \
    VMM_VM_DIR=/run/form-vm \
    VMM_IMAGES_DIR=/var/lib/formation/vm-images \
    RUST_BACKTRACE=1

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=10s --retries=3 \
    CMD curl -f http://localhost:${VMM_API_PORT:-3003}/health || exit 1

# The vmm-service requires elevated privileges for VM management
# In production, use seccomp profiles and capabilities instead of running as root
USER root

# Set entrypoint
ENTRYPOINT ["/usr/local/bin/run-vmm-service.sh"] 