# Dockerfile for form-dns service

FROM ubuntu:22.04

# Install runtime dependencies
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
    ca-certificates \
    libssl3 \
    dnsutils \
    bind9-utils \
    iputils-ping \
    iproute2 \
    dnsmasq \
    netcat \
    curl \
    libnss3-tools \
    wget \
    && rm -rf /var/lib/apt/lists/*

RUN wget https://github.com/FiloSottile/mkcert/releases/download/v1.4.3/mkcert-v1.4.3-linux-amd64

RUN mv mkcert-v1.4.3-linux-amd64 /usr/bin/mkcert

RUN chmod +x /usr/bin/mkcert


# Create required directories
RUN mkdir -p /var/lib/formation/dns/zones && \
    mkdir -p /etc/formation/dns

# Copy the pre-compiled binary
COPY ./target/release/form-dns /usr/local/bin/form-dns

COPY ./artifacts/resolvectl /usr/local/bin/resolvectl

# Copy internal run script
COPY ./scripts/run-form-dns.sh /usr/local/bin/run-form-dns.sh
RUN chmod +x /usr/local/bin/run-form-dns.sh

# Expose DNS ports
EXPOSE 53/udp
EXPOSE 53/tcp
EXPOSE 5453/udp
EXPOSE 5453/tcp

# Set environment variables
ENV DNS_LOG_LEVEL=info \
    DNS_PORT=53 \
    STATE_URL=http://localhost:3004

# Add health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
  CMD dig @localhost formation +short > /dev/null || exit 1

# Run the service
CMD ["/usr/local/bin/run-form-dns.sh"] 
