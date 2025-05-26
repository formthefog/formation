# Formation Operators Guide

Welcome to the Formation Operators documentation. This guide is designed for contributors to the network, local test network operators, and developer network operators who want to run and maintain Formation nodes.

## What is a Formation Operator?

A Formation Operator is responsible for running and maintaining one or more nodes in the Formation network. As an operator, you'll be managing the core infrastructure that powers the Formation AI marketplace - a vertically integrated platform for AI agents and models.

## Formation Network Architecture

Formation consists of five core services that work together to provide a decentralized AI marketplace:

### Core Services

- **`form-state`** (Port 3004): The central state management service that maintains network state, user accounts, instances, and marketplace data. Acts as the source of truth for the entire network.

- **`form-dns`** (Port 53/5453): Provides DNS resolution within the Formation network, enabling service discovery and human-readable names for instances and services.

- **`form-net`** (Port 51820): Establishes a secure WireGuard-based mesh network (Formnet) that enables encrypted communication between nodes, instances, and users.

- **`form-vmm`** (Port 3002): The Virtual Machine Monitor responsible for creating, running, and managing the lifecycle of virtual machines that host AI agents and models.

- **`form-pack-manager`** (Port 3003): Handles the packaging of AI models and agents from `Formfile` definitions into runnable VM images.

### Network Types

Formation supports different network configurations:

- **Development Network (devnet)**: Local testing environment with direct API communication
- **Production Network**: Full peer-to-peer network with queue-based communication
- **Local Test Network**: Single or multi-node setup for development and testing

## Prerequisites

Before you begin operating a Formation node, ensure you have:

### System Requirements

- **Operating System**: Ubuntu 22.04 LTS (or compatible Linux distribution)
- **Minimum Resources**: 4+ CPU cores, 8GB RAM, 50GB+ free disk space
- **Recommended Resources**: 8+ CPU cores, 16GB+ RAM, 100GB+ free disk space
- **For production nodes**: 32+ physical cores, 64GB+ RAM, 8TB+ storage

### Required Software

- Docker and Docker Compose
- `curl` for API interactions
- `bridge-utils` for network configuration
- Network bridge (`br0`) configured for VM networking

### Network Configuration

Formation requires specific network setup:

- **Ports to expose**: 3002-3004 (TCP), 51820 (UDP), 53 (UDP, optional)
- **Bridge interface**: `br0` for VM networking with NAT and DHCP
- **IP forwarding**: Enabled for VM internet access

## Quick Start Options

Formation offers multiple deployment methods:

### Option 1: Docker Compose (Recommended)
The easiest way to get started is using the provided Docker Compose configuration that includes all five core services.

### Option 2: Manual Binary Deployment
For advanced users who want more control over individual service configuration and deployment.

### Option 3: Joining Existing Network
Connect your node to an existing Formation network as a joining node rather than a bootstrap node.

## Key Concepts

### Bootstrap vs Joining Nodes

- **Bootstrap Node**: The first node in a network that initializes the network state and helps other nodes discover and join the network
- **Joining Node**: A node that connects to an existing network by contacting one or more bootstrap nodes

### Operator Configuration

The `form-config-wizard` tool helps you create an operator configuration file (`.operator-config.json`) that contains:

- **Network ID**: Identifies which Formation network to join (1=Mainnet, 2=Jericho, 3=Uruk, >50000=Local)
- **Cryptographic Keys**: ECDSA keypair for node identity and transaction signing
- **Network Settings**: Bootstrap nodes, regions, and port configurations
- **Admin Settings**: Initial admin public key for network administration
- **Service Ports**: Custom port assignments for each service

### Formnet Subnet

Each Formation network operates on a configurable private IP range (CIDR) for internal communication:

- Default: `10.42.0.0/16`
- Configurable during network initialization
- Ensures isolation and secure communication between nodes

## Common Operator Tasks

As a Formation operator, you'll typically perform these tasks:

### Initial Setup
1. Configure your system with required dependencies
2. Set up network bridge and port forwarding
3. Run the configuration wizard to create operator config
4. Deploy and start Formation services

### Ongoing Operations
- Monitor service health using built-in health endpoints
- Check network connectivity and peer status
- Update node capabilities and metrics
- Manage operator keys and admin access

### Troubleshooting
- Diagnose network connectivity issues
- Resolve service startup problems
- Debug authentication and key management issues
- Monitor logs for errors and warnings

## Service Health Monitoring

Each Formation service provides health check endpoints:

- `form-state`: `curl http://localhost:3004/health`
- `form-vmm`: `curl http://localhost:3002/health` 
- `form-pack-manager`: `curl http://localhost:3003/health`
- `form-net`: `curl http://localhost:51820/health`
- `form-dns`: `dig @localhost -p 5453 formation +short`

## Authentication and Security

Formation uses ECDSA-based authentication for all operations:

- **Node Identity**: Each node has a unique ECDSA keypair
- **Admin Access**: Configurable admin keys for network management
- **API Security**: All state-changing operations require signed requests
- **Network Encryption**: WireGuard provides encrypted mesh networking

## Getting Help

If you encounter issues while operating your Formation node:

1. Check the service logs using `docker-compose logs <service-name>`
2. Verify network connectivity between services
3. Confirm your operator configuration is correct
4. Review the troubleshooting guides in this documentation
5. Check GitHub issues for known problems and solutions

## Next Steps

Choose your deployment path:

- **[Quick Start Guide](quick-start.md)** - Get a single node running quickly
- **[Two-Node Setup](two-node-setup.md)** - Learn multi-node networking
- **[Troubleshooting](troubleshooting.md)** - Solve common problems

Ready to become a Formation operator and contribute to the decentralized AI marketplace? Let's get started! 