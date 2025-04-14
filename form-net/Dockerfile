# Dockerfile for formnet service

# Build stage
FROM rust:1.75-slim-bullseye as builder

WORKDIR /usr/src/formation

# Install dependencies
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
    pkg-config \
    libsqlite3-dev \
    libssl-dev \
    git \
    ca-certificates \
    clang \
    libclang-dev \
    build-essential \
    && rm -rf /var/lib/apt/lists/*

# Copy only the necessary files for dependency resolution
COPY Cargo.toml Cargo.lock ./
COPY form-net/server/Cargo.toml ./form-net/server/
COPY form-net/client/Cargo.toml ./form-net/client/
COPY form-net/shared/Cargo.toml ./form-net/shared/
COPY form-net/publicip/Cargo.toml ./form-net/publicip/
COPY form-net/wireguard-control/Cargo.toml ./form-net/wireguard-control/
COPY form-net/netlink-request/Cargo.toml ./form-net/netlink-request/
COPY form-net/hostsfile/Cargo.toml ./form-net/hostsfile/
COPY form-state/Cargo.toml ./form-state/
COPY form-types/Cargo.toml ./form-types/
COPY form-p2p/Cargo.toml ./form-p2p/

# Create dummy source files to build dependencies
RUN mkdir -p form-net/server/src form-net/client/src form-net/shared/src \
    form-net/publicip/src form-net/wireguard-control/src form-net/netlink-request/src \
    form-net/hostsfile/src form-state/src form-types/src form-p2p/src && \
    echo "fn main() {}" > form-net/server/src/main.rs && \
    echo "pub fn main() {}" > form-net/server/src/lib.rs && \
    echo "fn main() {}" > form-net/client/src/main.rs && \
    echo "pub fn main() {}" > form-net/shared/src/lib.rs && \
    echo "pub fn main() {}" > form-net/publicip/src/lib.rs && \
    echo "pub fn main() {}" > form-net/wireguard-control/src/lib.rs && \
    echo "pub fn main() {}" > form-net/netlink-request/src/lib.rs && \
    echo "pub fn main() {}" > form-net/hostsfile/src/lib.rs && \
    echo "pub fn main() {}" > form-state/src/lib.rs && \
    echo "pub fn main() {}" > form-types/src/lib.rs && \
    echo "pub fn main() {}" > form-p2p/src/lib.rs

# Build dependencies (this will be cached if dependencies don't change)
RUN cargo build --release --bin formnet-server --bin formnet-client

# Copy the actual source code
COPY form-net/server/src ./form-net/server/src
COPY form-net/client/src ./form-net/client/src
COPY form-net/shared/src ./form-net/shared/src
COPY form-net/publicip/src ./form-net/publicip/src
COPY form-net/wireguard-control/src ./form-net/wireguard-control/src
COPY form-net/netlink-request/src ./form-net/netlink-request/src
COPY form-net/hostsfile/src ./form-net/hostsfile/src
COPY form-state/src ./form-state/src
COPY form-types/src ./form-types/src
COPY form-p2p/src ./form-p2p/src

# Build the binaries
RUN cargo build --release --bin formnet-server --bin formnet-client

# WireGuard stage
FROM golang:1.22-bullseye as wireguard-builder
ARG wg_go_tag=0.0.20230223

RUN mkdir /repo \
    && curl -L https://github.com/WireGuard/wireguard-go/archive/refs/tags/${wg_go_tag}.tar.gz \
    | tar -xzC /repo --strip-components=1 \
    && cd /repo \
    && CGO_ENABLED=0 make

# Final stage
FROM debian:bullseye-slim

# Install runtime dependencies
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
    ca-certificates \
    libsqlite3-0 \
    iproute2 \
    iputils-ping \
    && rm -rf /var/lib/apt/lists/*

# Create directories
RUN mkdir -p /etc/formnet /var/lib/formnet /var/log/formnet

# Copy the binaries from the builder stage
COPY --from=builder /usr/src/formation/target/release/formnet-server /usr/local/bin/
COPY --from=builder /usr/src/formation/target/release/formnet-client /usr/local/bin/
COPY --from=wireguard-builder /repo/wireguard-go /usr/local/bin/

# Create entrypoint script
RUN echo '#!/bin/bash\n\
set -e\n\
\n\
# Set defaults if not provided through environment variables\n\
: "${FORMNET_SERVER_PORT:=8080}"\n\
: "${FORMNET_DATA_DIR:=/var/lib/formnet}"\n\
: "${FORMNET_CONFIG_DIR:=/etc/formnet}"\n\
: "${FORMNET_NETWORK_NAME:=formnet}"\n\
: "${FORMNET_EXTERNAL_ENDPOINT:=auto}"\n\
: "${FORMNET_LISTEN_PORT:=51820}"\n\
: "${FORMNET_LOG_LEVEL:=info}"\n\
\n\
# If config file doesn'\''t exist, create a new network\n\
if [ ! -f "$FORMNET_CONFIG_DIR/$FORMNET_NETWORK_NAME.conf" ]; then\n\
    echo "Creating new formnet network: $FORMNET_NETWORK_NAME"\n\
    formnet-server install "$FORMNET_NETWORK_NAME" \\\n\
        --db-path "$FORMNET_DATA_DIR/$FORMNET_NETWORK_NAME.db" \\\n\
        --config-path "$FORMNET_CONFIG_DIR/$FORMNET_NETWORK_NAME.conf" \\\n\
        --external-endpoint "$FORMNET_EXTERNAL_ENDPOINT" \\\n\
        --listen-port "$FORMNET_LISTEN_PORT"\n\
fi\n\
\n\
# Print startup message\n\
echo "Starting formnet service..."\n\
echo "Network name: $FORMNET_NETWORK_NAME"\n\
echo "Server port: $FORMNET_SERVER_PORT"\n\
echo "WireGuard port: $FORMNET_LISTEN_PORT"\n\
echo "Data directory: $FORMNET_DATA_DIR"\n\
echo "Config directory: $FORMNET_CONFIG_DIR"\n\
echo "Log level: $FORMNET_LOG_LEVEL"\n\
\n\
# Execute command\n\
RUST_LOG=$FORMNET_LOG_LEVEL exec "$@"\n\
' > /usr/local/bin/entrypoint.sh
RUN chmod +x /usr/local/bin/entrypoint.sh

# Expose ports (API and WireGuard)
EXPOSE 8080 51820/udp

# Set working directory
WORKDIR /var/lib/formnet

# Define volumes for persistent data
VOLUME ["/var/lib/formnet", "/etc/formnet"]

# Set capabilities for network operations (required for WireGuard)
RUN apt-get update && \
    apt-get install -y --no-install-recommends libcap2-bin && \
    setcap cap_net_admin+ep /usr/local/bin/formnet-server && \
    setcap cap_net_admin+ep /usr/local/bin/formnet-client && \
    apt-get remove -y libcap2-bin && \
    apt-get autoremove -y && \
    rm -rf /var/lib/apt/lists/*

# Run as non-root user for improved security (except for network operations)
RUN groupadd -r formation && useradd --no-log-init -r -g formation formation
RUN chown -R formation:formation /var/lib/formnet /etc/formnet /var/log/formnet
USER formation

# Set the entrypoint
ENTRYPOINT ["/usr/local/bin/entrypoint.sh"]

# Default command
CMD ["formnet-server", "run", "--network", "${FORMNET_NETWORK_NAME}", "--api-server"] 