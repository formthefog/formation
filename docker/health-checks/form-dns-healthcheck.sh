#!/bin/bash
# Health check script for form-dns service
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CONTAINER_NAME=$1
HEALTH_SCRIPT="${SCRIPT_DIR}/container-health.sh"

if [ -z "$CONTAINER_NAME" ]; then
    CONTAINER_NAME="formation-form-dns"
fi

echo "Performing health check for form-dns container: $CONTAINER_NAME"
echo "========================================================"

# Check if container is running
docker ps | grep -q "$CONTAINER_NAME"
if [ $? -ne 0 ]; then
    echo "❌ Container $CONTAINER_NAME is not running!"
    exit 1
fi
echo "✅ Container is running"

# Basic checks within the container
docker exec "$CONTAINER_NAME" bash -c "ps aux | grep -v grep | grep -q form-dns"
if [ $? -ne 0 ]; then
    echo "❌ form-dns process is not running inside the container!"
    exit 1
fi
echo "✅ form-dns process is running inside the container"

# Check if DNS ports are exposed
docker exec "$CONTAINER_NAME" bash -c "netstat -tuln | grep -q ':53'"
if [ $? -ne 0 ]; then
    echo "❌ DNS port 53 is not open inside the container!"
    exit 1
fi
echo "✅ DNS port 53 is open inside the container"

# Check if required directories exist
docker exec "$CONTAINER_NAME" bash -c "[ -d /var/lib/formation/dns/zones ]"
if [ $? -ne 0 ]; then
    echo "❌ Directory /var/lib/formation/dns/zones does not exist!"
    exit 1
fi
echo "✅ Directory /var/lib/formation/dns/zones exists"

docker exec "$CONTAINER_NAME" bash -c "[ -d /etc/formation/dns ]"
if [ $? -ne 0 ]; then
    echo "❌ Directory /etc/formation/dns does not exist!"
    exit 1
fi
echo "✅ Directory /etc/formation/dns exists"

# Test actual DNS resolution by creating a temporary test domain
echo "Testing DNS resolution functionality"
TEST_DOMAIN="test.formation.local"
TEST_IP="192.168.100.100"

# Create a temporary test DNS record
docker exec "$CONTAINER_NAME" bash -c "echo '$TEST_DOMAIN. IN A $TEST_IP' > /var/lib/formation/dns/zones/test.zone"
if [ $? -ne 0 ]; then
    echo "❌ Failed to create temporary DNS test zone!"
    exit 1
fi

# Wait a moment for DNS to update
sleep 2

# Test the DNS resolution
RESULT=$(docker exec "$CONTAINER_NAME" bash -c "dig +short $TEST_DOMAIN @localhost")
if [ "$RESULT" != "$TEST_IP" ]; then
    echo "❌ DNS resolution test failed! Expected $TEST_IP, got: $RESULT"
    docker exec "$CONTAINER_NAME" bash -c "rm -f /var/lib/formation/dns/zones/test.zone"
    exit 1
fi
echo "✅ DNS resolution test passed"

# Clean up the temporary zone file
docker exec "$CONTAINER_NAME" bash -c "rm -f /var/lib/formation/dns/zones/test.zone"

echo "========================================================="
echo "✅ All form-dns health checks passed!"
exit 0 