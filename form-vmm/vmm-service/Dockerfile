# Dockerfile for vmm-service

FROM ubuntu:22.04

# Build args
ARG FEATURES=""

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
    mkdir -p /run/form-vmm && \
    mkdir -p /var/run/form-vmm

# Copy the pre-compiled binary (built with appropriate features)
COPY ./target/release/vmm-service /usr/local/bin/

# Copy the hypervisor firmware
COPY ./artifacts/hypervisor-fw /var/lib/formation/kernel/hypervisor-fw

# Copy the startup script
COPY ./scripts/run-vmm-service.sh /usr/local/bin/run-vmm-service.sh
RUN chmod +x /usr/local/bin/run-vmm-service.sh

# Expose API port
EXPOSE 3002

# Set environment variables
ENV VMM_LOG_LEVEL=info
ENV VMM_API_PORT=3002
ENV VMM_STATE_URL=http://localhost:3004
ENV RUST_BACKTRACE=1
ENV FORMNET_ENDPOINT=localhost:51820

# Add health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=10s --retries=3 \
  CMD curl -f http://localhost:3002/health || exit 1

# Run the service
CMD ["/usr/local/bin/run-vmm-service.sh"] 