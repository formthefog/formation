#!/bin/bash
set -euo pipefail

function check_root() {
    [[ $EUID -eq 0 ]] || { echo "Requires root" >&2; exit 1; }
}

apt-get update
apt-get install dnsmasq

function get_default_interface() {
    ip route show default | awk '{print $5}' | head -n1
}

function check_bridge_ip() {
    ip addr show dev "$1" 2>/dev/null | grep -q "inet "
}

function find_available_range() {
    local occupied=($(ip addr | grep "inet " | awk '{print $2}'))
    for prefix in "192.168" "172.16"; do
        for third in {0..255}; do
            local candidate="${prefix}.${third}.0/24"
            local overlap=0
            for range in "${occupied[@]}"; do
                if [[ "$range" == "$candidate" ]]; then
                    overlap=1
                    break
                fi
            done
            if [[ $overlap -eq 0 ]]; then
                echo "$candidate"
                return 0
            fi
        done
    done
    return 1
}

function setup_bridge() {
    local range="$1"
    local iface="$2"
    local bridge_ip=$(echo "$range" | sed 's|0/24|1/24|')

    if ! brctl show br0 2>/dev/null; then
        brctl addbr br0
    fi

    if ! check_bridge_ip br0; then
        ip addr add "$bridge_ip" dev br0
    fi

    ip link set br0 up
}

function setup_nat() {
    local range="$1"
    local iface="$2"

    sysctl -w net.ipv4.ip_forward=1 >/dev/null
    iptables -t nat -A POSTROUTING -s "$range" -o "$iface" -j MASQUERADE
}

function setup_dnsmasq() {
    local range="$1"
    local start_ip=$(echo "$range" | sed 's|0/24|10|')
    local end_ip=$(echo "$range" | sed 's|0/24|200|')

    mkdir -p /etc/dnsmasq.d
    cat > /etc/dnsmasq.d/br0.conf <<EOF
interface=br0
port=0
dhcp-range=${start_ip},${end_ip},24h
dhcp-option=6,8.8.8.8,8.8.4.4,1.1.1.1
EOF

    systemctl restart dnsmasq
}

function validate_setup() {
    local range="$1"
    local test_ip=$(echo "$range" | sed 's|0/24|5|')
    local bridge_ip=$(echo "$range" | sed 's|0/24|1/24|')
    local gateway=$(echo "$bridge_ip" | sed 's|/24||')

    ip netns add testns
    ip link add veth-host type veth peer name veth-ns
    ip link set veth-host master br0
    ip link set veth-host up
    ip link set veth-ns netns testns
    ip netns exec testns ip addr add "${test_ip}/24" dev veth-ns
    ip netns exec testns ip link set veth-ns up
    ip netns exec testns ip link set lo up
    ip netns exec testns ip route add default via "$gateway" dev veth-ns
    ip netns exec testns ping -c 3 -W 5 8.8.8.8
    ip netns del testns
}

function main() {
    check_root
    local iface=$(get_default_interface)
    local range=$(find_available_range)

    setup_bridge "$range" "$iface"
    setup_nat "$range" "$iface"
    setup_dnsmasq "$range"
    validate_setup "$range"
}

main "$@"
