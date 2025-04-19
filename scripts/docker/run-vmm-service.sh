#!/bin/bash

# Create directories if they don't exist
mkdir -p $(pwd)/vm-images
mkdir -p $(pwd)/kernel-data
mkdir -p $(pwd)/secrets
mkdir -p /run/form-vm

# Run vmm-service container
docker run --name formation-vmm -p 3002:3002 \
  -v $(pwd)/vm-images:/var/lib/formation/vm-images \
  -v $(pwd)/kernel-data:/var/lib/formation/kernel \
  -v /run/form-vm:/run/form-vm \
  -v /lib/modules:/lib/modules:ro \
  -v /var/run/docker.sock:/var/run/docker.sock \
  -v $(pwd)/secrets:/var/lib/formation/secrets:ro \
  -e VMM_LOG_LEVEL=info \
  -e VMM_STATE_URL=http://host.docker.internal:3004 \
  -e SECRET_PATH=/var/lib/formation/secrets/config \
  -e PASSWORD=test-password \
  --privileged \
  --device=/dev/kvm \
  --device=/dev/vhost-net \
  --device=/dev/null \
  --device=/dev/zero \
  --device=/dev/random \
  --device=/dev/urandom \
  formationai/vmm-service:latest

# Verify service is running
echo "Checking vmm-service health..."
sleep 5
curl http://localhost:3002/health 