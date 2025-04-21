#!/bin/bash
# Simplified run script for vmm-service

set -e

# Get environment variables with defaults
VMM_API_PORT=${VMM_API_PORT:-3002}
VMM_STATE_URL=${VMM_STATE_URL:-http://localhost:3004}
VMM_KERNEL_PATH=${VMM_KERNEL_PATH:-/var/lib/formation/kernel/hypervisor-fw}
VMM_VM_DIR=${VMM_VM_DIR:-/run/form-vmm}
VMM_IMAGES_DIR=${VMM_IMAGES_DIR:-/var/lib/formation/vm-images}
SECRET_PATH=${SECRET_PATH:-/etc/formation/.operator-config.json}
PASSWORD=${PASSWORD:-formation-password}

# Print minimal service information
echo "Starting vmm-service..."
echo "API port: $VMM_API_PORT"
echo "State URL: $VMM_STATE_URL"

# Create necessary directories if they don't exist
mkdir -p $VMM_VM_DIR $VMM_IMAGES_DIR

# Start vmm-service
exec /usr/local/bin/vmm-service --config ${SECRET_PATH} --password ${PASSWORD} run
