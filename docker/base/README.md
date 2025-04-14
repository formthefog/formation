# Formation Base Image

This directory contains the base Docker image used by all Formation microservices. The base image provides common dependencies and configuration to ensure consistency across all service containers.

## Contents

The base image includes:

1. Common system packages required by most Formation services
2. Standard directory structure
3. Non-root user for running services
4. Volume mount points for logs
5. Basic security configurations

## Building the Base Image

To build the base image:

```bash
cd docker/base
docker build -t formation/base:latest .
```

Alternatively, use docker-compose:

```bash
cd docker/base
docker-compose build
```

## Using the Base Image

Each service-specific Dockerfile should extend the base image. Example:

```dockerfile
FROM formation/base:latest

# Install service-specific dependencies
RUN apt-get update -y && \
    apt-get install -y <service-specific-packages> && \
    apt-get clean && \
    rm -rf /var/lib/apt/lists/*

# Copy service binary
COPY ./target/release/<service-name> /usr/local/bin/

# Copy startup script
COPY ./scripts/run-<service-name>.sh /usr/local/bin/
RUN chmod +x /usr/local/bin/run-<service-name>.sh

# Create service-specific directories
RUN mkdir -p /var/lib/formation/<service-specific-dirs>

# Expose service ports
EXPOSE <service-port>

# Set entrypoint
ENTRYPOINT ["/usr/local/bin/run-<service-name>.sh"]
```

## Benefits of Using a Base Image

1. **Consistency**: All services use the same base packages and configurations
2. **Efficiency**: Reduces build time and image size through Docker layer caching
3. **Maintainability**: Updates to common dependencies only need to be made in one place
4. **Security**: Common security patches applied consistently across all services

## Image Versioning

The base image follows semantic versioning:

- `formation/base:latest` - Always points to the most recent version
- `formation/base:1.0` - Major version 1, minor version 0
- `formation/base:1.0.1` - Major version 1, minor version 0, patch version 1

For production services, always reference a specific version rather than `latest` to ensure consistency.

## Next Steps

1. Implement continuous integration to automatically build and publish the base image
2. Set up vulnerability scanning for the base image
3. Add additional security hardening measures 