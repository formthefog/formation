# Docker Compose Test Plan

This document outlines the procedure for testing the complete docker-compose deployment for the Formation platform. These tests must be executed on a Linux machine due to VMM service virtualization requirements and networking capabilities.

## Prerequisites

- Linux machine (Ubuntu 22.04 recommended)
- Docker Engine installed (version 24.0.0+)
- Docker Compose installed (version 2.17.0+)
- Git repository cloned

## Build Process

1. Make the build script executable:
   ```bash
   chmod +x scripts/build-formation-services.sh
   ```

2. Run the build script to create all service images:
   ```bash
   ./scripts/build-formation-services.sh
   ```

3. Verify all images were created:
   ```bash
   docker images | grep formation
   ```

## Test Procedure

### 1. Start Complete Environment

1. Start all services using docker-compose:
   ```bash
   docker-compose up -d
   ```

2. Check that all services are running:
   ```bash
   docker-compose ps
   ```

### 2. Verify Services Start Correctly

For each service, check its health status and logs:

```bash
# Check health status of all services
docker-compose ps

# Check logs for specific services
docker-compose logs form-dns
docker-compose logs form-state
docker-compose logs vmm-service
docker-compose logs form-broker
docker-compose logs form-pack-manager
docker-compose logs formnet
docker-compose logs form-p2p
```

Verify each service passes its healthcheck.

### 3. Test Service Intercommunication

1. Test DNS service:
   ```bash
   docker-compose exec form-dns dig @localhost -p 53 localhost
   ```

2. Test State Service API:
   ```bash
   docker-compose exec form-state curl -f http://localhost:3004/health
   ```

3. Verify VMM can communicate with State:
   ```bash
   docker-compose exec vmm-service curl -f http://form-state:3004/health
   ```

4. Verify Broker connects to State:
   ```bash
   docker-compose exec form-broker curl -f http://form-state:3004/health
   ```

5. Test P2P communication:
   ```bash
   docker-compose exec form-p2p curl -f http://localhost:53333/health
   ```

### 4. Verify Volume Sharing

1. Create test file in a shared volume:
   ```bash
   docker-compose exec form-state sh -c "echo 'test data' > /var/lib/formation/db/test-file.txt"
   ```

2. Verify data persistence after restart:
   ```bash
   docker-compose restart form-state
   docker-compose exec form-state cat /var/lib/formation/db/test-file.txt
   ```

### 5. Network Testing

1. Verify internal network connectivity:
   ```bash
   docker-compose exec form-dns ping -c 3 form-state
   ```

2. Test WireGuard connectivity (formnet):
   ```bash
   docker-compose exec formnet wg show
   ```

## Cleanup

```bash
docker-compose down
docker volume prune -f
```

## Expected Results

- All services should start without errors
- All healthchecks should pass
- Services should be able to communicate with each other
- Data should persist in volumes between restarts
- Network interfaces should be properly configured

## Documentation

Document any issues encountered and their resolutions for future reference. 