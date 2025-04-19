#!/bin/bash
# Extended verification tests for form-state service
set -e

CONTAINER_NAME="formation-form-state"
API_PORT=3004
API_ENDPOINT="http://localhost:$API_PORT"

echo "===================================================="
echo "Form-State Extended Verification Tests"
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

# Test 3: Store and retrieve data with different data types
echo "Testing data storage and retrieval with different data types..."

# 3.1 String data
STRING_KEY="test-string-$(date +%s)"
STRING_VALUE="Test string value $(date)"
echo "  - Testing string data ($STRING_KEY)..."
curl -s -X POST -H "Content-Type: application/json" -d "{\"key\":\"$STRING_KEY\",\"value\":\"$STRING_VALUE\"}" $API_ENDPOINT/state > /dev/null
STRING_RESULT=$(curl -s $API_ENDPOINT/state/$STRING_KEY)
if [[ "$STRING_RESULT" != *"$STRING_VALUE"* ]]; then
    echo "❌ String data test failed: got $STRING_RESULT, expected content with $STRING_VALUE"
    exit 1
fi
echo "  ✅ String data test passed"

# 3.2 Numeric data
NUMERIC_KEY="test-numeric-$(date +%s)"
NUMERIC_VALUE=12345
echo "  - Testing numeric data ($NUMERIC_KEY)..."
curl -s -X POST -H "Content-Type: application/json" -d "{\"key\":\"$NUMERIC_KEY\",\"value\":$NUMERIC_VALUE}" $API_ENDPOINT/state > /dev/null
NUMERIC_RESULT=$(curl -s $API_ENDPOINT/state/$NUMERIC_KEY)
if [[ "$NUMERIC_RESULT" != *"$NUMERIC_VALUE"* ]]; then
    echo "❌ Numeric data test failed: got $NUMERIC_RESULT, expected content with $NUMERIC_VALUE"
    exit 1
fi
echo "  ✅ Numeric data test passed"

# 3.3 JSON data
JSON_KEY="test-json-$(date +%s)"
echo "  - Testing JSON data ($JSON_KEY)..."
curl -s -X POST -H "Content-Type: application/json" -d "{\"key\":\"$JSON_KEY\",\"value\":{\"name\":\"test\",\"value\":123,\"nested\":{\"inner\":true}}}" $API_ENDPOINT/state > /dev/null
JSON_RESULT=$(curl -s $API_ENDPOINT/state/$JSON_KEY)
if [[ "$JSON_RESULT" != *"\"name\":\"test\""* ]] || [[ "$JSON_RESULT" != *"\"inner\":true"* ]]; then
    echo "❌ JSON data test failed: got $JSON_RESULT"
    exit 1
fi
echo "  ✅ JSON data test passed"

# Test 4: Update existing data
echo "Testing data update..."
UPDATE_KEY="test-update-$(date +%s)"
ORIGINAL_VALUE="Original value"
UPDATED_VALUE="Updated value"

# Store original value
curl -s -X POST -H "Content-Type: application/json" -d "{\"key\":\"$UPDATE_KEY\",\"value\":\"$ORIGINAL_VALUE\"}" $API_ENDPOINT/state > /dev/null

# Update value
curl -s -X POST -H "Content-Type: application/json" -d "{\"key\":\"$UPDATE_KEY\",\"value\":\"$UPDATED_VALUE\"}" $API_ENDPOINT/state > /dev/null

# Retrieve and verify
UPDATE_RESULT=$(curl -s $API_ENDPOINT/state/$UPDATE_KEY)
if [[ "$UPDATE_RESULT" != *"$UPDATED_VALUE"* ]]; then
    echo "❌ Update test failed: got $UPDATE_RESULT, expected content with $UPDATED_VALUE"
    exit 1
fi
echo "✅ Update test passed"

