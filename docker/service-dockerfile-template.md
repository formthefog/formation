# Service Dockerfile Template Guide

This guide provides templates and best practices for creating service-specific Dockerfiles that extend the Formation base image.

## Basic Service Template

Below is a template for a service Dockerfile:

```dockerfile
# Service Dockerfile Template
# Replace <SERVICE_NAME> with your service name (e.g., form-dns, form-state)

# Build stage
FROM rust:1.65-slim as builder

WORKDIR /build

# Copy only files needed for dependency resolution first (for better caching)
COPY Cargo.toml Cargo.lock ./
COPY .cargo ./.cargo
COPY <SERVICE_NAME>/ ./<SERVICE_NAME>/

# Build dependencies to cache them
RUN mkdir -p src && \
    echo "fn main() {}" > src/main.rs && \
    cargo build --release --bin <SERVICE_NAME> && \
    rm -rf src

# Copy the rest of the source code
COPY . .

# Build the service
RUN cargo build --release --bin <SERVICE_NAME>

# Runtime stage
FROM formation/base:latest

ARG SERVICE_NAME=<SERVICE_NAME>
ARG SERVICE_VERSION=0.1.0

# Add labels
LABEL maintainer="Formation Platform Team"
LABEL service="${SERVICE_NAME}"
LABEL version="${SERVICE_VERSION}"

# Install service-specific runtime dependencies
RUN apt-get update -y && \
    apt-get install -y \
    # Add service-specific packages here
    && apt-get clean && \
    rm -rf /var/lib/apt/lists/*

# Create service-specific directories
RUN mkdir -p /var/lib/formation/${SERVICE_NAME}

# Copy the binary from builder
COPY --from=builder /build/target/release/${SERVICE_NAME} /usr/local/bin/

# Copy service-specific startup script
COPY ./scripts/run-${SERVICE_NAME}.sh /usr/local/bin/
RUN chmod +x /usr/local/bin/run-${SERVICE_NAME}.sh

# Copy any additional configuration files
# COPY ./config/${SERVICE_NAME}/* /etc/formation/${SERVICE_NAME}/

# Expose necessary ports
# EXPOSE <SERVICE_PORT>

# Use the service user
USER formation

# Health check (customize for your service)
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:<SERVICE_PORT>/health || exit 1

# Set entrypoint
ENTRYPOINT ["/usr/local/bin/run-${SERVICE_NAME}.sh"]
```

## Image Inheritance Pattern

The Formation microservices follow a two-stage build process:

1. **Builder Stage**: Uses the Rust image to compile the service binary
2. **Runtime Stage**: Uses the Formation base image with only the necessary runtime components

```
┌────────────────┐     ┌────────────────┐
│                │     │                │
│   Rust Image   │     │ Formation Base │
│   (Builder)    │     │    Image       │
│                │     │                │
└───────┬────────┘     └────────┬───────┘
        │                       │
        ▼                       ▼
┌────────────────┐     ┌────────────────┐
│                │     │                │
│  Service       │     │  Service       │
│  Binary        │──►  │  Image         │
│                │     │                │
└────────────────┘     └────────────────┘
```

This pattern ensures:
- Smaller final images (no build tools in runtime image)
- Faster builds through caching
- Consistent runtime environment

## Service-Specific Customization

### 1. Dependencies

Add only the dependencies specific to your service:

```dockerfile
RUN apt-get update -y && \
    apt-get install -y \
    # Service-specific dependencies only
    libspecific-dev \
    specific-tool \
    && apt-get clean && \
    rm -rf /var/lib/apt/lists/*
```

### 2. Directories

Create only directories required by your service:

```dockerfile
RUN mkdir -p /var/lib/formation/${SERVICE_NAME}/data && \
    mkdir -p /etc/formation/${SERVICE_NAME}/config && \
    chmod 755 /var/lib/formation/${SERVICE_NAME} && \
    chown -R formation:formation /var/lib/formation/${SERVICE_NAME}
```

### 3. Configuration

Copy service-specific configuration:

