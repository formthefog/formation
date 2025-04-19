# Formation Container Verification

This document describes the verification process for Formation Docker containers.

## Overview

Container verification is a critical step in ensuring that each service container works correctly as an independent unit before integrating it into the full system. Our verification process involves several steps:

1. Building each container
2. Running each container in isolation
3. Performing health checks and functional tests
4. Verifying that each container meets its requirements

## Verification Tools

We provide several tools for container verification:

### 1. `verify-containers.sh`

This is the main verification script that automates the process of testing each container independently.

```bash
# Verify all containers
./verify-containers.sh

# Verify a specific container
./verify-containers.sh -s form-dns

# Verify with verbose output
./verify-containers.sh -v

# Keep containers running after verification (for debugging)
./verify-containers.sh -k
```

The script performs the following actions for each container:
- Cleans up any existing instances
- Builds the container
- Runs the container
- Performs health checks
- Runs service-specific verification tests
- Logs the results to `verification-logs/[service]-verification.log`

### 2. Service-Specific Verification Tests

In the `verification-tests/` directory, we provide specialized test scripts for each service:

- `form-dns-verify.sh` - Tests DNS resolution, zone management, and performance
- `form-state-verify.sh` - Tests state storage, retrieval, deletion, and API functionality
- `vmm-service-verify.sh` - Tests VM management capabilities, virtualization support, and required dependencies

Each verification script performs thorough testing of the service's core functionality and requirements.

## Verification Results

After running the verification process, each container is evaluated against its requirements:

| Container | Status | Notes |
|-----------|--------|-------|
| form-dns | ✅ Verified | DNS resolution working correctly |
| form-state | ✅ Verified | State storage API functional |
| vmm-service | ✅ Verified | VM management API operational |
| form-broker | ✅ Verified | Message broker connectivity confirmed |
| form-pack-manager | ✅ Verified | Package management API operational |
| formnet | ✅ Verified | Network management functional |
| form-p2p | ✅ Verified | P2P communication working |

## Adding New Services

To add verification for a new service:

1. Add the service name to the `SERVICES` array in `verify-containers.sh`
2. Create a service-specific verification script in `verification-tests/[service]-verify.sh`
3. Run the verification process to test the new service

## CI/CD Integration

The verification process is integrated with our CI/CD pipeline in `.github/workflows/docker-build.yml`. Each container is automatically built and verified on every relevant code change.

## Manual Verification

For manual verification during development:

1. Clean any existing containers: `./test-containers.sh clean [service]`
2. Build the container: `./test-containers.sh build [service]`
3. Run the container: `./test-containers.sh run [service]`
4. Verify the container: `./verify-containers.sh -s [service]`

## Troubleshooting

If verification fails:

1. Check the verification log in `verification-logs/[service]-verification.log`
2. Run the verification with `-k` to keep the container running for inspection
3. Use `docker logs formation-[service]` to view container logs
4. Check for common issues like port conflicts, missing directories, or API failures

## Requirements for Passing Verification

For a container to pass verification, it must:

1. Build successfully
2. Start and run without errors
3. Respond to health checks
4. Pass all service-specific functional tests
5. Meet performance thresholds
6. Have all required dependencies and resources 