# Formation Operator Quick Start Guide

This guide provides two deployment options for running a Formation node: Docker Compose (recommended for quick setup) and building from source (for development and customization).

## Option 1: Docker Compose Deployment (Recommended)

This will get you running quickly using pre-built containers.

### Prerequisites

#### System Requirements

- **OS**: Ubuntu 22.04 LTS (or compatible Linux distribution)
- **CPU**: 4+ cores (8+ recommended)
- **RAM**: 8GB minimum (16GB+ recommended)
- **Storage**: 50GB+ free disk space
- **Network**: Public IP with port forwarding capability

#### Required Software

Install the following dependencies:

```bash
# Update system packages
sudo apt update && sudo apt upgrade -y

# Install essential tools
sudo apt install -y curl bridge-utils dnsmasq

# Install Docker
sudo apt install -y apt-transport-https ca-certificates curl software-properties-common
curl -fsSL https://download.docker.com/linux/ubuntu/gpg | sudo apt-key add -
sudo add-apt-repository "deb [arch=amd64] https://download.docker.com/linux/ubuntu $(lsb_release -cs) stable"
sudo apt update
sudo apt install -y docker-ce

# Add your user to docker group
sudo usermod -aG docker $USER

# Install Docker Compose
sudo curl -L "https://github.com/docker/compose/releases/latest/download/docker-compose-$(uname -s)-$(uname -m)" -o /usr/local/bin/docker-compose
sudo chmod +x /usr/local/bin/docker-compose

# Log out and back in for docker group changes to take effect
```

## Option 2: Build from Source Prerequisites

If you prefer to build Formation from source for development, customization, or to run the latest code, you'll need additional prerequisites.

### System Requirements for Source Builds

Building Formation requires more resources than running pre-built containers:

- **OS**: Ubuntu 22.04 LTS, Ubuntu 20.04 LTS, Debian 11+, or compatible Linux distribution
- **CPU**: 8+ cores (16+ recommended for faster builds)
- **RAM**: 16GB minimum (32GB+ recommended for parallel builds)
- **Storage**: 100GB+ free disk space (Rust compilation requires significant space)
- **Network**: Stable internet connection for downloading dependencies

### Rust Toolchain Installation

Formation is written in Rust and requires a modern Rust toolchain:

```bash
# Install Rust using rustup (official installer)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Source the environment (or log out and back in)
source "$HOME/.cargo/env"

# Verify installation
rustc --version
cargo --version

# Update to latest stable Rust
rustup update stable

# Set stable as default toolchain
rustup default stable

# Add required Rust components
rustup component add clippy rustfmt

# Add cargo tools for development (optional but recommended)
cargo install cargo-watch cargo-audit cargo-outdated
```

### System Dependencies for Compilation

Formation requires various system libraries and development tools:

```bash
# Essential build tools
sudo apt update
sudo apt install -y build-essential pkg-config libssl-dev git

# Networking and system libraries
sudo apt install -y libudev-dev libsqlite3-dev

# Protocol Buffers compiler (required for gRPC)
sudo apt install -y protobuf-compiler

# Additional development dependencies
sudo apt install -y cmake clang llvm-dev libclang-dev

# WireGuard development headers
sudo apt install -y wireguard-tools libwireguard-dev

# VM and virtualization support
sudo apt install -y qemu-system-x86 qemu-utils qemu-kvm

# Bridge networking tools
sudo apt install -y bridge-utils dnsmasq iptables

# Additional utilities for development
sudo apt install -y curl wget jq htop net-tools
```

### Verification of Build Prerequisites

Verify all required tools are installed correctly:

```bash
# Check Rust toolchain
echo "=== Rust Toolchain ==="
rustc --version
cargo --version
rustup --version

# Check essential build tools
echo "=== Build Tools ==="
gcc --version | head -1
pkg-config --version
cmake --version | head -1

# Check required libraries
echo "=== System Libraries ==="
pkg-config --exists openssl && echo "✓ OpenSSL" || echo "✗ OpenSSL missing"
pkg-config --exists sqlite3 && echo "✓ SQLite3" || echo "✗ SQLite3 missing"
pkg-config --exists libudev && echo "✓ libudev" || echo "✗ libudev missing"

# Check Protocol Buffers
echo "=== Protocol Buffers ==="
protoc --version

# Check virtualization support
echo "=== Virtualization ==="
which qemu-system-x86_64 && echo "✓ QEMU" || echo "✗ QEMU missing"
lsmod | grep kvm && echo "✓ KVM module loaded" || echo "✗ KVM module not loaded"

# Check WireGuard
echo "=== WireGuard ==="
which wg && echo "✓ WireGuard tools" || echo "✗ WireGuard tools missing"
lsmod | grep wireguard && echo "✓ WireGuard module loaded" || echo "✗ WireGuard module not loaded"

# Check networking tools
echo "=== Networking Tools ==="
which brctl && echo "✓ bridge-utils" || echo "✗ bridge-utils missing"
which iptables && echo "✓ iptables" || echo "✗ iptables missing"
```

### Rust Build Configuration

Optimize Rust compilation for Formation:

```bash
# Create Cargo configuration directory
mkdir -p ~/.cargo

# Configure Cargo for optimal builds
cat > ~/.cargo/config.toml << 'EOF'
[build]
# Use all available CPU cores for compilation
jobs = 0

[target.x86_64-unknown-linux-gnu]
# Use system linker for faster linking
linker = "clang"
rustflags = ["-C", "link-arg=-fuse-ld=lld"]

[profile.dev]
# Faster debug builds
debug = 1
opt-level = 0

[profile.release]
# Optimized release builds
debug = false
opt-level = 3
lto = "thin"
codegen-units = 1
panic = "abort"

[net]
# Use sparse protocol for faster index updates
git-fetch-with-cli = true
EOF

# Install faster linker (optional but recommended)
sudo apt install -y lld

# Set environment variables for optimal builds
echo 'export RUST_BACKTRACE=1' >> ~/.bashrc
echo 'export CARGO_INCREMENTAL=1' >> ~/.bashrc
echo 'export RUSTC_WRAPPER=""' >> ~/.bashrc

# Source the updated bashrc
source ~/.bashrc
```