```dockerfile
COPY ./config/${SERVICE_NAME}/default.conf /etc/formation/${SERVICE_NAME}/
```

### 4. Environment Variables

Set default environment variables:

```dockerfile
ENV SERVICE_PORT=3000 \
    LOG_LEVEL=info \
    CONFIG_PATH=/etc/formation/${SERVICE_NAME}/default.conf
```

## Versioning Strategy

The Formation platform follows semantic versioning for all Docker images:

### Base Image Versioning

- `formation/base:latest` - Latest development version (not for production)
- `formation/base:1.0` - Major version 1, minor version 0
- `formation/base:1.0.1` - Major version 1, minor version 0, patch version 1

### Service Image Versioning

Service images follow the same pattern and include the service name:

- `formation/<service-name>:latest` - Latest development version
- `formation/<service-name>:1.0` - Major version 1, minor version 0
- `formation/<service-name>:1.0.1` - Major version 1, minor version 0, patch version 1

### Version Tagging Rules

1. **Major version change (X.y.z)**: Breaking API changes or substantial architectural changes
2. **Minor version change (x.Y.z)**: New features, non-breaking changes
3. **Patch version change (x.y.Z)**: Bug fixes, security patches

### Base Image Reference

Always use specific base image version in production Dockerfiles:

```dockerfile
# Development
FROM formation/base:latest

# Production (preferred)
FROM formation/base:1.0.1
```

## Service-Specific Examples

### Example: form-dns Service

```dockerfile
# Build stage
FROM rust:1.65-slim as builder

WORKDIR /build

# Copy files for dependency resolution
COPY Cargo.toml Cargo.lock ./
COPY .cargo ./.cargo
COPY form-dns/ ./form-dns/

# Build dependencies
RUN mkdir -p src && \
    echo "fn main() {}" > src/main.rs && \
    cargo build --release --bin form-dns && \
    rm -rf src

# Copy the rest of the source code
COPY . .

# Build the service
RUN cargo build --release --bin form-dns

# Runtime stage
FROM formation/base:1.0

ARG SERVICE_VERSION=0.1.0

LABEL maintainer="Formation Platform Team"
LABEL service="form-dns"
LABEL version="${SERVICE_VERSION}"

# DNS-specific dependencies
RUN apt-get update -y && \
    apt-get install -y \
    bind9-utils \
    && apt-get clean && \
    rm -rf /var/lib/apt/lists/*

# Create DNS-specific directories
RUN mkdir -p /var/lib/formation/dns && \
    chown -R formation:formation /var/lib/formation/dns

# Copy the binary
COPY --from=builder /build/target/release/form-dns /usr/local/bin/

# Copy startup script
COPY ./scripts/run-form-dns.sh /usr/local/bin/
RUN chmod +x /usr/local/bin/run-form-dns.sh

# Expose DNS ports
EXPOSE 53/udp
EXPOSE 53/tcp

# Use non-root user
USER formation

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD dig @localhost -p 53 localhost || exit 1

# Set entrypoint
ENTRYPOINT ["/usr/local/bin/run-form-dns.sh"]
```

## Best Practices

1. **Minimal Images**: Include only what's necessary for your service
2. **Layer Caching**: Structure Dockerfiles to maximize cache utilization
3. **Multi-stage Builds**: Separate build and runtime environments
4. **Non-root User**: Run services as the `formation` user
5. **Health Checks**: Always include a service-specific health check
6. **Specific Versions**: Pin dependency versions
7. **Documentation**: Add comments to explain non-obvious choices
8. **Security**: Follow security best practices (minimal permissions, no secrets in image)

## CI/CD Integration

Sample Makefile target for building a service:

```makefile
.PHONY: build-<service-name>

SERVICE_NAME = <service-name>
VERSION = $(shell git describe --tags --always --dirty)

build-$(SERVICE_NAME):
	docker build \
		--build-arg SERVICE_VERSION=$(VERSION) \
		-t formation/$(SERVICE_NAME):latest \
		-t formation/$(SERVICE_NAME):$(VERSION) \
		-f ./$(SERVICE_NAME)/Dockerfile .
``` 