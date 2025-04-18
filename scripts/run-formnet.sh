#!/bin/bash
# Run script for formnet service

set -e

# Default configuration
FORMNET_LOG_LEVEL=${FORMNET_LOG_LEVEL:-debug}
FORMNET_CONFIG_DIR=${FORMNET_CONFIG_DIR:-/etc/formnet}
FORMNET_DATA_DIR=${FORMNET_DATA_DIR:-/var/lib/formnet}
SECRET_PATH=${SECRET_PATH:-/etc/formation/operator-config.json}
PASSWORD=${PASSWORD:-formation-password}
STATE_URL=${STATE_URL:-http://localhost:3004}

echo "Starting formnet setup..."
echo "Log level: $FORMNET_LOG_LEVEL"
echo "Config directory: $FORMNET_CONFIG_DIR"
echo "Data directory: $FORMNET_DATA_DIR"
echo "State URL: $STATE_URL"

# Create required directories
if [ ! -d "$FORMNET_DATA_DIR" ]; then
  echo "Creating data directory $FORMNET_DATA_DIR..."
  mkdir -p "$FORMNET_DATA_DIR"
fi

if [ ! -d "$FORMNET_CONFIG_DIR" ]; then
  echo "Creating config directory $FORMNET_CONFIG_DIR..."
  mkdir -p "$FORMNET_CONFIG_DIR"
fi

# Check if state service is available (with retry)
if [ ! -z "$WAIT_FOR_STATE" ] && [ "$WAIT_FOR_STATE" = "true" ]; then
  MAX_RETRIES=5
  RETRY_COUNT=0
  
  echo "Checking state service availability at $STATE_URL..."
  while [ $RETRY_COUNT -lt $MAX_RETRIES ]; do
    if curl -s -f $STATE_URL/ping >/dev/null 2>&1; then
      echo "State service is available"
      break
    else
      RETRY_COUNT=$((RETRY_COUNT+1))
      if [ $RETRY_COUNT -eq $MAX_RETRIES ]; then
        echo "WARNING: State service at $STATE_URL is not available. Proceeding anyway."
      else
        echo "State service not available, retrying in 2 seconds... (attempt $RETRY_COUNT/$MAX_RETRIES)"
        sleep 2
      fi
    fi
  done
fi

# Enable IP forwarding
echo "Configuring system for network connectivity..."
sysctl -w net.ipv4.ip_forward=1 >/dev/null 2>&1 || echo "WARNING: Failed to enable IP forwarding"
sysctl -w net.ipv4.conf.all.src_valid_mark=1 >/dev/null 2>&1 || echo "WARNING: Failed to set src_valid_mark"

# Clean up any existing configuration
if [ -f "$FORMNET_CONFIG_DIR/formnet.conf" ]; then
  echo "Cleaning up existing formnet configuration..."
  rm -f "$FORMNET_CONFIG_DIR/formnet.conf"
fi

# Leave any existing network
echo "Leaving any existing formnet network..."
/usr/local/bin/formnet operator leave --yes >/dev/null 2>&1 || echo "No existing network to leave"

# Join the network
echo "Joining formnet network..."
echo "Starting formnet operator join with config: $SECRET_PATH"
exec /usr/local/bin/formnet operator join -C "$SECRET_PATH" -p "$PASSWORD" 
