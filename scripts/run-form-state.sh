#!/bin/bash
# Run script for form-state service

set -e

# Default configuration
STATE_CONFIG_PATH=${STATE_CONFIG_PATH:-/etc/formation/state/default.conf}
STATE_LOG_LEVEL=${STATE_LOG_LEVEL:-info}
STATE_API_PORT=${STATE_API_PORT:-3004}
STATE_DB_PATH=${STATE_DB_PATH:-/var/lib/formation/db/state.db}
AUTH_MODE=${AUTH_MODE:-production}
MARKETPLACE_ENABLED=${MARKETPLACE_ENABLED:-true}
BILLING_ENABLED=${BILLING_ENABLED:-true}
API_KEYS_ENABLED=${API_KEYS_ENABLED:-true}

# Print service information
echo "Starting form-state service..."
echo "Config path: $STATE_CONFIG_PATH"
echo "Log level: $STATE_LOG_LEVEL"
echo "API port: $STATE_API_PORT"
echo "Database path: $STATE_DB_PATH"
echo "Auth mode: $AUTH_MODE"
echo "Marketplace enabled: $MARKETPLACE_ENABLED"
echo "Billing enabled: $BILLING_ENABLED"
echo "API keys enabled: $API_KEYS_ENABLED"

# Check database directory exists and is writable
DB_DIR=$(dirname "$STATE_DB_PATH")
if [ ! -d "$DB_DIR" ]; then
    echo "Creating database directory $DB_DIR"
    mkdir -p "$DB_DIR"
fi

if [ ! -w "$DB_DIR" ]; then
    echo "Error: Database directory $DB_DIR is not writable"
    exit 1
fi

# If config file doesn't exist, create a basic one
if [ ! -f "$STATE_CONFIG_PATH" ]; then
    echo "Configuration file not found, creating default configuration..."
    mkdir -p $(dirname $STATE_CONFIG_PATH)
    cat > $STATE_CONFIG_PATH << EOF
# Default State Service Configuration
api_port = $STATE_API_PORT
log_level = "$STATE_LOG_LEVEL"
db_path = "$STATE_DB_PATH"
auth_mode = "$AUTH_MODE"
marketplace_enabled = $MARKETPLACE_ENABLED
billing_enabled = $BILLING_ENABLED
api_keys_enabled = $API_KEYS_ENABLED
EOF
    echo "Default configuration created at $STATE_CONFIG_PATH"
fi

# Prepare arguments for form-state
STATE_ARGS=""

# Add config path if it exists
if [ -f "$STATE_CONFIG_PATH" ]; then
    STATE_ARGS="$STATE_ARGS --config $STATE_CONFIG_PATH"
fi

# Add log level
STATE_ARGS="$STATE_ARGS --log-level $STATE_LOG_LEVEL"

# Add API port
STATE_ARGS="$STATE_ARGS --port $STATE_API_PORT"

# Add database path
STATE_ARGS="$STATE_ARGS --db-path $STATE_DB_PATH"

# Add feature flags
if [ "$MARKETPLACE_ENABLED" = "true" ]; then
    STATE_ARGS="$STATE_ARGS --enable-marketplace"
fi

if [ "$BILLING_ENABLED" = "true" ]; then
    STATE_ARGS="$STATE_ARGS --enable-billing"
fi

if [ "$API_KEYS_ENABLED" = "true" ]; then
    STATE_ARGS="$STATE_ARGS --enable-api-keys"
fi

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

# Initialize the database if it doesn't exist
if [ ! -f "$STATE_DB_PATH" ]; then
    echo "Initializing state database at $STATE_DB_PATH"
    /usr/local/bin/form-state $STATE_ARGS --init-db
    if [ $? -ne 0 ]; then
        echo "Failed to initialize database"
        exit 1
    fi
    echo "Database initialized successfully"
fi

# Start form-state with proper arguments
echo "Starting form-state with arguments: $STATE_ARGS"
exec /usr/local/bin/form-state $STATE_ARGS  
