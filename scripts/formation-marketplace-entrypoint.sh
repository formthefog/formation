#!/bin/bash
set -e

# Print environment information
echo "Starting Formation Marketplace"
echo "=============================="
echo "Auth Mode: $AUTH_MODE"
echo "Marketplace Enabled: $MARKETPLACE_ENABLED"
echo "Billing Enabled: $BILLING_ENABLED"
echo "API Keys Enabled: $API_KEYS_ENABLED"
echo "=============================="

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

# 4. Start form-p2p
echo "Starting form-p2p message queue..."
/usr/local/bin/run-form-p2p.sh &
sleep 2

# 5. Start form-state with auth and billing
echo "Starting form-state with auth and billing..."
/usr/local/bin/run-form-state.sh &
sleep 2

# 6. Start form-dns
echo "Starting form-dns..."
/usr/local/bin/run-form-dns.sh &
sleep 2

# 7. Start form-node-metrics
echo "Starting form-node-metrics..."
/usr/local/bin/run-form-node-metrics.sh &
sleep 2

echo "All services started!"
echo "Formation Marketplace is running."
echo "API is available at port 3004"

# Keep container running
tail -f /var/log/formation/form-state.log 