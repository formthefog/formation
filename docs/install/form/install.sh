#!/bin/bash
set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Define installation paths
INSTALL_DIR="/usr/local/bin"
VERSION_FILE="/usr/local/share/formation/.version"
TEMP_DIR=$(mktemp -d)

# Base URLs
RELEASE_BASE="https://dev.formation.cloud/install"

# Cleanup function
cleanup() {
    local exit_code=$?
    echo "Cleaning up temporary files..."
    rm -rf "$TEMP_DIR"
    if [ $exit_code -ne 0 ]; then
        echo -e "${RED}Installation failed. Rolling back changes...${NC}"
        rm -f "$INSTALL_DIR/form"
        rm -f "$INSTALL_DIR/formnet-up"
    fi
}
trap cleanup EXIT

# Helper functions
log() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

error() {
    echo -e "${RED}[ERROR]${NC} $1" >&2
    exit 1
}

warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

verify_checksum() {
    local file=$1
    local expected_checksum=$2
    log "Verifying checksum for $(basename "$file")..."
    local actual_checksum
    if command -v sha256sum >/dev/null 2>&1; then
        actual_checksum=$(sha256sum "$file" | cut -d ' ' -f 1)
    elif command -v shasum >/dev/null 2>&1; then
        actual_checksum=$(shasum -a 256 "$file" | cut -d ' ' -f 1)
    else
        error "No checksum utility found"
    fi
    
    if [ "$actual_checksum" != "$expected_checksum" ]; then
        error "Checksum verification failed for $(basename "$file")"
    fi
}

get_platform() {
    local platform=$(uname -s)
    local arch=$(uname -m)
    
    case "$platform" in
        Linux)
            if [ "$arch" != "x86_64" ]; then
                error "Linux installation only supported on x86_64 architecture"
            fi
            echo "linux-x86_64"
            ;;
        Darwin)
            case "$arch" in
                x86_64)
                    echo "darwin-x86_64"
                    ;;
                arm64)
                    echo "darwin-arm64"
                    ;;
                *)
                    error "Unsupported macOS architecture: $arch"
                    ;;
            esac
            ;;
        *)
            error "Unsupported platform: $platform. This installer only supports Linux (x86_64) and macOS"
            ;;
    esac
}

check_dependencies() {
    local deps=("curl" "grep")
    for dep in "${deps[@]}"; do
        if ! command -v "$dep" >/dev/null 2>&1; then
            error "Required dependency not found: $dep"
        fi
    done
}

# Check for root/sudo
if [ "$EUID" -ne 0 ]; then 
    error "Please run as root or with sudo"
fi

# Check dependencies
check_dependencies

# Detect platform
PLATFORM=$(get_platform)
log "Detected platform: $PLATFORM"

# Create directories
log "Creating installation directories..."
mkdir -p "$INSTALL_DIR"
mkdir -p "$(dirname "$VERSION_FILE")"

# Download and install form
log "Downloading form..."
curl -fsSL "$RELEASE_BASE/form" -o "$TEMP_DIR/form"
chmod +x "$TEMP_DIR/form"
mv "$TEMP_DIR/form" "$INSTALL_DIR/form"

# Download and install formnet-up
log "Downloading formnet-up..."
curl -fsSL "$RELEASE_BASE/formnet-up" -o "$TEMP_DIR/formnet-up"
chmod +x "$TEMP_DIR/formnet-up"
mv "$TEMP_DIR/formnet-up" "$INSTALL_DIR/formnet-up"

# Verify installation
log "Verifying installation..."
if [ ! -x "$INSTALL_DIR/form" ]; then
    error "form binary not executable"
fi

if [ ! -x "$INSTALL_DIR/formnet-up" ]; then
    error "formnet-up binary not executable"
fi

# Verify binaries work
log "Testing binaries..."
if ! "$INSTALL_DIR/form" --version >/dev/null 2>&1; then
    error "form binary verification failed"
fi

if ! "$INSTALL_DIR/formnet-up" --version >/dev/null 2>&1; then
    error "formnet-up binary verification failed"
fi

# Installation complete
log "Installation successful!"
echo -e "${GREEN}form and formnet-up have been installed to $INSTALL_DIR${NC}"

# Usage instructions
cat << EOF

Usage:
  form [options] <command>
  formnet-up [options] <command>

For more information, run:
  form --help
  formnet-up --help

EOF
