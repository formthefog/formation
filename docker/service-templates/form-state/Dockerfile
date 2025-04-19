# Dockerfile for form-state service

# Build stage
FROM rust:1.65-slim as builder

WORKDIR /build

# Install build dependencies
RUN apt-get update && \
    apt-get install -y libsqlite3-dev && \
    apt-get clean && \
    rm -rf /var/lib/apt/lists/*

# Copy only files needed for dependency resolution first (for better caching)
COPY Cargo.toml Cargo.lock ./
COPY .cargo ./.cargo
COPY form-state/ ./form-state/

# Build dependencies to cache them
RUN mkdir -p src && \
    echo "fn main() {}" > src/main.rs && \
    cargo build --release --bin form-state && \
    rm -rf src

# Copy the rest of the source code
COPY . .

# Build the service
RUN cargo build --release --bin form-state

# Runtime stage
FROM formation/base:1.0

ARG SERVICE_VERSION=0.1.0

# Add labels
LABEL maintainer="Formation Platform Team"
LABEL service="form-state"
LABEL version="${SERVICE_VERSION}"

# Create state-specific directories
RUN mkdir -p /var/lib/formation/db && \
    mkdir -p /etc/formation/state && \
    mkdir -p /var/lib/formation/marketplace && \
    chown -R formation:formation /var/lib/formation/db /var/lib/formation/marketplace

# Copy the binary from builder
COPY --from=builder /build/target/release/form-state /usr/local/bin/

# Copy service-specific startup script
COPY ./scripts/run-form-state.sh /usr/local/bin/
RUN chmod +x /usr/local/bin/run-form-state.sh

# Copy API specification
COPY ./form-state/openapi.yaml /var/lib/formation/marketplace/openapi.yaml

# Copy default configuration
COPY ./form-state/.env.example /etc/formation/auth/.env.example
COPY ./form-state/config/default.conf /etc/formation/state/

# Expose API port
EXPOSE 3004

# Use the service user
USER formation

# Environment variables
ENV STATE_API_PORT=3004 \
    STATE_LOG_LEVEL=info \
    STATE_CONFIG_PATH=/etc/formation/state/default.conf \
    STATE_DB_PATH=/var/lib/formation/db/state.db \
    AUTH_MODE=production \
    MARKETPLACE_ENABLED=true \
    BILLING_ENABLED=true \
    API_KEYS_ENABLED=true

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:3004/health || exit 1

# Set entrypoint
ENTRYPOINT ["/usr/local/bin/run-form-state.sh"] 