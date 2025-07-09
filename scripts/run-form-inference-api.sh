#!/bin/bash
# Run script for form-inference-api service

set -e

# Default configuration
CONFIG_PATH=${CONFIG_PATH:-/etc/formation/config/.operator-config.json}

echo "Starting form-inference-api service..."
echo "Config path: $CONFIG_PATH"

# Check if config file exists
if [ ! -f "$CONFIG_PATH" ]; then
    echo "Error: Configuration file not found at $CONFIG_PATH"
    echo "Please mount the operator config file to this path"
    exit 1
fi

# Build command arguments
ARGS="--config $CONFIG_PATH"

echo "Starting form-inference-api with arguments: $ARGS"
exec /usr/local/bin/form-inference-api $ARGS