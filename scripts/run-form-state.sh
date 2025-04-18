#!/bin/bash
# Run script for form-state service

set -e

# Default configuration
CONFIG_PATH=/etc/formation/.operator-config.json
DB_PATH=${DB_PATH:-/var/lib/formation/db/formation.db}

echo "Starting form-state service..."
echo "Config path: $SECRET_PATH"
echo "Database path: $DB_PATH"
echo "Dev mode: $DEV_MODE"

# Check database directory exists and is writable
DB_DIR=$(dirname "$DB_PATH")
if [ ! -d "$DB_DIR" ]; then
    echo "Creating database directory $DB_DIR"
    mkdir -p "$DB_DIR"
fi

# Build command arguments
ARGS="-C $CONFIG_PATH -p $PASSWORD --encrypted"

echo "Starting form-state with arguments: $ARGS"
exec /usr/local/bin/form-state $ARGS  
