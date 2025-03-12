#!/bin/bash
set -e

# Initialize network interfaces based on the node role
setup_network() {
    local node_role=$1
    local node_id=$2
    local asn=$3
    
    echo "Setting up network for $node_role node $node_id (ASN: $asn)"
    
    # Configure loopback interface with a stable router ID
    ip addr add 10.0.0.$node_id/32 dev lo
    
    # Enable IP forwarding
    echo "net.ipv4.ip_forward=1" >> /etc/sysctl.conf
    sysctl -p
    
    # Create a bridge for internal network
    ip link add name br0 type bridge
    ip link set dev br0 up
    
    # Create subnet based on node ID
    ip addr add 10.10.$node_id.1/24 dev br0
    
    echo "Network setup complete for $node_role node $node_id"
}

# Setup BIRD BGP daemon
setup_bird() {
    local node_id=$1
    local asn=$2
    
    echo "Configuring BIRD BGP daemon for node $node_id (ASN: $asn)"
    
    # Create a basic BIRD configuration with BGP enabled
    cat > /etc/bird/bird.conf <<EOF
# BIRD configuration for node $node_id (ASN: $asn)
log syslog all;
router id 10.0.0.$node_id;

# Configure BGP protocol
protocol bgp {
    local as $asn;
    neighbor 10.10.0.1 as 64512;  # Connect to core router
    direct;
    next hop self;
    import all;
    export all;
}

# Static routes for testing
protocol static {
    route 10.10.$node_id.0/24 via "br0";
}

# Kernel protocol to install routes
protocol kernel {
    persist;
    scan time 20;
    import all;
    export all;
}

# Device protocol to scan interfaces
protocol device {
    scan time 10;
}
EOF
    
    # Restart BIRD service
    service bird restart
    
    echo "BIRD BGP configuration complete"
}

# Setup FRRouting BGP daemon
setup_frr() {
    local node_id=$1
    local asn=$2
    
    echo "Configuring FRRouting for node $node_id (ASN: $asn)"
    
    # Enable the FRR daemon
    sed -i 's/bgpd=no/bgpd=yes/g' /etc/frr/daemons
    
    # Create vtysh.conf
    cat > /etc/frr/vtysh.conf <<EOF
!
service integrated-vtysh-config
!
EOF
    
    # Create FRR configuration
    cat > /etc/frr/frr.conf <<EOF
!
frr version 8.1
frr defaults traditional
hostname node-$node_id
log syslog informational
service integrated-vtysh-config
!
router bgp $asn
 bgp router-id 10.0.0.$node_id
 neighbor 10.10.0.1 remote-as 64512
 !
 address-family ipv4 unicast
  network 10.10.$node_id.0/24
 exit-address-family
!
line vty
!
EOF
    
    # Set permissions
    chown -R frr:frr /etc/frr
    
    # Restart FRR service
    service frr restart
    
    echo "FRRouting BGP configuration complete"
}

# Setup GoBGP daemon
setup_gobgp() {
    local node_id=$1
    local asn=$2
    
    echo "Configuring GoBGP for node $node_id (ASN: $asn)"
    
    # Create GoBGP configuration
    mkdir -p /etc/gobgp
    cat > /etc/gobgp/gobgp.conf <<EOF
{
  "global": {
    "config": {
      "as": $asn,
      "router-id": "10.0.0.$node_id"
    }
  },
  "neighbors": [
    {
      "config": {
        "neighbor-address": "10.10.0.1",
        "peer-as": 64512
      }
    }
  ]
}
EOF
    
    # Start GoBGP in the background
    gobgpd -f /etc/gobgp/gobgp.conf &
    
    echo "GoBGP configuration complete"
}

# Main execution
if [ $# -lt 3 ]; then
    echo "Usage: $0 <role> <node_id> <asn> [bgp_daemon]"
    echo "  role: 'core' or 'edge'"
    echo "  node_id: numeric ID (1-254)"
    echo "  asn: BGP Autonomous System Number (private range: 64512-65534)"
    echo "  bgp_daemon: 'bird', 'frr', or 'gobgp' (default: bird)"
    exit 1
fi

NODE_ROLE=$1
NODE_ID=$2
ASN=$3
BGP_DAEMON=${4:-bird}

# Setup network interfaces
setup_network "$NODE_ROLE" "$NODE_ID" "$ASN"

# Configure BGP daemon based on selection
case $BGP_DAEMON in
    bird)
        setup_bird "$NODE_ID" "$ASN"
        ;;
    frr)
        setup_frr "$NODE_ID" "$ASN"
        ;;
    gobgp)
        setup_gobgp "$NODE_ID" "$ASN"
        ;;
    *)
        echo "Unsupported BGP daemon: $BGP_DAEMON"
        echo "Supported options: bird, frr, gobgp"
        exit 1
        ;;
esac

echo "Node $NODE_ID initialized with $BGP_DAEMON BGP daemon"
echo "ASN: $ASN"
echo "Router ID: 10.0.0.$NODE_ID"

# Keep the container running
exec "$@" 