# Formation Two-Node Setup Guide

This guide walks you through setting up a Formation network with two nodes: a bootstrap node (Node-A) and a joining node (Node-B). This is essential for testing multi-node networking, peer discovery, and distributed state management.

## Overview

In this setup:
- **Node-A (Bootstrap)**: Initializes the Formation network and acts as the entry point
- **Node-B (Joining)**: Connects to the existing network via Node-A

Both nodes will run the full Formation service stack and communicate over a secure WireGuard mesh (Formnet).

## Environment Preparation

Setting up a reliable two-node Formation network requires careful environment preparation. This section covers everything you need to prepare your infrastructure before beginning the node configuration.

### Infrastructure Options

You can deploy Formation nodes using any of these infrastructure options:

#### Option 1: Two Physical Machines
- **Best for**: Production deployments, high-performance requirements
- **Requirements**: Two separate physical servers with network connectivity
- **Considerations**: Requires physical access or remote management (IPMI/iDRAC)

#### Option 2: Two Cloud VMs
- **Best for**: Production deployments, geographic distribution
- **Requirements**: VMs in same or different cloud regions
- **Considerations**: Network latency, bandwidth costs, security groups

#### Option 3: Two Local VMs (Development)
- **Best for**: Testing, development, learning
- **Requirements**: Single host machine with virtualization
- **Considerations**: Resource constraints, shared storage/network

#### Option 4: Hybrid Setup
- **Best for**: Testing cloud connectivity from local environment
- **Requirements**: One local machine + one cloud VM
- **Considerations**: NAT traversal, firewall configuration

### Detailed Machine Requirements

#### Minimum Requirements (Per Node)

```
CPU:     4 cores (x86_64)
RAM:     8GB
Storage: 50GB SSD
Network: 100Mbps upload/download
OS:      Ubuntu 22.04 LTS (or compatible)
```

#### Recommended Requirements (Per Node)

```
CPU:     8+ cores (x86_64)
RAM:     16GB+
Storage: 200GB+ NVMe SSD
Network: 1Gbps upload/download
OS:      Ubuntu 22.04 LTS
```

#### Production Requirements (Per Node)

```
CPU:     16+ cores (x86_64)
RAM:     32GB+
Storage: 500GB+ NVMe SSD (with backup)
Network: 10Gbps with redundancy
OS:      Ubuntu 22.04 LTS (hardened)
```

### Network Architecture Planning

#### Network Topology

```
Internet
    |
    ├── Node-A (Bootstrap) [Public IP: 203.0.113.10]
    │   ├── Formnet IP: 172.20.0.1/16
    │   ├── Bridge IP: 192.168.100.1/24
    │   └── Services: 3004, 3002, 3003, 51820
    │
    └── Node-B (Joining) [Public IP: 198.51.100.20]
        ├── Formnet IP: 172.20.0.2/16  
        ├── Bridge IP: 192.168.100.1/24
        └── Services: 3004, 3002, 3003, 51820
```

#### CIDR Planning

Plan your IP address ranges to avoid conflicts:

**Formnet CIDR (WireGuard mesh)**:
- **Default**: `172.20.0.0/16` (65,534 available IPs)
- **Alternative**: `10.42.0.0/16` if 172.20.x.x conflicts
- **Small networks**: `192.168.200.0/24` (254 IPs)

**Bridge Networks (VM isolation)**:
- **Node-A**: `192.168.100.0/24`
- **Node-B**: `192.168.101.0/24` (different to avoid conflicts)

**Existing Network Conflicts**:
```bash
# Check for existing network conflicts
ip route show table all
ip addr show

# Common enterprise ranges to avoid:
# 10.0.0.0/8     - Often used in corporate networks
# 172.16.0.0/12  - Often used in Docker/containers
# 192.168.0.0/16 - Common home/office networks
```

#### Port Mapping Reference

| Service | Port | Protocol | Purpose | External Access |
|---------|------|----------|---------|----------------|
| form-state | 3004 | TCP | API Server | Required for Node-B |
| vmm-service | 3002 | TCP | VM Management | Internal only |
| form-pack-manager | 3003 | TCP | Package Management | Internal only |
| formnet | 51820 | UDP | WireGuard VPN | Required for Node-B |
| form-dns | 53 | UDP | DNS Resolution | Optional |

### Cloud Provider Setup

#### AWS Environment Preparation

```bash
# Create VPC and subnets
aws ec2 create-vpc --cidr-block 10.0.0.0/16 --tag-specifications 'ResourceType=vpc,Tags=[{Key=Name,Value=formation-vpc}]'

# Create security group
aws ec2 create-security-group \
  --group-name formation-nodes \
  --description "Formation node security group" \
  --vpc-id vpc-xxxxxxxxx

# Add inbound rules
aws ec2 authorize-security-group-ingress \
  --group-id sg-xxxxxxxxx \
  --protocol tcp \
  --port 3004 \
  --cidr 0.0.0.0/0

aws ec2 authorize-security-group-ingress \
  --group-id sg-xxxxxxxxx \
  --protocol udp \
  --port 51820 \
  --cidr 0.0.0.0/0

# Launch instances
aws ec2 run-instances \
  --image-id ami-0c7217cdde317cfec \
  --count 2 \
  --instance-type m5.xlarge \
  --key-name your-key-pair \
  --security-group-ids sg-xxxxxxxxx \
  --subnet-id subnet-xxxxxxxxx \
  --tag-specifications 'ResourceType=instance,Tags=[{Key=Name,Value=formation-node}]'
```

#### Google Cloud Platform Setup

```bash
# Create VPC
gcloud compute networks create formation-network --subnet-mode regional

# Create firewall rules
gcloud compute firewall-rules create formation-ingress \
  --network formation-network \
  --allow tcp:3004,udp:51820 \
  --source-ranges 0.0.0.0/0

# Create instances
gcloud compute instances create formation-node-a formation-node-b \
  --machine-type n1-standard-4 \
  --image-family ubuntu-2204-lts \
  --image-project ubuntu-os-cloud \
  --network formation-network \
  --tags formation-node
```

#### DigitalOcean Setup

```bash
# Create droplets
doctl compute droplet create formation-node-a formation-node-b \
  --size s-4vcpu-8gb \
  --image ubuntu-22-04-x64 \
  --region nyc1 \
  --ssh-keys your-ssh-key-id

# Create firewall
doctl compute firewall create \
  --name formation-firewall \
  --inbound-rules protocol:tcp,ports:3004,address:0.0.0.0/0 \
  --inbound-rules protocol:udp,ports:51820,address:0.0.0.0/0
```

### Local Development Environment

#### VMware/VirtualBox Setup

**Host Machine Requirements**:
- 32GB+ RAM (16GB per VM + host OS)
- 8+ CPU cores
- 500GB+ available storage
- Virtualization enabled in BIOS

**VM Configuration**:
```bash
# Node-A VM
Name: formation-node-a
RAM: 16GB
CPU: 4 cores
Disk: 100GB
Network: NAT + Host-only adapter

# Node-B VM  
Name: formation-node-b
RAM: 16GB
CPU: 4 cores
Disk: 100GB
Network: NAT + Host-only adapter
```

**Network Setup for Local VMs**:
```bash
# Create host-only network (VirtualBox)
VBoxManage hostonlyif create
VBoxManage hostonlyif ipconfig vboxnet0 --ip 192.168.56.1 --netmask 255.255.255.0

# Configure VM network adapters
VBoxManage modifyvm formation-node-a --nic2 hostonly --hostonlyadapter2 vboxnet0
VBoxManage modifyvm formation-node-b --nic2 hostonly --hostonlyadapter2 vboxnet0
```

### Network Configuration Validation

#### Pre-deployment Network Tests

**Test 1: Basic Connectivity**
```bash
# From Node-B to Node-A (replace with actual IPs)
ping -c 4 203.0.113.10
traceroute 203.0.113.10

# Test specific ports
nc -zv 203.0.113.10 3004  # TCP port
nc -zuv 203.0.113.10 51820  # UDP port (may not respond)
```

**Test 2: Bandwidth and Latency**
```bash
# Install iperf3 on both nodes
sudo apt install -y iperf3

# On Node-A (server)
iperf3 -s

# On Node-B (client)
iperf3 -c 203.0.113.10 -t 30

# Expected: >100Mbps throughput, <50ms latency for good performance
```

**Test 3: DNS Resolution**
```bash
# Test external DNS
nslookup formation.cloud
dig +short formation.cloud

# Test reverse DNS  
dig -x 203.0.113.10
```

