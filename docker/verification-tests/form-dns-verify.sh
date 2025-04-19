#!/bin/bash
# Extended verification tests for form-dns service
set -e

CONTAINER_NAME="formation-form-dns"
TEST_DOMAINS=(
    "example.com"
    "formation.local"
    "service.formation.local"
    "test-1.formation.local"
    "test-2.formation.local"
)

echo "===================================================="
echo "Form-DNS Extended Verification Tests"
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
PORT_TEST=$(nc -z localhost 53 && echo "success" || echo "failed")
if [ "$PORT_TEST" != "success" ]; then
    echo "❌ Unable to connect to DNS port from host"
    exit 1
fi
echo "✅ DNS port is accessible from host"

# Test 2: Create multiple test zones
echo "Testing zone file creation and loading..."
for i in {1..3}; do
    TEST_DOMAIN="test-$i.formation.local"
    TEST_IP="192.168.100.$i"
    
    # Create a test zone file
    docker exec "$CONTAINER_NAME" bash -c "echo '$TEST_DOMAIN. IN A $TEST_IP' > /var/lib/formation/dns/zones/test-$i.zone"
    
    # Verify the file was created
    docker exec "$CONTAINER_NAME" bash -c "ls -la /var/lib/formation/dns/zones/test-$i.zone"
    if [ $? -ne 0 ]; then
        echo "❌ Failed to create zone file for $TEST_DOMAIN"
        exit 1
    fi
done
echo "✅ Multiple zone files created successfully"

# Wait for DNS to load new zones
sleep 3

# Test 3: Resolve multiple domains
echo "Testing multiple domain resolution..."
FAILED_DOMAINS=()
for i in {1..3}; do
    TEST_DOMAIN="test-$i.formation.local"
    TEST_IP="192.168.100.$i"
    
    RESULT=$(docker exec "$CONTAINER_NAME" bash -c "dig +short $TEST_DOMAIN @localhost")
    if [ "$RESULT" != "$TEST_IP" ]; then
        echo "❌ Failed to resolve $TEST_DOMAIN (got: $RESULT, expected: $TEST_IP)"
        FAILED_DOMAINS+=("$TEST_DOMAIN")
    fi
done

if [ ${#FAILED_DOMAINS[@]} -gt 0 ]; then
    echo "❌ Some domains failed to resolve: ${FAILED_DOMAINS[*]}"
    exit 1
fi
echo "✅ All test domains resolved successfully"

# Test 4: Test DNS server performance
echo "Testing DNS server performance..."
START_TIME=$(date +%s.%N)
for i in {1..10}; do
    docker exec "$CONTAINER_NAME" bash -c "dig +short test-1.formation.local @localhost > /dev/null"
done
END_TIME=$(date +%s.%N)
DURATION=$(echo "$END_TIME - $START_TIME" | bc)
QUERIES_PER_SEC=$(echo "10 / $DURATION" | bc -l)

echo "Performance: $QUERIES_PER_SEC queries per second"
if (( $(echo "$QUERIES_PER_SEC < 1.0" | bc -l) )); then
    echo "❌ DNS server performance is below threshold"
    exit 1
fi
echo "✅ DNS server performance is acceptable"

# Test 5: Test external DNS resolution
echo "Testing external DNS resolution..."
EXTERNAL_TEST=$(docker exec "$CONTAINER_NAME" bash -c "dig +short example.com @8.8.8.8 | head -1")
if [ -z "$EXTERNAL_TEST" ]; then
    echo "❌ External DNS resolution failed"
    exit 1
fi
echo "✅ External DNS resolution successful"

# Test 6: Check log output
echo "Checking DNS server logs..."
LOG_TEST=$(docker logs "$CONTAINER_NAME" 2>&1 | grep -E 'error|failed')
if [ $? -eq 0 ]; then
    echo "⚠️ Warning: Potential errors found in logs:"
    docker logs "$CONTAINER_NAME" 2>&1 | grep -E 'error|failed'
fi

# Clean up test zone files
echo "Cleaning up test zone files..."
for i in {1..3}; do
    docker exec "$CONTAINER_NAME" bash -c "rm -f /var/lib/formation/dns/zones/test-$i.zone"
done
echo "✅ Test zone files cleaned up"

echo "===================================================="
echo "✅ Form-DNS extended verification completed successfully"
echo "===================================================="
exit 0 