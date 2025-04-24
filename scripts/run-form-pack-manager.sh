#!/bin/bash
# Simplified run script for form-pack-manager service

set -e

# Get environment variables with defaults
PACK_MANAGER_PORT=${PACK_MANAGER_PORT:-3003}
PACK_MANAGER_INTERFACE=${PACK_MANAGER_INTERFACE:-all}
PACK_MANAGER_CONFIG_PATH=${PACK_MANAGER_CONFIG_PATH:-/etc/formation/.operator-config.json}
PACK_MANAGER_PASSWORD=${PACK_MANAGER_PASSWORD:-formation-password}
PACK_MANAGER_DATA_DIR=${PACK_MANAGER_DATA_DIR:-/var/lib/formation/pack-manager}

# Print minimal service information
echo "Starting form-pack-manager service..."
echo "API port: $PACK_MANAGER_PORT"
echo "Data directory: $PACK_MANAGER_DATA_DIR"

# Create necessary directories if they don't exist
mkdir -p $PACK_MANAGER_DATA_DIR

# Start form-pack-manager
exec /usr/local/bin/form-pack-manager \
  --interface $PACK_MANAGER_INTERFACE \
  --port $PACK_MANAGER_PORT \
  --config $PACK_MANAGER_CONFIG_PATH \
  --password $PACK_MANAGER_PASSWORD 