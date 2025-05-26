# Formation Operators Troubleshooting Guide

This guide helps Formation operators diagnose and resolve common issues when running Formation nodes. Follow the diagnostic steps to identify problems and apply the appropriate solutions.

## Quick Diagnostic Commands

Start with these commands to get an overview of your node's health:

```bash
# Check all service status
docker-compose ps

# Check service health endpoints
curl -s http://localhost:3004/health | jq .
curl -s http://localhost:3002/health | jq .
curl -s http://localhost:3003/health | jq .
curl -s http://localhost:51820/health | jq .

# Check network interfaces
ip addr show br0
ip addr show formnet0

# Check logs for recent errors
docker-compose logs --tail=50 | grep -i error
```

## Common Build Errors and Solutions

This section covers issues that occur when building Formation from source code. If you're using Docker Compose, skip to the [Service Startup Issues](#service-startup-issues) section.

### Rust Toolchain Issues

#### Error: `cargo: command not found`

**Symptoms**: Terminal shows cargo command not found when trying to build

**Diagnosis**:
```bash
# Check if Rust is installed
which cargo
which rustc

# Check PATH
echo $PATH | grep -o "[^:]*cargo[^:]*"
```

**Solutions**:

##### 1. Install Rust Using rustup

```bash
# Install Rust using official installer
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Source the environment
source "$HOME/.cargo/env"

# Verify installation
cargo --version
rustc --version
```

##### 2. Fix PATH Environment

```bash
# Add to shell profile
echo 'export PATH="$HOME/.cargo/bin:$PATH"' >> ~/.bashrc

# Reload shell environment
source ~/.bashrc
```

#### Error: `rustc version too old`

**Symptoms**: Build fails with message about unsupported Rust version

**Example Error**:
```
error: package requires rustc 1.70.0 or newer
```

**Solutions**:

```bash
# Update Rust to latest stable
rustup update stable

# Set stable as default
rustup default stable

# Verify version
rustc --version
# Should show 1.70.0 or newer
```

#### Error: Missing Rust Components

**Symptoms**: Build fails with missing clippy, rustfmt, or other components

**Example Error**:
```
error: the 'rustfmt' component is not available
```

**Solutions**:

```bash
# Add missing components
rustup component add clippy rustfmt

# For specific targets
rustup target add x86_64-unknown-linux-gnu

# List installed components
rustup component list --installed
```

### Compilation Errors

#### Error: `linking with cc failed`

**Symptoms**: Build completes but linking fails

**Example Error**:
```
error: linking with `cc` failed: exit status: 1
note: /usr/bin/ld: cannot find -lssl
```

**Diagnosis**:
```bash
# Check for development libraries
pkg-config --exists openssl && echo "OpenSSL found" || echo "OpenSSL missing"
pkg-config --exists sqlite3 && echo "SQLite3 found" || echo "SQLite3 missing"
pkg-config --exists libudev && echo "libudev found" || echo "libudev missing"

# Check linker
which cc
which ld
```

**Solutions**:

##### 1. Install Missing System Dependencies

```bash
# Ubuntu/Debian
sudo apt update
sudo apt install build-essential pkg-config libssl-dev libsqlite3-dev libudev-dev

# Additional dependencies for Formation
sudo apt install protobuf-compiler cmake clang libclang-dev
sudo apt install wireguard-tools libwireguard-dev
sudo apt install qemu-system-x86 qemu-utils qemu-kvm
```

##### 2. Install Alternative Linker (Optional)

```bash
# Install faster linker
sudo apt install lld

# Configure Cargo to use it
mkdir -p ~/.cargo
cat >> ~/.cargo/config.toml << EOF
[target.x86_64-unknown-linux-gnu]
linker = "clang"
rustflags = ["-C", "link-arg=-fuse-ld=lld"]
EOF
```

#### Error: `could not compile` with dependency issues

**Symptoms**: Compilation fails during dependency building

**Example Error**:
```
error: failed to compile `ring v0.16.20`
Could not find libclang
```

**Solutions**:

##### 1. Install Clang and LLVM

```bash
# Ubuntu/Debian
sudo apt install clang llvm-dev libclang-dev

# Set environment variables
export LIBCLANG_PATH=/usr/lib/llvm-14/lib
export BINDGEN_EXTRA_CLANG_ARGS="-I/usr/include"

# Add to ~/.bashrc for persistence
echo 'export LIBCLANG_PATH=/usr/lib/llvm-14/lib' >> ~/.bashrc
```

##### 2. Clean and Rebuild

```bash
# Clean previous build artifacts
cargo clean

# Rebuild with verbose output
cargo build --release --verbose
```

#### Error: `out of memory` during compilation

**Symptoms**: Build fails with memory allocation errors

**Example Error**:
```
error: could not compile: Signal: 9 (SIGKILL)
```

**Diagnosis**:
```bash
# Check available memory
free -h
cat /proc/meminfo | grep MemAvailable

# Monitor memory during build
watch free -h
```

**Solutions**:

##### 1. Reduce Parallel Jobs

```bash
# Limit parallel jobs to reduce memory usage
CARGO_BUILD_JOBS=2 cargo build --release

# Or set in config
mkdir -p ~/.cargo
echo '[build]\njobs = 2' >> ~/.cargo/config.toml
```

##### 2. Add Swap Space

```bash
# Create swap file (if needed)
sudo fallocate -l 4G /swapfile
sudo chmod 600 /swapfile
sudo mkswap /swapfile
sudo swapon /swapfile

# Make permanent
echo '/swapfile none swap sw 0 0' | sudo tee -a /etc/fstab
```

##### 3. Build Individual Packages

```bash
# Build one package at a time
cargo build --release --package form-config
cargo build --release --package form-state
cargo build --release --package formnet
cargo build --release --package vmm-service
cargo build --release --package form-pack
cargo build --release --package form-dns
```

### Dependency Issues

#### Error: `failed to get` dependency from crates.io

**Symptoms**: Network or registry access issues

**Example Error**:
```
error: failed to get `serde` as a dependency of package `formation`
Caused by: failed to load source for dependency
```

**Solutions**:

##### 1. Check Network Connectivity

```bash
# Test crates.io connectivity
curl -I https://crates.io
curl -I https://static.crates.io

# Check proxy settings
echo $HTTP_PROXY
echo $HTTPS_PROXY
```

##### 2. Update Registry Index

```bash
# Update Cargo registry
cargo update

# Force registry refresh
rm -rf ~/.cargo/registry/index/*
cargo fetch
```

##### 3. Use Alternative Registry Mirror

```bash
# Add to ~/.cargo/config.toml
mkdir -p ~/.cargo
cat >> ~/.cargo/config.toml << EOF
[source.crates-io]
replace-with = "tuna"

[source.tuna]
registry = "https://mirrors.tuna.tsinghua.edu.cn/git/crates.io-index.git"
EOF
```

#### Error: `version solving failed`

**Symptoms**: Cargo cannot resolve dependency versions

**Example Error**:
```
error: failed to select a version for `tokio`
required by package `formation`
```

**Solutions**:

##### 1. Update Cargo.lock

```bash
# Delete lockfile and regenerate
rm Cargo.lock
cargo build

# Or update specific dependency
cargo update tokio
```

##### 2. Check Dependency Conflicts

```bash
# Show dependency tree
cargo tree

# Check for conflicts
cargo tree --duplicates
```

### Protocol Buffers Issues

#### Error: `protoc: command not found`

**Symptoms**: Build fails when compiling .proto files

**Example Error**:
```
error: failed to run custom build command for `formation-proto`
protoc: command not found
```

**Solutions**:

```bash
# Install Protocol Buffers compiler
sudo apt update
sudo apt install protobuf-compiler

# Verify installation
protoc --version

# Should show libprotoc 3.x.x or newer
```

#### Error: `protoc version too old`

**Symptoms**: protoc version incompatible

**Solutions**:

```bash
# Install newer protoc manually
PROTOC_ZIP=protoc-21.12-linux-x86_64.zip
curl -OL https://github.com/protocolbuffers/protobuf/releases/download/v21.12/$PROTOC_ZIP
sudo unzip -o $PROTOC_ZIP -d /usr/local bin/protoc
sudo unzip -o $PROTOC_ZIP -d /usr/local 'include/*'
rm -f $PROTOC_ZIP

# Verify version
protoc --version
```

### Platform-Specific Issues

#### Error: KVM/Virtualization Support

**Symptoms**: VMM service fails to build or run

**Example Error**:
```
error: KVM device not accessible
```

**Diagnosis**:
```bash
# Check KVM support
ls -la /dev/kvm
lsmod | grep kvm

# Check if virtualization is enabled
grep -E 'vmx|svm' /proc/cpuinfo
```

**Solutions**:

##### 1. Enable KVM Module

```bash
# Load KVM module
sudo modprobe kvm
sudo modprobe kvm_intel  # For Intel CPUs
# OR
sudo modprobe kvm_amd    # For AMD CPUs

# Add user to kvm group
sudo usermod -aG kvm $USER

# Log out and back in for changes to take effect
```

##### 2. Enable Virtualization in BIOS

- Restart and enter BIOS/UEFI settings
- Enable Intel VT-x or AMD-V virtualization
- Save and restart

#### Error: Cross-compilation Issues

**Symptoms**: Building for different architecture fails

**Solutions**:

```bash
# Install cross-compilation target
rustup target add x86_64-unknown-linux-musl

# Install cross-compilation tools
sudo apt install musl-tools

# Build for specific target
cargo build --target x86_64-unknown-linux-musl --release
```

### Build Environment Issues

#### Error: `permission denied` during build

**Symptoms**: Build fails with file permission errors

**Solutions**:

```bash
# Fix repository permissions
sudo chown -R $USER:$USER /path/to/formation

# Fix Cargo cache permissions
sudo chown -R $USER:$USER ~/.cargo

# Check disk space
df -h .
```

#### Error: `disk full` during build

**Symptoms**: Build fails due to insufficient disk space

**Solutions**:

```bash
# Check disk usage
df -h
du -sh target/

# Clean build artifacts
cargo clean

# Clean Cargo cache
rm -rf ~/.cargo/registry/cache/*
rm -rf ~/.cargo/git/db/*

# Use different target directory
export CARGO_TARGET_DIR=/tmp/formation-build
cargo build --release
```

### Advanced Build Issues

#### Error: Incremental compilation corruption

**Symptoms**: Weird compilation errors that don't make sense

**Solutions**:

```bash
# Disable incremental compilation
export CARGO_INCREMENTAL=0

# Clean and rebuild
cargo clean
cargo build --release

# Or set in config
mkdir -p ~/.cargo
echo '[build]\nincremental = false' >> ~/.cargo/config.toml
```

#### Error: `failed to run custom build command`

**Symptoms**: Build scripts fail for various packages

**Example Error**:
```
error: failed to run custom build command for `openssl-sys`
```

**Solutions**:

##### 1. Install OpenSSL Development Files

```bash
# Ubuntu/Debian
sudo apt install libssl-dev

# Set environment variables if needed
export OPENSSL_DIR=/usr
export OPENSSL_LIB_DIR=/usr/lib/x86_64-linux-gnu
export OPENSSL_INCLUDE_DIR=/usr/include/openssl
```

##### 2. Use System OpenSSL

```bash
# Build with system OpenSSL
export OPENSSL_NO_VENDOR=1
cargo build --release
```

### Build Optimization Issues

#### Very Slow Build Times

**Diagnosis**:
```bash
# Check build parallelism
echo "CPU cores: $(nproc)"
echo "Cargo jobs: ${CARGO_BUILD_JOBS:-$(nproc)}"

# Monitor build resources
htop
iotop
```

**Solutions**:

##### 1. Optimize Build Configuration

```bash
# Create optimized Cargo config
mkdir -p ~/.cargo
cat > ~/.cargo/config.toml << EOF
[build]
jobs = 0  # Use all CPU cores

[target.x86_64-unknown-linux-gnu]
linker = "clang"
rustflags = ["-C", "link-arg=-fuse-ld=lld"]

[profile.dev]
debug = 1
opt-level = 0

[profile.release]
debug = false
opt-level = 3
lto = "thin"
codegen-units = 1
panic = "abort"
EOF
```

##### 2. Use Compilation Cache

```bash
# Install sccache
cargo install sccache

# Configure sccache
export RUSTC_WRAPPER=sccache
export SCCACHE_CACHE_SIZE=10G

# Add to ~/.bashrc
echo 'export RUSTC_WRAPPER=sccache' >> ~/.bashrc
echo 'export SCCACHE_CACHE_SIZE=10G' >> ~/.bashrc
```

### Complete Build Troubleshooting Script

Create this script to diagnose build environment issues:

```bash
#!/bin/bash
# Save as: diagnose-build-environment.sh

echo "=== Formation Build Environment Diagnosis ==="

errors=0

# Check Rust toolchain
echo "=== Rust Toolchain ==="
if command -v rustc >/dev/null 2>&1; then
    rust_version=$(rustc --version)
    echo "✓ Rust: $rust_version"
    
    # Check version is recent enough
    version_num=$(echo $rust_version | grep -oE '[0-9]+\.[0-9]+' | head -1)
    if [[ $(echo "$version_num >= 1.70" | bc -l) == 1 ]]; then
        echo "✓ Rust version adequate"
    else
        echo "✗ Rust version too old (need 1.70+)"
        errors=$((errors + 1))
    fi
else
    echo "✗ Rust not found"
    errors=$((errors + 1))
fi

# Check system dependencies
echo "=== System Dependencies ==="
deps=("gcc" "pkg-config" "cmake" "protoc" "clang")
for dep in "${deps[@]}"; do
    if command -v $dep >/dev/null 2>&1; then
        echo "✓ $dep"
    else
        echo "✗ $dep missing"
        errors=$((errors + 1))
    fi
done

# Check development libraries
echo "=== Development Libraries ==="
libs=("openssl" "sqlite3" "libudev")
for lib in "${libs[@]}"; do
    if pkg-config --exists $lib 2>/dev/null; then
        version=$(pkg-config --modversion $lib 2>/dev/null)
        echo "✓ $lib ($version)"
    else
        echo "✗ $lib missing"
        errors=$((errors + 1))
    fi
done

# Check virtualization support
echo "=== Virtualization Support ==="
if ls /dev/kvm >/dev/null 2>&1; then
    echo "✓ KVM device available"
    if lsmod | grep -q kvm; then
        echo "✓ KVM modules loaded"
    else
        echo "⚠ KVM modules not loaded"
    fi
else
    echo "✗ KVM device not found"
    errors=$((errors + 1))
fi

# Check available resources
echo "=== System Resources ==="
cpu_cores=$(nproc)
ram_gb=$(free -g | awk 'NR==2{print $2}')
disk_gb=$(df . | tail -1 | awk '{print int($4/1024/1024)}')

echo "CPU cores: $cpu_cores"
echo "RAM: ${ram_gb}GB"
echo "Available disk: ${disk_gb}GB"

if [ $cpu_cores -lt 4 ]; then
    echo "⚠ Low CPU count (recommend 4+)"
fi
if [ $ram_gb -lt 8 ]; then
    echo "⚠ Low RAM (recommend 8GB+)"
fi
if [ $disk_gb -lt 20 ]; then
    echo "⚠ Low disk space (need 20GB+)"
fi

echo "========================"
if [ $errors -eq 0 ]; then
    echo "✅ Build environment looks good!"
    echo "Ready to build Formation from source."
else
    echo "❌ Build environment has $errors issues."
    echo "Fix the issues above before building."
    exit 1
fi
```

Run this script before attempting to build:
```bash
chmod +x diagnose-build-environment.sh
./diagnose-build-environment.sh
```

## Service Startup Issues

### Services Won't Start

**Symptoms**: Containers exit immediately or show unhealthy status

**Diagnosis**:
```bash
# Check container status
docker-compose ps

# View service logs
docker-compose logs form-state
docker-compose logs form-net
docker-compose logs form-vmm
docker-compose logs form-pack-manager
docker-compose logs form-dns
```

**Common Causes & Solutions**:

#### 1. Configuration File Missing or Invalid

```bash
# Check if config file exists
ls -la secrets/.operator-config.json

# Validate JSON syntax
cat secrets/.operator-config.json | jq .

# Check file permissions
chmod 600 secrets/.operator-config.json
```

**Solution**: Ensure your operator configuration exists and has valid JSON format.

#### 2. Environment Variables Not Set

```bash
# Check environment configuration
docker-compose config

# Verify .env file exists
ls -la .env
cat .env
```

**Solution**: Create or fix your `.env` file:
```bash
cat > .env << EOF
SECRET_PATH=/etc/formation/.operator-config.json
PASSWORD=your-secure-password
EOF
```

#### 3. Port Conflicts

```bash
# Check if ports are already in use
sudo netstat -tulpn | grep -E "(3002|3003|3004|51820|53)"
```

**Solution**: Stop conflicting services or change Formation service ports in your configuration.

#### 4. Docker Permissions

```bash
# Check if user is in docker group
groups $USER

# Add user to docker group if missing
sudo usermod -aG docker $USER
# Log out and back in
```

### Service Keeps Restarting

**Symptoms**: Container status shows "Restarting" or frequent restarts

**Diagnosis**:
```bash
# Monitor live logs
docker-compose logs -f <service-name>

# Check restart count
docker stats
```

**Common Causes & Solutions**:

#### 1. Insufficient Resources

```bash
# Check system resources
free -h
df -h
top
```

**Solution**: Increase available RAM/CPU or reduce resource requirements.

#### 2. Failed Health Checks

```bash
# Test health endpoints manually
curl -v http://localhost:3004/health
curl -v http://localhost:3002/health
```

**Solution**: Fix underlying issues causing health check failures.

## Network Connectivity Issues

Formation's network architecture has multiple layers that can experience connectivity issues. This section provides comprehensive troubleshooting for all network-related problems, organized by the network layer where the issue occurs.

### Network Architecture Overview

Formation uses a layered network approach:
1. **Physical Layer**: Internet/LAN connectivity between nodes
2. **WireGuard Layer**: Encrypted P2P tunnels (formnet interface)
3. **Service Layer**: HTTP APIs between Formation services
4. **Application Layer**: CRDT gossip and task dispatch
5. **VM Network Layer**: Bridge interfaces for virtual machine connectivity

### Bootstrap Node Discovery Issues

**Symptoms**: Node can't find or connect to bootstrap nodes, "Unable to acquire bootstrap information" errors

**Diagnosis**:
```bash
# Test basic connectivity to bootstrap nodes
ping <bootstrap-node-ip>
nc -zv <bootstrap-node-ip> 51820

# Check bootstrap configuration
cat secrets/.operator-config.json | jq '.bootstrap_nodes, .bootstrap_domain'

# Test bootstrap API endpoints
curl -v http://<bootstrap-node-ip>:51820/bootstrap
curl -v http://<bootstrap-node-ip>:51820/health

# Check DNS resolution for bootstrap domains
nslookup bootstrap.formation.cloud
dig bootstrap.formation.cloud A
```

**Solutions**:

#### 1. Bootstrap Domain Resolution Issues

```bash
# Check if bootstrap domain resolves
host bootstrap.formation.cloud

# Try alternative DNS servers
nslookup bootstrap.formation.cloud 8.8.8.8
nslookup bootstrap.formation.cloud 1.1.1.1

# Check local DNS configuration
cat /etc/resolv.conf

# Flush DNS cache
sudo systemctl flush-dns
# OR on older systems:
sudo /etc/init.d/dns-clean restart
```

#### 2. Bootstrap Node Unreachable

```bash
# Check if bootstrap node is actually running
curl http://<bootstrap-ip>:51820/health
curl http://<bootstrap-ip>:3004/health

# Test different ports
nc -zv <bootstrap-ip> 51820  # WireGuard/formnet API
nc -zv <bootstrap-ip> 3004   # form-state API
nc -zv <bootstrap-ip> 5453   # DNS service
```

#### 3. Incorrect Bootstrap Configuration

```bash
# Verify bootstrap nodes format in config
cat secrets/.operator-config.json | jq .bootstrap_nodes

# Should be array of "IP:PORT" or "DOMAIN:PORT" strings
# Correct format: ["192.168.1.100:51820", "bootstrap.formation.cloud:51820"]
# Incorrect: ["192.168.1.100", "bootstrap.formation.cloud"]
```

**Fix**: Update configuration with proper format:
```bash
# Edit config to include port numbers
vim secrets/.operator-config.json
# Ensure bootstrap_nodes entries include ":51820"
```

### WireGuard Interface Issues

**Symptoms**: `ip addr show formnet0` shows no interface, WireGuard connection failures

