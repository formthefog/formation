# BGP Test Environment for Formation Network

This directory contains the necessary files to create a test environment for evaluating BGP daemons and implementing virtual Anycast within the Formation network.

## Overview

The test environment consists of:

- A Docker-based virtualized network 
- Multiple BGP-enabled nodes with different BGP daemon options
- Core router node with ASN 64512
- Edge router nodes with ASNs in the private range (64513-65534)
- Support for three different BGP daemons:
  - BIRD - A lightweight and efficient BGP daemon
  - FRRouting (FRR) - A comprehensive routing suite with advanced BGP capabilities 
  - GoBGP - A modern, Go-based BGP implementation

This environment allows for testing and evaluating different BGP daemons to determine which is the most appropriate for the Formation network's virtual Anycast implementation.

## Prerequisites

- Docker
- Bash
- Linux with support for network namespaces

## Usage

### Basic Usage

```bash
# First time: Build the Docker image and run the test environment
./run-bgp-test-env.sh --build

# Subsequent runs: Start the test environment
./run-bgp-test-env.sh
```

### Advanced Options

```bash
# Build the Docker image
./run-bgp-test-env.sh --build

# Specify which BGP daemon to use (bird, frr, gobgp)
./run-bgp-test-env.sh --daemon=frr

# Specify the number of edge nodes (min: 2)
./run-bgp-test-env.sh --nodes=5

# Interactive mode (won't automatically clean up after 60 seconds)
./run-bgp-test-env.sh --interactive

# Combine options
./run-bgp-test-env.sh --build --daemon=gobgp --nodes=4 --interactive
```

## Testing Different BGP Daemons

The environment allows testing all three BGP daemons to compare:

1. Performance
2. Memory usage
3. Configuration ease
4. Feature completeness
5. Stability

### BIRD (Default)

BIRD is lightweight and has good performance characteristics. To test BIRD:

```bash
./run-bgp-test-env.sh --daemon=bird --interactive
```

To check BGP status with BIRD:
```bash
docker exec -it bgp-node-core birdc show protocols
docker exec -it bgp-node-core birdc show route
```

### FRRouting (FRR)

FRR is a comprehensive routing suite with advanced features. To test FRR:

```bash
./run-bgp-test-env.sh --daemon=frr --interactive
```

To check BGP status with FRR:
```bash
docker exec -it bgp-node-core vtysh -c "show ip bgp summary"
docker exec -it bgp-node-core vtysh -c "show ip route"
```

### GoBGP

GoBGP is a modern, Go-based implementation. To test GoBGP:

```bash
./run-bgp-test-env.sh --daemon=gobgp --interactive
```

To check BGP status with GoBGP:
```bash
docker exec -it bgp-node-core gobgp neighbor
docker exec -it bgp-node-core gobgp global rib
```

## Network Structure

The test environment creates a Docker network with the following structure:

- Core Router: 172.20.0.1 (AS 64512)
- Edge Node 1: 172.20.0.2 (AS 64513)
- Edge Node 2: 172.20.0.3 (AS 64514)
- (Additional nodes follow the same pattern)

Each node also has its own internal networks:
- Core Router: 10.10.1.0/24
- Edge Node 1: 10.10.2.0/24
- Edge Node 2: 10.10.3.0/24

## Cleanup

In standard mode, the test environment runs for 60 seconds and then automatically cleans up.

In interactive mode (`--interactive`), you'll need to clean up manually:

```bash
# Stop and remove containers
docker stop $(docker ps -a -q --filter "name=bgp-node-")
docker rm $(docker ps -a -q --filter "name=bgp-node-")

# Remove network
docker network rm bgp-test-net
```

## Files

- `Dockerfile.bgp-test` - Docker image definition with BGP daemons installed
- `setup_network.sh` - Script to configure network interfaces and BGP daemons
- `run-bgp-test-env.sh` - Main script to build and run the test environment

## Next Steps

After evaluating the different BGP daemons, choose the most appropriate one for implementing the virtual Anycast solution in the Formation network. 