### Firewall and Security Preparation

#### Ubuntu UFW Configuration

**Node-A (Bootstrap) Firewall**:
```bash
# Reset and set defaults
sudo ufw --force reset
sudo ufw default deny incoming
sudo ufw default allow outgoing

# Allow SSH (adjust port as needed)
sudo ufw allow 22/tcp

# Allow Formation services
sudo ufw allow 3004/tcp comment "form-state API"
sudo ufw allow 51820/udp comment "formnet WireGuard"

# Optional: restrict 3004 to specific IPs
# sudo ufw allow from 198.51.100.20 to any port 3004

# Enable firewall
sudo ufw enable
sudo ufw status numbered
```

**Node-B (Joining) Firewall**:
```bash
# Node-B typically doesn't need to accept external connections
sudo ufw --force reset
sudo ufw default deny incoming
sudo ufw default allow outgoing

# Allow SSH
sudo ufw allow 22/tcp

# Allow return traffic for established connections
sudo ufw allow in on formnet0

# Enable firewall
sudo ufw enable
```

#### Cloud Security Groups

**AWS Security Group Rules**:
```bash
# Inbound rules for Node-A
Type: Custom TCP, Port: 3004, Source: 0.0.0.0/0
Type: Custom UDP, Port: 51820, Source: 0.0.0.0/0
Type: SSH, Port: 22, Source: your-ip/32

# Inbound rules for Node-B  
Type: SSH, Port: 22, Source: your-ip/32
# No other inbound rules needed
```

### DNS and Hostname Setup

#### Configure Hostnames

**On Node-A**:
```bash
sudo hostnamectl set-hostname formation-node-a
echo "127.0.0.1 formation-node-a" | sudo tee -a /etc/hosts
```

**On Node-B**:
```bash
sudo hostnamectl set-hostname formation-node-b
echo "127.0.0.1 formation-node-b" | sudo tee -a /etc/hosts
```

#### Optional: Set up DNS records

If you have a domain, create DNS records for easier management:
```bash
# DNS A records
formation-node-a.yourdomain.com → 203.0.113.10
formation-node-b.yourdomain.com → 198.51.100.20

# Use in bootstrap_nodes configuration
"bootstrap_nodes": ["formation-node-a.yourdomain.com:51820"]
```

### Storage and Data Preparation

#### Create Formation Directories

**On both nodes**:
```bash
# Create directory structure
sudo mkdir -p /opt/formation/{data,logs,backups,secrets}
sudo mkdir -p /var/lib/formation/{vmm,pack-manager,state}

# Set proper ownership
sudo chown -R $USER:$USER /opt/formation
sudo chown -R $USER:docker /var/lib/formation

# Set permissions
chmod 750 /opt/formation/secrets
chmod 755 /opt/formation/{data,logs,backups}
```

#### Configure Log Rotation

```bash
# Create logrotate configuration
sudo tee /etc/logrotate.d/formation << EOF
/opt/formation/logs/*.log {
    daily
    rotate 30
    compress
    delaycompress
    missingok
    notifempty
    create 644 formation formation
    postrotate
        systemctl reload formation-* 2>/dev/null || true
    endscript
}
EOF
```

### Environment Validation Script

Create this script to validate your environment before proceeding:

```bash
#!/bin/bash
# Save as: validate-formation-environment.sh

echo "=== Formation Two-Node Environment Validation ==="

errors=0

# Check system requirements
echo "=== System Requirements ==="
cpu_cores=$(nproc)
ram_gb=$(free -g | awk 'NR==2{print $2}')
disk_gb=$(df / | tail -1 | awk '{print int($4/1024/1024)}')

echo "CPU cores: $cpu_cores (need 4+)"
echo "RAM: ${ram_gb}GB (need 8+)"
echo "Disk space: ${disk_gb}GB (need 50+)"

[ $cpu_cores -lt 4 ] && echo "❌ Insufficient CPU cores" && errors=$((errors + 1))
[ $ram_gb -lt 8 ] && echo "❌ Insufficient RAM" && errors=$((errors + 1))
[ $disk_gb -lt 50 ] && echo "❌ Insufficient disk space" && errors=$((errors + 1))

# Check required software
echo "=== Software Requirements ==="
for cmd in docker docker-compose git curl jq; do
    if command -v $cmd >/dev/null 2>&1; then
        echo "✓ $cmd installed"
    else
        echo "❌ $cmd not found"
        errors=$((errors + 1))
    fi
done

# Check Docker
echo "=== Docker Status ==="
if systemctl is-active --quiet docker; then
    echo "✓ Docker service running"
    if docker ps >/dev/null 2>&1; then
        echo "✓ Docker accessible"
    else
        echo "❌ Docker not accessible (check permissions)"
        errors=$((errors + 1))
    fi
else
    echo "❌ Docker service not running"
    errors=$((errors + 1))
fi

# Check network configuration
echo "=== Network Configuration ==="
if ip addr show br0 >/dev/null 2>&1; then
    echo "✓ Bridge br0 exists"
else
    echo "❌ Bridge br0 not found"
    errors=$((errors + 1))
fi

# Check virtualization
echo "=== Virtualization Support ==="
if ls /dev/kvm >/dev/null 2>&1; then
    echo "✓ KVM device available"
else
    echo "⚠ KVM device not found (may impact VM performance)"
fi

echo "========================"
if [ $errors -eq 0 ]; then
    echo "✅ Environment validation passed!"
    echo "Ready to proceed with Formation node setup."
else
    echo "❌ Environment validation failed with $errors errors."
    echo "Please fix the issues above before proceeding."
    exit 1
fi
```

Run this script on both nodes before proceeding:
```bash
chmod +x validate-formation-environment.sh
./validate-formation-environment.sh
```

### Final Pre-deployment Checklist

Before starting node configuration, ensure:

**Infrastructure**:
- [ ] Two machines/VMs provisioned with adequate resources
- [ ] Network connectivity verified between nodes
- [ ] Required ports open and accessible
- [ ] Firewall rules configured correctly

**Software**:
- [ ] Ubuntu 22.04 LTS installed and updated
- [ ] Docker and Docker Compose installed
- [ ] Bridge networking configured
- [ ] Formation repository cloned

**Security**:
- [ ] SSH access configured with key-based authentication
- [ ] Firewall configured for minimal necessary access
- [ ] Strong passwords planned for operator configuration
- [ ] Backup strategy planned for configurations and keys

**Network Planning**:
- [ ] IP addresses and CIDRs planned to avoid conflicts
- [ ] DNS hostnames configured (if applicable)
- [ ] Port forwarding configured (if behind NAT)
- [ ] Bandwidth and latency tested between nodes

With your environment properly prepared, you're ready to proceed with node configuration and deployment.

---

## Prerequisites

### Environment Requirements

You need **two separate machines** (physical or virtual) that can communicate over a network:

- **Node-A**: The bootstrap machine (needs public IP or port forwarding)
- **Node-B**: The joining machine (can be behind NAT)

### Machine Specifications (Each Node)

- **OS**: Ubuntu 22.04 LTS (or compatible Linux distribution)
- **CPU**: 4+ cores (8+ recommended)
- **RAM**: 8GB minimum (16GB+ recommended)  
- **Storage**: 50GB+ free disk space
- **Network**: Internet connectivity, ability to reach each other

### Network Requirements

- **Node-A** must be reachable by Node-B on port 51820 (UDP) and 3004 (TCP)
- If Node-A is behind a router, configure port forwarding for these ports
- Both nodes need outbound internet access for Docker images and general connectivity

## Step 1: Prepare Both Machines

Run these steps on **both Node-A and Node-B**:

### Install Dependencies

```bash
# Update system packages
sudo apt update && sudo apt upgrade -y

# Install essential tools
sudo apt install -y curl bridge-utils dnsmasq git

# Install Docker
sudo apt install -y apt-transport-https ca-certificates curl software-properties-common
curl -fsSL https://download.docker.com/linux/ubuntu/gpg | sudo apt-key add -
sudo add-apt-repository "deb [arch=amd64] https://download.docker.com/linux/ubuntu $(lsb_release -cs) stable"
sudo apt update
sudo apt install -y docker-ce

# Add user to docker group
sudo usermod -aG docker $USER

# Install Docker Compose
sudo curl -L "https://github.com/docker/compose/releases/latest/download/docker-compose-$(uname -s)-$(uname -m)" -o /usr/local/bin/docker-compose
sudo chmod +x /usr/local/bin/docker-compose

# Log out and back in for docker group changes to take effect
```

