#!/bin/bash

# Build all Formation services for Docker Compose testing
set -e

echo "Building Formation services Docker images..."

# Build base image first (if needed)
# docker build -t formation/base:1.0 -f docker/base/Dockerfile .

# Build form-dns
echo "Building form-dns..."
docker build -t formation/form-dns:latest -f form-dns/Dockerfile .

# Build form-state
echo "Building form-state..."
docker build -t formation/form-state:latest -f form-state/Dockerfile .

# Build vmm-service
echo "Building vmm-service..."
docker build -t formation/vmm-service:latest -f form-vmm/Dockerfile .

# Build form-broker
echo "Building form-broker..."
docker build -t formation/form-broker:latest -f form-broker/Dockerfile .

# Build form-pack-manager
echo "Building form-pack-manager..."
docker build -t formation/form-pack-manager:latest -f form-pack/Dockerfile .

# Build formnet
echo "Building formnet..."
docker build -t formation/formnet:latest -f form-net/Dockerfile .

# Build form-p2p
echo "Building form-p2p..."
docker build -t formation/form-p2p:latest -f form-p2p/Dockerfile .

echo "All Formation services Docker images built successfully." 