#!/bin/bash
# Extended verification tests for vmm-service
set -e

CONTAINER_NAME="formation-vmm-service"
API_PORT=3002
API_ENDPOINT="http://localhost:$API_PORT"

echo "===================================================="
echo "VMM-Service Extended Verification Tests"
echo "===================================================="

# Check container is running
docker ps | grep -q "$CONTAINER_NAME"
if [ $? -ne 0 ]; then
    echo "❌ Container $CONTAINER_NAME is not running!"
    exit 1
fi
echo "✅ Container is running"

# Test 1: Verify container ports are accessible from host
echo "Testing container port accessibility from host..."
PORT_TEST=$(nc -z localhost $API_PORT && echo "success" || echo "failed")
if [ "$PORT_TEST" != "success" ]; then
    echo "❌ Unable to connect to API port from host"
    exit 1
fi
echo "✅ API port is accessible from host"

# Test 2: Health check endpoint
echo "Testing API health endpoint..."
HEALTH_RESPONSE=$(curl -s -o /dev/null -w "%{http_code}" $API_ENDPOINT/health)
if [ "$HEALTH_RESPONSE" != "200" ]; then
    echo "❌ Health check failed: got status $HEALTH_RESPONSE, expected 200"
    exit 1
fi
echo "✅ Health check passed"

# Test 3: Verify required directories
echo "Testing required directories..."
REQUIRED_DIRS=(
    "/var/lib/formation/vm-images"
    "/run/form-vm"
)

for dir in "${REQUIRED_DIRS[@]}"; do
    DIR_CHECK=$(docker exec "$CONTAINER_NAME" bash -c "[ -d $dir ] && echo 'exists' || echo 'missing'")
    if [ "$DIR_CHECK" != "exists" ]; then
        echo "❌ Required directory $dir is missing"
        exit 1
    fi
done
echo "✅ All required directories exist"

# Test 4: Check virtualization capabilities
echo "Testing virtualization capabilities..."
KVM_CHECK=$(docker exec "$CONTAINER_NAME" bash -c "[ -c /dev/kvm ] && echo 'available' || echo 'missing'")
if [ "$KVM_CHECK" = "missing" ]; then
    echo "⚠️ Warning: /dev/kvm is not available in the container"
    echo "    This will limit virtualization capabilities"
fi

# Check if QEMU/KVM is working
QEMU_CHECK=$(docker exec "$CONTAINER_NAME" bash -c "command -v qemu-system-x86_64 >/dev/null && echo 'available' || echo 'missing'")
if [ "$QEMU_CHECK" = "missing" ]; then
    echo "❌ QEMU is not available in the container"
    exit 1
fi
echo "✅ QEMU is available in the container"

# Test 5: Check hypervisor firmware
echo "Testing hypervisor firmware availability..."
FIRMWARE_CHECK=$(docker exec "$CONTAINER_NAME" bash -c "[ -f /var/lib/formation/kernel/hypervisor-fw ] && echo 'exists' || echo 'missing'")
if [ "$FIRMWARE_CHECK" = "missing" ]; then
    echo "❌ Hypervisor firmware is missing"
    exit 1
fi
echo "✅ Hypervisor firmware is available"

# Test 6: Test VM operations if API supports them
echo "Testing VM API operations..."

# 6.1 List VMs (should work even with no VMs)
echo "  - Testing VM listing..."
LIST_RESPONSE=$(curl -s -o /dev/null -w "%{http_code}" $API_ENDPOINT/vms)
if [ "$LIST_RESPONSE" != "200" ]; then
    echo "❌ VM listing failed: got status $LIST_RESPONSE, expected 200"
    exit 1
fi
echo "  ✅ VM listing works"

# 6.2 Check VM creation capability (we won't actually create one as it needs images)
echo "  - Testing VM template information..."
TEMPLATE_RESPONSE=$(curl -s -o /dev/null -w "%{http_code}" $API_ENDPOINT/templates)
if [ "$TEMPLATE_RESPONSE" != "200" ] && [ "$TEMPLATE_RESPONSE" != "404" ]; then
    echo "❌ Template information failed: got status $TEMPLATE_RESPONSE"
    exit 1
fi
if [ "$TEMPLATE_RESPONSE" = "200" ]; then
    echo "  ✅ VM template information is available"
else
    echo "  ⚠️ VM template information endpoint not found (404), but this is acceptable for basic verification"
fi

# Test 7: Check network capabilities
echo "Testing network configuration..."
NETWORK_CHECK=$(docker exec "$CONTAINER_NAME" bash -c "ip a | grep -c 'eth0'")
if [ "$NETWORK_CHECK" -eq 0 ]; then
    echo "❌ Container network interface not found"
    exit 1
fi
echo "✅ Container network interface is available"

# Test 8: Check dependencies
echo "Testing required dependencies..."
DEPENDENCIES=(
    "qemu-kvm"
    "curl"
    "ip"
    "bridge"
)

for dep in "${DEPENDENCIES[@]}"; do
    DEP_CHECK=$(docker exec "$CONTAINER_NAME" bash -c "command -v $dep >/dev/null && echo 'available' || echo 'missing'")
    if [ "$DEP_CHECK" = "missing" ]; then
        echo "❌ Required dependency '$dep' is missing"
        exit 1
    fi
done
echo "✅ All required dependencies are available"

# Test 9: Check privileged capabilities (required for VM management)
echo "Testing privileged capabilities..."
CAP_CHECK=$(docker exec "$CONTAINER_NAME" bash -c "capsh --print | grep -q 'cap_sys_admin' && echo 'available' || echo 'missing'")
if [ "$CAP_CHECK" = "missing" ]; then
    echo "⚠️ Warning: Container may not have required capabilities for full VM management"
    echo "    This might limit some VM operations"
fi

# Test 10: Check log output
echo "Checking service logs..."
LOG_TEST=$(docker logs "$CONTAINER_NAME" 2>&1 | grep -E 'error|failed')
if [ $? -eq 0 ]; then
    echo "⚠️ Warning: Potential errors found in logs:"
    docker logs "$CONTAINER_NAME" 2>&1 | grep -E 'error|failed'
fi

echo "===================================================="
echo "✅ VMM-Service extended verification completed"
echo "===================================================="
exit 0 