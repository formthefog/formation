# Dockerfile for form-state service

FROM ubuntu:22.04

# Install minimal runtime dependencies
RUN apt-get update && \
    apt-get install -y ca-certificates curl && \
    apt-get clean && \
    rm -rf /var/lib/apt/lists/*

# Create required directories
RUN mkdir -p /var/lib/formation/db && \
    mkdir -p /etc/formation/auth

# Add cache bust argument to invalidate cache for binary copy
ARG CACHE_BUST=1

# Copy the pre-compiled binary
COPY ./target/release/form-state /usr/local/bin/

# Copy default configuration
COPY ./form-state/.env.example /etc/formation/auth/.env.example

COPY ./scripts/run-form-state.sh /usr/local/bin/run-form-state.sh

RUN chmod +x /usr/local/bin/run-form-state.sh 

# Expose API port
EXPOSE 3004

# Set environment variables
ENV STATE_DB_PATH=/var/lib/formation/db/state.db \
    AUTH_MODE=production

# Run the service
CMD ["/usr/local/bin/run-form-state.sh"] 
