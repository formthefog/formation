#!/bin/bash

# Create directories if they don't exist
mkdir -p $(pwd)/net-data
mkdir -p $(pwd)/secrets

# Run formnet container
docker run --name formation-network -p 51820:51820/udp \
  -v $(pwd)/net-data:/var/lib/formation/formnet \
  -v $(pwd)/secrets:/var/lib/formation/secrets:ro \
  -e FORMNET_LOG_LEVEL=info \
  -e SECRET_PATH=/var/lib/formation/secrets/config \
  -e PASSWORD=test-password \
  --cap-add=NET_ADMIN \
  --sysctl net.ipv4.ip_forward=1 \
  --sysctl net.ipv4.conf.all.src_valid_mark=1 \
  formationai/formnet:latest

# Monitor logs to verify service is running
echo "Checking formnet startup logs..."
docker logs formation-network 