### Development Environment Setup

Additional tools for Formation development:

```bash
# Install development utilities
cargo install cargo-nextest    # Faster test runner
cargo install cargo-deny       # License and security checking
cargo install cargo-machete    # Remove unused dependencies
cargo install sccache          # Shared compilation cache

# Configure sccache for faster rebuilds
echo 'export RUSTC_WRAPPER=sccache' >> ~/.bashrc
echo 'export SCCACHE_CACHE_SIZE=10G' >> ~/.bashrc

# Install formatting and linting tools
rustup component add rustfmt clippy

# Install documentation tools
cargo install mdbook mdbook-linkcheck
```

### Hardware Requirements for Development

For optimal development experience:

**Minimum Requirements:**
- CPU: 8 cores (Intel i7 or AMD Ryzen 7 equivalent)
- RAM: 16GB
- Storage: 100GB SSD free space
- Network: Stable broadband connection

**Recommended Requirements:**
- CPU: 16+ cores (Intel i9 or AMD Ryzen 9 equivalent)
- RAM: 32GB+ 
- Storage: 500GB+ NVMe SSD with high write endurance
- Network: High-speed connection for dependency downloads

**Build Time Expectations:**
- Clean build (release): 15-45 minutes depending on hardware
- Incremental builds: 1-10 minutes
- Test suite: 5-20 minutes
- Documentation generation: 2-5 minutes

### Verification of Complete Prerequisites

Run this comprehensive check to ensure you're ready for source builds:

```bash
#!/bin/bash
echo "=== Formation Source Build Prerequisites Check ==="

errors=0

# Check Rust toolchain
if command -v rustc >/dev/null 2>&1; then
    rust_version=$(rustc --version | grep -oE '[0-9]+\.[0-9]+\.[0-9]+')
    echo "✓ Rust: $rust_version"
    
    # Check if Rust version is recent enough (1.70+)
    if [[ $(echo $rust_version | cut -d. -f1-2) < "1.70" ]]; then
        echo "  ⚠ Warning: Rust version may be too old, consider updating"
    fi
else
    echo "✗ Rust not found"
    errors=$((errors + 1))
fi

# Check essential build tools
for tool in gcc pkg-config cmake clang protoc; do
    if command -v $tool >/dev/null 2>&1; then
        echo "✓ $tool"
    else
        echo "✗ $tool missing"
        errors=$((errors + 1))
    fi
done

# Check libraries
for lib in openssl sqlite3 libudev; do
    if pkg-config --exists $lib 2>/dev/null; then
        echo "✓ $lib"
    else
        echo "✗ $lib missing"
        errors=$((errors + 1))
    fi
done

# Check virtualization
if command -v qemu-system-x86_64 >/dev/null 2>&1; then
    echo "✓ QEMU"
else
    echo "✗ QEMU missing"
    errors=$((errors + 1))
fi

# Check KVM
if lsmod | grep -q kvm; then
    echo "✓ KVM module loaded"
else
    echo "✗ KVM module not loaded (may need 'sudo modprobe kvm')"
    errors=$((errors + 1))
fi

# Check WireGuard
if command -v wg >/dev/null 2>&1; then
    echo "✓ WireGuard tools"
else
    echo "✗ WireGuard tools missing"
    errors=$((errors + 1))
fi

# Check available disk space (need at least 50GB free)
available=$(df . | tail -1 | awk '{print $4}')
available_gb=$((available / 1024 / 1024))
if [ $available_gb -gt 50 ]; then
    echo "✓ Disk space: ${available_gb}GB available"
else
    echo "✗ Insufficient disk space: ${available_gb}GB available (need 50GB+)"
    errors=$((errors + 1))
fi

# Check RAM
total_ram=$(free -g | awk 'NR==2{print $2}')
if [ $total_ram -gt 15 ]; then
    echo "✓ RAM: ${total_ram}GB available"
else
    echo "⚠ RAM: ${total_ram}GB available (recommend 16GB+)"
fi

echo "========================"
if [ $errors -eq 0 ]; then
    echo "✅ All prerequisites satisfied! Ready for source builds."
else
    echo "❌ $errors prerequisites missing. Please install missing components."
    exit 1
fi
```

---

## Build Instructions (Source Deployment)

If you've completed the source build prerequisites above, follow these steps to build Formation from source code.

### Clone the Repository

```bash
# Clone the Formation repository
git clone https://github.com/formthefog/formation.git
cd formation

# Check the current branch and latest commit
git branch -v
git log --oneline -5

# Optional: Check available tags/releases
git tag -l
```

### Understanding the Build Structure

Formation is organized as a Rust workspace with multiple packages. The main services you'll build are:

| Service | Package | Binary Name | Purpose |
|---------|---------|-------------|---------|
| Configuration | `form-config` | `form-config-wizard` | Generate operator configuration |
| State Management | `form-state` | `form-state` | Network state and API server |
| Networking | `form-net/formnet` | `formnet` | WireGuard mesh networking |
| Virtual Machines | `form-vmm/vmm-service` | `vmm-service` | VM lifecycle management |
| Package Management | `form-pack` | `form-pack-manager` | Build and manage VM images |
| DNS | `form-dns` | `form-dns` | DNS resolution within network |

### Build All Services (Recommended)

Build all Formation services in release mode:

```bash
# Build all workspace members in release mode
# This will take 15-45 minutes depending on hardware
cargo build --release --workspace

# Monitor build progress (optional, in another terminal)
# watch 'ps aux | grep cargo'
```

**Expected output location**: All binaries will be built to `target/release/`

### Build Individual Services

For faster iteration or specific requirements, build individual services:

#### Core Configuration Tool

```bash
# Build the configuration wizard (needed first)
cargo build --release --package form-config

# Verify build
ls -la target/release/form-config-wizard
./target/release/form-config-wizard --help
```

#### State Management Service

