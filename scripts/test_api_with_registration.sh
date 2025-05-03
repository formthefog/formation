#!/bin/bash

# Base URL
BASE_URL="http://localhost:3004"

# Colors for better output
GREEN='\033[0;32m'
RED='\033[0;31m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}Signature Authentication Test Suite${NC}"
echo "================================================"

# Step 1: Generate ECDSA key pair if it doesn't exist
echo -e "\n${BLUE}Step 1: Generate Key Pair${NC}"
if [ ! -f test_private_key.pem ]; then
  echo "Generating new secp256k1 key pair..."
  openssl ecparam -name secp256k1 -genkey -noout -out test_private_key.pem
  openssl ec -in test_private_key.pem -pubout -out test_public_key.pem
else
  echo "Using existing key pair."
fi

# Extract public key in hex format
PUBKEY_HEX=$(openssl ec -in test_private_key.pem -pubout -outform DER | tail -c 65 | xxd -p -c 65)
echo "Public Key: $PUBKEY_HEX"

# Step 2: Register an account with the public key
echo -e "\n${BLUE}Step 2: Register Account${NC}"
ACCOUNT_ADDRESS="0x$(echo -n $PUBKEY_HEX | sha256sum | cut -c1-40)"
echo "Generated Account Address: $ACCOUNT_ADDRESS"

# Create account with the public key
echo "Registering account in the system..."
# Format as AccountRequest::Create with a properly structured Account object
ACCOUNT_DATA='{
  "Create": {
    "address": "'$ACCOUNT_ADDRESS'",
    "name": "Test Account",
    "credits": 1000,
    "created_at": '$(date +%s)',
    "updated_at": '$(date +%s)',
    "public_key": "'$PUBKEY_HEX'"
  }
}'

echo "Account data: $ACCOUNT_DATA"

# Register the account (POST to /account/create endpoint)
REGISTER_RESPONSE=$(curl -s -X POST "$BASE_URL/account/create" \
  -H "Content-Type: application/json" \
  -d "$ACCOUNT_DATA")

echo "Registration response: $REGISTER_RESPONSE"

# Step 3: Test authenticated endpoints
echo -e "\n${BLUE}Step 3: Test Protected Endpoints${NC}"

# Function to sign and send a request
function signed_request() {
  local METHOD=$1
  local ENDPOINT=$2
  local DATA=$3

  echo -e "\n${BLUE}Testing $METHOD $ENDPOINT${NC}"
  
  # Current timestamp
  TIMESTAMP=$(date +%s)
  
  # Create message to sign
  # In form-auth/src/signature.rs, create_message_hash combines message and timestamp
  if [ -z "$DATA" ]; then
    # For GET requests with no body, use the endpoint as the message
    echo -n "$ENDPOINT" > message.txt
    MESSAGE="$ENDPOINT"
  else
    # For POST requests with body, use the JSON data as the message
    echo -n "$DATA" > message.txt
    MESSAGE="$DATA" # Body is the message for POST requests
  fi
  
  # Add timestamp to the hash calculation (similar to how create_message_hash does it in Rust)
  echo -n "$TIMESTAMP" >> timestamp.txt
  
  # Hash the message and timestamp together (similar to Rust's SHA256 implementation)
  cat message.txt timestamp.txt > combined.txt
  HASH=$(openssl dgst -sha256 -binary combined.txt | xxd -p -c 256)
  
  # Sign the hash with the private key
  SIGNATURE=$(openssl dgst -sha256 -sign test_private_key.pem combined.txt | xxd -p -c 256)
  RECOVERY_ID="00"  # Usually 0 or 1, for testing we use 0
  
  echo "Message: $MESSAGE"
  echo "Timestamp: $TIMESTAMP"
  echo "Signature: $SIGNATURE"
  
  # Make the request with signature headers
  if [ "$METHOD" == "GET" ]; then
    RESPONSE=$(curl -s -X GET "$BASE_URL$ENDPOINT" \
      -H "X-Signature: $SIGNATURE" \
      -H "X-Recovery-ID: $RECOVERY_ID" \
      -H "X-Timestamp: $TIMESTAMP" \
      -H "Content-Type: application/json")
  else
    RESPONSE=$(curl -s -X "$METHOD" "$BASE_URL$ENDPOINT" \
      -H "X-Signature: $SIGNATURE" \
      -H "X-Recovery-ID: $RECOVERY_ID" \
      -H "X-Timestamp: $TIMESTAMP" \
      -H "Content-Type: application/json" \
      -d "$DATA")
  fi
  
  echo "Response: $RESPONSE"
  
  # Clean up
  rm -f message.txt timestamp.txt combined.txt
}

# Test GET endpoints
signed_request "GET" "/auth/test" ""

# Test creating a model with authenticated request
MODEL_ID="test-model-$(date +%s)"
MODEL_DATA='{
  "Create": {
    "model_id": "'$MODEL_ID'",
    "name": "Test Model",
    "description": "A test model created with signature auth",
    "version": "1.0.0",
    "owner_id": "'$ACCOUNT_ADDRESS'",
    "created_at": '$(date +%s)',
    "updated_at": '$(date +%s)'
  }
}'

signed_request "POST" "/models/create" "$MODEL_DATA"

echo -e "\n${GREEN}Testing complete!${NC}"
echo "If requests still fail with 401, check that your signature validation logic matches the signing process in this script." 