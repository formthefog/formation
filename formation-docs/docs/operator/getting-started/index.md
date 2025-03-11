# Getting Started as a Formation Operator

Welcome to the Formation Operator documentation. This guide will help you set up and run a Formation node to participate in the network.

## What is a Formation Operator?

Formation operators are individuals or organizations who contribute compute resources to the Formation network. Operators run nodes that host virtual machines and provide the infrastructure for the network's fog compute capabilities.

## Prerequisites

### Hardware Requirements

To effectively run a Formation node, you need the following minimum hardware specifications:

- **CPU**: 32 physical cores (64 logical cores recommended)
- **RAM**: 64 GB minimum (128 GB recommended)
- **Storage**: 8 TB minimum (NVMe SSD recommended for optimal performance)
- **Network**: 1 Gbps connection with low latency
- **GPU** (Optional): NVIDIA RTX 4090 series or higher for AI workloads

**Note**: While GPUs are not required they are highly recommended for both performance and financial reasons. Non-GPU nodes still earn rewards, but at significantly reduced rates.

### Software Requirements

- **Operating System**: Ubuntu 22.04 LTS
- **System Dependencies**:
  - build-essential
  - bridge-utils 
  - kmod
  - pkg-config
  - libssl-dev
  - libudev-dev
  - protobuf-compiler
  - libsqlite3-dev

### Network Requirements

Your node must have the following ports open/forwarded:

- **3002**: `form-vmm` API
- **3003**: `form-pack-manager` API
- **51820**: `formnet` interface
- **3004**: `form-state` API (BFT-CRDT Globally Replicated Datastore)
- **53333**: `form-p2p` BFT-CRDT Message queue

### Staking Requirements

To participate as a node operator, you'll need to stake ETH to guarantee resource availability and network participation:

- **Minimum Stake**: 1 ETH (subject to change)
- **Restaked ETH**: Support for Eigenlayer and other restaking protocols coming soon

## Installation

Follow these steps to install and configure your Formation node:

1. **Set up your environment**:
   ```bash
   sudo apt update
   sudo apt install -y build-essential bridge-utils kmod pkg-config libssl-dev libudev-dev protobuf-compiler libsqlite3-dev
   ```

2. **Install Rust**:
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   source "$HOME/.cargo/env"
   rustup update
   ```

3. **Install Docker**:
   ```bash
   sudo apt install -y apt-transport-https ca-certificates curl software-properties-common
   curl -fsSL https://download.docker.com/linux/ubuntu/gpg | sudo apt-key add -
   sudo add-apt-repository "deb [arch=amd64] https://download.docker.com/linux/ubuntu $(lsb_release -cs) stable"
   sudo apt update
   sudo apt install -y docker-ce
   sudo usermod -aG docker $USER
   ```
   
4. **Configure network bridge**:
   ```bash
   sudo apt-get install bridge-utils
   sudo brctl addbr br0
   sudo ip addr add 192.168.100.1/24 dev br0
   sudo ip link set br0 up
   sudo sysctl -w net.ipv4.ip_forward=1 >/dev/null
   sudo iptables -t nat -A POSTROUTING -s 192.168.100.1/24 -j MASQUERADE
   ```

5. **Set up DNS for the bridge**:
   ```bash
   sudo apt-get install dnsmasq
   sudo mkdir -p /etc/dnsmasq.d
   sudo bash -c 'cat > /etc/dnsmasq.d/br0.conf << EOF
   interface=br0
   port=0
   dhcp-range=192.168.100.10,192.168.100.250,24h
   dhcp-option=6,8.8.8.8,8.8.4.4,1.1.1.1
   EOF'
   sudo systemctl restart dnsmasq
   ```

## Configuration

Once you've installed the necessary dependencies, you need to configure your Formation node:

1. **Build the configuration wizard**:
   ```bash
   git clone https://github.com/formthefog/formation
   cd formation
   cargo build --release --bin form-config-wizard
   ```

2. **Run the wizard**:
   ```bash
   ./target/release/form-config-wizard
   ```
   
   This will guide you through the setup process, including:
   - Setting up network parameters
   - Configuring your operator identity
   - Setting resource allocation limits
   - Configuring staking and rewards

## Running Your Node

After configuration, you can run your Formation node:

### Using Docker (Recommended)

```bash
# Pull the official image
docker pull cryptonomikhan/formation-minimal:v0.1.0
docker tag cryptonomikhan/formation-minimal:v0.1.0 formation-minimal

# Run the node
docker run --rm --privileged --network=host \
    --device=/dev/kvm \
    --device=/dev/vhost-net \
    --device=/dev/null \
    --device=/dev/zero \
    --device=/dev/random \
    --device=/dev/urandom \
    -v /lib/modules:/lib/modules:ro \
    -v /var/run/docker.sock:/var/run/docker.sock \
    -v /path/to/operator/config:/path/to/config \
    -v /path/to/secrets:/path/to/secrets \
    -e SECRET_PATH=/path/to/secrets \
    -e PASSWORD=<your-encryption-password> \
    --mount type=tmpfs,destination=/dev/hugepages,tmpfs-mode=1770 \
    -dit formation-minimal
```

### Manual Setup

For full operator documentation on manual setup, see the [Advanced Operator Guide](../guides/advanced-setup.md).

## Next Steps

- [Monitoring Your Node](../guides/monitoring.md)
- [Troubleshooting](../guides/troubleshooting.md)
- [Joining the Official Developer Network](../guides/joining-devnet.md)
- [Operator Security Best Practices](../guides/security.md) 
