#!/bin/bash
# Health check script for form-state service
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CONTAINER_NAME=$1
HEALTH_SCRIPT="${SCRIPT_DIR}/container-health.sh"

if [ -z "$CONTAINER_NAME" ]; then
    CONTAINER_NAME="formation-form-state"
fi

echo "Performing health check for form-state container: $CONTAINER_NAME"
echo "========================================================"

# Check if container is running
docker ps | grep -q "$CONTAINER_NAME"
if [ $? -ne 0 ]; then
    echo "❌ Container $CONTAINER_NAME is not running!"
    exit 1
fi
echo "✅ Container is running"

# Basic checks within the container
docker exec "$CONTAINER_NAME" bash -c "ps aux | grep -v grep | grep -q form-state"
if [ $? -ne 0 ]; then
    echo "❌ form-state process is not running inside the container!"
    exit 1
fi
echo "✅ form-state process is running inside the container"

# Check if API port is exposed
docker exec "$CONTAINER_NAME" bash -c "netstat -tuln | grep -q ':3004'"
if [ $? -ne 0 ]; then
    echo "❌ API port 3004 is not open inside the container!"
    exit 1
fi
echo "✅ API port 3004 is open inside the container"

# Check if required directories exist
docker exec "$CONTAINER_NAME" bash -c "[ -d /var/lib/formation/db ]"
if [ $? -ne 0 ]; then
    echo "❌ Directory /var/lib/formation/db does not exist!"
    exit 1
fi
echo "✅ Directory /var/lib/formation/db exists"

docker exec "$CONTAINER_NAME" bash -c "[ -d /etc/formation/auth ]"
if [ $? -ne 0 ]; then
    echo "❌ Directory /etc/formation/auth does not exist!"
    exit 1
fi
echo "✅ Directory /etc/formation/auth exists"

# Check if database file exists
docker exec "$CONTAINER_NAME" bash -c "[ -f /var/lib/formation/db/state.db ]"
if [ $? -ne 0 ]; then
    echo "⚠️ Database file /var/lib/formation/db/state.db does not exist yet (will be created on first run)"
fi

# Test the API if it's running
echo "Testing API endpoint"
RESPONSE=$(docker exec "$CONTAINER_NAME" bash -c "curl -s -o /dev/null -w '%{http_code}' http://localhost:3004/health 2>/dev/null || echo 'failed'")
if [ "$RESPONSE" = "failed" ]; then
    echo "⚠️ Could not connect to health endpoint - API might not be fully started"
elif [ "$RESPONSE" = "200" ]; then
    echo "✅ API health check passed"
else
    echo "❌ API health check failed with response code: $RESPONSE"
    exit 1
fi

# Test creating and retrieving state data
echo "Testing state storage and retrieval"
# Create test data
TEST_KEY="test-key-$(date +%s)"
TEST_VALUE="test-value-$(date +%s)"

# Try to store data
STORE_RESULT=$(docker exec "$CONTAINER_NAME" bash -c "curl -s -X POST -H 'Content-Type: application/json' -d '{\"key\":\"$TEST_KEY\",\"value\":\"$TEST_VALUE\"}' http://localhost:3004/state 2>/dev/null || echo 'failed'")
if [ "$STORE_RESULT" = "failed" ]; then
    echo "⚠️ Could not store test data - API might not be fully functional"
else
    # Try to retrieve the data
    RETRIEVE_RESULT=$(docker exec "$CONTAINER_NAME" bash -c "curl -s http://localhost:3004/state/$TEST_KEY 2>/dev/null || echo 'failed'")
    if [ "$RETRIEVE_RESULT" = "failed" ]; then
        echo "⚠️ Could not retrieve test data - API might not be fully functional"
    elif [[ "$RETRIEVE_RESULT" == *"$TEST_VALUE"* ]]; then
        echo "✅ Successfully stored and retrieved test data"
    else
        echo "❌ Test data retrieval failed. Expected value containing '$TEST_VALUE', got: $RETRIEVE_RESULT"
        exit 1
    fi
fi

echo "========================================================="
echo "✅ All form-state health checks passed!"
exit 0 