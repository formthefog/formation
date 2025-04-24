#!/bin/bash
# Run script for form-dns service

set -e

# Default configuration
DNS_LOG_LEVEL=${DNS_LOG_LEVEL:-info}
DNS_PORT=${DNS_PORT:-5453}
STATE_URL=${STATE_URL:-http://localhost:3004}

echo "Starting form-dns service..."
echo "DNS port: $DNS_PORT"
echo "Log level: $DNS_LOG_LEVEL"
echo "State URL: $STATE_URL"

# Check dependent services
if [ ! -z "$WAIT_FOR_STATE" ] && [ "$WAIT_FOR_STATE" = "true" ]; then
    echo "Waiting for form-state service..."
    until curl -s -f $STATE_URL/ping >/dev/null 2>&1; do
        echo "Waiting for form-state at $STATE_URL..."
        sleep 2
    done
    echo "form-state is available!"
fi

# Start form-dns
echo "Starting form-dns"
exec /usr/local/bin/form-dns
