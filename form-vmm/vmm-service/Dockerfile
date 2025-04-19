# Dockerfile for vmm-service

FROM ubuntu:22.04

# Install runtime dependencies
RUN apt-get update && \
    apt-get install -y \
    # Network tools
    iputils-ping \
    wget \
    curl \
    iproute2 \
    bridge-utils \
    socat \
    # Virtualization
    qemu-kvm \
    libvirt-dev \
    libguestfs-tools \
    qemu-utils \
    # System libraries
    libdbus-1-dev \
    libudev-dev \
    libfuse-dev \
    libseccomp-dev \
    cloud-utils \
    libnetfilter-queue-dev \
    libnl-3-dev \
    libnl-route-3-dev \
    zlib1g-dev \
    libbpf-dev \
    liburing-dev \
    libssl-dev \
    # Security
    libnss3-tools \
    mkcert \
    ca-certificates && \
    apt-get clean && \
    rm -rf /var/lib/apt/lists/*

# Create required directories
RUN mkdir -p /var/lib/formation/vm-images && \
    mkdir -p /run/form-vm

# Copy the pre-compiled binary
COPY ./target/release/vmm-service /usr/local/bin/

# Copy the hypervisor firmware
COPY ./artifacts/hypervisor-fw /var/lib/formation/kernel/hypervisor-fw

# Expose ports
EXPOSE 3002

# Set environment variables
ENV VMM_LOG_LEVEL=info

# Add health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=10s --retries=3 \
  CMD curl -f http://localhost:3002/health || exit 1

# Run the service
CMD ["/usr/local/bin/vmm-service"] 