#!/bin/bash
set -e

# Set defaults if not provided through environment variables
: "${BROKER_API_PORT:=3005}"
: "${BROKER_AMQP_PORT:=5672}"
: "${BROKER_MQTT_PORT:=1883}"
: "${BROKER_DATA_DIR:=/var/lib/formation/broker}"
: "${BROKER_LOG_LEVEL:=info}"
: "${BROKER_STATE_URL:=http://form-state:3004}"

# If the config file doesn't exist, generate it based on environment variables
CONFIG_FILE="${BROKER_CONFIG_PATH:-/etc/formation/broker/default.conf}"
if [ ! -f "$CONFIG_FILE" ] || [ "$REGENERATE_CONFIG" = "true" ]; then
    echo "Generating broker configuration at $CONFIG_FILE"
    cat > "$CONFIG_FILE" << EOF
api_port = $BROKER_API_PORT
amqp_port = $BROKER_AMQP_PORT
mqtt_port = $BROKER_MQTT_PORT
data_dir = "$BROKER_DATA_DIR"
log_level = "$BROKER_LOG_LEVEL"
state_url = "$BROKER_STATE_URL"
EOF
    
    # Add optional configurations if provided
    if [ ! -z "$BROKER_TLS_CERT" ] && [ ! -z "$BROKER_TLS_KEY" ]; then
        echo "tls_cert = \"$BROKER_TLS_CERT\"" >> "$CONFIG_FILE"
        echo "tls_key = \"$BROKER_TLS_KEY\"" >> "$CONFIG_FILE"
    fi
    
    if [ ! -z "$BROKER_AUTH_ENABLED" ]; then
        echo "auth_enabled = $BROKER_AUTH_ENABLED" >> "$CONFIG_FILE"
    fi
    
    if [ ! -z "$BROKER_MAX_CONNECTIONS" ]; then
        echo "max_connections = $BROKER_MAX_CONNECTIONS" >> "$CONFIG_FILE"
    fi
    
    if [ ! -z "$BROKER_MAX_MESSAGE_SIZE" ]; then
        echo "max_message_size = $BROKER_MAX_MESSAGE_SIZE" >> "$CONFIG_FILE"
    fi
fi

# Create data directory if it doesn't exist
if [ ! -d "$BROKER_DATA_DIR" ]; then
    mkdir -p "$BROKER_DATA_DIR"
fi

# Print startup message
echo "Starting form-broker service..."
echo "API port: $BROKER_API_PORT"
echo "AMQP port: $BROKER_AMQP_PORT" 
echo "MQTT port: $BROKER_MQTT_PORT"
echo "Data directory: $BROKER_DATA_DIR"
echo "Log level: $BROKER_LOG_LEVEL"
echo "State service URL: $BROKER_STATE_URL"
echo "Configuration file: $CONFIG_FILE"

# Execute command
exec "$@" 