#!/bin/bash
set -e

# Colors for output
GREEN="\033[0;32m"
BLUE="\033[0;34m"
YELLOW="\033[1;33m"
RED="\033[0;31m"
NC="\033[0m" # No Color

# Helper functions
info() {
    echo -e "${BLUE}[INFO]${NC} $@"
}

success() {
    echo -e "${GREEN}[SUCCESS]${NC} $@"
}

warn() {
    echo -e "${YELLOW}[WARNING]${NC} $@"
}

error() {
    echo -e "${RED}[ERROR]${NC} $@"
    exit 1
}

# Make sure we're in the right directory
cd "$(dirname "$0")/.."
ROOT_DIR=$(pwd)

# Create a temporary directory for our test
TMP_DIR=$(mktemp -d)
trap 'rm -rf "$TMP_DIR"' EXIT

info "Testing bootstrap domain integration"
info "Temporary directory: $TMP_DIR"

# Step 1: Run the bootstrap domain example to verify DNS functionality
info "Step 1: Running bootstrap domain example to verify DNS functionality"
cd $ROOT_DIR/form-dns
cargo run --example bootstrap_domain

# Check if it worked
if [ $? -ne 0 ]; then
    error "Bootstrap domain example failed"
fi
success "Bootstrap domain example completed successfully"

# Step 2: Add the bootstrap.formation.cloud to the hosts file to point to localhost for testing
info "Step 2: Adding bootstrap.formation.cloud to /etc/hosts for testing"
if ! grep -q "bootstrap.formation.cloud" /etc/hosts; then
    echo "127.0.0.1 bootstrap.formation.cloud" | sudo tee -a /etc/hosts
    success "Added bootstrap.formation.cloud to /etc/hosts"
else
    warn "bootstrap.formation.cloud already exists in /etc/hosts"
fi

# Step 3: Start DNS server in the background
info "Step 3: Starting DNS server in the background"
cd $ROOT_DIR/form-dns
cargo run -- &
DNS_PID=$!

# Give it time to start
sleep 3
success "DNS server started with PID $DNS_PID"

# Step 4: Use the form-net CLI to join with bootstrap domain
info "Step 4: Demonstrating how to use the bootstrap domain parameter"

cat << EOF
Here's how to use the bootstrap domain with the formation CLI:

# Using the bootstrap domain directly:
form-net operator join --bootstrap-domain bootstrap.formation.cloud --signing-key <your-key>

# Using both bootstrap domain and specific bootstrap nodes:
form-net operator join --bootstrap-domain bootstrap.formation.cloud --bootstraps 198.51.100.2:51820 --signing-key <your-key>

# Leaving the network with bootstrap domain:
form-net operator leave --bootstrap-domain bootstrap.formation.cloud --signing-key <your-key>
EOF

# Step 5: Explain what happens under the hood
info "Step 5: Understanding the process"

cat << EOF

How the bootstrap domain works:

1. When you specify --bootstrap-domain:
   - The CLI adds the domain to the bootstrap list
   - The join process resolves the domain using DNS
   - Resolution uses the form-dns GeoDNS system
   
2. Health filtering happens automatically:
   - Unhealthy bootstrap nodes are filtered out
   - You connect to the nearest healthy node
   
3. Fallback mechanisms:
   - If all bootstrap nodes are unhealthy, all IPs are returned
   - If domain resolution fails, direct bootstrap IPs are used if provided
EOF

# Step 6: Clean up
info "Step 6: Cleaning up"
kill $DNS_PID
wait $DNS_PID 2>/dev/null || true
success "DNS server stopped"

# Optional: Remove hosts entry
read -p "Do you want to remove the bootstrap.formation.cloud entry from /etc/hosts? [y/N] " remove_hosts
if [[ "$remove_hosts" =~ ^[Yy]$ ]]; then
    sudo sed -i '/bootstrap.formation.cloud/d' /etc/hosts
    success "Removed bootstrap.formation.cloud from /etc/hosts"
fi

success "Test completed successfully!"
echo
echo "To use the bootstrap domain in production:"
echo "1. Configure your DNS to point bootstrap.formation.cloud to your bootstrap nodes"
echo "2. Configure TTL values appropriately for failover speed"
echo "3. Monitor the health of your bootstrap nodes"
echo
echo "For more information, see the virtual_anycast_implementation_plan.md document." 