**Diagnosis**:
```bash
# Check if WireGuard is installed and loaded
which wg
lsmod | grep wireguard
sudo wg show

# Check formnet interface status
ip addr show formnet0
ip link show formnet0

# Check WireGuard configuration
sudo cat /etc/wireguard/formnet.conf
# OR check Formation's config location
cat ~/.config/formnet/formnet.conf

# Check form-net service logs
docker-compose logs form-net | grep -i "wireguard\|interface\|error"
```

**Solutions**:

#### 1. WireGuard Not Installed

```bash
# Ubuntu/Debian
sudo apt update
sudo apt install wireguard wireguard-tools

# CentOS/RHEL/Fedora
sudo dnf install wireguard-tools
# OR
sudo yum install wireguard-tools

# Load kernel module
sudo modprobe wireguard

# Verify module loaded
lsmod | grep wireguard
```

#### 2. Insufficient Privileges for WireGuard

```bash
# Check if user can access WireGuard
sudo wg show

# Add user to appropriate groups
sudo usermod -aG netdev $USER

# For Docker containers, ensure NET_ADMIN capability
docker inspect formation-form-net | grep -A 10 CapAdd

# If missing, add to docker-compose.yml:
# cap_add:
#   - NET_ADMIN
# OR
# privileged: true
```

#### 3. WireGuard Configuration Issues

```bash
# Check for valid WireGuard config
sudo wg-quick down formnet  # Stop if running
sudo wg-quick up formnet    # Start with verbose output

# Check config file syntax
sudo wg-quick parse formnet

# Manually test WireGuard interface creation
sudo ip link add formnet type wireguard
sudo ip addr add 10.42.0.2/16 dev formnet
sudo ip link set formnet up
```

#### 4. Port Conflicts

```bash
# Check if WireGuard port (51820) is in use
sudo netstat -ulnp | grep 51820
sudo ss -ulnp | grep 51820

# Check for conflicting services
sudo lsof -i :51820

# If port is in use, either:
# 1. Stop conflicting service
# 2. Change WireGuard port in configuration
```

### Peer Discovery and Connection Issues

**Symptoms**: Nodes can't discover each other, peer list empty, "no peers found" errors

**Diagnosis**:
```bash
# Check current peer status
sudo wg show formnet
sudo wg show formnet peers

# Check form-state peer information
curl http://localhost:3004/v1/network/peers | jq .
curl http://localhost:3004/v1/peer/list_active

# Check if node is registered in network
curl http://localhost:3004/v1/node/list | jq .

# Test connectivity to known peers
ping <peer-formnet-ip>
nc -zv <peer-external-ip> 51820
```

**Solutions**:

#### 1. NAT/Firewall Blocking Peer Communication

```bash
# Check firewall rules
sudo iptables -L INPUT -v -n
sudo ufw status verbose

# Allow WireGuard traffic
sudo iptables -A INPUT -p udp --dport 51820 -j ACCEPT
sudo iptables -A OUTPUT -p udp --sport 51820 -j ACCEPT

# For UFW users
sudo ufw allow 51820/udp

# Check NAT rules for outbound connections
sudo iptables -t nat -L POSTROUTING -v -n
```

#### 2. Router/NAT Configuration for Bootstrap Nodes

For nodes behind NAT that need to accept incoming connections:

```bash
# Configure port forwarding on router:
# External Port 51820/UDP → Internal IP:51820/UDP

# Test if port forwarding works
# From external network:
nc -zv <external-ip> 51820

# Check if node detects correct public IP
curl http://api.ipify.org
# Compare with what formnet thinks is the public IP
```

#### 3. Peer Endpoint Resolution Issues

```bash
# Check if peers have correct endpoint information
sudo wg show formnet dump

# Look for peers with missing or incorrect endpoints
# Endpoints should show external IP:port

# Force peer endpoint update
sudo wg set formnet peer <peer-public-key> endpoint <correct-ip>:51820

# Restart formnet to refresh peer discovery
docker-compose restart form-net
```

#### 4. Clock Synchronization Issues

```bash
# Check system time on all nodes
date
timedatectl status

# Sync time if needed
sudo ntpdate -s time.nist.gov
# OR
sudo chrony sources -v

# Large time differences can cause peer authentication failures
```

### Service-to-Service Communication Issues

**Symptoms**: Services can't communicate, API calls failing, "connection refused" errors

**Diagnosis**:
```bash
# Test all Formation service endpoints
curl http://localhost:3004/health  # form-state
curl http://localhost:3001/health  # form-pack
curl http://localhost:3002/health  # form-vmm-service
curl http://localhost:5453/health  # form-dns
curl http://localhost:51820/health # formnet API

# Check service binding and ports
sudo netstat -tlnp | grep -E "(3001|3002|3003|3004|5453|51820)"
sudo ss -tlnp | grep -E "(3001|3002|3003|3004|5453|51820)"

# Check Docker network connectivity
docker network ls
docker network inspect formation_default
```

**Solutions**:

#### 1. Service Not Listening on Expected Ports

```bash
# Check which services are actually running
docker-compose ps

# Check service logs for binding errors
docker-compose logs form-state | grep -i "bind\|listen\|port"
docker-compose logs form-pack | grep -i "bind\|listen\|port"

# Restart services that failed to bind
docker-compose restart <service-name>
```

#### 2. Docker Network Issues

```bash
# Recreate Docker network
docker-compose down
docker network prune
docker-compose up -d

# Check container connectivity within Docker network
docker exec formation-form-state ping formation-form-pack
docker exec formation-form-state curl http://formation-form-pack:3001/health
```

#### 3. Service Discovery Issues

```bash
# Check if services can resolve each other by name
docker exec formation-form-state nslookup formation-form-pack
docker exec formation-form-state nslookup formation-form-dns

# Check /etc/hosts in containers
docker exec formation-form-state cat /etc/hosts
```

### CRDT Gossip and State Synchronization Issues

**Symptoms**: Nodes have different state, data not propagating, "gossip failed" errors

**Diagnosis**:
```bash
# Compare state between nodes
curl http://node1:3004/v1/network/peers | jq 'length'
curl http://node2:3004/v1/network/peers | jq 'length'

# Check for gossip errors in logs
docker-compose logs form-state | grep -i "gossip\|sync\|crdt"

# Check devnet vs production mode
docker-compose logs form-state | grep -i "devnet\|production"

# Test direct API connectivity between nodes
curl http://<peer-formnet-ip>:3004/health
curl http://<peer-formnet-ip>:3004/v1/network/peers
```

**Solutions**:

#### 1. Devnet Mode Gossip Issues

In devnet mode, gossip happens via direct API calls:

```bash
# Check if nodes can reach each other's APIs
curl http://<peer-formnet-ip>:3004/v1/devnet_gossip/apply_op

# Check authentication for gossip calls
docker-compose logs form-state | grep -i "devnet_gossip\|signature\|auth"

# Verify ECDSA signatures are working
docker-compose logs form-state | grep -i "recovered address"
```

#### 2. Production Mode Queue Issues

In production mode, gossip uses the message queue:

```bash
# Check form-p2p queue service
docker-compose logs form-p2p | grep -i "queue\|message"

# Check queue connectivity
curl http://localhost:53333/health  # form-p2p queue port

# Test queue operations
curl -X POST http://localhost:53333/queue/write \
  -H "Content-Type: application/json" \
  -d '{"test": "message"}'
```

#### 3. Network Partition Recovery

```bash
# Force state refresh from bootstrap node
curl -X POST http://localhost:3004/v1/bootstrap/full_state

# Restart form-state to trigger resync
docker-compose restart form-state

# Check if state converges after restart
sleep 30
curl http://localhost:3004/v1/network/peers | jq 'length'
```

### DNS Resolution Issues

**Symptoms**: Domain names not resolving, DNS queries failing, bootstrap domain issues

**Diagnosis**:
```bash
# Test Formation DNS service
dig @localhost -p 5453 bootstrap.formation.cloud
nslookup bootstrap.formation.cloud localhost

# Check form-dns service status
docker-compose logs form-dns | grep -i "dns\|resolve\|error"

# Test external DNS resolution
dig @8.8.8.8 bootstrap.formation.cloud
dig @1.1.1.1 bootstrap.formation.cloud

# Check system DNS configuration
cat /etc/resolv.conf
systemd-resolve --status
```

**Solutions**:

#### 1. form-dns Service Issues

```bash
# Check if form-dns is listening
sudo netstat -ulnp | grep 5453
sudo ss -ulnp | grep 5453

# Restart form-dns service
docker-compose restart form-dns

# Check DNS service health
curl http://localhost:5453/health
```

#### 2. DNS Configuration Issues

```bash
# Check if system is using Formation DNS
cat /etc/resolv.conf
# Should include: nameserver 127.0.0.1

# Temporarily test with Formation DNS
dig @127.0.0.1 -p 5453 bootstrap.formation.cloud

# Check DNS forwarding configuration
docker-compose logs form-dns | grep -i "fallback\|forward"
```

#### 3. Bootstrap Domain Registration Issues

```bash
# Check if bootstrap nodes are registered
curl http://localhost:5453/api/records | jq .

# Manually register bootstrap node (if needed)
curl -X POST http://localhost:5453/api/record \
  -H "Content-Type: application/json" \
  -d '{
    "domain": "bootstrap.formation.cloud",
    "record_type": "A",
    "public_ip": ["<bootstrap-ip>"],
    "ttl": 300
  }'
```

### VM Network Connectivity Issues

**Symptoms**: VMs can't access internet, VMs can't communicate with host or other VMs

**Diagnosis**:
```bash
# Check bridge interface status
ip addr show br0
brctl show br0

# Check if VMs are connected to bridge
brctl show br0 | grep tap

# Test VM connectivity from host
ping <vm-ip>

# Check IP forwarding
cat /proc/sys/net/ipv4/ip_forward

# Check NAT rules for VM traffic
sudo iptables -t nat -L POSTROUTING -v -n | grep br0
```

**Solutions**:

#### 1. Bridge Interface Not Created

```bash
# Create bridge interface manually
sudo brctl addbr br0
sudo ip addr add 192.168.100.1/24 dev br0
sudo ip link set br0 up

# Make persistent (Ubuntu/Debian)
cat >> /etc/netplan/01-formation-bridge.yaml << EOF
network:
  version: 2
  bridges:
    br0:
      addresses: [192.168.100.1/24]
      dhcp4: false
EOF

sudo netplan apply
```

#### 2. IP Forwarding Disabled

