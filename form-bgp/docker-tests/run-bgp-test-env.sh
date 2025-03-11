#!/bin/bash
set -e

SELF_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" >/dev/null 2>&1 && pwd )"
cd "$SELF_DIR"

# Colors for output
GREEN="\033[0;32m"
BLUE="\033[0;34m"
PURPLE="\033[0;35m"
YELLOW="\033[1;33m"
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
    echo -e "\033[0;31m[ERROR]${NC} $@"
}

cmd() {
    echo -e "${PURPLE}[COMMAND]${NC} $@"
    "$@"
}

help() {
    cat >&2 <<-_EOF
Usage: ${0##*/} [options...]
 --build          Build the Docker image (required first time)
 --daemon=NAME    BGP daemon to use (bird, frr, gobgp) (default: bird)
 --nodes=N        Number of nodes to create (default: 3)
 --interactive    Don't automatically clean up, allow interactive exploration
 --help           Show this help message
_EOF
}

# Default options
BUILD=false
BGP_DAEMON="bird"
NODE_COUNT=3
INTERACTIVE=false

# Parse command line arguments
while [[ $# -gt 0 ]]; do
  case $1 in
    --build)
        BUILD=true
        shift
        ;;
    --daemon=*)
        BGP_DAEMON="${1#*=}"
        shift
        ;;
    --nodes=*)
        NODE_COUNT="${1#*=}"
        shift
        ;;
    --interactive)
        INTERACTIVE=true
        shift
        ;;
    --help)
        help
        exit 0
        ;;
    *)
        echo "Invalid option: $1"
        help
        exit 1
        ;;
  esac
done

# Validate BGP daemon selection
if [[ ! "$BGP_DAEMON" =~ ^(bird|frr|gobgp)$ ]]; then
    error "Invalid BGP daemon: $BGP_DAEMON"
    info "Supported daemons: bird, frr, gobgp"
    exit 1
fi

# Validate node count
if ! [[ "$NODE_COUNT" =~ ^[0-9]+$ ]] || [ "$NODE_COUNT" -lt 2 ]; then
    error "Invalid node count: $NODE_COUNT"
    info "Node count must be a number >= 2"
    exit 1
fi

# Setup temporary directory for test environment
TMP_DIR=$(mktemp -d -t bgp-test-XXXXXXXXXX)
info "Using temporary directory: $TMP_DIR"

# Cleanup function
cleanup() {
    if [ "$INTERACTIVE" = false ]; then
        info "Cleaning up test environment..."
        # Stop and remove containers
        docker ps -a -q --filter "name=bgp-node-" | xargs -r docker stop >/dev/null
        docker ps -a -q --filter "name=bgp-node-" | xargs -r docker rm >/dev/null
        # Remove network
        docker network rm bgp-test-net >/dev/null 2>&1 || true
        # Remove temporary directory
        rm -rf "$TMP_DIR"
        success "Cleanup complete"
    else
        warn "Interactive mode: Manual cleanup required"
        info "To clean up, run:"
        echo "  docker stop \$(docker ps -a -q --filter \"name=bgp-node-\")"
        echo "  docker rm \$(docker ps -a -q --filter \"name=bgp-node-\")"
        echo "  docker network rm bgp-test-net"
        echo "  rm -rf $TMP_DIR"
    fi
}

# Register cleanup function
trap cleanup EXIT

# Build Docker image if requested
if [ "$BUILD" = true ]; then
    info "Building BGP test environment Docker image..."
    cmd docker build -t bgp-test-env -f Dockerfile.bgp-test ..
    success "Docker image built successfully"
fi

# Create Docker network for BGP testing
info "Creating Docker network for BGP testing..."
cmd docker network create --subnet=172.20.0.0/16 bgp-test-net
success "Network created: bgp-test-net (172.20.0.0/16)"

# Start core router (AS 64512)
info "Starting core router (AS 64512)..."
CORE_CONTAINER=$(cmd docker run -d --rm \
    --name bgp-node-core \
    --network bgp-test-net \
    --ip 172.20.0.1 \
    --hostname bgp-core \
    --cap-add NET_ADMIN \
    --volume /dev/net/tun:/dev/net/tun \
    --volume "$TMP_DIR:/shared" \
    bgp-test-env /app/setup_network.sh core 1 64512 "$BGP_DAEMON" /bin/sleep infinity)
success "Core router started: $CORE_CONTAINER"

# Start edge nodes (private ASNs: 64513+)
for ((i=1; i<=NODE_COUNT; i++)); do
    NODE_ASN=$((64512 + i))
    NODE_IP="172.20.0.$((i+1))"
    
    info "Starting edge node $i (AS $NODE_ASN)..."
    EDGE_CONTAINER=$(cmd docker run -d --rm \
        --name "bgp-node-edge-$i" \
        --network bgp-test-net \
        --ip "$NODE_IP" \
        --hostname "bgp-edge-$i" \
        --cap-add NET_ADMIN \
        --volume /dev/net/tun:/dev/net/tun \
        --volume "$TMP_DIR:/shared" \
        bgp-test-env /app/setup_network.sh edge $((i+1)) "$NODE_ASN" "$BGP_DAEMON" /bin/sleep infinity)
    success "Edge node $i started: $EDGE_CONTAINER"
done

success "BGP test environment running with $NODE_COUNT edge nodes"
info "Core router: bgp-node-core (AS 64512)"
for ((i=1; i<=NODE_COUNT; i++)); do
    info "Edge node $i: bgp-node-edge-$i (AS $((64512 + i)))"
done

if [ "$INTERACTIVE" = true ]; then
    info "Interactive mode: Press Ctrl+C to stop and clean up when done"
    
    # Print command examples
    cat <<EOF

==========================================
Example commands for working with the environment:
==========================================

# Connect to core router:
docker exec -it bgp-node-core /bin/bash

# Connect to edge node 1:
docker exec -it bgp-node-edge-1 /bin/bash

# Check BGP status on core router (BIRD):
docker exec -it bgp-node-core birdc show protocols

# Check BGP status on core router (FRRouting):
docker exec -it bgp-node-core vtysh -c "show ip bgp summary"

# Check BGP status on core router (GoBGP):
docker exec -it bgp-node-core gobgp neighbor

# Check routing table on any node:
docker exec -it bgp-node-core ip route
docker exec -it bgp-node-edge-1 ip route

==========================================
EOF
    
    # Wait for Ctrl+C
    echo "Press Ctrl+C to stop and exit..."
    while true; do
        sleep 1
    done
else
    # Run for 60 seconds then exit
    info "Test environment will run for 60 seconds..."
    sleep 60
    info "Test complete"
fi 