#!/bin/bash
set -e

# Print environment information
echo "Starting Formation Minimal (Development Mode)"
echo "============================================="
echo "Auth Mode: $AUTH_MODE"
echo "Skip JWT Validation: $SKIP_JWT_VALIDATION"
echo "============================================="

# Create required directories if they don't exist
mkdir -p /var/log/formation
mkdir -p /var/lib/formation/db

# Start the services

# 1. Start VMM service
echo "Starting VMM service..."
/usr/local/bin/run-vmm-service.sh &
sleep 2

# 2. Start FormNet
echo "Starting FormNet..."
/usr/local/bin/run-formnet.sh &
sleep 2

# 3. Start Pack Manager
echo "Starting Pack Manager..."
/usr/local/bin/run-pack-manager.sh &
sleep 2

# 4. Start form-state in development mode (with mock server)
echo "Starting form-state in development mode..."
/usr/local/bin/mock-server --skip-jwt --env-file /etc/formation/auth/.env.example &
sleep 2

# 5. Start form-dns
echo "Starting form-dns..."
/usr/local/bin/run-form-dns.sh &
sleep 2

echo "All services started!"
echo "Formation Minimal is running in development mode."
echo "API is available at port 3004"
echo "JWT validation is disabled for development"

# Keep container running
tail -f /var/log/formation/form-state.log || true