```bash
# Enable IP forwarding temporarily
sudo sysctl -w net.ipv4.ip_forward=1

# Make permanent
echo 'net.ipv4.ip_forward=1' | sudo tee -a /etc/sysctl.conf

# Apply immediately
sudo sysctl -p
```

#### 3. Missing NAT Rules for VM Traffic

```bash
# Add NAT rule for VM subnet
sudo iptables -t nat -A POSTROUTING -s 192.168.100.0/24 ! -o br0 -j MASQUERADE

# Allow forwarding for VM traffic
sudo iptables -A FORWARD -i br0 -o eth0 -j ACCEPT
sudo iptables -A FORWARD -i eth0 -o br0 -m state --state RELATED,ESTABLISHED -j ACCEPT

# Make iptables rules persistent
sudo iptables-save > /etc/iptables/rules.v4
```

#### 4. TAP Interface Issues

```bash
# Check if TAP interfaces are created for VMs
ip link show | grep tap

# Check TAP interface permissions
ls -la /dev/net/tun

# Ensure user can create TAP interfaces
sudo usermod -aG netdev $USER

# For Docker containers, ensure proper capabilities
# Add to docker-compose.yml:
# cap_add:
#   - NET_ADMIN
#   - NET_RAW
# devices:
#   - /dev/net/tun:/dev/net/tun
```

### Advanced Network Debugging

#### 1. Packet Capture and Analysis

```bash
# Capture WireGuard traffic
sudo tcpdump -i any -n port 51820

# Capture formnet interface traffic
sudo tcpdump -i formnet0 -n

# Capture bridge traffic
sudo tcpdump -i br0 -n

# Capture specific service traffic
sudo tcpdump -i any -n port 3004  # form-state API
```

#### 2. Network Performance Testing

```bash
# Test bandwidth between nodes
iperf3 -s  # On one node
iperf3 -c <peer-formnet-ip>  # On another node

# Test latency
ping -c 10 <peer-formnet-ip>
mtr <peer-formnet-ip>

# Test DNS resolution performance
time dig @localhost -p 5453 bootstrap.formation.cloud
```

#### 3. Comprehensive Network Diagnostic Script

```bash
#!/bin/bash
# save as network-diagnostic.sh

echo "=== Formation Network Diagnostic $(date) ==="

echo "--- Physical Network ---"
ip route show
ip addr show | grep -E "(eth0|wlan0|enp)"

echo "--- WireGuard Status ---"
sudo wg show formnet 2>/dev/null || echo "formnet interface not found"

echo "--- Formation Services ---"
for port in 3001 3002 3004 5453 51820; do
    if nc -z localhost $port 2>/dev/null; then
        echo "✓ Port $port: listening"
    else
        echo "✗ Port $port: not listening"
    fi
done

echo "--- Peer Connectivity ---"
curl -s http://localhost:3004/v1/peer/list_active | jq -r '.[]' | while read peer_ip; do
    if ping -c 1 -W 2 $peer_ip >/dev/null 2>&1; then
        echo "✓ Peer $peer_ip: reachable"
    else
        echo "✗ Peer $peer_ip: unreachable"
    fi
done

echo "--- DNS Resolution ---"
for domain in bootstrap.formation.cloud; do
    if dig @localhost -p 5453 +short $domain >/dev/null 2>&1; then
        echo "✓ DNS $domain: resolves"
    else
        echo "✗ DNS $domain: fails"
    fi
done

echo "--- Bridge Status ---"
if ip addr show br0 >/dev/null 2>&1; then
    echo "✓ Bridge br0: exists"
    brctl show br0
else
    echo "✗ Bridge br0: not found"
fi

echo "========================"
```

### Network Recovery Procedures

#### 1. Complete Network Reset

```bash
# Stop all Formation services
docker-compose down

# Remove all network interfaces
sudo ip link del formnet0 2>/dev/null || true
sudo ip link del br0 2>/dev/null || true

# Clear WireGuard configuration
sudo rm -f /etc/wireguard/formnet.conf
rm -f ~/.config/formnet/formnet.conf

# Reset iptables (CAUTION: may affect other services)
sudo iptables -F
sudo iptables -t nat -F

# Restart networking
sudo systemctl restart networking
# OR
sudo systemctl restart NetworkManager

# Restart Formation services
docker-compose up -d
```

#### 2. Selective Network Recovery

```bash
# Reset only WireGuard
sudo wg-quick down formnet
sudo ip link del formnet0
docker-compose restart form-net

# Reset only bridge
sudo ip link del br0
docker-compose restart form-vmm-service

# Reset only DNS
docker-compose restart form-dns
```

#### 3. Emergency Connectivity Restoration

```bash
# If all else fails, manually configure basic connectivity
sudo ip link add formnet0 type wireguard
sudo ip addr add 10.42.0.2/16 dev formnet0
sudo ip link set formnet0 up

# Add route to peer subnet
sudo ip route add 10.42.0.0/16 dev formnet0

# Test basic connectivity
ping 10.42.0.1  # Bootstrap node
```

## Authentication & Key Management Issues

Formation uses ECDSA (Elliptic Curve Digital Signature Algorithm) for authentication across all services. This section covers comprehensive troubleshooting for authentication failures, key management problems, and admin access issues.

### Authentication Architecture Overview

Formation's authentication system has multiple layers:
1. **ECDSA Signature Verification**: All API requests must be signed with private keys
2. **Address Recovery**: Public addresses are recovered from signatures for identity verification
3. **Account-Based Authorization**: Accounts have different privilege levels (user, admin)
4. **Admin Key System**: Global admin keys can bypass normal authorization
5. **Localhost Bypass**: Local requests bypass authentication for bootstrap operations

### ECDSA Signature Verification Issues

**Symptoms**: API calls return 401 Unauthorized, "signature verification failed", "invalid signature format" errors

**Diagnosis**:
```bash
# Check logs for detailed auth errors
docker-compose logs form-state | grep -i "signature\|auth\|unauthorized\|recovered"

# Check if request headers are properly formatted
docker-compose logs form-state | grep -i "missing signature\|invalid signature format"

# Test a simple authenticated request
curl -X GET http://localhost:3004/v1/account/list \
  -H "X-Signature: <signature>" \
  -H "X-Recovery-Id: <recovery_id>" \
  -H "X-Message: <message>"
```

**Solutions**:

#### 1. Missing or Malformed Signature Headers

Formation requires three specific headers for ECDSA authentication:

```bash
# Required headers for authenticated requests:
# X-Signature: hex-encoded signature bytes
# X-Recovery-Id: recovery ID (0 or 1)  
# X-Message: message that was signed

# Example of properly formatted headers:
curl -X GET http://localhost:3004/v1/account/list \
  -H "X-Signature: 304502210089ab..." \
  -H "X-Recovery-Id: 0" \
  -H "X-Message: GET/v1/account/list1640995200"
```

**Common Header Issues**:
- Missing any of the three required headers
- Invalid hex encoding in X-Signature
- Wrong recovery ID (must be 0 or 1)
- Incorrect message format for signing

#### 2. Incorrect Message Construction for Signing

The message format must exactly match what the server expects:

```bash
# Message format: METHOD + PATH + TIMESTAMP
# Example: "GET/v1/account/list1640995200"

# Common mistakes:
# - Including query parameters in path
# - Wrong timestamp format
# - Including protocol/host in path
# - Case sensitivity issues

# Correct message construction:
METHOD="GET"
PATH="/v1/account/list"
TIMESTAMP=$(date +%s)
MESSAGE="${METHOD}${PATH}${TIMESTAMP}"
echo "Message to sign: $MESSAGE"
```

#### 3. Key Format and Encoding Issues

```bash
# Check if private key is properly formatted
cat secrets/.operator-config.json | jq -r '.secret_key' | wc -c
# Should be 64 characters (32 bytes hex-encoded)

# Verify key is valid hex
cat secrets/.operator-config.json | jq -r '.secret_key' | grep -E '^[0-9a-fA-F]{64}$'

# Check corresponding address
cat secrets/.operator-config.json | jq -r '.address'
# Should be 40 characters (20 bytes hex-encoded)
```

#### 4. Signature Generation Issues

```bash
# Test signature generation with a known message
# This requires a tool that can generate ECDSA signatures

# Example using Node.js (if available):
node -e "
const crypto = require('crypto');
const secp256k1 = require('secp256k1');

const privateKey = Buffer.from('YOUR_PRIVATE_KEY', 'hex');
const message = 'GET/v1/account/list1640995200';
const messageHash = crypto.createHash('sha256').update(message).digest();

const signature = secp256k1.ecdsaSign(messageHash, privateKey);
console.log('Signature:', signature.signature.toString('hex'));
console.log('Recovery ID:', signature.recid);
"
```

### Account and Address Issues

**Symptoms**: "Account not found", "Address not registered", authentication succeeds but authorization fails

**Diagnosis**:
```bash
# Check if account exists in form-state
ACCOUNT_ADDRESS="0x1234..."  # Your account address
curl http://localhost:3004/v1/account/${ACCOUNT_ADDRESS}/get | jq .

# Check account privileges
curl http://localhost:3004/v1/account/${ACCOUNT_ADDRESS}/is_global_admin

# List all accounts to see what exists
curl http://localhost:3004/v1/account/list | jq .

# Check if address matches what's in config
CONFIG_ADDRESS=$(cat secrets/.operator-config.json | jq -r '.address')
echo "Config address: $CONFIG_ADDRESS"
```

**Solutions**:

#### 1. Account Not Created

```bash
# Create account if it doesn't exist
curl -X POST http://localhost:3004/v1/account/create \
  -H "Content-Type: application/json" \
  -H "X-Signature: <signature>" \
  -H "X-Recovery-Id: <recovery_id>" \
  -H "X-Message: <message>" \
  -d '{
    "address": "'$ACCOUNT_ADDRESS'",
    "is_global_admin": false
  }'
```

#### 2. Address Mismatch Between Config and Derived Address

```bash
# Regenerate address from private key to verify consistency
# This requires a tool that can derive addresses from private keys

# Check if public key in config matches private key
PRIVATE_KEY=$(cat secrets/.operator-config.json | jq -r '.secret_key')
PUBLIC_KEY=$(cat secrets/.operator-config.json | jq -r '.public_key')
ADDRESS=$(cat secrets/.operator-config.json | jq -r '.address')

echo "Private key: $PRIVATE_KEY"
echo "Public key: $PUBLIC_KEY"  
echo "Address: $ADDRESS"

# If there's a mismatch, regenerate config
form-config-wizard wizard
```

