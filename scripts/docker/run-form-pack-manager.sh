#!/bin/bash

# Create directories if they don't exist
mkdir -p $(pwd)/pack-data
mkdir -p $(pwd)/secrets

# Run form-pack-manager container
docker run --name formation-pack-manager -p 3003:3003 \
  -v $(pwd)/pack-data:/var/lib/formation/packs \
  -v /var/run/docker.sock:/var/run/docker.sock \
  -v $(pwd)/secrets:/var/lib/formation/secrets:ro \
  -e PACK_MANAGER_LOG_LEVEL=info \
  -e SECRET_PATH=/var/lib/formation/secrets/config \
  -e PASSWORD=test-password \
  --privileged \
  formationai/form-pack-manager:latest

# Verify service is running
echo "Checking form-pack-manager health..."
sleep 5
curl http://localhost:3003/health 