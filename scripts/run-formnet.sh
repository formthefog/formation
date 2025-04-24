#!/bin/bash
# Run script for formnet service

set -e

# Default configuration
FORMNET_LOG_LEVEL=${FORMNET_LOG_LEVEL:-debug}
FORMNET_CONFIG_DIR=${FORMNET_CONFIG_DIR:-/etc/formation}
FORMNET_DATA_DIR=${FORMNET_DATA_DIR:-/var/lib/formnet}
SECRET_PATH=${SECRET_PATH:-/etc/formation/.operator-config.json}
PASSWORD=${PASSWORD:-formation-password}
STATE_URL=${STATE_URL:-http://localhost:3004}
API_KEY=${API_KEY:-}

echo "Starting formnet setup..."
echo "Log level: $FORMNET_LOG_LEVEL"
echo "Config directory: $FORMNET_CONFIG_DIR"
echo "Data directory: $FORMNET_DATA_DIR"
echo "State URL: $STATE_URL"

exec /usr/local/bin/formnet operator join -C "$SECRET_PATH" -p "$PASSWORD"
