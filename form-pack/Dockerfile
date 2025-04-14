# Dockerfile for form-pack-manager service

# Build stage
FROM rust:1.75-slim-bullseye as builder

WORKDIR /usr/src/formation

# Install dependencies
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
    pkg-config \
    libssl-dev \
    git \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Copy only the necessary files for dependency resolution
COPY Cargo.toml Cargo.lock ./
COPY form-pack/Cargo.toml ./form-pack/
COPY form-types/Cargo.toml ./form-types/
COPY form-p2p/Cargo.toml ./form-p2p/
COPY form-state/Cargo.toml ./form-state/
COPY form-config/Cargo.toml ./form-config/

# Create dummy source files to build dependencies
RUN mkdir -p form-pack/src form-types/src form-p2p/src form-state/src form-config/src && \
    echo "fn main() {}" > form-pack/src/lib.rs && \
    echo "fn main() {}" > form-pack/src/bin/pack_manager.rs && \
    echo "pub mod manager { pub struct FormPackManager; impl FormPackManager { pub fn new(_: std::net::SocketAddr, _: String) -> Self { Self {} } pub async fn run(&self, _: tokio::sync::broadcast::Receiver<()>) -> Result<(), Box<dyn std::error::Error + Send + Sync>> { Ok(()) } } }" >> form-pack/src/lib.rs && \
    echo "pub fn main() {}" > form-types/src/lib.rs && \
    echo "pub fn main() {}" > form-p2p/src/lib.rs && \
    echo "pub fn main() {}" > form-state/src/lib.rs && \
    echo "pub struct OperatorConfig { pub secret_key: Option<String> } impl OperatorConfig { pub fn from_file(_: std::path::PathBuf, _: bool, _: Option<&String>) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> { Ok(Self { secret_key: Some(String::new()) }) } }" > form-config/src/lib.rs

# Build dependencies (this will be cached if dependencies don't change)
RUN cargo build --release --bin form-pack-manager

# Copy the actual source code
COPY form-pack/src ./form-pack/src
COPY form-types/src ./form-types/src
COPY form-p2p/src ./form-p2p/src
COPY form-state/src ./form-state/src
COPY form-config/src ./form-config/src

# Build the binary
RUN touch form-pack/src/bin/pack_manager.rs && \
    cargo build --release --bin form-pack-manager

# Use a small base image for the final container
FROM debian:bullseye-slim

# Install runtime dependencies
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
    ca-certificates \
    libssl1.1 \
    && rm -rf /var/lib/apt/lists/*

# Create directories
RUN mkdir -p /etc/formation/pack-manager /var/lib/formation/pack-manager

# Copy the binary from the builder stage
COPY --from=builder /usr/src/formation/target/release/form-pack-manager /usr/local/bin/form-pack-manager

# Create entrypoint script
RUN echo '#!/bin/bash\n\
set -e\n\
\n\
# Set defaults if not provided through environment variables\n\
: "${PACK_MANAGER_PORT:=8080}"\n\
: "${PACK_MANAGER_INTERFACE:=All}"\n\
: "${PACK_MANAGER_CONFIG_PATH:=/etc/formation/pack-manager/config.json}"\n\
: "${PACK_MANAGER_ENCRYPTED:=true}"\n\
: "${PACK_MANAGER_PASSWORD:=default_password}"\n\
\n\
# If config file doesn'\''t exist, create a minimal one\n\
if [ ! -f "$PACK_MANAGER_CONFIG_PATH" ]; then\n\
    echo "Creating default configuration at $PACK_MANAGER_CONFIG_PATH"\n\
    echo "{\n\
  \"secret_key\": \"1111111111111111111111111111111111111111111111111111111111111111\"\n\
}" > "$PACK_MANAGER_CONFIG_PATH"\n\
fi\n\
\n\
# Print startup message\n\
echo "Starting form-pack-manager service..."\n\
echo "Interface: $PACK_MANAGER_INTERFACE"\n\
echo "Port: $PACK_MANAGER_PORT"\n\
echo "Config: $PACK_MANAGER_CONFIG_PATH"\n\
\n\
# Execute command\n\
exec "$@"\n\
' > /usr/local/bin/entrypoint.sh
RUN chmod +x /usr/local/bin/entrypoint.sh

# Expose API port
EXPOSE 8080

# Set working directory
WORKDIR /var/lib/formation/pack-manager

# Define volume for persistent data
VOLUME ["/var/lib/formation/pack-manager", "/etc/formation/pack-manager"]

# Run as non-root user for improved security
RUN groupadd -r formation && useradd --no-log-init -r -g formation formation
RUN chown -R formation:formation /var/lib/formation/pack-manager /etc/formation/pack-manager
USER formation

# Set the entrypoint
ENTRYPOINT ["/usr/local/bin/entrypoint.sh"]

# Default command
CMD ["form-pack-manager", "--interface", "All", "--port", "8080", "--config", "/etc/formation/pack-manager/config.json", "--encrypted", "--password", "${PACK_MANAGER_PASSWORD}"] 