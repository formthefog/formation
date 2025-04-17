#!/bin/bash

# Create directories if they don't exist
mkdir -p $(pwd)/dns-data
mkdir -p $(pwd)/secrets

# Run form-dns container
docker run --name formation-dns -p 53:53/tcp -p 53:53/udp \
  -v $(pwd)/dns-data:/var/lib/formation/dns \
  -v $(pwd)/secrets:/var/lib/formation/secrets:ro \
  -e DNS_LOG_LEVEL=info \
  -e DNS_UPSTREAM_SERVERS=8.8.8.8,1.1.1.1 \
  -e SECRET_PATH=/var/lib/formation/secrets/config \
  -e PASSWORD=test-password \
  formationai/form-dns:latest

# Verify service is running
echo "Checking form-dns functionality..."
sleep 5
dig @localhost -p 53 localhost 