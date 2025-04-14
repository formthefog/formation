# Dockerfile for form-dns service

# Build stage
FROM rust:1.65-slim as builder

WORKDIR /build

# Copy only files needed for dependency resolution first (for better caching)
COPY Cargo.toml Cargo.lock ./
COPY .cargo ./.cargo
COPY form-dns/ ./form-dns/

# Build dependencies to cache them
RUN mkdir -p src && \
    echo "fn main() {}" > src/main.rs && \
    cargo build --release --bin form-dns && \
    rm -rf src

# Copy the rest of the source code
COPY . .

# Build the service
RUN cargo build --release --bin form-dns

# Runtime stage
FROM formation/base:1.0

ARG SERVICE_VERSION=0.1.0

# Add labels
LABEL maintainer="Formation Platform Team"
LABEL service="form-dns"
LABEL version="${SERVICE_VERSION}"

# Install DNS-specific runtime dependencies
RUN apt-get update -y && \
    apt-get install -y \
    bind9-utils \
    dnsutils \
    && apt-get clean && \
    rm -rf /var/lib/apt/lists/*

# Create DNS-specific directories
RUN mkdir -p /var/lib/formation/dns/zones && \
    mkdir -p /etc/formation/dns && \
    chown -R formation:formation /var/lib/formation/dns

# Copy the binary from builder
COPY --from=builder /build/target/release/form-dns /usr/local/bin/

# Copy service-specific startup script
COPY ./scripts/run-form-dns.sh /usr/local/bin/
RUN chmod +x /usr/local/bin/run-form-dns.sh

# Copy DNS configuration
COPY ./form-dns/config/default.conf /etc/formation/dns/

# Expose DNS ports
EXPOSE 53/udp
EXPOSE 53/tcp

# Use the service user
USER formation

# Environment variables
ENV DNS_CONFIG_PATH=/etc/formation/dns/default.conf \
    DNS_LOG_LEVEL=info

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD dig @localhost -p 53 localhost || exit 1

# Set entrypoint
ENTRYPOINT ["/usr/local/bin/run-form-dns.sh"] 