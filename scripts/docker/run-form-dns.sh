#!/bin/bash

# Create directories if they don't exist
mkdir -p $(pwd)/dns-data
mkdir -p $(pwd)/secrets

# Run form-dns container
docker run -d --name formation-dns \
  --network host \
  --privileged \
  -v $(pwd)/dns-data:/var/lib/formation/dns \
  -v $(pwd)/secrets:/etc/formation \
  -v /var/run/dbus:/var/run/dbus \
  -v /etc/resolv.conf:/etc/resolv.conf \
  -v /etc/hosts:/etc/hosts \
  -e DNS_LOG_LEVEL=trace \
  -e RUST_LOG=trace \
  -e RUST_BACKTRACE=full \
  -e DNS_PORT=53 \
  -e STATE_URL=http://localhost:3004 \
  -e WAIT_FOR_STATE=true \
  --cap-add=NET_ADMIN \
  --cap-add=SYS_PTRACE \
  formationai/form-dns:latest
