#!/bin/bash

# Base URL
BASE_URL="http://localhost:3004"

echo "Testing public endpoints (should work without authentication):"
echo "-----------------------------------------------------------------"

# Test the health endpoint
echo "Testing /health endpoint:"
curl -s "$BASE_URL/health"
echo -e "\n"

# Test listing models (public endpoint)
echo "Testing /models endpoint (list all models):"
curl -s "$BASE_URL/models"
echo -e "\n"

echo "Testing protected endpoints (should fail without proper signature):"
echo "-----------------------------------------------------------------"

# Try a protected endpoint without authentication
echo "Testing /models/create endpoint without auth (should fail):"
curl -s -X POST "$BASE_URL/models/create" \
  -H "Content-Type: application/json" \
  -d '{"model_id": "test-model", "name": "Test Model", "description": "A test model"}'
echo -e "\n"

# If you have a valid account and private key, you can test with a signature
echo "Testing with signature authentication (requires valid keys):"
echo "-----------------------------------------------------------------"

# A fake private key for testing (not secure, just for demo)
# This generates a new secp256k1 private key for testing purposes
if [ ! -f test_private_key.pem ]; then
  echo "Generating test key pair..."
  openssl ecparam -name secp256k1 -genkey -noout -out test_private_key.pem
  openssl ec -in test_private_key.pem -pubout -out test_public_key.pem
fi

# The endpoint to test
ENDPOINT="/models"
TIMESTAMP=$(date +%s)

# Create a message hash (using openssl to simulate the Rust SHA256 hashing)
echo -n "${ENDPOINT}${TIMESTAMP}" > message.txt
HASH=$(openssl dgst -sha256 -binary message.txt | xxd -p -c 256)

# Sign the hash with the private key
SIGNATURE=$(openssl dgst -sha256 -sign test_private_key.pem message.txt | xxd -p -c 256)

# Recovery ID is typically 0 or 1 for testing
RECOVERY_ID="00"

echo "Signature: $SIGNATURE"
echo "Timestamp: $TIMESTAMP"

# Make the request with signature headers
echo "Testing $ENDPOINT with signature:"
curl -s "$BASE_URL$ENDPOINT" \
  -H "X-Signature: $SIGNATURE" \
  -H "X-Recovery-ID: $RECOVERY_ID" \
  -H "X-Timestamp: $TIMESTAMP" \
  -H "Content-Type: application/json"
echo -e "\n"

# Clean up
rm -f message.txt

echo "Note: The signature test will likely fail with 401 Unauthorized"
echo "because the test key isn't registered with any account in the system."
echo "This is expected behavior - it confirms that signature validation is working." 