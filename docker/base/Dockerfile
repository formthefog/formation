FROM ubuntu:22.04 as formation-base

LABEL maintainer="Formation Platform Team"
LABEL description="Base image for Formation microservices"
LABEL version="1.0"

# Install common build and runtime dependencies
# These are the packages required by multiple services
RUN apt-get update -y && \
    apt-get upgrade -y && \
    apt-get install -y \
    # Build dependencies
    protobuf-compiler \
    libprotobuf-dev \
    build-essential \
    clang \
    llvm \
    pkg-config \
    # Runtime dependencies
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
    iproute2 \
    bridge-utils \
    ssh \
    socat \
    libsqlite3-dev \
    iputils-ping \
    wget \
    curl \
    # Clean up to reduce image size
    && apt-get clean && \
    rm -rf /var/lib/apt/lists/*

# Create common directories used by multiple services
RUN mkdir -p /usr/local/bin && \
    mkdir -p /var/log/formation && \
    mkdir -p /etc/formation/auth && \
    mkdir -p /etc/formation/billing

# Add a non-root user for running services
RUN groupadd -r formation && \
    useradd -r -g formation -s /bin/false -d /home/formation formation && \
    mkdir -p /home/formation && \
    chown -R formation:formation /home/formation

# Create volume mount points
VOLUME ["/var/log/formation"]

# Set working directory
WORKDIR /app

# Default command - will be overridden by service-specific Dockerfiles
CMD ["echo", "This is a base image. Please use a service-specific image instead."] 