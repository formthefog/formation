# Formation Docker Container System

This directory contains the build, test, and health check system for Formation Docker containers.

## Overview

The Formation platform consists of multiple services, each deployed as a Docker container:

- `form-dns` - DNS service
- `form-state` - State management service
- `vmm-service` - Virtual Machine Manager service
- `form-broker` - Message broker service
- `form-pack-manager` - Package manager service
- `formnet` - Network management service
- `form-p2p` - Peer-to-peer communication service

## Building Containers

To build the containers, use the `Makefile`:

```bash
# Build all containers
cd docker
make all

# Build a specific container
make form-dns
```

## Container Health Checks

Each container includes a Docker HEALTHCHECK configuration to ensure the service is running correctly. Additionally, external health check scripts are provided in the `health-checks/` directory.

### Using Health Check Scripts

The container health check system includes several scripts:

1. **container-health.sh** - General-purpose health check utilities
2. **[service]-healthcheck.sh** - Service-specific health check scripts
3. **run-all-checks.sh** - Script to run health checks on all or selected containers

To run a health check:

```bash
# Run health check on all services
./health-checks/run-all-checks.sh

# Run health check on a specific service
./health-checks/run-all-checks.sh -s form-dns

# Run health check with verbose output
./health-checks/run-all-checks.sh -v -s form-state
```

## Testing Containers

The `test-containers.sh` script provides a comprehensive testing framework for containers:

```bash
# Show usage
./test-containers.sh --help

# Build, run, and check a specific container
./test-containers.sh all form-dns

# Build all containers
./test-containers.sh build

# Run all containers
./test-containers.sh run

# Run health checks on all containers
./test-containers.sh check

# Stop all containers
./test-containers.sh stop

# Clean up (stop and remove) all containers
./test-containers.sh clean
```

## Health Check Details

Each service health check verifies:

1. **Process running** - Checks if the service process is running in the container
2. **Port listening** - Verifies that the service is listening on its expected port(s)
3. **Functional test** - Tests the core functionality of the service
4. **Resource verification** - Checks for required files, directories, and resources

### Service-Specific Health Checks

#### form-dns
- Verifies DNS port (53) is listening
- Tests DNS resolution functionality
- Checks for zone files directory

#### form-state
- Verifies API port (3004) is listening
- Tests HTTP health endpoint
- Tests state storage and retrieval
- Checks for database directory

#### vmm-service
- Verifies API port (3002) is listening
- Tests HTTP health endpoint
- Checks for VM images directory

#### form-broker
- Verifies API port (3005) is listening
- Tests HTTP health endpoint
- Tests message publication and subscription

#### form-pack-manager
- Verifies API port (8080) is listening
- Tests HTTP health endpoint
- Tests package listing and management

#### formnet
- Verifies API port (8080) is listening
- Tests HTTP health endpoint
- Tests network configuration

#### form-p2p
- Verifies API port (3003) is listening
- Tests HTTP health endpoint
- Tests peer discovery and communication

## Extending the Test System

To add a new service:

1. Add the service name to the `SERVICES` array in `test-containers.sh` and `run-all-checks.sh`
2. Create a service-specific health check script
3. Add a case for your service in the `run_service()` function in `test-containers.sh`

## CI/CD Integration

The health check scripts are integrated with GitHub Actions workflows in the `.github/workflows/docker-build.yml` file, which runs automated health checks on each container after building. 