```bash
# Build form-state (core service)
cargo build --release --package form-state

# Verify build
ls -la target/release/form-state
./target/release/form-state --help
```

#### Networking Service

```bash
# Build formnet and related networking tools
cargo build --release --package formnet

# This builds multiple binaries:
ls -la target/release/formnet*
# - formnet (main networking service)
# - formnet-vm-join (VM network joining)
# - formnet-up (network interface management)
```

#### Virtual Machine Manager

```bash
# Build VMM service
cargo build --release --package vmm-service

# Verify build
ls -la target/release/vmm-service
./target/release/vmm-service --help
```

#### Package Manager

```bash
# Build form-pack tools
cargo build --release --package form-pack

# This builds multiple binaries:
ls -la target/release/form-pack*
# - form-pack (CLI tool)
# - form-pack-manager (service)
# - form-build-server (build service)
```

#### DNS Service

```bash
# Build DNS service
cargo build --release --package form-dns

# Verify build
ls -la target/release/form-dns
./target/release/form-dns --help
```

### Development Builds

For development and testing, use debug builds (faster compilation, larger binaries):

```bash
# Build all services in debug mode (much faster)
cargo build --workspace

# Debug binaries are located in target/debug/
ls -la target/debug/form-*
ls -la target/debug/formnet*
ls -la target/debug/vmm-service
```

### Verify Complete Build

Check that all essential binaries were built successfully:

```bash
#!/bin/bash
echo "=== Verifying Formation Build ==="

# Essential binaries for a complete Formation node
BINARIES=(
    "form-config-wizard"
    "form-state"
    "formnet"
    "vmm-service"
    "form-pack-manager"
    "form-dns"
)

BUILD_TYPE="release"  # or "debug"
BINARY_DIR="target/${BUILD_TYPE}"

echo "Checking binaries in ${BINARY_DIR}/"

all_found=true
for binary in "${BINARIES[@]}"; do
    if [ -f "${BINARY_DIR}/${binary}" ]; then
        size=$(du -h "${BINARY_DIR}/${binary}" | cut -f1)
        echo "✓ ${binary} (${size})"
    else
        echo "✗ ${binary} - NOT FOUND"
        all_found=false
    fi
done

echo "========================"
if $all_found; then
    echo "✅ All essential binaries built successfully!"
    echo "Total build size: $(du -sh ${BINARY_DIR} | cut -f1)"
else
    echo "❌ Some binaries missing. Run cargo build to rebuild."
    exit 1
fi
```

### Install Binaries (Optional)

Install binaries to system PATH for easier access:

```bash
# Install all Formation binaries to ~/.cargo/bin
cargo install --path form-config
cargo install --path form-state
cargo install --path form-net/formnet
cargo install --path form-vmm/vmm-service
cargo install --path form-pack
cargo install --path form-dns

# Verify installation
which form-config-wizard
which form-state
which formnet
which vmm-service
which form-pack-manager
which form-dns

# Test installation
form-config-wizard --help
form-state --help
```

### Binary Locations and Usage

After building, your binaries will be located at:

```bash
# Release builds (optimized, smaller, slower compilation)
target/release/form-config-wizard
target/release/form-state
target/release/formnet
target/release/vmm-service
target/release/form-pack-manager
target/release/form-dns

# Debug builds (unoptimized, larger, faster compilation)
target/debug/form-config-wizard
target/debug/form-state
target/debug/formnet
target/debug/vmm-service
target/debug/form-pack-manager
target/debug/form-dns
```

**Usage Examples:**

```bash
# Generate operator configuration
./target/release/form-config-wizard wizard

# Start state management service
./target/release/form-state --config-path ./secrets/.operator-config.json

# Initialize or join formnet
./target/release/formnet operator join --config-path ./secrets/.operator-config.json

# Start VMM service
./target/release/vmm-service --config-path ./secrets/.operator-config.json

# Start package manager
./target/release/form-pack-manager --config-path ./secrets/.operator-config.json

# Start DNS service
./target/release/form-dns --config-path ./secrets/.operator-config.json
```

### Optimization Tips

#### Faster Builds

```bash
# Use multiple CPU cores for parallel compilation
export CARGO_BUILD_JOBS=$(nproc)

# Use faster linker if available
export RUSTFLAGS="-C link-arg=-fuse-ld=lld"

# Enable compiler cache for faster rebuilds
export RUSTC_WRAPPER=sccache
```

#### Smaller Binaries

```bash
# Build with size optimization
RUSTFLAGS="-C opt-level=s -C strip=symbols" cargo build --release --workspace

# Check binary sizes
du -h target/release/form-* target/release/formnet* target/release/vmm-service
```

#### Target-Specific Optimizations

```bash
# Build optimized for your specific CPU
RUSTFLAGS="-C target-cpu=native" cargo build --release --workspace
```

### Troubleshooting Build Issues

#### Common Build Errors

**Missing system dependencies:**
```bash
# If you see linking errors about missing libraries
sudo apt install -y build-essential pkg-config libssl-dev libsqlite3-dev libudev-dev

# For protobuf compilation errors
sudo apt install -y protobuf-compiler
```

**Out of memory during compilation:**
```bash
# Reduce parallel jobs to use less memory
CARGO_BUILD_JOBS=2 cargo build --release --workspace

# Or build packages individually
for pkg in form-config form-state formnet vmm-service form-pack form-dns; do
    echo "Building $pkg..."
    cargo build --release --package $pkg
done
```

**Rust version too old:**
```bash
# Update Rust to latest stable
rustup update stable
rustc --version  # Should be 1.70+ for best compatibility
```

#### Clean and Rebuild

```bash
# Clean build artifacts and rebuild
cargo clean
cargo build --release --workspace

# Clean only dependencies (faster)
cargo clean --package form-state
cargo build --release --package form-state
```

### Next Steps

With your binaries built, you can proceed to:

1. **Configure your node** using `form-config-wizard`
2. **Set up networking** (bridge interface, ports)
3. **Start services** manually or create systemd service files
4. **Join or initialize** a Formation network

---

## Configuration Wizard Walkthrough

