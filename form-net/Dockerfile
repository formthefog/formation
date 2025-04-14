# Dockerfile for formnet service

FROM ubuntu:22.04

# Install runtime dependencies
RUN apt-get update && \
    apt-get install -y \
    libsqlite3-0 \
    ca-certificates && \
    apt-get clean && \
    rm -rf /var/lib/apt/lists/*

# Create required directories
RUN mkdir -p /var/lib/formation/formnet

# Copy the pre-compiled binary
COPY ./target/release/formnet /usr/local/bin/

# Expose ports
EXPOSE 51820/udp

# Set environment variables
ENV FORMENT_LOG_LEVEL=info

# Run the service
CMD ["/usr/local/bin/formnet"] 