#### 3. Case Sensitivity Issues

```bash
# Ensure address is in correct format (lowercase hex)
CORRECT_ADDRESS=$(echo "$ACCOUNT_ADDRESS" | tr '[:upper:]' '[:lower:]' | sed 's/^0x//')
echo "Corrected address: $CORRECT_ADDRESS"

# Update config if needed
jq --arg addr "$CORRECT_ADDRESS" '.address = $addr' secrets/.operator-config.json > tmp.json
mv tmp.json secrets/.operator-config.json
```

### Admin Access and Privilege Issues

**Symptoms**: "Not authorized for admin operation", admin endpoints return 403/401, can't perform admin tasks

**Diagnosis**:
```bash
# Check admin status of current account
ADMIN_ADDRESS=$(cat secrets/.operator-config.json | jq -r '.address')
curl http://localhost:3004/v1/account/${ADMIN_ADDRESS}/is_global_admin

# Check initial admin configuration
cat secrets/.operator-config.json | jq '.initial_admin_public_key'

# Check if admin account was properly created during bootstrap
docker-compose logs form-state | grep -i "admin account\|ensure_admin"

# Test admin endpoint access
curl -X GET http://localhost:3004/v1/node/create \
  -H "X-Signature: <signature>" \
  -H "X-Recovery-Id: <recovery_id>" \
  -H "X-Message: <message>"
```

**Solutions**:

#### 1. Admin Account Not Properly Initialized

```bash
# Check if bootstrap process created admin account
curl http://localhost:3004/v1/account/list | jq '.[] | select(.is_global_admin == true)'

# If no admin accounts exist, manually ensure admin account
ADMIN_KEY=$(cat secrets/.operator-config.json | jq -r '.initial_admin_public_key // .address')
curl -X POST http://localhost:3004/bootstrap/ensure_admin_account \
  -H "Content-Type: application/json" \
  -d '{"admin_public_key": "'$ADMIN_KEY'"}'
```

#### 2. Wrong Admin Key Configuration

```bash
# Check what admin key is configured
cat secrets/.operator-config.json | jq '.initial_admin_public_key'

# If null or wrong, update configuration
CURRENT_ADDRESS=$(cat secrets/.operator-config.json | jq -r '.address')
jq --arg admin "$CURRENT_ADDRESS" '.initial_admin_public_key = $admin' \
  secrets/.operator-config.json > tmp.json
mv tmp.json secrets/.operator-config.json

# Restart services to pick up new config
docker-compose restart form-state
```

#### 3. Admin Privileges Not Granted

```bash
# Manually grant admin privileges (requires existing admin or localhost access)
ACCOUNT_TO_PROMOTE="0x1234..."
curl -X POST http://localhost:3004/v1/account/update \
  -H "Content-Type: application/json" \
  -d '{
    "address": "'$ACCOUNT_TO_PROMOTE'",
    "is_global_admin": true
  }'
```

### Key Generation and Management Issues

**Symptoms**: "Invalid private key", key generation fails, corrupted keys

**Diagnosis**:
```bash
# Check key file integrity
ls -la secrets/.operator-config.json
file secrets/.operator-config.json

# Validate JSON structure
cat secrets/.operator-config.json | jq . > /dev/null && echo "Valid JSON" || echo "Invalid JSON"

# Check key lengths and format
cat secrets/.operator-config.json | jq -r '.secret_key' | wc -c  # Should be 65 (64 + newline)
cat secrets/.operator-config.json | jq -r '.public_key' | wc -c  # Should be 131 (130 + newline)
cat secrets/.operator-config.json | jq -r '.address' | wc -c    # Should be 41 (40 + newline)

# Check for null or empty values
cat secrets/.operator-config.json | jq 'to_entries[] | select(.value == null or .value == "")'
```

**Solutions**:

#### 1. Regenerate Corrupted Keys

```bash
# Backup existing config
cp secrets/.operator-config.json secrets/.operator-config.json.backup

# Generate new keys using form-config-wizard
form-config-wizard wizard

# Or manually regenerate just the keys (if other config is good)
# This requires a key generation tool
```

#### 2. Key File Permissions Issues

```bash
# Check and fix file permissions
ls -la secrets/.operator-config.json
chmod 600 secrets/.operator-config.json
chown $USER:$USER secrets/.operator-config.json

# Ensure secrets directory exists and has correct permissions
mkdir -p secrets
chmod 700 secrets
```

#### 3. Key Backup and Recovery

```bash
# Create secure backup of keys
cp secrets/.operator-config.json secrets/.operator-config.json.$(date +%Y%m%d_%H%M%S)

# Encrypt backup for storage
gpg -c secrets/.operator-config.json.$(date +%Y%m%d_%H%M%S)

# Restore from backup
cp secrets/.operator-config.json.backup secrets/.operator-config.json
docker-compose restart form-state
```

### Clock Synchronization and Timestamp Issues

**Symptoms**: Authentication works intermittently, "timestamp too old/new" errors

**Diagnosis**:
```bash
# Check system time on all nodes
date
timedatectl status

# Check time synchronization service
systemctl status systemd-timesyncd
# OR
systemctl status ntp
# OR  
systemctl status chrony

# Check time difference between nodes
for node in node1 node2 node3; do
  echo "=== $node ==="
  ssh $node date
done

# Check if timestamps in requests are reasonable
docker-compose logs form-state | grep -i "timestamp\|time"
```

**Solutions**:

#### 1. Synchronize System Time

```bash
# Enable time synchronization
sudo timedatectl set-ntp true

# Force immediate sync
sudo ntpdate -s time.nist.gov
# OR
sudo chrony sources -v

# Check sync status
timedatectl status | grep "System clock synchronized"
```

#### 2. Configure Automatic Time Sync

```bash
# Install and configure NTP
sudo apt install ntp
sudo systemctl enable ntp
sudo systemctl start ntp

# Or use systemd-timesyncd
sudo systemctl enable systemd-timesyncd
sudo systemctl start systemd-timesyncd

# Configure timezone if needed
sudo timedatectl set-timezone UTC
```

#### 3. Handle Time Skew in Requests

```bash
# If nodes have slight time differences, adjust timestamp tolerance
# This may require configuration changes in form-state

# Check current timestamp tolerance in logs
docker-compose logs form-state | grep -i "timestamp.*tolerance\|time.*window"
```

### Multi-Node Authentication Issues

**Symptoms**: Authentication works on one node but fails on others, inconsistent auth behavior

**Diagnosis**:
```bash
# Check if all nodes have consistent account state
for node in node1 node2 node3; do
  echo "=== $node ==="
  curl http://$node:3004/v1/account/list | jq 'length'
  curl http://$node:3004/v1/account/$ACCOUNT_ADDRESS/get
done

# Check CRDT synchronization for accounts
docker-compose logs form-state | grep -i "account.*sync\|account.*gossip"

# Test authentication on each node
for node in node1 node2 node3; do
  echo "=== Testing auth on $node ==="
  curl -X GET http://$node:3004/v1/account/list \
    -H "X-Signature: <signature>" \
    -H "X-Recovery-Id: <recovery_id>" \
    -H "X-Message: <message>"
done
```

**Solutions**:

#### 1. Force Account State Synchronization

```bash
# Restart form-state on problematic nodes to trigger resync
docker-compose restart form-state

# Check if accounts propagate after restart
sleep 30
curl http://localhost:3004/v1/account/list | jq 'length'
```

#### 2. Manually Propagate Account Information

```bash
# Create account on each node if missing
for node in node1 node2 node3; do
  curl -X POST http://$node:3004/v1/account/create \
    -H "Content-Type: application/json" \
    -d '{
      "address": "'$ACCOUNT_ADDRESS'",
      "is_global_admin": false
    }'
done
```

### Development vs Production Authentication

**Symptoms**: Authentication behaves differently in different environments

**Diagnosis**:
```bash
# Check if running in devnet mode
docker-compose logs form-state | grep -i "devnet\|production"

# Check environment variables
docker-compose config | grep -i "allow_internal\|devnet"

# Check if localhost bypass is active
docker-compose logs form-state | grep -i "localhost.*bypass\|localhost.*auth"
```

**Solutions**:

#### 1. Configure Development Environment

```bash
# For development, allow internal endpoints without auth
echo "ALLOW_INTERNAL_ENDPOINTS=true" >> .env
docker-compose restart form-state

# Test internal access
curl http://localhost:3004/v1/account/list  # Should work without auth headers
```

#### 2. Configure Production Environment

```bash
# For production, ensure strict authentication
echo "ALLOW_INTERNAL_ENDPOINTS=false" >> .env
docker-compose restart form-state

# All requests must include proper auth headers
```

### Advanced Authentication Debugging

#### 1. Signature Verification Testing

```bash
# Create a test script to verify signature generation
cat > test_signature.py << 'EOF'
#!/usr/bin/env python3
import hashlib
import ecdsa
import binascii
import sys

def test_signature(private_key_hex, message):
    # Convert hex private key to bytes
    private_key_bytes = binascii.unhexlify(private_key_hex)
    
    # Create signing key
    signing_key = ecdsa.SigningKey.from_string(private_key_bytes, curve=ecdsa.SECP256k1)
    
    # Hash the message
    message_hash = hashlib.sha256(message.encode()).digest()
    
    # Sign the hash
    signature = signing_key.sign_digest(message_hash, sigencode=ecdsa.util.sigencode_der)
    
    print(f"Message: {message}")
    print(f"Message hash: {message_hash.hex()}")
    print(f"Signature: {signature.hex()}")
    
    # Verify signature
    verifying_key = signing_key.get_verifying_key()
    try:
        verifying_key.verify_digest(signature, message_hash, sigdecode=ecdsa.util.sigdecode_der)
        print("Signature verification: SUCCESS")
    except:
        print("Signature verification: FAILED")

if __name__ == "__main__":
    if len(sys.argv) != 3:
        print("Usage: python3 test_signature.py <private_key_hex> <message>")
        sys.exit(1)
    
    test_signature(sys.argv[1], sys.argv[2])
EOF

chmod +x test_signature.py

# Test signature generation
PRIVATE_KEY=$(cat secrets/.operator-config.json | jq -r '.secret_key')
MESSAGE="GET/v1/account/list$(date +%s)"
python3 test_signature.py "$PRIVATE_KEY" "$MESSAGE"
```

#### 2. Authentication Flow Tracing