The `form-config-wizard` is an interactive tool that guides you through setting up your Formation node configuration. This section walks through each step of the wizard and explains the options.

### Running the Configuration Wizard

Start the wizard using your built binary:

```bash
# For source builds
./target/release/form-config-wizard wizard

# Or if installed to PATH
form-config-wizard wizard

# For Docker Compose users (run inside a temporary container)
docker run --rm -it -v $(pwd)/secrets:/secrets formationai/form-config:latest form-config-wizard wizard
```

### Step-by-Step Walkthrough

#### 1. Network ID Configuration

```
=== Network Configuration ===
The network ID is used to distinguish between different networks.
Common values:
  1 = Mainnet
  2 = Jericho  
  3 = Uruk
  >50000 = Local

Enter network ID [5]: 
```

**Options:**
- **1 (Mainnet)**: Production Formation network (coming soon)
- **2 (Jericho)**: Development testnet
- **3 (Uruk)**: Staging testnet  
- **50000+**: Local development networks
- **Default**: 5 (development)

**Recommendation**: Use 50001 for local testing, 2 for development network participation.

#### 2. Keyfile Configuration

```
=== Keyfile Configuration ===
The keyfile stores your encrypted credentials.

Use default keyfile path? (/home/user/.config/.keystore) [y/N]: 
```

**Options:**
- **y**: Use default location (`~/.config/.keystore`)
- **n**: Specify custom path

**Recommendation**: Use default unless you have specific security requirements.

#### 3. Key Configuration (Most Important Step)

```
=== Key Configuration ===
Your keys are used to sign transactions and prove your identity.

How would you like to configure your keys?
  > Generate new keys from mnemonic
    Import from existing mnemonic
    Import from private key
    Skip for now
```

##### Option A: Generate New Keys (Recommended for New Operators)

```
How many words would you like your mnemonic to be?
  > 12
    24

Would you like to enhance security with a password for your mnemonic phrase? [Y/n]: y

Provide a secure password to enhance the security of your mnemonic phrase: [hidden]
Confirm your password: [hidden]

=== Generated New Keys: ===
Mnemonic Phrase: word1, word2, word3, ... word12, 
Address: 0x1234567890123456789012345678901234567890

⚠️  IMPORTANT ⚠️ 
Please store your mnemonic phrase safely. It cannot be recovered if lost!

I have safely stored my mnemonic phrase [y/N]: y
```

**CRITICAL**: Write down your mnemonic phrase and store it securely offline. This is your only way to recover your keys.

##### Option B: Import Existing Mnemonic

```
Enter your mnemonic phrase: word1 word2 word3 ... word12

Did you provide a password for your mnemonic phrase? [Y/n]: y
Enter your password: [hidden]

Derived address: 0x1234567890123456789012345678901234567890
```

##### Option C: Import Private Key

```
Enter your private key (hex): 0x1234567890123456789012345678901234567890123456789012345678901234
Derived address: 0x1234567890123456789012345678901234567890
```

#### 4. Bootstrap Nodes Configuration

```
=== Bootstrap Nodes Configuration ===
Bootstrap nodes help you connect to the network initially.

Enter bootstrap node address (leave empty to finish): 
```

**For Joining an Existing Network:**
```
Enter bootstrap node address (leave empty to finish): 203.0.113.10:51820
Enter bootstrap node address (leave empty to finish): 198.51.100.20:51820  
Enter bootstrap node address (leave empty to finish): [press enter to finish]
```

**For Starting a New Network:**
Leave empty (just press enter) to create a bootstrap node.

#### 5. Bootstrap Domain Configuration

```
=== Bootstrap Domain Configuration ===
A bootstrap domain allows you to connect to the nearest healthy bootstrap node automatically.

Would you like to use a bootstrap domain for network discovery? [Y/n]: y
Enter bootstrap domain [bootstrap.formation.cloud]: 
```

**Options:**
- **bootstrap.formation.cloud**: Official Formation bootstrap domain
- **Custom domain**: Your own bootstrap domain
- **Empty**: No domain-based discovery

#### 6. Bootstrap Node Role

```
=== Bootstrap Node Role ===
Bootstrap nodes help other nodes discover and join the network.
They need to be reliable and have good connectivity.

Should this node serve as a bootstrap node? [y/N]: 
```

**Choose "y" if:**
- This is the first node in a new network
- You want to help others join the network
- You have a stable, publicly accessible server

**Choose "n" if:**
- Joining an existing network
- Running on a personal/development machine
- Behind NAT without port forwarding

#### 7. Region Configuration

```
=== Node Region Configuration ===
The geographic region helps optimize network connectivity.

Select the geographic region of this node:
  > none
    us-east
    us-west
    us-central
    eu-west
    eu-central
    asia-east
    asia-southeast
    custom
```

**Recommendation**: Choose the region closest to your server's physical location.

#### 8. Service Port Configuration

The wizard will prompt for each service port:

```
=== Datastore Port Configuration ===
Enter Datastore port [3004]: 

=== Formnet Join Server Port Configuration ===
Enter Formnet Join Server port [3001]: 

=== Formnet Service Port Configuration ===
Enter Formnet Service port [51820]: 

=== VMM Service Port Configuration ===
Enter VMM Service port [3002]: 

=== Pack Manager Port Configuration ===
Enter Pack Manager port [3003]: 

=== Event Queue Port Configuration ===
Enter Event Queue port [3005]: 
```

**Default Ports:**
- **3004**: form-state API
- **3001**: Formnet join server
- **51820**: WireGuard (Formnet)
- **3002**: VMM service API
- **3003**: Pack manager API
- **3005**: Event queue

**Recommendation**: Use defaults unless you have port conflicts.

#### 9. Contract Configuration

```
=== Contract Configuration ===
Enter the staking contract address for your AVS.

Enter contract address (leave empty to skip): 
```

**Recommendation**: Leave empty for local development and testing.

#### 10. Formnet CIDR Configuration