# Test 5: Retrieve non-existent data
echo "Testing retrieval of non-existent data..."
NONEXISTENT_KEY="nonexistent-key-$(date +%s)"
NONEXISTENT_RESULT=$(curl -s -w "%{http_code}" -o /dev/null $API_ENDPOINT/state/$NONEXISTENT_KEY)
if [ "$NONEXISTENT_RESULT" != "404" ]; then
    echo "❌ Non-existent data test failed: got status $NONEXISTENT_RESULT, expected 404"
    exit 1
fi
echo "✅ Non-existent data test passed"

# Test 6: Delete data
echo "Testing data deletion..."
DELETE_KEY="test-delete-$(date +%s)"
DELETE_VALUE="Value to be deleted"

# Store data
curl -s -X POST -H "Content-Type: application/json" -d "{\"key\":\"$DELETE_KEY\",\"value\":\"$DELETE_VALUE\"}" $API_ENDPOINT/state > /dev/null

# Delete data
DELETE_RESULT=$(curl -s -X DELETE -w "%{http_code}" -o /dev/null $API_ENDPOINT/state/$DELETE_KEY)
if [ "$DELETE_RESULT" != "200" ] && [ "$DELETE_RESULT" != "204" ]; then
    echo "❌ Delete test failed: got status $DELETE_RESULT, expected 200 or 204"
    exit 1
fi

# Verify data is deleted
VERIFY_DELETE=$(curl -s -w "%{http_code}" -o /dev/null $API_ENDPOINT/state/$DELETE_KEY)
if [ "$VERIFY_DELETE" != "404" ]; then
    echo "❌ Delete verification failed: got status $VERIFY_DELETE, expected 404"
    exit 1
fi
echo "✅ Delete test passed"

# Test 7: List all keys
echo "Testing list all keys..."
LIST_RESULT=$(curl -s $API_ENDPOINT/state)
if [ -z "$LIST_RESULT" ]; then
    echo "❌ List all keys test failed: no data returned"
    exit 1
fi
echo "✅ List all keys test passed"

# Test 8: Performance test
echo "Testing API performance..."
START_TIME=$(date +%s.%N)
for i in {1..10}; do
    TEST_KEY="perf-test-$i"
    TEST_VALUE="Performance test value $i"
    curl -s -X POST -H "Content-Type: application/json" -d "{\"key\":\"$TEST_KEY\",\"value\":\"$TEST_VALUE\"}" $API_ENDPOINT/state > /dev/null
    curl -s $API_ENDPOINT/state/$TEST_KEY > /dev/null
done
END_TIME=$(date +%s.%N)
DURATION=$(echo "$END_TIME - $START_TIME" | bc)
OPERATIONS_PER_SEC=$(echo "20 / $DURATION" | bc -l)  # 10 writes + 10 reads

echo "Performance: $OPERATIONS_PER_SEC operations per second"
if (( $(echo "$OPERATIONS_PER_SEC < 1.0" | bc -l) )); then
    echo "❌ API performance is below threshold"
    exit 1
fi
echo "✅ API performance is acceptable"

# Test 9: Verify database file has been created
echo "Checking database file creation..."
DB_CHECK=$(docker exec "$CONTAINER_NAME" bash -c "ls -la /var/lib/formation/db/state.db 2>/dev/null || echo 'not found'")
if [[ "$DB_CHECK" == *"not found"* ]]; then
    echo "❌ Database file was not created"
    exit 1
fi
echo "✅ Database file was created successfully"

# Test 10: Check log output
echo "Checking service logs..."
LOG_TEST=$(docker logs "$CONTAINER_NAME" 2>&1 | grep -E 'error|failed')
if [ $? -eq 0 ]; then
    echo "⚠️ Warning: Potential errors found in logs:"
    docker logs "$CONTAINER_NAME" 2>&1 | grep -E 'error|failed'
fi

echo "===================================================="
echo "✅ Form-State extended verification completed successfully"
echo "===================================================="
exit 0 