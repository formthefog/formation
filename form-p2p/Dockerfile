# Dockerfile for form-p2p service

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
COPY form-p2p/Cargo.toml ./form-p2p/
COPY form-types/Cargo.toml ./form-types/
COPY form-traits/Cargo.toml ./form-traits/
COPY form-net/shared/Cargo.toml ./form-net/shared/
COPY form-config/Cargo.toml ./form-config/

# Create dummy source files to build dependencies
RUN mkdir -p form-p2p/src form-types/src form-traits/src form-net/shared/src form-config/src && \
    echo "pub mod api { pub async fn serve(_: std::sync::Arc<tokio::sync::RwLock<crate::queue::FormMQ<Vec<u8>>>>, _: u16) -> Result<(), Box<dyn std::error::Error>> { Ok(()) } }" > form-p2p/src/lib.rs && \
    echo "pub mod queue { pub struct FormMQ<T>(std::marker::PhantomData<T>); impl<T> FormMQ<T> { pub fn new(_: String, _: String, _: String) -> Self { Self(std::marker::PhantomData) } } pub const QUEUE_PORT: u16 = 3006; }" >> form-p2p/src/lib.rs && \
    echo "pub mod db { pub fn open_db(_: std::path::PathBuf) -> std::sync::Arc<redb::Database> { unimplemented!() } pub fn store_topic_queue(_: &std::sync::Arc<redb::Database>, _: &str, _: &crdts::bft_topic_queue::TopicQueue<Vec<u8>>) -> Result<(), Box<dyn std::error::Error>> { Ok(()) } }" >> form-p2p/src/lib.rs && \
    echo "pub fn main() {}" > form-types/src/lib.rs && \
    echo "pub fn main() {}" > form-traits/src/lib.rs && \
    echo "pub fn main() {}" > form-net/shared/src/lib.rs && \
    echo "pub fn main() {}" > form-config/src/lib.rs

# Create a simple binary for the p2p service
RUN mkdir -p form-p2p/src/bin && \
    echo 'use std::sync::Arc;\
    use tokio::sync::RwLock;\
    use form_p2p::{api, queue::FormMQ};\
    use clap::Parser;\
    \
    #[derive(Parser)]\
    struct Args {\
        #[clap(long, short, default_value = "3006")]\
        port: u16,\
        #[clap(long, short)]\
        node_id: String,\
        #[clap(long, short)]\
        private_key: String,\
        #[clap(long, short, default_value = "localhost:3004")]\
        state_url: String,\
    }\
    \
    #[tokio::main]\
    async fn main() -> Result<(), Box<dyn std::error::Error>> {\
        let args = Args::parse();\
        let queue = Arc::new(RwLock::new(FormMQ::new(\
            args.node_id,\
            args.private_key,\
            args.state_url,\
        )));\
        \
        api::serve(queue, args.port).await\
    }' > form-p2p/src/bin/form-p2p-service.rs

# Build dependencies (this will be cached if dependencies don't change)
RUN cargo build --release --bin form-p2p-service

# Copy the actual source code
COPY form-p2p/src ./form-p2p/src
COPY form-types/src ./form-types/src
COPY form-traits/src ./form-traits/src
COPY form-net/shared/src ./form-net/shared/src
COPY form-config/src ./form-config/src

# Build the binary
RUN touch form-p2p/src/bin/form-p2p-service.rs && \
    cargo build --release --bin form-p2p-service

# Use a small base image for the final container
FROM debian:bullseye-slim

# Install runtime dependencies
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
    ca-certificates \
    libssl1.1 \
    && rm -rf /var/lib/apt/lists/*

# Create directories
RUN mkdir -p /etc/formation/p2p /var/lib/formation/p2p /var/lib/formation/db

# Copy the binary from the builder stage
COPY --from=builder /usr/src/formation/target/release/form-p2p-service /usr/local/bin/form-p2p-service

# Create entrypoint script
RUN echo '#!/bin/bash\n\
set -e\n\
\n\
# Set defaults if not provided through environment variables\n\
: "${P2P_PORT:=3006}"\n\
: "${P2P_NODE_ID:=$(head -c 20 /dev/urandom | md5sum | head -c 40)}"\n\
: "${P2P_PRIVATE_KEY:=$(head -c 32 /dev/urandom | xxd -p -c 64)}"\n\
: "${P2P_STATE_URL:=http://form-state:3004}"\n\
: "${P2P_LOG_LEVEL:=info}"\n\
: "${P2P_DATA_DIR:=/var/lib/formation/p2p}"\n\
: "${P2P_DB_DIR:=/var/lib/formation/db}"\n\
\n\
# Create data directory if it doesn'\''t exist\n\
if [ ! -d "$P2P_DATA_DIR" ]; then\n\
    mkdir -p "$P2P_DATA_DIR"\n\
fi\n\
\n\
# Create DB directory if it doesn'\''t exist\n\
if [ ! -d "$P2P_DB_DIR" ]; then\n\
    mkdir -p "$P2P_DB_DIR"\n\
fi\n\
\n\
# Print startup message\n\
echo "Starting form-p2p service..."\n\
echo "Port: $P2P_PORT"\n\
echo "Node ID: $P2P_NODE_ID"\n\
echo "State URL: $P2P_STATE_URL"\n\
echo "Log level: $P2P_LOG_LEVEL"\n\
echo "Data directory: $P2P_DATA_DIR"\n\
echo "DB directory: $P2P_DB_DIR"\n\
\n\
# Execute command with environment variables set\n\
RUST_LOG=$P2P_LOG_LEVEL exec "$@"\n\
' > /usr/local/bin/entrypoint.sh
RUN chmod +x /usr/local/bin/entrypoint.sh

# Expose service port
EXPOSE 3006

# Set working directory
WORKDIR /var/lib/formation/p2p

# Define volume for persistent data
VOLUME ["/var/lib/formation/p2p", "/var/lib/formation/db"]

# Run as non-root user for improved security
RUN groupadd -r formation && useradd --no-log-init -r -g formation formation
RUN chown -R formation:formation /var/lib/formation/p2p /var/lib/formation/db /etc/formation/p2p
USER formation

# Set the entrypoint
ENTRYPOINT ["/usr/local/bin/entrypoint.sh"]

# Default command
CMD ["form-p2p-service", "--port", "${P2P_PORT}", "--node-id", "${P2P_NODE_ID}", "--private-key", "${P2P_PRIVATE_KEY}", "--state-url", "${P2P_STATE_URL}"] 