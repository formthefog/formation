# Dockerfile for form-pack-manager service

FROM ubuntu:22.04

# Install runtime dependencies
RUN apt-get update && \
    apt-get install -y \
    docker.io \
    ca-certificates && \
    apt-get clean && \
    rm -rf /var/lib/apt/lists/*

# Create required directories
RUN mkdir -p /var/lib/formation/packs

# Copy the pre-compiled binary
COPY ./target/release/form-pack-manager /usr/local/bin/

# Expose API port
EXPOSE 3005

# Set environment variables
ENV PACK_MANAGER_LOG_LEVEL=info

# Run the service
CMD ["/usr/local/bin/form-pack-manager"] 