```bash
# Enable detailed auth logging
echo "RUST_LOG=form_state::auth=debug" >> .env
docker-compose restart form-state

# Make a test request and trace the auth flow
curl -X GET http://localhost:3004/v1/account/list \
  -H "X-Signature: <signature>" \
  -H "X-Recovery-Id: <recovery_id>" \
  -H "X-Message: <message>" \
  -v

# Check detailed auth logs
docker-compose logs form-state | grep -i "auth\|signature\|recover"
```

#### 3. Comprehensive Authentication Diagnostic Script

```bash
#!/bin/bash
# save as auth-diagnostic.sh

echo "=== Formation Authentication Diagnostic $(date) ==="

CONFIG_FILE="secrets/.operator-config.json"

if [[ ! -f "$CONFIG_FILE" ]]; then
    echo "❌ Config file not found: $CONFIG_FILE"
    exit 1
fi

echo "--- Configuration Check ---"
if jq . "$CONFIG_FILE" >/dev/null 2>&1; then
    echo "✓ Config file is valid JSON"
else
    echo "❌ Config file has invalid JSON"
    exit 1
fi

# Check required fields
REQUIRED_FIELDS=("secret_key" "public_key" "address")
for field in "${REQUIRED_FIELDS[@]}"; do
    value=$(jq -r ".$field" "$CONFIG_FILE")
    if [[ "$value" != "null" && -n "$value" ]]; then
        echo "✓ $field: present"
    else
        echo "❌ $field: missing or null"
    fi
done

echo "--- Key Format Check ---"
SECRET_KEY=$(jq -r '.secret_key' "$CONFIG_FILE")
PUBLIC_KEY=$(jq -r '.public_key' "$CONFIG_FILE")
ADDRESS=$(jq -r '.address' "$CONFIG_FILE")

if [[ ${#SECRET_KEY} -eq 64 ]]; then
    echo "✓ Secret key length correct (64 chars)"
else
    echo "❌ Secret key length incorrect (${#SECRET_KEY} chars, expected 64)"
fi

if [[ ${#PUBLIC_KEY} -eq 130 ]]; then
    echo "✓ Public key length correct (130 chars)"
else
    echo "❌ Public key length incorrect (${#PUBLIC_KEY} chars, expected 130)"
fi

if [[ ${#ADDRESS} -eq 40 ]]; then
    echo "✓ Address length correct (40 chars)"
else
    echo "❌ Address length incorrect (${#ADDRESS} chars, expected 40)"
fi

echo "--- Service Connectivity ---"
if curl -s http://localhost:3004/health >/dev/null; then
    echo "✓ form-state service reachable"
else
    echo "❌ form-state service unreachable"
fi

echo "--- Account Status ---"
ACCOUNT_RESPONSE=$(curl -s http://localhost:3004/v1/account/${ADDRESS}/get)
if echo "$ACCOUNT_RESPONSE" | jq . >/dev/null 2>&1; then
    echo "✓ Account exists in form-state"
    IS_ADMIN=$(echo "$ACCOUNT_RESPONSE" | jq -r '.is_global_admin')
    echo "  Admin status: $IS_ADMIN"
else
    echo "❌ Account not found or error retrieving account"
fi

echo "--- Time Synchronization ---"
if command -v timedatectl >/dev/null; then
    SYNC_STATUS=$(timedatectl status | grep "System clock synchronized" | awk '{print $4}')
    if [[ "$SYNC_STATUS" == "yes" ]]; then
        echo "✓ System clock synchronized"
    else
        echo "⚠ System clock not synchronized"
    fi
else
    echo "⚠ Cannot check time sync (timedatectl not available)"
fi

echo "--- Recent Auth Errors ---"
AUTH_ERRORS=$(docker-compose logs form-state 2>/dev/null | grep -i "unauthorized\|signature.*fail\|auth.*error" | tail -5)
if [[ -n "$AUTH_ERRORS" ]]; then
    echo "⚠ Recent authentication errors found:"
    echo "$AUTH_ERRORS"
else
    echo "✓ No recent authentication errors"
fi

echo "========================"
```

### Authentication Recovery Procedures

#### 1. Complete Authentication Reset

```bash
# Stop services
docker-compose down

# Backup current config
cp secrets/.operator-config.json secrets/.operator-config.json.backup

# Generate new keys and config
form-config-wizard wizard

# Clear any cached authentication state
docker volume rm formation_state-data

# Restart services
docker-compose up -d

# Wait for services to start
sleep 30

# Verify new authentication works
curl http://localhost:3004/v1/account/list
```

#### 2. Key Rotation Procedure

```bash
# Generate new keys while preserving other config
OLD_CONFIG=$(cat secrets/.operator-config.json)
form-config-wizard wizard

# Extract new keys
NEW_SECRET=$(jq -r '.secret_key' secrets/.operator-config.json)
NEW_PUBLIC=$(jq -r '.public_key' secrets/.operator-config.json)
NEW_ADDRESS=$(jq -r '.address' secrets/.operator-config.json)

# Update account in form-state with new address
curl -X POST http://localhost:3004/v1/account/create \
  -H "Content-Type: application/json" \
  -d '{
    "address": "'$NEW_ADDRESS'",
    "is_global_admin": true
  }'

# Test new authentication
# ... make authenticated request with new keys
```

#### 3. Emergency Admin Access

```bash
# If locked out of admin access, use localhost bypass
curl -X POST http://localhost:3004/bootstrap/ensure_admin_account \
  -H "Content-Type: application/json" \
  -d '{"admin_public_key": "'$(cat secrets/.operator-config.json | jq -r '.address')'"}'

# Verify admin access restored
curl http://localhost:3004/v1/account/$(cat secrets/.operator-config.json | jq -r '.address')/is_global_admin
```

## Resource & Performance Issues

### High CPU/Memory Usage

**Symptoms**: System sluggish, containers using excessive resources

**Diagnosis**:
```bash
# Monitor resource usage
docker stats
htop
iotop
```

**Solutions**:

#### 1. Reduce Log Verbosity

```bash
# Update .env file
echo "RUST_LOG=warn" >> .env
echo "FORMNET_LOG_LEVEL=warn" >> .env

# Restart services
docker-compose restart
```

#### 2. Adjust Resource Limits

```bash
# Add to docker-compose.yml under services
deploy:
  resources:
    limits:
      cpus: '2.0'
      memory: 4G
```

### Disk Space Issues

**Symptoms**: Services failing, "no space left on device" errors

**Diagnosis**:
```bash
# Check disk usage
df -h
du -sh /var/lib/docker

# Check large log files
find /var/log -size +100M -ls
docker system df
```

**Solutions**:

#### 1. Clean Docker Resources

```bash
# Remove unused containers, networks, images
docker system prune -a

# Clean up volumes (CAUTION: may remove data)
docker volume prune
```

#### 2. Configure Log Rotation

```bash
# Add to docker-compose.yml for each service
logging:
  driver: "json-file"
  options:
    max-size: "10m"
    max-file: "3"
```

## Database & State Issues

### Corrupted Database

**Symptoms**: Services fail to start, database errors in logs

**Diagnosis**:
```bash
# Check database files
ls -la /var/lib/docker/volumes/formation_state-data/_data/

# Check for database errors in logs
docker-compose logs form-state | grep -i "database\|sqlite\|corruption"
```

**Solutions**:

#### 1. Restore from Backup

```bash
# Stop services
docker-compose down

# Restore database from backup
sudo cp /path/to/backup/formation.db /var/lib/docker/volumes/formation_state-data/_data/

# Start services
docker-compose up -d
```

#### 2. Reinitialize Database (Data Loss)

```bash
# Stop services
docker-compose down

# Remove corrupted data
docker volume rm formation_state-data

# Start services (will reinitialize)
docker-compose up -d
```

### State Synchronization Issues

**Symptoms**: Nodes have different state, data not propagating

**Diagnosis**:
```bash
# Compare state between nodes
curl http://node1:3004/v1/network/peers
curl http://node2:3004/v1/network/peers

# Check gossip logs
docker-compose logs form-state | grep -i gossip
```

**Solutions**:

#### 1. Restart State Sync

```bash
# Restart form-state on problematic node
docker-compose restart form-state
```

#### 2. Force State Refresh

```bash
# Clear local state and resync (if implemented)
# This is deployment-specific
```

## Log Analysis & Debugging

Formation generates extensive logs across multiple services and layers. This section provides comprehensive guidance for locating, analyzing, and interpreting logs to diagnose issues effectively.

### Log Architecture Overview

Formation's logging system has multiple layers:
1. **Application Logs**: Service-specific logs from each Formation component
2. **Container Logs**: Docker container stdout/stderr captured by Docker
3. **System Logs**: Host system logs including kernel, systemd, and Docker daemon
4. **Network Logs**: WireGuard, iptables, and network interface logs
5. **Database Logs**: SQLite and CRDT operation logs

### Service Log Locations

#### Docker Compose Deployment

```bash
# Primary Formation services
docker-compose logs form-state      # Central datastore and API
docker-compose logs form-net        # Network management (formnet)
docker-compose logs form-vmm        # Virtual machine management
docker-compose logs form-pack       # Container building service
docker-compose logs form-dns        # DNS and service discovery
docker-compose logs form-p2p        # Message queue and gossip

# Follow live logs
docker-compose logs -f form-state
docker-compose logs -f --tail=100 form-state

# Filter by time
docker-compose logs --since="2024-01-01T00:00:00" form-state
docker-compose logs --until="2024-01-01T23:59:59" form-state

# All services combined
docker-compose logs --tail=50
```

#### Individual Container Logs

```bash
# List all Formation containers
docker ps --filter "name=formation"

# View specific container logs
docker logs formation-form-state-1
docker logs formation-form-net-1
docker logs formation-form-vmm-1

# Follow container logs with timestamps
docker logs -f -t formation-form-state-1

# Export logs to file
docker logs formation-form-state-1 > form-state.log 2>&1
```

#### Host System Log Locations

```bash
# Docker daemon logs
sudo journalctl -u docker.service
sudo journalctl -u docker.service --since="1 hour ago"

# System logs
sudo journalctl -f                    # Follow all system logs
sudo journalctl --since="today"      # Today's logs
sudo journalctl -p err               # Error level and above

# Network-related system logs
sudo journalctl -u systemd-networkd
sudo journalctl -u NetworkManager
sudo journalctl -k | grep -i wireguard  # Kernel logs for WireGuard

# Formation-specific system logs
sudo journalctl | grep -i formation
sudo journalctl | grep -i formnet
```