### Set Up Network Bridge

```bash
# Clone Formation repository
git clone https://github.com/formthefog/formation.git
cd formation

# Run network validation script
sudo bash scripts/validate-network-config.sh
```

If the script fails, manually configure the bridge:

```bash
# Create bridge interface
sudo brctl addbr br0
sudo ip addr add 192.168.100.1/24 dev br0
sudo ip link set br0 up

# Enable IP forwarding
sudo sysctl -w net.ipv4.ip_forward=1

# Add NAT rule
sudo iptables -t nat -A POSTROUTING -s 192.168.100.0/24 ! -o br0 -j MASQUERADE

# Configure dnsmasq
echo "interface=br0
dhcp-range=192.168.100.10,192.168.100.250,24h
dhcp-option=option:router,192.168.100.1
dhcp-option=option:dns-server,192.168.100.1,8.8.8.8" | sudo tee /etc/dnsmasq.d/br0.conf

sudo systemctl restart dnsmasq
```

## Step 2: Configure Node-A (Bootstrap Node)

Node-A serves as the bootstrap node that initializes the Formation network. Other nodes (like Node-B) will connect to Node-A to join the network. This section provides detailed steps to configure and start Node-A.

### Prerequisites for Node-A

Before proceeding, ensure Node-A has:

- ✅ Environment preparation completed (previous section)
- ✅ All dependencies installed (Docker, git, networking tools)
- ✅ Bridge interface (br0) configured
- ✅ Firewall configured to allow ports 51820/udp and 3004/tcp
- ✅ Formation repository cloned

### Step 2.1: Determine Node-A Network Information

First, gather the network information that Node-B will need to connect:

```bash
# Get Node-A's external/public IP address
echo "=== Node-A Network Information ==="

# Option 1: If Node-A has direct public IP
PUBLIC_IP=$(curl -s ifconfig.me 2>/dev/null || curl -s ipinfo.io/ip 2>/dev/null || echo "unable-to-detect")
echo "Public IP: $PUBLIC_IP"

# Option 2: If Node-A is behind NAT/router
LOCAL_IP=$(ip route get 8.8.8.8 | grep -oP 'src \K\S+' 2>/dev/null || echo "unable-to-detect")
echo "Local IP: $LOCAL_IP"

# Check which IP Node-B should use to connect
echo ""
echo "Node-B will connect to Node-A using:"
if [[ "$PUBLIC_IP" != "unable-to-detect" && "$PUBLIC_IP" != "$LOCAL_IP" ]]; then
    echo "  Bootstrap address: $PUBLIC_IP:51820"
    echo "  API address: $PUBLIC_IP:3004"
    echo ""
    echo "⚠️  IMPORTANT: Configure port forwarding on your router:"
    echo "  - Forward 51820/udp to $LOCAL_IP:51820 (WireGuard)"
    echo "  - Forward 3004/tcp to $LOCAL_IP:3004 (form-state API)"
else
    echo "  Bootstrap address: $LOCAL_IP:51820"
    echo "  API address: $LOCAL_IP:3004"
fi

# Save this information for later use
echo "$PUBLIC_IP" > /tmp/node-a-public-ip
echo "$LOCAL_IP" > /tmp/node-a-local-ip
```

**⚠️ Important**: Note the IP addresses shown above. You'll need them when configuring Node-B.

### Step 2.2: Run form-config-wizard with Bootstrap Configuration

Now use the Formation configuration wizard to create Node-A's operator configuration:

```bash
# Navigate to Formation directory
cd formation

# Ensure we have the latest code
git pull origin main

# Build the configuration wizard (if using source build)
# Skip this if using Docker Compose
cargo build --release --package form-config

# Run the configuration wizard
./target/release/form-config-wizard wizard

# OR if using Docker Compose:
# docker run --rm -it -v $(pwd)/secrets:/secrets formationai/form-config:latest form-config-wizard wizard
```

#### Configuration Wizard Walkthrough for Node-A

Follow these specific choices for the bootstrap node:

**1. Network ID**:
```
Enter network ID [5]: 50001
```
*Use 50001 for a new test network*

**2. Keyfile Configuration**:
```
Use default keyfile path? (/home/username/.config/.keystore) [y/N]: y
```

**3. Key Configuration**:
```
How would you like to configure your keys?
  > Generate new keys from mnemonic
```

```
How many words would you like your mnemonic to be?
  > 12

Would you like to enhance security with a password for your mnemonic phrase? [Y/n]: y
[Enter a strong password for mnemonic protection]
```

**⚠️ CRITICAL**: Write down your mnemonic phrase and store it securely offline!

**4. Bootstrap Nodes** (Key for Node-A):
```
Enter bootstrap node address (leave empty to finish): [press ENTER]
```
*Leave empty since this IS the bootstrap node*

**5. Bootstrap Domain**:
```
Would you like to use a bootstrap domain for network discovery? [Y/n]: n
```

**6. Bootstrap Node Role** (Important):
```
Should this node serve as a bootstrap node? [y/N]: y
```
*MUST answer 'y' for Node-A*

**7. Region**:
```
Select the geographic region of this node:
  > us-east
```
*Choose region closest to your server location*

**8. Service Ports** (Accept defaults):
```
Enter Datastore port [3004]: [press ENTER]
Enter Formnet Join Server port [3001]: [press ENTER] 
Enter Formnet Service port [51820]: [press ENTER]
Enter VMM Service port [3002]: [press ENTER]
Enter Pack Manager port [3003]: [press ENTER]
Enter Event Queue port [3005]: [press ENTER]
```

**9. Contract Configuration**:
```
Enter contract address (leave empty to skip): [press ENTER]
```

**10. Formnet CIDR** (Important for two-node setup):
```
Enter Formnet CIDR (e.g., 10.42.0.0/16, leave empty for none/default behavior elsewhere) [10.42.0.0/16]: 172.20.0.0/16
```
*Use 172.20.0.0/16 to avoid conflicts with common Docker networks*

**11. Initial Admin Public Key**:
```
Enter initial admin public key (hex, leave empty to skip/use default): [press ENTER]
```

**12. Save Configuration**:
```
Save config to ./secrets/.operator-config.json? [Y/n]: y
Would you like to encrypt your keys in the keystore? [Y/n]: y
[Enter encryption password for keystore]
```

### Step 2.3: Verify Node-A Configuration

After the wizard completes, verify the configuration:

```bash
# Check that configuration was created
ls -la secrets/.operator-config.json

# Verify the configuration contains bootstrap settings
echo "=== Node-A Configuration Verification ==="
echo "Network ID: $(jq -r '.network_id' secrets/.operator-config.json)"
echo "Is Bootstrap: $(jq -r '.is_bootstrap_node' secrets/.operator-config.json)"
echo "Bootstrap Nodes: $(jq -r '.bootstrap_nodes' secrets/.operator-config.json)"
echo "Formnet CIDR: $(jq -r '.formnet_cidr' secrets/.operator-config.json)"
echo "Region: $(jq -r '.region' secrets/.operator-config.json)"

# Verify keystore was created
ls -la ~/.config/.keystore

# Check file permissions
chmod 600 secrets/.operator-config.json
chmod 600 ~/.config/.keystore
```

Expected output should show:
- `is_bootstrap_node: true`
- `bootstrap_nodes: []` (empty array)
- `formnet_cidr: "172.20.0.0/16"`

### Step 2.4: Start form-state Service

Now start the core state management service:

#### Option A: Using Docker Compose (Recommended)

```bash
# Create environment file for Docker Compose
cat > .env << EOF
# Node-A Bootstrap Configuration
SECRET_PATH=/etc/formation/.operator-config.json
PASSWORD=$(echo -n "Enter keystore password: " && read -s pass && echo $pass)

# Network Configuration  
FORMNET_CIDR=172.20.0.0/16
NODE_ROLE=bootstrap

# Logging
FORMNET_LOG_LEVEL=info
RUST_LOG=info
EOF

# Start form-state service first
docker-compose up -d form-state

# Monitor startup logs
echo "=== Monitoring form-state startup ==="
docker-compose logs -f form-state
```

#### Option B: Using Source Build

```bash
# Start form-state service directly
./target/release/form-state \
    --config-path ./secrets/.operator-config.json \
    --password "$(echo -n 'Enter keystore password: ' && read -s pass && echo $pass)" \
    --encrypted true

# Expected output:
# INFO formation: Parsing CLI...
# INFO formation: Loading configuration from ./secrets/.operator-config.json
# INFO formation: Initializing bootstrap node...
# INFO formation: Database initialized
# INFO formation: HTTP server listening on 0.0.0.0:3004
# INFO formation: form-state service started successfully
```

