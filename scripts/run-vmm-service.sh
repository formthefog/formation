#!/bin/bash
# Run script for vmm-service

set -e

# Default configuration
VMM_CONFIG_PATH=${VMM_CONFIG_PATH:-/etc/formation/vmm/default.conf}
VMM_LOG_LEVEL=${VMM_LOG_LEVEL:-info}
VMM_API_PORT=${VMM_API_PORT:-3003}
VMM_STATE_URL=${VMM_STATE_URL:-http://form-state:3004}
VMM_KERNEL_PATH=${VMM_KERNEL_PATH:-/var/lib/formation/kernel/hypervisor-fw}
VMM_VM_DIR=${VMM_VM_DIR:-/run/form-vm}
VMM_IMAGES_DIR=${VMM_IMAGES_DIR:-/var/lib/formation/vm-images}

# Print service information
echo "Starting vmm-service..."
echo "Config path: $VMM_CONFIG_PATH"
echo "Log level: $VMM_LOG_LEVEL"
echo "API port: $VMM_API_PORT"
echo "State URL: $VMM_STATE_URL"
echo "Kernel path: $VMM_KERNEL_PATH"
echo "VM directory: $VMM_VM_DIR"
echo "Images directory: $VMM_IMAGES_DIR"

# Check kernel file exists
if [ ! -f "$VMM_KERNEL_PATH" ]; then
    echo "Error: Kernel file not found at $VMM_KERNEL_PATH"
    exit 1
fi

# Check directories exist and are writable
for DIR in "$VMM_VM_DIR" "$VMM_IMAGES_DIR"; do
    if [ ! -d "$DIR" ]; then
        echo "Creating directory $DIR"
        mkdir -p "$DIR"
    fi

    if [ ! -w "$DIR" ]; then
        echo "Error: Directory $DIR is not writable"
        exit 1
    fi
done

# If config file doesn't exist, create a basic one
if [ ! -f "$VMM_CONFIG_PATH" ]; then
    echo "Configuration file not found, creating default configuration..."
    mkdir -p $(dirname $VMM_CONFIG_PATH)
    cat > $VMM_CONFIG_PATH << EOF
# Default VMM Service Configuration
api_port = $VMM_API_PORT
log_level = "$VMM_LOG_LEVEL"
state_url = "$VMM_STATE_URL"
kernel_path = "$VMM_KERNEL_PATH"
vm_dir = "$VMM_VM_DIR"
images_dir = "$VMM_IMAGES_DIR"
EOF
    echo "Default configuration created at $VMM_CONFIG_PATH"
fi

# Check if KVM is available
if [ ! -c /dev/kvm ]; then
    echo "Warning: /dev/kvm not found. Hardware virtualization may not be available."
    echo "VMs will run much slower without hardware virtualization support."
else
    # Set proper permissions for /dev/kvm if needed
    if [ ! -r /dev/kvm ] || [ ! -w /dev/kvm ]; then
        echo "Setting permissions for /dev/kvm"
        chmod 666 /dev/kvm
    fi
fi

# Load required kernel modules if running in privileged mode
if [ -w /sys/module ]; then
    echo "Loading required kernel modules..."
    modprobe kvm || echo "Warning: Failed to load kvm module"
    modprobe kvm_intel || modprobe kvm_amd || echo "Warning: Failed to load CPU-specific KVM module"
    modprobe vhost_net || echo "Warning: Failed to load vhost_net module"
fi

# Prepare arguments for vmm-service
VMM_ARGS=""

# Add config path if it exists
if [ -f "$VMM_CONFIG_PATH" ]; then
    VMM_ARGS="$VMM_ARGS --config $VMM_CONFIG_PATH"
fi

# Add log level
VMM_ARGS="$VMM_ARGS --log-level $VMM_LOG_LEVEL"

# Add API port
VMM_ARGS="$VMM_ARGS --port $VMM_API_PORT"

# Add state URL
VMM_ARGS="$VMM_ARGS --state-url $VMM_STATE_URL"

# Add additional paths
VMM_ARGS="$VMM_ARGS --kernel-path $VMM_KERNEL_PATH"
VMM_ARGS="$VMM_ARGS --vm-dir $VMM_VM_DIR"
VMM_ARGS="$VMM_ARGS --images-dir $VMM_IMAGES_DIR"

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

# Clean up any stale VM resources from previous runs
echo "Cleaning up any stale VM resources..."
for VM_DIR in $VMM_VM_DIR/*; do
    if [ -d "$VM_DIR" ]; then
        echo "Cleaning up $VM_DIR"
        rm -rf "$VM_DIR"
    fi
done

# Start vmm-service with proper arguments
echo "Starting vmm-service with arguments: $VMM_ARGS"
exec /usr/local/bin/vmm-service $VMM_ARGS
