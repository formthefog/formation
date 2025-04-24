# Dockerfile for formnet service

FROM ubuntu:22.04

# Install runtime dependencies
RUN apt-get update && \
    apt-get install -y \
    curl \
    libsqlite3-0 \
    ca-certificates && \
    apt-get clean && \
    rm -rf /var/lib/apt/lists/*

# Create required directories
RUN mkdir -p /var/lib/formation/formnet

# Copy the pre-compiled binary
COPY ./target/release/formnet /usr/local/bin/formnet

COPY ./scripts/run-formnet.sh /usr/local/bin/run-formnet.sh

# Expose ports
EXPOSE 51820

# Set environment variables
ENV FORMENT_LOG_LEVEL=info

# Run the service
CMD ["/usr/local/bin/run-formnet.sh"] 