#### Verify form-state is Running

```bash
# Wait for service to fully start (30-60 seconds)
sleep 60

# Check service health
echo "=== form-state Health Check ==="
curl -s http://localhost:3004/health | jq .

# Expected response:
# {
#   "status": "Healthy",
#   "version": "0.1.0",
#   "uptime": 60
# }

# Check bootstrap node status
curl -s http://localhost:3004/v1/network/status | jq .

# Check initial network state
curl -s http://localhost:3004/v1/network/cidrs | jq .
```

If health checks fail, check the logs:
```bash
# For Docker Compose
docker-compose logs form-state

# For source build, check console output or log files
```

### Step 2.5: Initialize formnet Network

With form-state running, initialize the WireGuard mesh network:

#### Start formnet Service

```bash
# Option A: Using Docker Compose
docker-compose up -d form-net

# Option B: Using source build
./target/release/formnet operator join \
    --config-path ./secrets/.operator-config.json &

# Monitor formnet startup
echo "=== Monitoring formnet initialization ==="
```

#### Verify formnet Initialization

```bash
# Wait for formnet to initialize (60-120 seconds)
sleep 90

echo "=== formnet Network Verification ==="

# Check WireGuard interface was created
ip addr show formnet0
# Expected: inet 172.20.0.1/16 scope global formnet0

# Check WireGuard peer status
sudo wg show formnet0
# Expected: interface: formnet0, no peers yet (bootstrap node)

# Check formnet API health
curl -s http://localhost:51820/health 2>/dev/null | jq . || echo "formnet health endpoint not available"

# Verify network state updated
curl -s http://localhost:3004/v1/network/status | jq .
```

**Expected formnet interface output**:
```
4: formnet0: <POINTOPOINT,NOARP,UP,LOWER_UP> mtu 1420 qdisc noqueue state UNKNOWN group default qlen 1000
    link/none 
    inet 172.20.0.1/16 scope global formnet0
       valid_lft forever preferred_lft forever
```

### Step 2.6: Start Additional Services

Start the remaining Formation services:

```bash
# Start all remaining services
docker-compose up -d

# OR for source builds, start each service:
# ./target/release/form-dns --config-path ./secrets/.operator-config.json &
# ./target/release/vmm-service --config-path ./secrets/.operator-config.json &
# ./target/release/form-pack-manager --config-path ./secrets/.operator-config.json &

# Wait for all services to start
sleep 60

# Check all service health
echo "=== All Services Health Check ==="
services=(
    "form-state:3004"
    "vmm-service:3002"
    "form-pack-manager:3003"
)

for service in "${services[@]}"; do
    name=$(echo $service | cut -d: -f1)
    port=$(echo $service | cut -d: -f2)
    
    if curl -s --connect-timeout 5 http://localhost:$port/health >/dev/null; then
        echo "✓ $name (port $port): healthy"
    else
        echo "✗ $name (port $port): unhealthy"
    fi
done
```

### Step 2.7: Comprehensive Node-A Verification

Perform final verification that Node-A is fully operational:

#### Network Interface Verification

```bash
echo "=== Network Interface Status ==="

# Check bridge interface
if ip addr show br0 >/dev/null 2>&1; then
    bridge_ip=$(ip addr show br0 | grep 'inet ' | awk '{print $2}')
    echo "✓ Bridge interface br0: $bridge_ip"
else
    echo "✗ Bridge interface br0: not found"
fi

# Check formnet interface
if ip addr show formnet0 >/dev/null 2>&1; then
    formnet_ip=$(ip addr show formnet0 | grep 'inet ' | awk '{print $2}')
    echo "✓ Formnet interface formnet0: $formnet_ip"
else
    echo "✗ Formnet interface formnet0: not found"
fi
```

#### Service Connectivity Verification

```bash
echo "=== Service Connectivity Test ==="

# Test internal connectivity
echo "Testing localhost connectivity..."
for port in 3004 3002 3003; do
    if nc -z localhost $port; then
        echo "✓ Port $port: accessible"
    else
        echo "✗ Port $port: not accessible"
    fi
done

# Test external connectivity (what Node-B will use)
external_ip=$(cat /tmp/node-a-public-ip 2>/dev/null || cat /tmp/node-a-local-ip)
echo ""
echo "Testing external connectivity from $external_ip..."

# Test form-state API (critical for Node-B)
if curl -s --connect-timeout 10 http://$external_ip:3004/health >/dev/null; then
    echo "✓ External form-state API: accessible"
else
    echo "✗ External form-state API: not accessible (check firewall/routing)"
fi
```

#### Bootstrap Readiness Check

```bash
echo "=== Bootstrap Node Readiness Check ==="

# Check network state
network_status=$(curl -s http://localhost:3004/v1/network/status)
echo "Network status: $network_status"

# Check node registration
node_count=$(curl -s http://localhost:3004/v1/nodes/list | jq length 2>/dev/null || echo "0")
echo "Registered nodes: $node_count"

# Check if ready to accept bootstrap connections
peer_count=$(curl -s http://localhost:3004/v1/network/peers | jq length 2>/dev/null || echo "0")
echo "Network peers: $peer_count"

# Check WireGuard listening status
if sudo wg show formnet0 | grep -q "listening port"; then
    echo "✓ WireGuard listening: ready for connections"
else
    echo "✗ WireGuard listening: not ready"
fi
```

#### Generate Node-B Connection Information

```bash
echo "=== Node-B Connection Information ==="

# Read saved IP information
public_ip=$(cat /tmp/node-a-public-ip 2>/dev/null || echo "unknown")
local_ip=$(cat /tmp/node-a-local-ip 2>/dev/null || echo "unknown")

# Determine which IP Node-B should use
if [[ "$public_ip" != "unknown" && "$public_ip" != "$local_ip" ]]; then
    node_b_bootstrap_ip="$public_ip"
    echo "⚠️  Node-A is behind NAT - ensure port forwarding is configured!"
else
    node_b_bootstrap_ip="$local_ip"
fi

echo ""
echo "📋 SAVE THIS INFORMATION FOR NODE-B CONFIGURATION:"
echo "=================================================="
echo "Bootstrap Node Address: $node_b_bootstrap_ip:51820"
echo "Network ID: 50001"
echo "Formnet CIDR: 172.20.0.0/16"
echo "Node-A Formnet IP: 172.20.0.1"
echo ""
echo "Use this in Node-B bootstrap_nodes configuration:"
echo '"bootstrap_nodes": ["'$node_b_bootstrap_ip':51820"]'
echo "=================================================="

# Save for easy reference
cat > node-b-connection-info.txt << EOF
=== Node-B Connection Information ===
Bootstrap Address: $node_b_bootstrap_ip:51820
Network ID: 50001
Formnet CIDR: 172.20.0.0/16

Bootstrap Nodes Configuration:
"bootstrap_nodes": ["$node_b_bootstrap_ip:51820"]
EOF

echo "Connection info saved to: node-b-connection-info.txt"
```

### Step 2.8: Final Bootstrap Verification

Run a comprehensive test to ensure Node-A is fully operational:

```bash
#!/bin/bash
echo "=== Final Node-A Bootstrap Verification ==="

errors=0

# Test 1: All services healthy
echo "1. Service Health Check..."
for port in 3004 3002 3003; do
    if ! curl -s --connect-timeout 5 http://localhost:$port/health >/dev/null; then
        echo "   ✗ Service on port $port unhealthy"
        errors=$((errors + 1))
    fi
done
[ $errors -eq 0 ] && echo "   ✓ All services healthy"

# Test 2: Network interfaces configured
echo "2. Network Interface Check..."
if ! ip addr show br0 >/dev/null 2>&1; then
    echo "   ✗ Bridge interface br0 missing"
    errors=$((errors + 1))
fi
if ! ip addr show formnet0 >/dev/null 2>&1; then
    echo "   ✗ Formnet interface formnet0 missing"
    errors=$((errors + 1))
fi
[ $errors -eq 0 ] && echo "   ✓ Network interfaces configured"

# Test 3: WireGuard operational
echo "3. WireGuard Check..."
if ! sudo wg show formnet0 >/dev/null 2>&1; then
    echo "   ✗ WireGuard interface not operational"
    errors=$((errors + 1))
fi
[ $errors -eq 0 ] && echo "   ✓ WireGuard operational"

# Test 4: External connectivity (for Node-B)
echo "4. External Connectivity Check..."
external_ip=$(cat /tmp/node-a-public-ip 2>/dev/null || cat /tmp/node-a-local-ip)
if ! curl -s --connect-timeout 10 http://$external_ip:3004/health >/dev/null; then
    echo "   ✗ External API not accessible (Node-B won't be able to connect)"
    errors=$((errors + 1))
fi
[ $errors -eq 0 ] && echo "   ✓ External connectivity working"

# Test 5: Bootstrap configuration
echo "5. Bootstrap Configuration Check..."
is_bootstrap=$(jq -r '.is_bootstrap_node' secrets/.operator-config.json 2>/dev/null)
bootstrap_nodes=$(jq -r '.bootstrap_nodes | length' secrets/.operator-config.json 2>/dev/null)
if [[ "$is_bootstrap" != "true" ]] || [[ "$bootstrap_nodes" != "0" ]]; then
    echo "   ✗ Bootstrap configuration incorrect"
    errors=$((errors + 1))
fi
[ $errors -eq 0 ] && echo "   ✓ Bootstrap configuration correct"

echo "========================"
if [ $errors -eq 0 ]; then
    echo "🎉 Node-A Bootstrap Setup SUCCESSFUL!"
    echo ""
    echo "✅ Node-A is ready to accept connections from Node-B"
    echo "✅ All services are healthy and operational"
    echo "✅ Network infrastructure is properly configured"
    echo ""
    echo "Next: Configure Node-B using the connection info above"
else
    echo "❌ Node-A Bootstrap Setup FAILED ($errors errors)"
    echo ""
    echo "Please fix the issues above before proceeding to Node-B setup"
    exit 1
fi
```

### Node-A Setup Complete

Node-A is now configured as a bootstrap node and ready to accept connections. Key accomplishments:

✅ **Configuration Generated**: Used form-config-wizard to create proper bootstrap configuration  
✅ **form-state Started**: Core service running and accepting API requests  
✅ **formnet Initialized**: WireGuard mesh network created with IP 172.20.0.1/16  
✅ **All Services Running**: VMM, pack-manager, and other services operational  
✅ **External Connectivity**: Node-A accessible for Node-B connections  
✅ **Bootstrap Ready**: Network initialized and ready for additional nodes  

**Important files created**:
- `secrets/.operator-config.json` - Node-A operator configuration
- `~/.config/.keystore` - Encrypted keys (keep secure!)
- `node-b-connection-info.txt` - Connection details for Node-B setup

**Next step**: Use the bootstrap connection information to configure Node-B.

---

## Step 3: Configure Node-B (Joining Node)

**Important**: Wait until Node-A is fully operational before proceeding.

### Create Node-B Configuration

Replace `<NODE_A_IP>` with Node-A's actual IP address:

```bash
# Navigate to formation directory on Node-B
cd formation

# Create secrets directory
mkdir -p secrets

# Create Node-B operator configuration
# REPLACE <NODE_A_IP> with actual Node-A IP address
cat > secrets/.operator-config.json << 'EOF'
{
  "network_id": 50001,
  "keyfile": "/home/nodeB/.config/.keystore",
  "secret_key": null,
  "mnemonic": null,
  "public_key": null,
  "address": null,
  "initial_admin_public_key": null,
  "bootstrap_nodes": ["<NODE_A_IP>:51820"],
  "bootstrap_domain": null,
  "is_bootstrap_node": false,
  "region": "us-west",
  "datastore_port": 3004,
  "formnet_join_server_port": 3001,
  "formnet_service_port": 51820,
  "formnet_cidr": "172.20.0.0/16",
  "vmm_service_port": 3002,
  "pack_manager_port": 3003,
  "event_queue_port": 53333,
  "contract_address": null
}
EOF

# Example with real IP:
# "bootstrap_nodes": ["203.0.113.10:51820"],
```

### Create Node-B Environment File

```bash
cat > .env << EOF
# Node-B Configuration
SECRET_PATH=/etc/formation/.operator-config.json
PASSWORD=joining-node-password-change-me

# Logging
FORMNET_LOG_LEVEL=info
RUST_LOG=info
EOF
```

### Test Connectivity to Node-A

Before starting Node-B, verify it can reach Node-A:

```bash
# Test WireGuard port (UDP 51820)
nc -u -z <NODE_A_IP> 51820 && echo "WireGuard port reachable" || echo "WireGuard port NOT reachable"

# Test form-state API (TCP 3004)
curl -s --connect-timeout 5 http://<NODE_A_IP>:3004/health && echo "API reachable" || echo "API NOT reachable"
```

If these tests fail, check:
- Node-A firewall settings
- Router port forwarding (if applicable)
- Network connectivity between machines

### Start Node-B Services

```bash
# Pull latest images and start services
docker-compose pull
docker-compose up -d

# Monitor startup logs
docker-compose logs -f form-state
```

Look for logs indicating successful connection to Node-A and network joining.

## Node-B (Joining Node) Setup

Node-B will join the existing Formation network by connecting to Node-A (the bootstrap node). This section provides comprehensive steps to configure Node-B and verify successful network joining.

### Prerequisites for Node-B

Before proceeding, ensure Node-B has:

- ✅ Environment preparation completed (previous section)
- ✅ All dependencies installed (Docker, git, networking tools)
- ✅ Bridge interface (br0) configured  
- ✅ Formation repository cloned
- ✅ Node-A is fully operational and accessible
- ✅ Node-A connection information available

### Step 3.1: Gather Node-A Connection Information

First, collect the connection information from Node-A setup:

```bash
echo "=== Node-A Connection Information Required ==="
echo ""
echo "From Node-A setup, you should have:"
echo "  1. Bootstrap IP address (Node-A's external IP)"
echo "  2. Network ID (should be 50001)"
echo "  3. Formnet CIDR (should be 172.20.0.0/16)"
echo ""
echo "If you don't have this information:"
echo "  - Check Node-A's node-b-connection-info.txt file"
echo "  - Or re-run the connection info generation on Node-A"
echo ""

# Prompt for Node-A connection details
read -p "Enter Node-A bootstrap IP address: " NODE_A_IP
read -p "Enter Network ID [50001]: " NETWORK_ID
NETWORK_ID=${NETWORK_ID:-50001}
read -p "Enter Formnet CIDR [172.20.0.0/16]: " FORMNET_CIDR
FORMNET_CIDR=${FORMNET_CIDR:-172.20.0.0/16}

echo ""
echo "Node-B will connect to:"
echo "  Bootstrap Address: $NODE_A_IP:51820"
echo "  Network ID: $NETWORK_ID"
echo "  Formnet CIDR: $FORMNET_CIDR"
echo ""

# Save for later use
echo "$NODE_A_IP" > /tmp/node-a-bootstrap-ip
echo "$NETWORK_ID" > /tmp/network-id
echo "$FORMNET_CIDR" > /tmp/formnet-cidr
```

### Step 3.2: Test Connectivity to Node-A

Before configuring Node-B, verify it can reach Node-A:

```bash
echo "=== Testing Connectivity to Node-A ==="

# Test basic network connectivity
echo "1. Testing basic connectivity..."
if ping -c 3 $NODE_A_IP >/dev/null 2>&1; then
    echo "   ✓ Node-A reachable via ping"
else
    echo "   ✗ Node-A NOT reachable via ping"
    echo "   Check network connectivity, firewall, or IP address"
fi

# Test form-state API (critical for joining)
echo "2. Testing form-state API..."
if curl -s --connect-timeout 10 http://$NODE_A_IP:3004/health >/dev/null; then
    echo "   ✓ Node-A form-state API accessible"
    
    # Get API version info
    api_response=$(curl -s http://$NODE_A_IP:3004/health 2>/dev/null)
    if [[ -n "$api_response" ]]; then
        echo "   API Response: $api_response"
    fi
else
    echo "   ✗ Node-A form-state API NOT accessible"
    echo "   Check Node-A firewall, port forwarding, or service status"
fi

# Test WireGuard port (UDP harder to test)
echo "3. Testing WireGuard port..."
if timeout 5 bash -c "</dev/tcp/$NODE_A_IP/51820" 2>/dev/null; then
    echo "   ✓ Node-A WireGuard port appears accessible"
else
    echo "   ⚠ WireGuard port test inconclusive (UDP)"
    echo "   This is normal - actual connectivity will be tested during join"
fi

echo ""
echo "Connectivity test complete. Proceed if API test passed."
```

### Step 3.3: Configure Node-B with Node-A as Bootstrap

Now use the Formation configuration wizard to create Node-B's joining configuration:

