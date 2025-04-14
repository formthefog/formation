#!/bin/bash
# Run script for form-dns service

set -e

# Default configuration
DNS_CONFIG_PATH=${DNS_CONFIG_PATH:-/etc/formation/dns/default.conf}
DNS_LOG_LEVEL=${DNS_LOG_LEVEL:-info}
DNS_LISTEN_PORT=${DNS_LISTEN_PORT:-53}
DNS_CACHE_SIZE=${DNS_CACHE_SIZE:-1000}
DNS_UPSTREAM_SERVERS=${DNS_UPSTREAM_SERVERS:-"8.8.8.8,1.1.1.1"}

# Print service information
echo "Starting form-dns service..."
echo "Config path: $DNS_CONFIG_PATH"
echo "Log level: $DNS_LOG_LEVEL"
echo "Listen port: $DNS_LISTEN_PORT"
echo "Cache size: $DNS_CACHE_SIZE"
echo "Upstream servers: $DNS_UPSTREAM_SERVERS"

# If config file doesn't exist, create a basic one
if [ ! -f "$DNS_CONFIG_PATH" ]; then
    echo "Configuration file not found, creating default configuration..."
    mkdir -p $(dirname $DNS_CONFIG_PATH)
    cat > $DNS_CONFIG_PATH << EOF
# Default DNS Configuration
listen_port = $DNS_LISTEN_PORT
log_level = "$DNS_LOG_LEVEL"
cache_size = $DNS_CACHE_SIZE
upstream_servers = ["$(echo $DNS_UPSTREAM_SERVERS | sed 's/,/","/g')"]
EOF
    echo "Default configuration created at $DNS_CONFIG_PATH"
fi

# Prepare arguments for form-dns
DNS_ARGS=""

# Add config path if it exists
if [ -f "$DNS_CONFIG_PATH" ]; then
    DNS_ARGS="$DNS_ARGS --config $DNS_CONFIG_PATH"
fi

# Add log level
DNS_ARGS="$DNS_ARGS --log-level $DNS_LOG_LEVEL"

# Wait for dependent services (if any)
if [ ! -z "$WAIT_FOR" ]; then
    echo "Waiting for dependent services: $WAIT_FOR"
    for service in $(echo $WAIT_FOR | tr ',' ' '); do
        host=$(echo $service | cut -d: -f1)
        port=$(echo $service | cut -d: -f2)
        echo "Waiting for $host:$port..."
        
        until nc -z $host $port; do
            echo "Waiting for $host:$port..."
            sleep 1
        done
        
        echo "$host:$port is available"
    done
fi

# Start form-dns with proper arguments
echo "Starting form-dns with arguments: $DNS_ARGS"
exec /usr/local/bin/form-dns $DNS_ARGS