```
=== Formnet CIDR Configuration ===
Configure the IP address range for the internal Formnet VPN (e.g., 10.42.0.0/16).
Ensure this does not conflict with existing networks in your environment.

Enter Formnet CIDR (e.g., 10.42.0.0/16, leave empty for none/default behavior elsewhere) [10.42.0.0/16]: 
```

**Common Options:**
- **10.42.0.0/16**: Default (65,534 available IPs)
- **172.20.0.0/16**: Alternative private range
- **192.168.100.0/24**: Smaller network (254 IPs)

**Recommendation**: Use the default unless it conflicts with your existing networks.

#### 11. Initial Admin Configuration

```
=== Initial Admin Public Key Configuration ===
Configure the public key (hex format) of the account that will be the initial system administrator.
If left empty, the first node's own operator key might be designated as admin by default (behavior defined elsewhere).

Enter initial admin public key (hex, leave empty to skip/use default): 
```

**Recommendation**: Leave empty for local development (your operator key becomes admin).

#### 12. Saving Configuration

```
Save config to ./secrets/.operator-config.json? [Y/n]: y

Would you like to encrypt your keys in the keystore? [Y/n]: y

=== Encrypting Keystore ===
Please choose a strong password to encrypt your keys.
This password will be required to decrypt your keys later.
⚠️  Encrypting the Keystore may take up to 2 minutes⚠️  

Enter encryption password: [hidden]
Confirm your password: [hidden]

Keystore encrypted and saved successfully!
Location: /home/user/.config/.keystore

Configuration saved successfully!
Location: ./secrets/.operator-config.json
```

### Example Complete Configuration

Here's an example of a complete wizard session for setting up a local test network:

```bash
$ ./target/release/form-config-wizard wizard

Welcome to the Operator Configuration Wizard
This wizard will help you set up your operator configuration.

=== Network Configuration ===
Enter network ID [5]: 50001

=== Keyfile Configuration ===
Use default keyfile path? (/home/user/.config/.keystore) [y/N]: y

=== Key Configuration ===
How would you like to configure your keys?
  > Generate new keys from mnemonic

How many words would you like your mnemonic to be?
  > 12

Would you like to enhance security with a password for your mnemonic phrase? [Y/n]: y
[Enter secure password]

=== Generated New Keys: ===
Mnemonic Phrase: abandon, ability, able, about, above, absent, absorb, abstract, absurd, abuse, access, accident
Address: 0x742d35cc6e34c2c7b90b9c3c9b8a5e3d9f8c7b6a

I have safely stored my mnemonic phrase [y/N]: y

=== Bootstrap Nodes Configuration ===
Enter bootstrap node address (leave empty to finish): [press enter]

=== Bootstrap Domain Configuration ===
Would you like to use a bootstrap domain for network discovery? [Y/n]: n

=== Bootstrap Node Role ===
Should this node serve as a bootstrap node? [y/N]: y

=== Node Region Configuration ===
Select the geographic region: us-east

=== Port Configuration ===
[Accept all defaults: 3004, 3001, 51820, 3002, 3003, 3005]

=== Contract Configuration ===
Enter contract address (leave empty to skip): [press enter]

=== Formnet CIDR Configuration ===
Enter Formnet CIDR [10.42.0.0/16]: 172.20.0.0/16

=== Initial Admin Public Key Configuration ===
Enter initial admin public key: [press enter]

Save config to ./secrets/.operator-config.json? [Y/n]: y
Would you like to encrypt your keys in the keystore? [Y/n]: y
[Enter encryption password]

Configuration saved successfully!
```

### Verifying Configuration

After the wizard completes, verify your configuration:

```bash
# Check that configuration file was created
ls -la secrets/.operator-config.json

# View configuration (sensitive keys will be encrypted/redacted)
cat secrets/.operator-config.json

# Check keystore was created
ls -la ~/.config/.keystore

# Validate JSON syntax
jq . secrets/.operator-config.json
```

### Configuration File Structure

The generated configuration file will look like this:

```json
{
  "network_id": 50001,
  "keyfile": "/home/user/.config/.keystore",
  "secret_key": "encrypted_hex_string",
  "mnemonic": ["encrypted_word1", "encrypted_word2", "..."],
  "public_key": "hex_public_key",
  "address": "0x742d35cc6e34c2c7b90b9c3c9b8a5e3d9f8c7b6a",
  "initial_admin_public_key": null,
  "bootstrap_nodes": [],
  "bootstrap_domain": null,
  "is_bootstrap_node": true,
  "region": "us-east",
  "datastore_port": 3004,
  "formnet_join_server_port": 3001,
  "formnet_service_port": 51820,
  "formnet_cidr": "172.20.0.0/16",
  "vmm_service_port": 3002,
  "pack_manager_port": 3003,
  "event_queue_port": 3005,
  "contract_address": null
}
```

### Common Configuration Patterns

#### Pattern 1: Bootstrap Node (First Node in Network)

```json
{
  "network_id": 50001,
  "bootstrap_nodes": [],
  "is_bootstrap_node": true,
  "formnet_cidr": "10.42.0.0/16"
}
```

#### Pattern 2: Joining Node

```json
{
  "network_id": 50001,
  "bootstrap_nodes": ["203.0.113.10:51820"],
  "is_bootstrap_node": false,
  "formnet_cidr": "10.42.0.0/16"
}
```

#### Pattern 3: Development Node

```json
{
  "network_id": 50001,
  "bootstrap_domain": null,
  "region": "local",
  "formnet_cidr": "192.168.200.0/24"
}
```

### Security Best Practices

1. **Store mnemonic phrase offline** - Write it down, don't store digitally
2. **Use strong encryption password** - Different from mnemonic password
3. **Backup configuration files** - Store encrypted copies securely
4. **Limit file permissions** - `chmod 600 secrets/.operator-config.json`
5. **Use hardware security** - Consider hardware wallets for production

### Troubleshooting Configuration

#### Invalid JSON Error
```bash
# Check JSON syntax
jq . secrets/.operator-config.json

# Common fix: regenerate configuration
rm secrets/.operator-config.json
./target/release/form-config-wizard wizard
```

