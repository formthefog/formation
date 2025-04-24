#!/bin/bash
# Run script for form-broker service

set -e

# Default configuration
BROKER_CONFIG_PATH=${BROKER_CONFIG_PATH:-/etc/formation/broker/default.conf}
BROKER_LOG_LEVEL=${BROKER_LOG_LEVEL:-info}
BROKER_API_PORT=${BROKER_API_PORT:-3005}
BROKER_AMQP_PORT=${BROKER_AMQP_PORT:-5672}
BROKER_MQTT_PORT=${BROKER_MQTT_PORT:-1883}
BROKER_DATA_DIR=${BROKER_DATA_DIR:-/var/lib/formation/broker}
BROKER_STATE_URL=${BROKER_STATE_URL:-http://form-state:3004}

# Print service information
echo "Starting form-broker service..."
echo "Config path: $BROKER_CONFIG_PATH"
echo "Log level: $BROKER_LOG_LEVEL"
echo "API port: $BROKER_API_PORT"
echo "AMQP port: $BROKER_AMQP_PORT"
echo "MQTT port: $BROKER_MQTT_PORT"
echo "Data directory: $BROKER_DATA_DIR"
echo "State URL: $BROKER_STATE_URL"

# Check directories exist and are writable
if [ ! -d "$BROKER_DATA_DIR" ]; then
    echo "Creating data directory: $BROKER_DATA_DIR"
    mkdir -p "$BROKER_DATA_DIR"
fi

if [ ! -w "$BROKER_DATA_DIR" ]; then
    echo "Error: Data directory $BROKER_DATA_DIR is not writable"
    exit 1
fi

# If config file doesn't exist, create a basic one
if [ ! -f "$BROKER_CONFIG_PATH" ]; then
    echo "Configuration file not found, creating default configuration..."
    mkdir -p $(dirname $BROKER_CONFIG_PATH)
    cat > $BROKER_CONFIG_PATH << EOF
# Default Broker Service Configuration
api_port = $BROKER_API_PORT
log_level = "$BROKER_LOG_LEVEL"
amqp_port = $BROKER_AMQP_PORT
mqtt_port = $BROKER_MQTT_PORT
data_dir = "$BROKER_DATA_DIR"
state_url = "$BROKER_STATE_URL"
EOF
    echo "Default configuration created at $BROKER_CONFIG_PATH"
fi

# Prepare arguments for form-broker
BROKER_ARGS=""

# Add config path if it exists
if [ -f "$BROKER_CONFIG_PATH" ]; then
    BROKER_ARGS="$BROKER_ARGS --config $BROKER_CONFIG_PATH"
fi

# Add log level
BROKER_ARGS="$BROKER_ARGS --log-level $BROKER_LOG_LEVEL"

# Add ports
BROKER_ARGS="$BROKER_ARGS --api-port $BROKER_API_PORT"
BROKER_ARGS="$BROKER_ARGS --amqp-port $BROKER_AMQP_PORT"
BROKER_ARGS="$BROKER_ARGS --mqtt-port $BROKER_MQTT_PORT"

# Add data directory
BROKER_ARGS="$BROKER_ARGS --data-dir $BROKER_DATA_DIR"

# Add state URL
BROKER_ARGS="$BROKER_ARGS --state-url $BROKER_STATE_URL"

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

# Start form-broker with proper arguments
echo "Starting form-broker with arguments: $BROKER_ARGS"
exec /usr/local/bin/form-broker $BROKER_ARGS 