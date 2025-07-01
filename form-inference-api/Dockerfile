# Dockerfile for form-inference-api service

FROM ubuntu:22.04

# Install minimal runtime dependencies
RUN apt-get update && \
    apt-get install -y ca-certificates curl && \
    apt-get clean && \
    rm -rf /var/lib/apt/lists/*

# Create required directories
RUN mkdir -p /etc/formation/config

# Add cache bust argument to invalidate cache for binary copy
ARG CACHE_BUST=1

# Copy the pre-compiled binary
COPY ./target/release/form-inference-api /usr/local/bin/

# Copy run script
COPY ./scripts/run-form-inference-api.sh /usr/local/bin/run-form-inference-api.sh

RUN chmod +x /usr/local/bin/run-form-inference-api.sh

# Expose API port (will be overridden by config)
EXPOSE 3006

# Set environment variables
ENV CONFIG_PATH=/etc/formation/config/.operator-config.json

# Run the service
CMD ["/usr/local/bin/run-form-inference-api.sh"]