#### Key Decryption Fails
```bash
# Check if keystore exists
ls -la ~/.config/.keystore

# Verify password is correct (will prompt again)
# No automated way - you must remember the password
```

#### Network ID Conflicts
```bash
# Change network ID in existing config
jq '.network_id = 50002' secrets/.operator-config.json > tmp.json
mv tmp.json secrets/.operator-config.json
```

### Next Steps

With your configuration file ready, you can proceed to:

1. **Set up networking** (bridge interface, port forwarding)
2. **Start Formation services** using your configuration
3. **Initialize or join** the Formation network

---

## Single Node Initialization (Source Deployment)

After completing the build and configuration steps, this section walks through starting your Formation services from source and getting a single node operational.

### Prerequisites Check

Before starting services, ensure you have:

```bash
# 1. Built all required binaries
ls -la target/release/form-{config-wizard,state,dns,pack-manager} target/release/{formnet,vmm-service}

# 2. Completed network configuration
ip addr show br0  # Should show bridge interface

# 3. Generated operator configuration
ls -la secrets/.operator-config.json

# 4. Set proper file permissions
chmod 600 secrets/.operator-config.json
chmod 600 ~/.config/.keystore
```

### Service Startup Order

Formation services have dependencies and should be started in this order:

1. **form-state** (Core state management)
2. **form-dns** (DNS resolution)  
3. **formnet** (Network initialization)
4. **vmm-service** (VM management)
5. **form-pack-manager** (Package management)

### Step 1: Start form-state (Core Service)

The state service is the foundation and must start first:

```bash
# Start form-state service
./target/release/form-state \
    --config-path ./secrets/.operator-config.json \
    --password "your-encryption-password" \
    --encrypted true

# Expected output:
# INFO formation: Parsing CLI...
# INFO formation: Built data store, running...
# INFO formation: Service started successfully
```

**Verify form-state is running:**

```bash
# Check health endpoint
curl -s http://localhost:3004/health | jq .

# Expected response:
# {
#   "status": "Healthy",
#   "version": "0.1.0",
#   "uptime": 1234
# }

# Check network status
curl -s http://localhost:3004/v1/network/status | jq .
```

**Common startup issues:**

```bash
# If config file not found
ls -la secrets/.operator-config.json

# If password incorrect  
# Re-enter correct encryption password

# If port 3004 already in use
sudo netstat -tlnp | grep 3004
# Stop conflicting service or change port in config
```

### Step 2: Start form-dns (DNS Service)

```bash
# Start DNS service (in a new terminal or background)
./target/release/form-dns \
    --config-path ./secrets/.operator-config.json \
    --state-url http://localhost:3004 \
    --dns-port 5453

# Expected output:
# INFO form_dns: Starting DNS server on port 5453
# INFO form_dns: Connected to state service at http://localhost:3004
```

**Verify form-dns is running:**

```bash
# Test DNS resolution
dig @localhost -p 5453 formation +short

# Check DNS health (if health endpoint exists)
# curl -s http://localhost:5454/health

# Check DNS logs
./target/release/form-dns --help  # See logging options
```

### Step 3: Initialize formnet (Network Service)

This is the most critical step for network connectivity:

```bash
# Initialize or join formnet
./target/release/formnet operator join \
    --config-path ./secrets/.operator-config.json

# For bootstrap node (first node), expected output:
# INFO formnet: No bootstrap nodes specified, initializing new network
# INFO formnet: Initializing formnet with CIDR 172.20.0.0/16
# INFO formnet: WireGuard interface created: formnet0
# INFO formnet: Network initialized with IP 172.20.0.1/16
# INFO formnet: Bootstrap node ready, listening on 0.0.0.0:51820
```

**Verify formnet initialization:**

```bash
# Check WireGuard interface
ip addr show formnet0
# Should show: inet 172.20.0.1/16 (or your configured CIDR)

# Check WireGuard status
sudo wg show formnet0

# Test formnet API
curl -s http://localhost:51820/health | jq .

# Check network peers (initially empty for bootstrap node)
curl -s http://localhost:3004/v1/network/peers | jq .
```

**If formnet fails to start:**

```bash
# Check WireGuard is installed
which wg
sudo modprobe wireguard

# Check bridge interface exists
ip addr show br0

# Check ports aren't blocked
sudo iptables -L | grep -E "(3004|51820)"

# Check logs for specific errors
./target/release/formnet operator join --config-path ./secrets/.operator-config.json -v
```

### Step 4: Start vmm-service (VM Management)

```bash
# Start VM management service
./target/release/vmm-service \
    --config-path ./secrets/.operator-config.json \
    --vmm-api-port 3002

# Expected output:
# INFO vmm_service: Starting VMM service on port 3002
# INFO vmm_service: Connected to state service
# INFO vmm_service: VMM service ready
```

**Verify vmm-service:**

```bash
# Check VMM health
curl -s http://localhost:3002/health | jq .

# Check VM capabilities
curl -s http://localhost:3002/capabilities | jq .

# Verify KVM access
ls -la /dev/kvm
# Should be accessible to your user or docker group
```

### Step 5: Start form-pack-manager (Package Service)

```bash
# Start package management service
./target/release/form-pack-manager \
    --config-path ./secrets/.operator-config.json \
    --port 3003 \
    --data-dir ./pack-data

# Expected output:
# INFO form_pack: Starting pack manager on port 3003
# INFO form_pack: Connected to state service
# INFO form_pack: Docker daemon connected
# INFO form_pack: Pack manager ready
```

**Verify form-pack-manager:**

```bash
# Check pack manager health
curl -s http://localhost:3003/health | jq .

# Check Docker connectivity
curl -s http://localhost:3003/docker/info | jq .

# Test build capabilities
curl -s http://localhost:3003/capabilities | jq .
```

### Step 6: Verify Complete Node Operation

With all services running, verify your node is fully operational:

#### Service Health Dashboard

