# Dockerfile for form-broker service

# Build stage
FROM rust:1.75-slim-bullseye as builder

WORKDIR /usr/src/formation

# Install dependencies
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
    pkg-config \
    libssl-dev \
    git \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Copy only the necessary files for dependency resolution
COPY Cargo.toml Cargo.lock ./
COPY form-broker/Cargo.toml ./form-broker/
# Create dummy source files to build dependencies
RUN mkdir -p form-broker/src && \
    echo "fn main() {}" > form-broker/src/main.rs

# Build dependencies (this will be cached if dependencies don't change)
RUN cargo build --release --package form-broker

# Copy the actual source code
COPY form-broker/src ./form-broker/src

# Build the binary
RUN touch form-broker/src/main.rs && \
    cargo build --release --package form-broker

# Use a small base image for the final container
FROM debian:bullseye-slim

# Install runtime dependencies
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
    ca-certificates \
    libssl1.1 \
    && rm -rf /var/lib/apt/lists/*

# Create directories
RUN mkdir -p /etc/formation/broker /var/lib/formation/broker

# Copy the binary from the builder stage
COPY --from=builder /usr/src/formation/target/release/form-broker /usr/local/bin/form-broker

# Copy the default configuration
COPY form-broker/config/default.conf /etc/formation/broker/default.conf

# Copy the run script
COPY form-broker/scripts/entrypoint.sh /usr/local/bin/entrypoint.sh
RUN chmod +x /usr/local/bin/entrypoint.sh

# Expose ports (API, AMQP, MQTT)
EXPOSE 3005 5672 1883

# Set working directory
WORKDIR /var/lib/formation/broker

# Define volume for persistent data
VOLUME ["/var/lib/formation/broker", "/etc/formation/broker"]

# Run as non-root user for improved security
RUN groupadd -r formation && useradd --no-log-init -r -g formation formation
RUN chown -R formation:formation /var/lib/formation/broker /etc/formation/broker
USER formation

# Set the entrypoint
ENTRYPOINT ["/usr/local/bin/entrypoint.sh"]

# Default command
CMD ["form-broker", "--config", "/etc/formation/broker/default.conf"] 