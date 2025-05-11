#!/bin/bash
set -e

# Colors for better output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
NC='\033[0m' # No Color

echo -e "${YELLOW}Testing Delegated Authentication between form-state and form-pack-manager${NC}"

# Use environment variables if defined, otherwise use defaults
FORM_STATE_PORT=${FORM_STATE_PORT:-3004}
FORM_PACK_PORT=${FORM_PACK_PORT:-3003}
PASSWORD=${PASSWORD:-formation-password}
NETWORK="host"

# Create temporary directories
SECRETS_DIR=$(mktemp -d)
STATE_DATA_DIR=$(mktemp -d)
PACK_DATA_DIR=$(mktemp -d)
CONFIG_FILE="${SECRETS_DIR}/.operator-config.json"

cleanup() {
  echo -e "${YELLOW}Cleaning up...${NC}"
  docker stop formation-state-test formation-pack-test || true
  docker rm formation-state-test formation-pack-test || true
  rm -rf "$SECRETS_DIR" "$STATE_DATA_DIR" "$PACK_DATA_DIR"
  echo -e "${GREEN}Cleanup complete${NC}"
}

# Set up cleanup on exit
trap cleanup EXIT

# Create minimal config file
cat > "${CONFIG_FILE}" << EOF
{
  "encrypted": false,
  "password": "${PASSWORD}",
  "jwt_secret": "test_jwt_secret",
  "api_keys": {
    "test_api_key": {
      "id": "test_api_key",
      "name": "Test API Key",
      "key": "test_api_key_secret",
      "permissions": ["*"],
      "created_at": "$(date +%s)"
    }
  }
}
EOF

# Start form-state container
echo -e "${YELLOW}Starting form-state container...${NC}"
docker run -d --rm --name formation-state-test \
  --network ${NETWORK} \
  -p ${FORM_STATE_PORT}:${FORM_STATE_PORT} \
  -v "${STATE_DATA_DIR}:/var/lib/formation/db" \
  -v "${SECRETS_DIR}:/etc/formation" \
  -e DB_PATH=/var/lib/formation/db/formation.db \
  -e CONFIG_PATH=/etc/formation/.operator-config.json \
  -e PASSWORD=${PASSWORD} \
  -e FORM_STATE_PORT=${FORM_STATE_PORT} \
  -e ALLOW_INTERNAL_ENDPOINTS=true \
  -e RUST_LOG=info \
  formationai/form-state:latest \
  /usr/local/bin/form-state -C /etc/formation/.operator-config.json --encrypted=false -p ${PASSWORD}

# Wait for form-state to start
echo -e "${YELLOW}Waiting for form-state to start...${NC}"
sleep 5

# Start form-pack-manager container
echo -e "${YELLOW}Starting form-pack-manager container...${NC}"
docker run -d --rm --name formation-pack-test \
  --network ${NETWORK} \
  -p ${FORM_PACK_PORT}:${FORM_PACK_PORT} \
  -v "${PACK_DATA_DIR}:/var/lib/formation/pack-manager" \
  -v "${SECRETS_DIR}:/etc/formation" \
  -e PACK_MANAGER_PORT=${FORM_PACK_PORT} \
  -e PACK_MANAGER_INTERFACE=all \
  -e PACK_MANAGER_CONFIG_PATH=/etc/formation/.operator-config.json \
  -e PACK_MANAGER_PASSWORD=${PASSWORD} \
  -e STATE_URL=http://localhost:${FORM_STATE_PORT} \
  -e RUST_LOG=debug \
  formationai/form-pack:latest \
  /usr/local/bin/form-pack-manager --config /etc/formation/.operator-config.json --interface all --port ${FORM_PACK_PORT} --password ${PASSWORD}

# Wait for form-pack-manager to start
echo -e "${YELLOW}Waiting for form-pack-manager to start...${NC}"
sleep 5

# Generate ECDSA keys and signature for testing
echo -e "${YELLOW}Generating test ECDSA keys and signature...${NC}"
PRIVATE_KEY="0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef"
ADDRESS="0x$(echo -n "$PRIVATE_KEY" | xxd -p -u | tail -c 40)"
MESSAGE="test_message_$(date +%s)"
MESSAGE_HEX=$(echo -n "$MESSAGE" | xxd -p)
SIGNATURE="1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef"
RECOVERY_ID="0"
AUTH_HEADER="Signature ${SIGNATURE}.${RECOVERY_ID}.${MESSAGE_HEX}"

# Create test formfile
echo -e "${YELLOW}Creating test formfile...${NC}"
cat > "${SECRETS_DIR}/test.formfile" << EOF
NAME test_agent
DESCRIPTION "Test agent for delegated authentication"
FROM debian:bullseye-slim
RUN echo "Hello, world!"
EXPOSE 8080
ENTRYPOINT ["/bin/bash", "-c", "echo 'Running test agent'"]
EOF

# Create an account directly in form-state
echo -e "${YELLOW}Creating test account in form-state...${NC}"
curl -X POST "http://localhost:${FORM_STATE_PORT}/accounts/create" \
  -H "Content-Type: application/json" \
  -H "Authorization: ${AUTH_HEADER}" \
  -d '{
    "account_id": "test_account",
    "email": "test@example.com",
    "name": "Test Account",
    "wallet_address": "'"${ADDRESS}"'"
  }'

# Build an agent through form-pack-manager
echo -e "${YELLOW}Building agent through form-pack-manager...${NC}"
BUILD_ID=$(curl -X POST "http://localhost:${FORM_PACK_PORT}/build" \
  -H "Content-Type: application/json" \
  -d '{
    "formfile": "NAME test_agent\nDESCRIPTION \"Test agent for delegated authentication\"\nFROM debian:bullseye-slim\nRUN echo \"Hello, world!\"\nEXPOSE 8080\nENTRYPOINT [\"/bin/bash\", \"-c\", \"echo '"'"'Running test agent'"'"'\"]\n",
    "context": {},
    "user_id": "'"${ADDRESS}"'",
    "signature": "'"${SIGNATURE}"'",
    "recovery_id": "'"${RECOVERY_ID}"'",
    "message": "'"${MESSAGE}"'"
  }' | jq -r '.build_id')

echo -e "${YELLOW}Build ID: ${BUILD_ID}${NC}"

# Wait for the build to complete
echo -e "${YELLOW}Waiting for build to complete...${NC}"
sleep 5

# Check if the agent was created in form-state
echo -e "${YELLOW}Checking if agent was created in form-state...${NC}"
AGENT_RESPONSE=$(curl -s "http://localhost:${FORM_STATE_PORT}/agents/${BUILD_ID}")

if echo "$AGENT_RESPONSE" | grep -q "agent_id"; then
  echo -e "${GREEN}Success! Agent was created in form-state through delegated authentication.${NC}"
  echo -e "${YELLOW}Agent details:${NC}"
  echo "$AGENT_RESPONSE" | jq '.'
else
  echo -e "${RED}Failed to create agent in form-state.${NC}"
  echo "$AGENT_RESPONSE"
  exit 1
fi

# Check agent status in form-pack-manager
echo -e "${YELLOW}Checking agent status in form-pack-manager...${NC}"
STATUS_RESPONSE=$(curl -s "http://localhost:${FORM_PACK_PORT}/${BUILD_ID}/get_status")
echo -e "${YELLOW}Agent status:${NC}"
echo "$STATUS_RESPONSE" | jq '.'

echo -e "${GREEN}Test completed successfully!${NC}"
exit 0 