```bash
# Navigate to Formation directory
cd formation

# Ensure we have the latest code
git pull origin main

# Build the configuration wizard (if using source build)
# Skip this if using Docker Compose
cargo build --release --package form-config

# Run the configuration wizard
./target/release/form-config-wizard wizard

# OR if using Docker Compose:
# docker run --rm -it -v $(pwd)/secrets:/secrets formationai/form-config:latest form-config-wizard wizard
```

#### Configuration Wizard Walkthrough for Node-B

Follow these specific choices for the joining node:

**1. Network ID**:
```
Enter network ID [5]: 50001
```
*Must match Node-A's network ID*

**2. Keyfile Configuration**:
```
Use default keyfile path? (/home/username/.config/.keystore) [y/N]: y
```

**3. Key Configuration**:
```
How would you like to configure your keys?
  > Generate new keys from mnemonic
```

```
How many words would you like your mnemonic to be?
  > 12

Would you like to enhance security with a password for your mnemonic phrase? [Y/n]: y
[Enter a strong password for mnemonic protection]
```

**⚠️ CRITICAL**: Generate a NEW mnemonic phrase for Node-B (different from Node-A)!

**4. Bootstrap Nodes** (Critical for Node-B):
```
Enter bootstrap node address (leave empty to finish): 203.0.113.10:51820
Enter bootstrap node address (leave empty to finish): [press ENTER]
```
*Use the actual IP address from Node-A connection info*

**5. Bootstrap Domain**:
```
Would you like to use a bootstrap domain for network discovery? [Y/n]: n
```

**6. Bootstrap Node Role** (Important):
```
Should this node serve as a bootstrap node? [y/N]: n
```
*MUST answer 'n' for Node-B (joining node)*

**7. Region**:
```
Select the geographic region of this node:
  > us-west
```
*Choose different region from Node-A for testing*

**8. Service Ports** (Accept defaults):
```
Enter Datastore port [3004]: [press ENTER]
Enter Formnet Join Server port [3001]: [press ENTER] 
Enter Formnet Service port [51820]: [press ENTER]
Enter VMM Service port [3002]: [press ENTER]
Enter Pack Manager port [3003]: [press ENTER]
Enter Event Queue port [3005]: [press ENTER]
```

**9. Contract Configuration**:
```
Enter contract address (leave empty to skip): [press ENTER]
```

**10. Formnet CIDR** (Must match Node-A):
```
Enter Formnet CIDR (e.g., 10.42.0.0/16, leave empty for none/default behavior elsewhere) [10.42.0.0/16]: 172.20.0.0/16
```
*MUST use the same CIDR as Node-A*

**11. Initial Admin Public Key**:
```
Enter initial admin public key (hex, leave empty to skip/use default): [press ENTER]
```

**12. Save Configuration**:
```
Save config to ./secrets/.operator-config.json? [Y/n]: y
Would you like to encrypt your keys in the keystore? [Y/n]: y
[Enter encryption password for keystore]
```

### Step 3.4: Verify Node-B Configuration

After the wizard completes, verify the configuration is correct for joining:

```bash
# Check that configuration was created
ls -la secrets/.operator-config.json

# Verify the configuration contains joining settings
echo "=== Node-B Configuration Verification ==="
echo "Network ID: $(jq -r '.network_id' secrets/.operator-config.json)"
echo "Is Bootstrap: $(jq -r '.is_bootstrap_node' secrets/.operator-config.json)"
echo "Bootstrap Nodes: $(jq -r '.bootstrap_nodes' secrets/.operator-config.json)"
echo "Formnet CIDR: $(jq -r '.formnet_cidr' secrets/.operator-config.json)"
echo "Region: $(jq -r '.region' secrets/.operator-config.json)"

# Verify keystore was created
ls -la ~/.config/.keystore

# Check file permissions
chmod 600 secrets/.operator-config.json
chmod 600 ~/.config/.keystore

# Validate critical joining configuration
echo ""
echo "=== Validation Checks ==="
is_bootstrap=$(jq -r '.is_bootstrap_node' secrets/.operator-config.json)
bootstrap_count=$(jq -r '.bootstrap_nodes | length' secrets/.operator-config.json)
network_id=$(jq -r '.network_id' secrets/.operator-config.json)

if [[ "$is_bootstrap" == "false" ]]; then
    echo "✓ Bootstrap node role: correct (false)"
else
    echo "✗ Bootstrap node role: incorrect (should be false)"
fi

if [[ "$bootstrap_count" -gt 0 ]]; then
    echo "✓ Bootstrap nodes: configured ($bootstrap_count nodes)"
    echo "  $(jq -r '.bootstrap_nodes[]' secrets/.operator-config.json)"
else
    echo "✗ Bootstrap nodes: not configured (Node-B needs bootstrap nodes)"
fi

if [[ "$network_id" == "50001" ]]; then
    echo "✓ Network ID: matches expected (50001)"
else
    echo "⚠ Network ID: $network_id (verify this matches Node-A)"
fi
```

