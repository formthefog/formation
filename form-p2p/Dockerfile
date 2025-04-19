# Dockerfile for form-p2p service

FROM ubuntu:22.04

# Install runtime dependencies
RUN apt-get update && \
    apt-get install -y \
    ca-certificates && \
    apt-get clean && \
    rm -rf /var/lib/apt/lists/*

# Create required directories
RUN mkdir -p /var/lib/formation/p2p

# Copy the pre-compiled binary
COPY ./target/release/form-p2p /usr/local/bin/

# Expose ports
EXPOSE 3003

# Set environment variables
ENV P2P_LOG_LEVEL=info

# Run the service
CMD ["/usr/local/bin/form-p2p"] 