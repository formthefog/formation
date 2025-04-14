# Dockerfile for form-dns service

FROM ubuntu:22.04

# Install runtime dependencies
RUN apt-get update && \
    apt-get install -y \
    dnsutils \
    bind9-utils \
    iputils-ping \
    systemd \
    ca-certificates && \
    apt-get clean && \
    rm -rf /var/lib/apt/lists/*

# Create required directories
RUN mkdir -p /var/lib/formation/dns/zones && \
    mkdir -p /etc/formation/dns

# Copy the pre-compiled binary
COPY ./target/release/form-dns /usr/local/bin/

# Copy default configuration
COPY ./form-dns/config/default.conf /etc/formation/dns/

# Expose DNS ports
EXPOSE 53/udp
EXPOSE 53/tcp

# Set environment variables
ENV DNS_CONFIG_PATH=/etc/formation/dns/default.conf \
    DNS_LOG_LEVEL=info \
    DNS_LISTEN_PORT=53

# Add health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
  CMD dig @localhost example.com +short > /dev/null || exit 1

# Run the service
CMD ["/usr/local/bin/form-dns"] 