#### Log File Locations (if using file logging)

```bash
# Default Docker log locations
/var/lib/docker/containers/*/container-name-json.log

# Custom log locations (if configured)
/var/log/formation/
├── form-state.log
├── form-net.log
├── form-vmm.log
├── form-pack.log
└── form-dns.log

# WireGuard logs (if enabled)
/var/log/wireguard/
└── formnet.log

# System logs
/var/log/syslog
/var/log/kern.log
/var/log/daemon.log
```

### Log Level Configuration

#### Rust Application Logs (RUST_LOG)

```bash
# Set log levels via environment variables
echo "RUST_LOG=debug" >> .env                    # All components debug
echo "RUST_LOG=form_state=debug" >> .env         # Only form-state debug
echo "RUST_LOG=form_state::auth=trace" >> .env   # Specific module trace

# Multiple component levels
echo "RUST_LOG=form_state=debug,form_net=info,form_vmm=warn" >> .env

# Apply changes
docker-compose restart

# Common log levels (most to least verbose):
# trace, debug, info, warn, error
```

#### Service-Specific Log Configuration

```bash
# WireGuard/formnet logging
echo "FORMNET_LOG_LEVEL=debug" >> .env

# VMM service logging
echo "VMM_LOG_LEVEL=debug" >> .env

# DNS service logging
echo "DNS_LOG_LEVEL=debug" >> .env

# Enable specific debugging features
echo "ENABLE_AUTH_DEBUG=true" >> .env
echo "ENABLE_NETWORK_DEBUG=true" >> .env
echo "ENABLE_CRDT_DEBUG=true" >> .env
```

### Critical Log Patterns and Analysis

#### Startup and Health Patterns

**Successful Service Startup**:
```bash
# form-state startup success
grep -i "running.*api.*server\|datastore.*server\|health.*endpoint" docker-compose logs form-state

# Expected patterns:
# "Running datastore server with API and queue reader at 0.0.0.0:3004"
# "Health check endpoint available"
# "Built data store, running..."
```

**Service Health Check Patterns**:
```bash
# Health endpoint responses
docker-compose logs form-state | grep -i "health"
docker-compose logs form-vmm | grep -i "health"

# Expected patterns:
# "Health check successful"
# "Service healthy"
```

#### Network and Connectivity Patterns

**WireGuard Interface Creation**:
```bash
# Successful formnet interface creation
docker-compose logs form-net | grep -i "wireguard\|interface\|formnet"

# Expected patterns:
# "WireGuard interface created successfully"
# "formnet interface is up"
# "Network initialized with CIDR"
```

**Peer Discovery and Connection**:
```bash
# Peer connection events
docker-compose logs form-net | grep -i "peer\|bootstrap\|join"

# Expected patterns:
# "Successfully joined with IP"
# "Peer connected"
# "Bootstrap node discovered"
# "Network initialized"
```

**CRDT Gossip and State Sync**:
```bash
# State synchronization logs
docker-compose logs form-state | grep -i "gossip\|sync\|crdt\|merge"

# Expected patterns (devnet mode):
# "devnet mode: PeerOp applied locally. Gossiping directly"
# "Gossip operation successful"

# Expected patterns (production mode):
# "production mode: Queuing PeerOp"
# "CRDT operation applied successfully"
```

#### Authentication and Authorization Patterns

**ECDSA Authentication Success**:
```bash
# Authentication flow logs
docker-compose logs form-state | grep -i "auth\|signature\|recover\|ecdsa"

# Expected patterns:
# "ECDSA signature verified successfully"
# "Recovered address from signature: 0x..."
# "Authentication successful"
# "Admin account ensured"
```

**Admin Operations**:
```bash
# Admin access logs
docker-compose logs form-state | grep -i "admin\|global.*admin\|ensure.*admin"

# Expected patterns:
# "Admin account ensured successfully"
# "Global admin access granted"
# "Admin operation authorized"
```

#### Task Management and Proof of Claim

**Task Creation and Assignment**:
```bash
# Task lifecycle logs
docker-compose logs form-state | grep -i "task\|poc\|proof.*claim\|dispatch"

# Expected patterns:
# "Task created with ID"
# "PoC assessment completed"
# "Task assigned to responsible nodes"
# "Task dispatched successfully"
```

**Build and VM Operations**:
```bash
# form-pack build operations
docker-compose logs form-pack | grep -i "build\|pack\|formfile"

# form-vmm VM operations  
docker-compose logs form-vmm | grep -i "vm\|instance\|create\|boot"

# Expected patterns:
# "Build request received"
# "VM instance created successfully"
# "Task completed successfully"
```

### Error Pattern Analysis

#### Critical Error Patterns

**Authentication Failures**:
```bash
# Authentication error patterns
docker-compose logs form-state | grep -i "unauthorized\|signature.*fail\|invalid.*signature\|auth.*error"

# Common error patterns:
# "Authentication failed: Missing signature"
# "Signature verification failed"
# "Invalid signature format"
# "Unauthorized: Address not found"
# "Admin access denied"
```

**Network Connectivity Errors**:
```bash
# Network error patterns
docker-compose logs | grep -i "connection.*refused\|timeout\|unreachable\|network.*error"

# Common error patterns:
# "Connection refused"
# "Network unreachable"
# "Timeout waiting for response"
# "Failed to connect to bootstrap node"
# "WireGuard interface creation failed"
```

**CRDT and State Sync Errors**:
```bash
# State synchronization errors
docker-compose logs form-state | grep -i "gossip.*fail\|sync.*error\|crdt.*error\|merge.*fail"

# Common error patterns:
# "Gossip operation failed"
# "CRDT merge conflict"
# "State synchronization timeout"
# "Failed to apply operation"
```

**Resource and Performance Errors**:
```bash
# Resource exhaustion patterns
docker-compose logs | grep -i "out.*memory\|disk.*full\|resource.*exhausted\|too.*many"

# Common error patterns:
# "Out of memory"
# "Disk space exhausted"
# "Too many open files"
# "Resource temporarily unavailable"
```

#### Service-Specific Error Patterns

**form-state Errors**:
```bash
# Database and datastore errors
docker-compose logs form-state | grep -i "database\|sqlite\|corruption\|lock"

# API and HTTP errors
docker-compose logs form-state | grep -i "bind.*error\|port.*use\|http.*error"

# Common patterns:
# "Database is locked"
# "SQLite corruption detected"
# "Failed to bind to port"
# "HTTP request processing error"
```

**form-net/WireGuard Errors**:
```bash
# WireGuard specific errors
docker-compose logs form-net | grep -i "wireguard\|wg.*error\|interface.*fail"

# Network configuration errors
docker-compose logs form-net | grep -i "cidr\|ip.*conflict\|route.*error"

# Common patterns:
# "WireGuard module not loaded"
# "Interface already exists"
# "IP address conflict"
# "Route addition failed"
```

**form-vmm Errors**:
```bash
# VM creation and management errors
docker-compose logs form-vmm | grep -i "vm.*error\|kvm\|hypervisor\|create.*fail"

# Resource allocation errors
docker-compose logs form-vmm | grep -i "memory\|cpu\|disk.*error"

# Common patterns:
# "KVM device not accessible"
# "Insufficient memory for VM"
# "VM creation failed"
# "Hypervisor error"
```

### Advanced Log Analysis Techniques

#### Log Correlation and Timeline Analysis

```bash
# Correlate logs across services with timestamps
docker-compose logs -t | sort

# Extract logs for specific time window
START_TIME="2024-01-01T10:00:00"
END_TIME="2024-01-01T11:00:00"
docker-compose logs --since="$START_TIME" --until="$END_TIME" | sort

# Follow logs from multiple services simultaneously
docker-compose logs -f form-state form-net form-vmm
```

#### Log Filtering and Search

```bash
# Advanced grep patterns for log analysis
docker-compose logs form-state | grep -E "(ERROR|WARN|FATAL)" | tail -20

# Extract specific transaction/request flows
REQUEST_ID="req_12345"
docker-compose logs | grep "$REQUEST_ID" | sort

# Find logs related to specific addresses/nodes
NODE_ADDRESS="0x1234abcd..."
docker-compose logs | grep -i "$NODE_ADDRESS"

# Network-specific log filtering
docker-compose logs | grep -E "(formnet|wireguard|peer|gossip)" | tail -50
```

#### Performance and Timing Analysis

```bash
# Extract timing information from logs
docker-compose logs form-state | grep -i "took\|duration\|elapsed\|ms\|seconds"

# Find slow operations
docker-compose logs | grep -E "([0-9]+ms|[0-9]+\.[0-9]+s)" | grep -E "([5-9][0-9][0-9]ms|[0-9]+\.[5-9]s)"

# Database operation timing
docker-compose logs form-state | grep -i "database.*took\|query.*duration"
```

### Log Analysis Scripts and Tools

#### Comprehensive Log Analysis Script

