#!/bin/bash
# Build script for form-inference-api Docker image

set -e

echo "Building form-inference-api Docker image..."

# Build the release binary first
echo "Building release binary..."
cargo build --release

# Build the Docker image
echo "Building Docker image..."
docker build \
    --build-arg CACHE_BUST=$(date +%s) \
    -t form-inference-api:latest \
    .

echo "Docker image built successfully!"
echo "To run the container:"
echo "  docker run -p 3006:3006 -v /path/to/.operator-config.json:/etc/formation/config/.operator-config.json:ro form-inference-api:latest"
echo ""
echo "Or use docker-compose:"
echo "  docker-compose up"