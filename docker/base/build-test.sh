#!/bin/bash
set -e

echo "Building Formation base image..."
docker build -t formation/base:latest .

echo "Testing Formation base image..."
# Test that the image can be run
docker run --rm formation/base:latest echo "Base image test successful"

# Test that common directories exist
echo "Checking for required directories..."
DIRS_TO_CHECK=(
  "/usr/local/bin"
  "/var/log/formation"
  "/etc/formation/auth"
  "/etc/formation/billing"
  "/home/formation"
)

for dir in "${DIRS_TO_CHECK[@]}"; do
  if docker run --rm formation/base:latest test -d "$dir"; then
    echo "✅ Directory exists: $dir"
  else
    echo "❌ Directory missing: $dir"
    exit 1
  fi
done

# Test that required packages are installed
echo "Checking for required packages..."
PACKAGES_TO_CHECK=(
  "protobuf-compiler"
  "libprotobuf-dev"
  "libssl-dev"
  "libsqlite3-dev"
)

for pkg in "${PACKAGES_TO_CHECK[@]}"; do
  if docker run --rm formation/base:latest dpkg -l | grep -q "$pkg"; then
    echo "✅ Package installed: $pkg"
  else
    echo "❌ Package missing: $pkg"
    exit 1
  fi
done

# Test that the non-root user exists
echo "Checking for formation user..."
if docker run --rm formation/base:latest id formation > /dev/null 2>&1; then
  echo "✅ User exists: formation"
else
  echo "❌ User missing: formation"
  exit 1
fi

echo "All tests passed! Base image is ready for use." 