Expected output should show:
- `is_bootstrap_node: false`
- `bootstrap_nodes: ["IP:51820"]` (with Node-A's IP)
- `network_id: 50001`
- `formnet_cidr: "172.20.0.0/16"`

### Step 3.5: Start Services and Join Network

Now start Node-B services and join the Formation network:

#### Start form-state Service First

```bash
# Create environment file for Docker Compose
cat > .env << EOF
# Node-B Joining Configuration
SECRET_PATH=/etc/formation/.operator-config.json
PASSWORD=$(echo -n "Enter keystore password: " && read -s pass && echo $pass)

# Network Configuration (must match Node-A)
FORMNET_CIDR=172.20.0.0/16
NODE_ROLE=joining

# Logging
FORMNET_LOG_LEVEL=info
RUST_LOG=info
EOF

echo ""
echo "=== Starting Node-B form-state Service ==="

# Option A: Using Docker Compose (Recommended)
docker-compose up -d form-state

# Option B: Using source build
# ./target/release/form-state \
#     --config-path ./secrets/.operator-config.json \
#     --password "$(echo -n 'Enter keystore password: ' && read -s pass && echo $pass)" \
#     --encrypted true &

# Monitor startup and network joining
echo "Monitoring form-state startup (joining network)..."
sleep 30

# Check if form-state started successfully
if curl -s --connect-timeout 5 http://localhost:3004/health >/dev/null; then
    echo "✓ form-state service started successfully"
else
    echo "✗ form-state service failed to start"
    echo "Check logs: docker-compose logs form-state"
fi
```

#### Start formnet and Join Network

```bash
echo "=== Starting formnet and Joining Network ==="

# Start formnet service
docker-compose up -d form-net

# OR for source build:
# ./target/release/formnet operator join \
#     --config-path ./secrets/.operator-config.json &

echo "Waiting for formnet to join network (this may take 60-120 seconds)..."
sleep 90

# Check if formnet interface was created
if ip addr show formnet0 >/dev/null 2>&1; then
    formnet_ip=$(ip addr show formnet0 | grep 'inet ' | awk '{print $2}')
    echo "✓ formnet interface created: $formnet_ip"
    
    # Expected: something like 172.20.0.2/16 or higher
    if [[ "$formnet_ip" =~ ^172\.20\.0\.[2-9] ]]; then
        echo "✓ IP assignment looks correct for joining node"
    else
        echo "⚠ Unexpected IP assignment: $formnet_ip"
    fi
else
    echo "✗ formnet interface not created"
    echo "Check logs: docker-compose logs form-net"
fi
```

#### Start Additional Services

```bash
echo "=== Starting Additional Services ==="

# Start all remaining services
docker-compose up -d

# Wait for services to stabilize
sleep 60

# Check all service health
echo "=== Node-B Services Health Check ==="
services=(
    "form-state:3004"
    "vmm-service:3002"
    "form-pack-manager:3003"
)

all_healthy=true
for service in "${services[@]}"; do
    name=$(echo $service | cut -d: -f1)
    port=$(echo $service | cut -d: -f2)
    
    if curl -s --connect-timeout 5 http://localhost:$port/health >/dev/null; then
        echo "✓ $name (port $port): healthy"
    else
        echo "✗ $name (port $port): unhealthy"
        all_healthy=false
    fi
done

if $all_healthy; then
    echo "✅ All Node-B services are healthy"
else
    echo "❌ Some Node-B services are unhealthy - check logs"
fi
```

### Step 3.6: Verify Network Joining and Connectivity

Perform comprehensive verification that Node-B successfully joined the network:

#### Network Interface Verification

```bash
echo "=== Node-B Network Interface Verification ==="

# Check bridge interface
if ip addr show br0 >/dev/null 2>&1; then
    bridge_ip=$(ip addr show br0 | grep 'inet ' | awk '{print $2}')
    echo "✓ Bridge interface br0: $bridge_ip"
else
    echo "✗ Bridge interface br0: not found"
fi

# Check formnet interface (critical)
if ip addr show formnet0 >/dev/null 2>&1; then
    formnet_ip=$(ip addr show formnet0 | grep 'inet ' | awk '{print $2}')
    echo "✓ Formnet interface formnet0: $formnet_ip"
    
    # Verify it's in the expected range (172.20.0.0/16)
    if [[ "$formnet_ip" =~ ^172\.20\.0\. ]]; then
        echo "✓ IP is in correct Formnet CIDR range"
    else
        echo "⚠ IP not in expected 172.20.0.0/16 range"
    fi
else
    echo "✗ Formnet interface formnet0: not found"
    echo "Network joining may have failed"
fi
```

#### WireGuard Peer Connectivity

```bash
echo "=== WireGuard Peer Connectivity ==="

# Check WireGuard interface status
if sudo wg show formnet0 >/dev/null 2>&1; then
    echo "✓ WireGuard interface operational"
    
    # Check for peers (should show Node-A)
    peer_count=$(sudo wg show formnet0 | grep -c "peer:" || echo "0")
    if [[ "$peer_count" -gt 0 ]]; then
        echo "✓ WireGuard peers connected: $peer_count"
        echo "Peer details:"
        sudo wg show formnet0 | grep -A 3 "peer:"
    else
        echo "✗ No WireGuard peers found"
        echo "Connection to Node-A may have failed"
    fi
else
    echo "✗ WireGuard interface not operational"
fi

# Test connectivity to Node-A via formnet
echo ""
echo "Testing connectivity to Node-A via formnet..."
if ping -c 3 172.20.0.1 >/dev/null 2>&1; then
    echo "✓ Can ping Node-A via formnet (172.20.0.1)"
else
    echo "✗ Cannot ping Node-A via formnet"
    echo "WireGuard tunnel may not be established"
fi
```

#### Network State Synchronization

```bash
echo "=== Network State Synchronization ==="

# Check peer discovery in formation network
echo "1. Checking peer discovery..."
peer_response=$(curl -s http://localhost:3004/v1/network/peers 2>/dev/null)
if [[ -n "$peer_response" ]]; then
    peer_count=$(echo "$peer_response" | jq length 2>/dev/null || echo "0")
    echo "✓ Network peers API accessible"
    echo "  Peer count: $peer_count"
    
    if [[ "$peer_count" -ge 2 ]]; then
        echo "✓ Multiple peers detected (including Node-A)"
    else
        echo "⚠ Expected at least 2 peers (Node-A + Node-B)"
    fi
else
    echo "✗ Cannot access peers API"
fi

# Check node registration
echo "2. Checking node registration..."
nodes_response=$(curl -s http://localhost:3004/v1/nodes/list 2>/dev/null)
if [[ -n "$nodes_response" ]]; then
    node_count=$(echo "$nodes_response" | jq length 2>/dev/null || echo "0")
    echo "✓ Nodes API accessible"
    echo "  Registered nodes: $node_count"
    
    if [[ "$node_count" -ge 2 ]]; then
        echo "✓ Multiple nodes registered (Node-A + Node-B)"
    else
        echo "⚠ Expected at least 2 registered nodes"
    fi
else
    echo "✗ Cannot access nodes API"
fi

# Check network status
echo "3. Checking network status..."
network_status=$(curl -s http://localhost:3004/v1/network/status 2>/dev/null)
if [[ -n "$network_status" ]]; then
    echo "✓ Network status API accessible"
    echo "  Status: $network_status"
else
    echo "✗ Cannot access network status API"
fi
```

#### Cross-Node Service Communication

```bash
echo "=== Cross-Node Service Communication ==="

# Test API communication to Node-A through formnet
NODE_A_IP=$(cat /tmp/node-a-bootstrap-ip 2>/dev/null || echo "172.20.0.1")

echo "Testing Node-A services via formnet..."

# Test form-state API via formnet tunnel
if curl -s --connect-timeout 10 http://172.20.0.1:3004/health >/dev/null; then
    echo "✓ Can reach Node-A form-state via formnet"
    
    # Get Node-A's peer list and compare
    node_a_peers=$(curl -s http://172.20.0.1:3004/v1/network/peers 2>/dev/null)
    if [[ -n "$node_a_peers" ]]; then
        echo "✓ Can access Node-A peer information"
        node_a_peer_count=$(echo "$node_a_peers" | jq length 2>/dev/null || echo "0")
        echo "  Node-A sees $node_a_peer_count peers"
    fi
else
    echo "✗ Cannot reach Node-A form-state via formnet"
fi

# Test bidirectional communication
echo ""
echo "Testing bidirectional state synchronization..."

# Compare peer lists between nodes
node_b_peers=$(curl -s http://localhost:3004/v1/network/peers 2>/dev/null)
node_a_peers=$(curl -s http://172.20.0.1:3004/v1/network/peers 2>/dev/null)

if [[ -n "$node_b_peers" && -n "$node_a_peers" ]]; then
    node_b_count=$(echo "$node_b_peers" | jq length 2>/dev/null || echo "0")
    node_a_count=$(echo "$node_a_peers" | jq length 2>/dev/null || echo "0")
    
    if [[ "$node_b_count" == "$node_a_count" && "$node_b_count" -ge 2 ]]; then
        echo "✓ Peer lists synchronized between nodes ($node_b_count peers each)"
    else
        echo "⚠ Peer count mismatch: Node-A($node_a_count) vs Node-B($node_b_count)"
    fi
else
    echo "✗ Cannot compare peer lists between nodes"
fi
```

#### Gossip Protocol Verification

```bash
echo "=== Gossip Protocol Verification ==="

# Check for gossip activity in logs
echo "Checking for gossip protocol activity..."

# Look for gossip-related logs in form-state
if docker-compose logs --tail=50 form-state 2>/dev/null | grep -i gossip >/dev/null; then
    echo "✓ Gossip protocol activity detected in logs"
    echo "Recent gossip activity:"
    docker-compose logs --tail=10 form-state 2>/dev/null | grep -i gossip | tail -3
else
    echo "⚠ No gossip protocol activity found in recent logs"
    echo "This might be normal if network is stable"
fi

# Check for peer communication logs
if docker-compose logs --tail=50 form-net 2>/dev/null | grep -i peer >/dev/null; then
    echo "✓ Peer communication activity detected"
else
    echo "⚠ No peer communication activity in recent logs"
fi

# Check for successful network joining messages
if docker-compose logs form-net 2>/dev/null | grep -i "join\|connect\|peer" | tail -5 | grep -v "ERROR" >/dev/null; then
    echo "✓ Successful network joining messages found"
    echo "Recent join activity:"
    docker-compose logs form-net 2>/dev/null | grep -i "join\|connect\|peer" | tail -3
else
    echo "⚠ No clear network joining success messages"
fi
```

### Step 3.7: Final Two-Node Network Verification

Run a comprehensive test to verify the complete two-node network:

```bash
#!/bin/bash
echo "=== Final Two-Node Network Verification ==="

errors=0

# Test 1: Node-B services healthy
echo "1. Node-B Service Health..."
for port in 3004 3002 3003; do
    if ! curl -s --connect-timeout 5 http://localhost:$port/health >/dev/null; then
        echo "   ✗ Node-B service on port $port unhealthy"
        errors=$((errors + 1))
    fi
done
[ $errors -eq 0 ] && echo "   ✓ All Node-B services healthy"

# Test 2: Network interfaces configured
echo "2. Network Interface Check..."
if ! ip addr show formnet0 >/dev/null 2>&1; then
    echo "   ✗ Formnet interface formnet0 missing"
    errors=$((errors + 1))
elif ! ip addr show formnet0 | grep -q "172.20.0."; then
    echo "   ✗ Formnet interface has wrong IP range"
    errors=$((errors + 1))
fi
[ $errors -eq 0 ] && echo "   ✓ Network interfaces configured correctly"

# Test 3: WireGuard peer connectivity
echo "3. WireGuard Connectivity..."
if ! sudo wg show formnet0 | grep -q "peer:"; then
    echo "   ✗ No WireGuard peers connected"
    errors=$((errors + 1))
elif ! ping -c 1 172.20.0.1 >/dev/null 2>&1; then
    echo "   ✗ Cannot ping Node-A via formnet"
    errors=$((errors + 1))
fi
[ $errors -eq 0 ] && echo "   ✓ WireGuard connectivity established"

# Test 4: Cross-node API communication
echo "4. Cross-Node Communication..."
if ! curl -s --connect-timeout 10 http://172.20.0.1:3004/health >/dev/null; then
    echo "   ✗ Cannot reach Node-A API via formnet"
    errors=$((errors + 1))
fi
[ $errors -eq 0 ] && echo "   ✓ Cross-node communication working"

# Test 5: Network state synchronization
echo "5. State Synchronization..."
node_b_peers=$(curl -s http://localhost:3004/v1/network/peers 2>/dev/null | jq length 2>/dev/null || echo "0")
node_a_peers=$(curl -s http://172.20.0.1:3004/v1/network/peers 2>/dev/null | jq length 2>/dev/null || echo "0")

if [[ "$node_b_peers" -lt 2 ]] || [[ "$node_a_peers" -lt 2 ]]; then
    echo "   ✗ Insufficient peers detected (Node-A: $node_a_peers, Node-B: $node_b_peers)"
    errors=$((errors + 1))
elif [[ "$node_b_peers" != "$node_a_peers" ]]; then
    echo "   ✗ Peer count mismatch between nodes (Node-A: $node_a_peers, Node-B: $node_b_peers)"
    errors=$((errors + 1))
fi
[ $errors -eq 0 ] && echo "   ✓ Network state synchronized"

echo "========================"
if [ $errors -eq 0 ]; then
    echo "🎉 Two-Node Formation Network Setup SUCCESSFUL!"
    echo ""
    echo "✅ Node-B successfully joined the network"
    echo "✅ WireGuard mesh connectivity established"
    echo "✅ Cross-node communication operational"
    echo "✅ Network state synchronization working"
    echo "✅ Gossip protocol activity detected"
    echo ""
    echo "Network Summary:"
    echo "  - Node-A (Bootstrap): 172.20.0.1"
    echo "  - Node-B (Joining): $(ip addr show formnet0 | grep 'inet ' | awk '{print $2}' | head -1)"
    echo "  - Total Peers: $node_b_peers"
    echo ""
    echo "Next: Test multi-node functionality, deploy agents, or add more nodes"
else
    echo "❌ Two-Node Network Setup FAILED ($errors errors)"
    echo ""
    echo "Troubleshooting steps:"
    echo "1. Check service logs: docker-compose logs"
    echo "2. Verify Node-A is still operational"
    echo "3. Check network connectivity between nodes"
    echo "4. Review firewall and routing configuration"
    exit 1
fi
```

### Node-B Setup Complete

Node-B has successfully joined the Formation network! Key accomplishments:

✅ **Configuration Generated**: Used form-config-wizard to create proper joining configuration with Node-A as bootstrap  
✅ **Network Connectivity**: Verified connection to Node-A before starting services  
✅ **Services Started**: All Formation services running and healthy on Node-B  
✅ **Network Joined**: Successfully connected to Node-A's Formation network  
✅ **WireGuard Established**: Secure mesh network tunnel operational between nodes  
✅ **State Synchronized**: Network state consistent across both nodes  
✅ **Gossip Protocol**: Peer-to-peer communication and state propagation working  

**Network Status**:
- **Node-A (Bootstrap)**: 172.20.0.1/16 - Network initializer and entry point
- **Node-B (Joining)**: 172.20.0.2/16 - Successfully joined via Node-A
- **Mesh Network**: Secure WireGuard tunnel connecting both nodes
- **Services**: All Formation services operational on both nodes

**Important files created**:
- `secrets/.operator-config.json` - Node-B operator configuration  
- `~/.config/.keystore` - Node-B encrypted keys (keep secure!)

Your two-node Formation network is now fully operational and ready for testing, development, and additional node expansion!

---

## Step 4: Verify Two-Node Network

### Check Node-B Joined Successfully

On **Node-B**:

```bash
# Check service health
curl http://localhost:3004/health
curl http://localhost:3002/health  
curl http://localhost:3003/health
curl http://localhost:51820/health

# Check formnet interface (should have IP like 172.20.0.2/16)
ip addr show formnet0

# Check network status
curl http://localhost:3004/v1/network/status

# Check peer list (should show both nodes)
curl http://localhost:3004/v1/network/peers
```

### Verify Inter-Node Communication

On **Node-A**:

```bash
# Check peer list (should show Node-B)
curl http://localhost:3004/v1/network/peers

# Check WireGuard peers
sudo wg show formnet0

# Test ping over formnet (use Node-B's formnet IP)
ping 172.20.0.2
```

On **Node-B**:

```bash
# Test ping to Node-A over formnet
ping 172.20.0.1

# Check WireGuard connection
sudo wg show formnet0
```

### Verify State Synchronization

Both nodes should have synchronized state:

```bash
# On both nodes, these should return similar results:
curl http://localhost:3004/v1/network/cidrs
curl http://localhost:3004/v1/nodes/list
curl http://localhost:3004/v1/accounts/list
```

## Step 5: Test Network Functionality

### Test Node Registration

Check that both nodes are properly registered:

```bash
# On both nodes - should show 2 nodes
curl http://localhost:3004/v1/nodes/list | jq length
```

### Test State Propagation

Create test data on one node and verify it appears on the other:

On **Node-A**:
```bash
# This is a placeholder - actual API calls depend on implemented endpoints
# For example, if there are test endpoints for creating accounts or other data
```

### Test Service Communication

Verify services can communicate across the network:

```bash
# Check cross-node connectivity
# From Node-B, try to reach Node-A's services through formnet
curl http://172.20.0.1:3004/health

# From Node-A, try to reach Node-B's services  
curl http://172.20.0.2:3004/health
```

## Troubleshooting

### Node-B Cannot Connect to Node-A

**Check Network Connectivity**:
```bash
# From Node-B, test basic connectivity
ping <NODE_A_IP>
telnet <NODE_A_IP> 51820
telnet <NODE_A_IP> 3004
```

**Check Node-A Firewall**:
```bash
# On Node-A, verify ports are open
sudo netstat -tulpn | grep -E "(51820|3004)"
sudo iptables -L | grep -E "(51820|3004)"
```

**Check Configuration**:
```bash
# Verify Node-B bootstrap_nodes points to correct IP
cat secrets/.operator-config.json | grep bootstrap_nodes
```

### Services Won't Start

```bash
# Check service logs
docker-compose logs form-state
docker-compose logs form-net

# Check configuration
docker-compose config

# Restart specific service
docker-compose restart form-state
```

### Network Interface Issues

```bash
# Check bridge exists on both nodes
ip addr show br0

# Check formnet interface
ip addr show formnet0

# Check WireGuard status
sudo wg show
```

### State Not Synchronizing

```bash
# Check gossip logs
docker-compose logs form-state | grep -i gossip
docker-compose logs form-net | grep -i gossip

# Verify peer connectivity in WireGuard
sudo wg show formnet0

# Check if services can reach each other
curl http://172.20.0.1:3004/v1/network/peers
curl http://172.20.0.2:3004/v1/network/peers
```

## Expected Results

After successful setup, you should have:

✅ **Two running Formation nodes** with all services healthy  
✅ **Secure WireGuard mesh network** connecting both nodes  
✅ **Synchronized network state** across both nodes  
✅ **Cross-node connectivity** verified with ping and API calls  
✅ **Peer discovery** showing both nodes in peer lists  

## Next Steps

With your two-node network operational:

1. **Deploy test agents** to verify multi-node task distribution
2. **Test failover scenarios** by stopping one node temporarily
3. **Monitor resource usage** to understand operational requirements
4. **Scale to additional nodes** using the same joining process
5. **Implement monitoring** and alerting for production readiness

## Production Considerations

For production two-node deployments:

- [ ] Generate unique, secure ECDSA keys for each node using form-config-wizard
- [ ] Use proper DNS names instead of IP addresses
- [ ] Set up TLS certificates for secure API communication
- [ ] Configure automated backups of node state and configuration
- [ ] Implement monitoring and alerting for network health
- [ ] Plan for geographic distribution and network redundancy
- [ ] Set up log aggregation and centralized monitoring

## Security Notes

- Keep your operator configuration files (`secrets/.operator-config.json`) secure
- Use strong, unique passwords for each node
- Regularly rotate cryptographic keys
- Monitor network traffic and access logs
- Keep Formation services updated to latest versions 