```bash
#!/bin/bash
echo "=== Formation Node Status ==="

services=(
    "form-state:3004:/health"
    "vmm-service:3002:/health"  
    "form-pack-manager:3003:/health"
    "formnet:51820:/health"
)

for service in "${services[@]}"; do
    name=$(echo $service | cut -d: -f1)
    port=$(echo $service | cut -d: -f2)
    endpoint=$(echo $service | cut -d: -f3)
    
    if curl -s --connect-timeout 2 http://localhost:$port$endpoint > /dev/null; then
        echo "✓ $name (port $port): healthy"
    else
        echo "✗ $name (port $port): unhealthy"
    fi
done

echo "========================"
```

#### Network Status Check

```bash
# Check formnet interface
echo "=== Network Status ==="
if ip addr show formnet0 > /dev/null 2>&1; then
    formnet_ip=$(ip addr show formnet0 | grep 'inet ' | awk '{print $2}')
    echo "✓ Formnet interface: $formnet_ip"
else
    echo "✗ Formnet interface: not found"
fi

# Check network state
peer_count=$(curl -s http://localhost:3004/v1/network/peers | jq length 2>/dev/null || echo "0")
echo "✓ Network peers: $peer_count"

# Check node registration
node_count=$(curl -s http://localhost:3004/v1/nodes/list | jq length 2>/dev/null || echo "0")
echo "✓ Registered nodes: $node_count"
```

#### Basic Functionality Test

```bash
# Test state management
echo "=== Testing State Management ==="
curl -s http://localhost:3004/v1/network/cidrs | jq .

# Test account functionality
echo "=== Testing Account System ==="
curl -s http://localhost:3004/v1/accounts/list | jq .

# Test instance management  
echo "=== Testing Instance Management ==="
curl -s http://localhost:3004/v1/instances/list | jq .
```

### Running Services in Background

For production use, run services in the background:

#### Using nohup

```bash
# Start all services in background
nohup ./target/release/form-state --config-path ./secrets/.operator-config.json > logs/form-state.log 2>&1 &
sleep 5

nohup ./target/release/form-dns --config-path ./secrets/.operator-config.json > logs/form-dns.log 2>&1 &
sleep 2

nohup ./target/release/formnet operator join --config-path ./secrets/.operator-config.json > logs/formnet.log 2>&1 &
sleep 5

nohup ./target/release/vmm-service --config-path ./secrets/.operator-config.json > logs/vmm-service.log 2>&1 &
sleep 2

nohup ./target/release/form-pack-manager --config-path ./secrets/.operator-config.json > logs/form-pack-manager.log 2>&1 &

# Check all processes are running
pgrep -f "form-state|form-dns|formnet|vmm-service|form-pack-manager"
```

#### Using systemd (Recommended for Production)

Create systemd service files:

```bash
# Create service directory
sudo mkdir -p /etc/systemd/system

# Example service file for form-state
sudo tee /etc/systemd/system/form-state.service << EOF
[Unit]
Description=Formation State Service
After=network.target
Wants=network.target

[Service]
Type=simple
User=formation
Group=formation
WorkingDirectory=/home/formation/formation
ExecStart=/home/formation/formation/target/release/form-state --config-path /home/formation/formation/secrets/.operator-config.json --password-file /home/formation/.formation-password
Restart=always
RestartSec=5
StandardOutput=journal
StandardError=journal

[Install]
WantedBy=multi-user.target
EOF

# Enable and start service
sudo systemctl daemon-reload
sudo systemctl enable form-state
sudo systemctl start form-state

# Check status
sudo systemctl status form-state
```

### Monitoring and Logs

#### View Service Logs

```bash
# For background processes
tail -f logs/form-state.log
tail -f logs/formnet.log

# For systemd services
sudo journalctl -u form-state -f
sudo journalctl -u formnet -f

# Combined view
sudo journalctl -u form-state -u form-dns -u formnet -u vmm-service -u form-pack-manager -f
```

#### Performance Monitoring

```bash
# Monitor resource usage
watch 'ps aux | grep -E "(form-state|formnet|vmm-service)" | grep -v grep'

# Monitor network connections
watch 'ss -tulpn | grep -E "(3004|3002|3003|51820)"'

# Monitor disk usage
watch 'df -h && du -sh pack-data/ logs/'
```

### Troubleshooting Startup Issues

#### Service Won't Start

```bash
# Check configuration file
jq . secrets/.operator-config.json

# Check file permissions
ls -la secrets/.operator-config.json ~/.config/.keystore

# Check dependencies
ldd target/release/form-state

# Check system resources
free -h
df -h
```

#### Network Issues

```bash
# Check bridge configuration
ip addr show br0
brctl show br0

# Check firewall
sudo iptables -L
sudo ufw status

# Check port conflicts
sudo netstat -tlnp | grep -E "(3004|3002|3003|51820)"
```

#### Permission Issues

```bash
# Check KVM access
ls -la /dev/kvm
groups $USER

# Check Docker access (for pack manager)
docker ps
groups $USER | grep docker

# Fix permissions
sudo usermod -aG kvm $USER
sudo usermod -aG docker $USER
# Log out and back in
```

### Next Steps

With your single node operational:

1. **Deploy a test agent** to verify VM functionality
2. **Monitor resource usage** and performance
3. **Set up proper logging** and monitoring
4. **Configure backups** for configuration and data
5. **Consider adding a second node** for multi-node testing

Your Formation node is now ready for development and testing!

---

## Network Configuration (Both Deployment Options)

Formation requires a network bridge for VM operations. Run the provided script to set this up automatically:

```bash
# Clone the Formation repository
git clone https://github.com/formthefog/formation.git
cd formation

# Run the network validation script
sudo bash scripts/validate-network-config.sh
```

This script will:
- Create a `br0` bridge interface
- Set up NAT and IP forwarding
- Configure `dnsmasq` for DHCP
- Choose an available IP range automatically

### Manual Network Setup (Alternative)

If you prefer manual setup or the script fails:

