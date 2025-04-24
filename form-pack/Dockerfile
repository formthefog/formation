# Dockerfile for form-pack-manager service

FROM ubuntu:22.04

# Build args
ARG FEATURES=""

# Install runtime dependencies
RUN apt-get update && \
    apt-get install -y \
    docker.io \
    ca-certificates \
    curl \
    wget \
    jq \
    && apt-get clean && \
    rm -rf /var/lib/apt/lists/*

# Create required directories
RUN mkdir -p /var/lib/formation/pack-manager

# Copy the pre-compiled binary
COPY ./target/release/form-pack-manager /usr/local/bin/

# Copy the startup script
COPY ./scripts/run-form-pack-manager.sh /usr/local/bin/run-form-pack-manager.sh
RUN chmod +x /usr/local/bin/run-form-pack-manager.sh

# Expose API port
EXPOSE 3003

# Set environment variables
ENV PACK_MANAGER_LOG_LEVEL=info
ENV PACK_MANAGER_PORT=3003
ENV PACK_MANAGER_INTERFACE=all
ENV RUST_BACKTRACE=1

# Add health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=10s --retries=3 \
  CMD curl -f http://localhost:3003/health || exit 1

# Run the service
CMD ["/usr/local/bin/run-form-pack-manager.sh"] 