#!/bin/bash
# Docker run script for formnet service

set -e

# Default configuration
NETWORK_NAME=${NETWORK_NAME:-formation-network}
CONTAINER_NAME=${CONTAINER_NAME:-formation-network}
FORMNET_LOG_LEVEL=${FORMNET_LOG_LEVEL:-debug}
RUST_LOG=${RUST_LOG:-debug}
RUST_BACKTRACE=${RUST_BACKTRACE:-1}
SECRET_PATH=${SECRET_PATH:-/var/lib/formation/secrets/operator-config.json}
PASSWORD=${PASSWORD:-formation-password}
STATE_URL=${STATE_URL:-http://localhost:3004}
LISTEN_PORT=${LISTEN_PORT:-51820}
SERVER_PORT=${SERVER_PORT:-8080}

echo "Setting up formnet container..."

# Create directories if they don't exist
echo "Creating data directories..."
mkdir -p "$(pwd)/net-data"
mkdir -p "$(pwd)/secrets"

# Stop and remove existing container if it exists
if docker ps -a | grep -q $CONTAINER_NAME; then
  echo "Stopping and removing existing formnet container..."
  docker stop $CONTAINER_NAME >/dev/null 2>&1 || true
  docker rm $CONTAINER_NAME >/dev/null 2>&1 || true
fi

echo "Starting formnet container with the following configuration:"
echo "- Network name: $NETWORK_NAME"
echo "- Log level: $FORMNET_LOG_LEVEL"
echo "- State URL: $STATE_URL"
echo "- Listen port: $LISTEN_PORT"
echo "- Server port: $SERVER_PORT"

# Run formnet container
docker run -d --name $CONTAINER_NAME \
  --network host \
  -p ${LISTEN_PORT}:${LISTEN_PORT}/udp \
  -p ${SERVER_PORT}:${SERVER_PORT}/tcp \
  -v "$(pwd)/net-data:/var/lib/formnet" \
  -v "$(pwd)/secrets:/etc/formnet:ro" \
  -v "${HOME}/.config/formation/certs:${HOME}/.config/formation/certs" \
  -e FORMNET_LOG_LEVEL=$FORMNET_LOG_LEVEL \
  -e RUST_LOG=$RUST_LOG \
  -e RUST_BACKTRACE=$RUST_BACKTRACE \
  -e FORMNET_CONFIG_DIR=/etc/formnet \
  -e FORMNET_DATA_DIR=/var/lib/formnet \
  -e FORMNET_NETWORK_NAME=$NETWORK_NAME \
  -e FORMNET_SERVER_PORT=$SERVER_PORT \
  -e FORMNET_LISTEN_PORT=$LISTEN_PORT \
  -e FORMNET_EXTERNAL_ENDPOINT=auto \
  -e SECRET_PATH=$SECRET_PATH \
  -e PASSWORD=$PASSWORD \
  -e STATE_URL=$STATE_URL \
  -e WAIT_FOR_STATE=true \
  --privileged \
  --cap-add=NET_ADMIN \
  --sysctl net.ipv4.ip_forward=1 \
  --sysctl net.ipv4.conf.all.src_valid_mark=1 \
  formationai/formnet:latest

echo "Formnet container started."
echo "To check logs: docker logs $CONTAINER_NAME"
echo "To check status: docker exec $CONTAINER_NAME formnet status"
echo "Health check endpoint: http://localhost:$SERVER_PORT/health"

# Wait for container to start
echo "Waiting for formnet to initialize..."
sleep 3

# Display initial logs
echo "Initial formnet logs:"
docker logs $CONTAINER_NAME | tail -n 20 