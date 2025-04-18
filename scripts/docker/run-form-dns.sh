#!/bin/bash

# Create directories if they don't exist
mkdir -p $(pwd)/dns-data
mkdir -p $(pwd)/secrets

# Make sure form-state is running
if ! curl -s -f http://localhost:3004/health >/dev/null 2>&1; then
  echo "WARNING: form-state service does not appear to be running on port 3004."
  echo "form-dns depends on form-state and may not function correctly."
  echo "Consider running form-state first with: ./scripts/docker/run-form-state.sh"
fi

# Run form-dns container
docker run --rm --name formation-dns \
  --network host \
  --privileged \
  -v $(pwd)/dns-data:/var/lib/formation/dns \
  -v $(pwd)/secrets:/etc/formation \
  -v /var/run/dbus:/var/run/dbus \
  -v /etc/resolv.conf:/etc/resolv.conf \
  -e DNS_LOG_LEVEL=info \
  -e DNS_PORT=53 \
  -e STATE_URL=http://localhost:3004 \
  -e WAIT_FOR_STATE=true \
  formationai/form-dns:latest

# Verify service is running
echo "Checking form-dns functionality..."
sleep 5
dig @localhost formation 