```bash
# Create bridge interface
sudo brctl addbr br0
sudo ip addr add 192.168.100.1/24 dev br0
sudo ip link set br0 up

# Enable IP forwarding
sudo sysctl -w net.ipv4.ip_forward=1

# Add NAT rule (replace eth0 with your main interface if different)
sudo iptables -t nat -A POSTROUTING -s 192.168.100.0/24 ! -o br0 -j MASQUERADE

# Configure dnsmasq
echo "interface=br0
dhcp-range=192.168.100.10,192.168.100.250,24h
dhcp-option=option:router,192.168.100.1
dhcp-option=option:dns-server,192.168.100.1,8.8.8.8" | sudo tee /etc/dnsmasq.d/br0.conf

# Restart dnsmasq
sudo systemctl restart dnsmasq
```

## Step 2: Port Configuration

Ensure the following ports are open and forwarded to your machine:

```bash
# Required ports for Formation services
# 3002/tcp - form-vmm API
# 3003/tcp - form-pack-manager API  
# 3004/tcp - form-state API
# 51820/udp - form-net (WireGuard)
# 53/udp - form-dns (optional, for external DNS)

# Example iptables rules (adjust as needed)
sudo iptables -A INPUT -p tcp --dport 3002:3004 -j ACCEPT
sudo iptables -A INPUT -p udp --dport 51820 -j ACCEPT
sudo iptables -A INPUT -p udp --dport 53 -j ACCEPT
```

## Step 3: Generate Operator Configuration

Formation uses the `form-config-wizard` to generate your operator configuration. For quick start, we'll create a basic configuration manually:

```bash
# Create secrets directory
mkdir -p secrets

# Create a basic operator configuration
cat > secrets/.operator-config.json << 'EOF'
{
  "network_id": 50001,
  "keyfile": "/home/$USER/.config/.keystore",
  "secret_key": null,
  "mnemonic": null,
  "public_key": null,
  "address": null,
  "initial_admin_public_key": null,
  "bootstrap_nodes": [],
  "bootstrap_domain": null,
  "is_bootstrap_node": true,
  "region": "local",
  "datastore_port": 3004,
  "formnet_join_server_port": 3001,
  "formnet_service_port": 51820,
  "formnet_cidr": "10.42.0.0/16",
  "vmm_service_port": 3002,
  "pack_manager_port": 3003,
  "event_queue_port": 53333,
  "contract_address": null
}
EOF
```

**Important**: This creates a minimal config. For production use, you should generate proper keys using the form-config-wizard tool or your own secure key generation method.

## Step 4: Create Environment Configuration

Create a `.env` file for Docker Compose:

```bash
cat > .env << EOF
# Path to your operator configuration file
SECRET_PATH=/etc/formation/.operator-config.json

# Password for encrypting sensitive data (change this!)
PASSWORD=your-secure-password-here

# Optional: Set log levels
FORMNET_LOG_LEVEL=info
RUST_LOG=info
EOF
```

## Step 5: Deploy Formation Services

Start all Formation services using Docker Compose:

```bash
# Pull the latest images and start services
docker-compose pull
docker-compose up -d

# Check that all services are starting
docker-compose ps
```

You should see 5 services starting:
- `formation-state` (form-state)
- `formation-dns` (form-dns)  
- `formation-network` (form-net)
- `formation-vmm` (vmm-service)
- `formation-pack-manager` (form-pack-manager)

## Step 6: Verify Service Health

Check that all services are healthy:

```bash
# Check form-state
curl http://localhost:3004/health

# Check form-vmm  
curl http://localhost:3002/health

# Check form-pack-manager
curl http://localhost:3003/health

# Check form-net
curl http://localhost:51820/health

# Check form-dns
dig @localhost -p 5453 formation +short
```

Each service should return a healthy status response.

## Step 7: Monitor Service Logs

Watch the logs to ensure everything is working correctly:

```bash
# View all service logs
docker-compose logs -f

# View specific service logs
docker-compose logs -f form-state
docker-compose logs -f form-net
```

Look for successful startup messages and no error logs.

## Step 8: Verify Network Status

Check that your node has successfully initialized the Formation network:

```bash
# Check network state
curl http://localhost:3004/v1/network/status

# Check peer list (should show your node)
curl http://localhost:3004/v1/network/peers

# Check if formnet interface is created
ip addr show formnet0
```

## Step 9: Test Basic Operations

Verify your node is fully operational:

```bash
# Check node information
curl http://localhost:3004/v1/nodes/list

# Check account status (this will be empty initially)
curl http://localhost:3004/v1/accounts/list

# Test instance creation capability
curl http://localhost:3004/v1/instances/list
```

## Troubleshooting Quick Checks

If something isn't working:

### Service Won't Start
```bash
# Check container status
docker-compose ps

# Check specific service logs
docker-compose logs <service-name>

# Restart a specific service
docker-compose restart <service-name>
```

### Network Issues
```bash
# Verify bridge exists
ip addr show br0

# Check if formnet interface is created
ip addr show formnet0

# Verify port accessibility
netstat -tulpn | grep -E "(3002|3003|3004|51820)"
```

### Configuration Issues
```bash
# Verify config file exists and is readable
ls -la secrets/.operator-config.json

# Check environment variables
docker-compose config
```

## Next Steps

Congratulations! You now have a running Formation node. Here's what you can do next:

1. **[Set up a second node](two-node-setup.md)** - Learn multi-node networking
2. **Generate proper keys** - Use the form-config-wizard for production keys
3. **Deploy your first agent** - Follow the developer documentation
4. **Monitor your node** - Set up proper monitoring and alerting
5. **Join the network** - Connect to the official Formation network

## Production Considerations

Before running in production:

- [ ] Generate secure ECDSA keys using form-config-wizard
- [ ] Set up proper firewall rules and security hardening
- [ ] Configure monitoring and alerting
- [ ] Set up automated backups of your configuration and data
- [ ] Review and adjust resource limits based on expected load
- [ ] Configure log rotation and retention policies

## Getting Help

If you need assistance:

- Check the [troubleshooting guide](troubleshooting.md)
- Review service logs for error messages
- Ensure all prerequisites are properly installed
- Verify network connectivity and port accessibility 