```bash
#!/bin/bash
# save as analyze-formation-logs.sh

echo "=== Formation Log Analysis $(date) ==="

LOG_DIR="./formation-logs-$(date +%Y%m%d_%H%M%S)"
mkdir -p "$LOG_DIR"

echo "--- Collecting Logs ---"
# Export all service logs
docker-compose logs --no-color form-state > "$LOG_DIR/form-state.log"
docker-compose logs --no-color form-net > "$LOG_DIR/form-net.log"
docker-compose logs --no-color form-vmm > "$LOG_DIR/form-vmm.log"
docker-compose logs --no-color form-pack > "$LOG_DIR/form-pack.log"
docker-compose logs --no-color form-dns > "$LOG_DIR/form-dns.log"

# System logs
sudo journalctl -u docker.service --no-pager > "$LOG_DIR/docker.log"
sudo journalctl --since="1 hour ago" --no-pager > "$LOG_DIR/system.log"

echo "--- Error Analysis ---"
echo "=== Critical Errors ===" > "$LOG_DIR/error-summary.txt"
grep -i "error\|fail\|panic\|fatal" "$LOG_DIR"/*.log | head -20 >> "$LOG_DIR/error-summary.txt"

echo "=== Authentication Errors ===" >> "$LOG_DIR/error-summary.txt"
grep -i "unauthorized\|signature.*fail\|auth.*error" "$LOG_DIR"/*.log >> "$LOG_DIR/error-summary.txt"

echo "=== Network Errors ===" >> "$LOG_DIR/error-summary.txt"
grep -i "connection.*refused\|timeout\|unreachable" "$LOG_DIR"/*.log >> "$LOG_DIR/error-summary.txt"

echo "--- Service Health Analysis ---"
echo "=== Service Startup ===" > "$LOG_DIR/health-summary.txt"
grep -i "running.*server\|service.*started\|health.*endpoint" "$LOG_DIR"/*.log >> "$LOG_DIR/health-summary.txt"

echo "=== Recent Health Checks ===" >> "$LOG_DIR/health-summary.txt"
grep -i "health.*check\|ping.*response" "$LOG_DIR"/*.log | tail -10 >> "$LOG_DIR/health-summary.txt"

echo "--- Performance Analysis ---"
echo "=== Slow Operations ===" > "$LOG_DIR/performance-summary.txt"
grep -E "([5-9][0-9][0-9]ms|[0-9]+\.[5-9]s)" "$LOG_DIR"/*.log >> "$LOG_DIR/performance-summary.txt"

echo "--- Network Analysis ---"
echo "=== Network Events ===" > "$LOG_DIR/network-summary.txt"
grep -i "peer.*connect\|gossip\|wireguard\|formnet" "$LOG_DIR"/*.log | tail -20 >> "$LOG_DIR/network-summary.txt"

echo "--- Summary ---"
echo "Logs collected in: $LOG_DIR"
echo "Error count: $(grep -c -i "error\|fail" "$LOG_DIR"/*.log | awk -F: '{sum+=$2} END {print sum}')"
echo "Warning count: $(grep -c -i "warn" "$LOG_DIR"/*.log | awk -F: '{sum+=$2} END {print sum}')"

echo "=== Top Error Messages ==="
grep -i "error\|fail" "$LOG_DIR"/*.log | cut -d' ' -f4- | sort | uniq -c | sort -nr | head -10

echo "========================"
```

#### Real-time Log Monitoring Script

```bash
#!/bin/bash
# save as monitor-formation-logs.sh

echo "=== Formation Real-time Log Monitor ==="
echo "Press Ctrl+C to stop"

# Create named pipes for each service
mkfifo /tmp/form-state-pipe /tmp/form-net-pipe /tmp/form-vmm-pipe 2>/dev/null

# Start log followers in background
docker-compose logs -f form-state > /tmp/form-state-pipe &
docker-compose logs -f form-net > /tmp/form-net-pipe &
docker-compose logs -f form-vmm > /tmp/form-vmm-pipe &

# Monitor for critical patterns
{
    while read line; do
        echo "[STATE] $line"
        # Alert on critical errors
        if echo "$line" | grep -qi "error\|fail\|panic"; then
            echo "🚨 CRITICAL: $line" | tee -a /tmp/formation-alerts.log
        fi
    done < /tmp/form-state-pipe
} &

{
    while read line; do
        echo "[NET] $line"
        if echo "$line" | grep -qi "connection.*refused\|timeout"; then
            echo "🌐 NETWORK ISSUE: $line" | tee -a /tmp/formation-alerts.log
        fi
    done < /tmp/form-net-pipe
} &

{
    while read line; do
        echo "[VMM] $line"
        if echo "$line" | grep -qi "vm.*error\|kvm.*error"; then
            echo "🖥️ VM ISSUE: $line" | tee -a /tmp/formation-alerts.log
        fi
    done < /tmp/form-vmm-pipe
} &

# Wait for interrupt
wait
```

#### Log Rotation and Management

```bash
# Configure log rotation for Formation
cat > /etc/logrotate.d/formation << EOF
/var/log/formation/*.log {
    daily
    rotate 7
    compress
    delaycompress
    missingok
    notifempty
    create 644 root root
    postrotate
        docker-compose restart form-state form-net form-vmm form-pack form-dns
    endscript
}
EOF

# Manual log cleanup
find /var/lib/docker/containers -name "*.log" -mtime +7 -delete

# Docker log size management
cat >> docker-compose.yml << EOF
# Add to each service:
logging:
  driver: "json-file"
  options:
    max-size: "50m"
    max-file: "5"
EOF
```

### Debugging Workflows

#### Issue Investigation Workflow

```bash
# 1. Quick health check
docker-compose ps
curl -s http://localhost:3004/health | jq .

# 2. Recent error scan
docker-compose logs --tail=100 | grep -i "error\|fail" | tail -10

# 3. Service-specific investigation
SERVICE="form-state"  # or form-net, form-vmm, etc.
docker-compose logs --tail=200 $SERVICE | grep -i "error\|warn"

# 4. Timeline analysis
docker-compose logs --since="10 minutes ago" -t | sort

# 5. Correlation analysis
ISSUE_TIME="2024-01-01T10:30:00"
docker-compose logs --since="$ISSUE_TIME" --until="$(date -d "$ISSUE_TIME + 5 minutes" -Iseconds)" | sort
```

#### Performance Investigation Workflow

```bash
# 1. Resource usage check
docker stats --no-stream

# 2. Slow operation detection
docker-compose logs | grep -E "([0-9]+ms|[0-9]+\.[0-9]+s)" | grep -E "([5-9][0-9][0-9]ms|[0-9]+\.[5-9]s)"

# 3. Database performance
docker-compose logs form-state | grep -i "database\|sqlite" | grep -E "([0-9]+ms)"

# 4. Network performance
docker-compose logs form-net | grep -i "timeout\|slow\|delay"

# 5. Memory/CPU analysis
docker-compose logs | grep -i "memory\|cpu\|resource"
```

### Log-Based Alerting

#### Simple Alert Script

```bash
#!/bin/bash
# save as formation-log-alerts.sh

ALERT_LOG="/tmp/formation-alerts.log"
LAST_CHECK_FILE="/tmp/formation-last-check"

# Get timestamp of last check
if [[ -f "$LAST_CHECK_FILE" ]]; then
    LAST_CHECK=$(cat "$LAST_CHECK_FILE")
else
    LAST_CHECK=$(date -d "1 minute ago" -Iseconds)
fi

# Update last check timestamp
date -Iseconds > "$LAST_CHECK_FILE"

# Check for critical errors since last check
CRITICAL_ERRORS=$(docker-compose logs --since="$LAST_CHECK" | grep -i "error\|fail\|panic" | wc -l)

if [[ $CRITICAL_ERRORS -gt 0 ]]; then
    echo "$(date): $CRITICAL_ERRORS critical errors detected" >> "$ALERT_LOG"
    
    # Send notification (customize as needed)
    echo "Formation Alert: $CRITICAL_ERRORS critical errors detected" | \
        mail -s "Formation Alert" admin@example.com
    
    # Or send to Slack/Discord webhook
    # curl -X POST -H 'Content-type: application/json' \
    #   --data '{"text":"Formation Alert: '$CRITICAL_ERRORS' critical errors"}' \
    #   YOUR_WEBHOOK_URL
fi
```

### Log Retention and Archival

```bash
# Archive old logs
ARCHIVE_DIR="/var/backups/formation-logs"
mkdir -p "$ARCHIVE_DIR"

# Create daily archive
DATE=$(date +%Y%m%d)
tar -czf "$ARCHIVE_DIR/formation-logs-$DATE.tar.gz" \
    /var/lib/docker/containers/*/formation-*.log

# Clean up old archives (keep 30 days)
find "$ARCHIVE_DIR" -name "formation-logs-*.tar.gz" -mtime +30 -delete

# Export logs for external analysis
docker-compose logs --no-color > "formation-export-$(date +%Y%m%d_%H%M%S).log"
```

## Emergency Procedures

### Complete Service Reset

**When**: All else fails, need fresh start

```bash
# Stop all services
docker-compose down

# Remove all containers and volumes (DATA LOSS)
docker-compose down -v

# Clean up networks
docker network prune

# Restart services
docker-compose pull
docker-compose up -d
```

### Single Service Recovery

**When**: One service is problematic

```bash
# Restart specific service
docker-compose restart <service-name>

# Recreate specific service
docker-compose stop <service-name>
docker-compose rm <service-name>
docker-compose up -d <service-name>
```

### Network Recovery

**When**: Network connectivity broken

```bash
# Reset network interfaces
sudo ip link del formnet0
sudo ip link del br0

# Restart network setup
sudo bash scripts/validate-network-config.sh

# Restart networking services
docker-compose restart form-net form-dns
```

## Performance Monitoring

### Health Monitoring Commands

```bash
# Service health checks
watch -n 5 'curl -s http://localhost:3004/health | jq .status'

# Resource monitoring
watch -n 2 'docker stats --no-stream'

# Network connectivity
watch -n 10 'ping -c 1 172.20.0.1'
```

### Setting Up Monitoring

#### Basic Monitoring Script

```bash
#!/bin/bash
# save as monitor-formation.sh

echo "=== Formation Health Check $(date) ==="

# Check service health
services=("3004" "3002" "3003" "51820")
for port in "${services[@]}"; do
    if curl -s http://localhost:$port/health > /dev/null; then
        echo "✓ Port $port: healthy"
    else
        echo "✗ Port $port: unhealthy"
    fi
done

# Check network interfaces
if ip addr show formnet0 > /dev/null 2>&1; then
    echo "✓ Formnet interface: up"
else
    echo "✗ Formnet interface: down"
fi

# Check peers (if multi-node)
peer_count=$(curl -s http://localhost:3004/v1/network/peers | jq length 2>/dev/null || echo "0")
echo "Peers connected: $peer_count"

echo "========================"
```

### Automated Alerts

```bash
# Add to crontab for regular checks
# crontab -e
*/5 * * * * /path/to/monitor-formation.sh >> /var/log/formation-health.log 2>&1
```

## Getting Help

When troubleshooting doesn't resolve your issue:

1. **Gather diagnostic information**:
   ```bash
   # Create diagnostic report
   {
     echo "=== System Info ==="
     uname -a
     docker --version
     docker-compose --version
     
     echo "=== Service Status ==="
     docker-compose ps
     
     echo "=== Recent Logs ==="
     docker-compose logs --tail=100
     
     echo "=== Network Info ==="
     ip addr show
     sudo wg show
   } > formation-diagnostic.txt
   ```

2. **Check documentation** for similar issues
3. **Search GitHub issues** for known problems
4. **Report issues** with diagnostic information included

## Prevention Best Practices

- **Regular backups** of configuration and state data
- **Monitor disk space** and set up alerts
- **Keep services updated** to latest versions
- **Test recovery procedures** in non-production environments
- **Document custom configurations** and changes
- **Set up proper monitoring** and alerting
- **Regular health checks** and maintenance windows 