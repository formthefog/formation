# Formation Architecture Overview

## Introduction

Formation is a public verifiable and self-replicating protocol for trustless, confidential virtual private servers (VPS) coordinating as a Fog Compute network. It's designed to power the "Age of Autonomy" by providing a decentralized computation infrastructure.

## Core Components

### 1. VMM (Virtual Machine Monitor)

The `form-vmm` component is based on Cloud Hypervisor and is responsible for managing virtual machines. It provides a service that can create, start, stop, and manage VMs with various configurations. Key features include:

- KVM-based virtualization for Linux hosts
- Support for device passthrough via VFIO
- API for VM lifecycle management
- Resource allocation and monitoring

The VMM service exposes an API that allows other components to interact with it, providing operations like:
- Creating new VM instances
- Booting, pausing, resuming VMs
- Adding devices to running VMs
- Monitoring VM state and health

### 2. VM Image Management (form-pack)

The `form-pack` system is responsible for creating and managing virtual machine images. It includes:

- **pack-manager**: Manages VM image packages, their versions, and dependencies
- **image-builder**: Builds customized VM images based on specifications
- Support for different image formats and requirements

This component ensures that properly configured VM images are available for the VMM to boot, providing a standardized way to define VM configurations and customizations.

### 3. Networking (form-net)

The networking component is based on Innernet (a WireGuard-based private network system) and provides secure networking between Formation nodes and VMs. It creates a private overlay network that allows:

- Secure communication between nodes in the network
- Private networking for VM instances
- NAT and traffic forwarding capabilities
- Peer-to-peer connections between nodes

Each VM instance gets connected to the network with its own IP address and can communicate with other VMs and services as needed.

### 4. Peer-to-Peer System (form-p2p)

The P2P component handles node discovery, messaging, and coordination between Formation nodes. It includes:

- Message queue for asynchronous communication between services
- Peer discovery mechanisms
- Network resilience and redundancy
- Event routing between services

The form-p2p message queue acts as the central communication mechanism for the entire system, allowing components to communicate without a dedicated broker.

### 5. State Management (form-state)

The state management system implements a distributed state store using a BFT-CRDT (Byzantine Fault Tolerant Conflict-free Replicated Data Type) approach. It provides:

- Globally replicated data storage
- Consistency guarantees
- State synchronization between nodes
- Distributed consensus

### 6. Command Line Interface (form-cli)

The CLI provides a user interface for interacting with the Formation system, allowing users to:

- Create and manage VM instances
- Monitor system state
- Configure network settings
- Deploy applications

### 7. Types and Communication (form-types)

This component defines the data structures and event types used throughout the system, including:

- VM events (create, start, stop, etc.)
- Network events (peer joining, heartbeats)
- Quorum events (consensus-related)
- Formnet events (network configuration)

Events are used for communication between different components and are serialized using JSON.

### 8. Supporting Services

Additional services that enhance the system's functionality:
- **form-dns**: DNS services for the network
- **form-rplb**: Load balancing services
- **form-metrics**: Performance monitoring for VMs and nodes

## Communication Patterns

Formation uses two primary communication patterns:

1. **Asynchronous Event-Based Communication (Primary)**:
   - Services publish events to the form-p2p message queue
   - Other services subscribe to and consume relevant events
   - Enables loose coupling between components
   - Provides scalability and resilience

2. **Synchronous API Calls (Secondary)**:
   - Used only when immediate responses are required
   - Direct service-to-service API calls
   - Typically used for queries or critical operations where the caller needs to wait for a result

This dual approach provides flexibility while maintaining system responsiveness.

## System Flow

1. Users interact with the system through the CLI or directly via APIs
2. Commands are either:
   - Published as events to the form-p2p message queue
   - Sent as direct API calls when immediate responses are needed
3. The form-pack system prepares VM images based on specifications
4. The VMM service deploys and manages VMs using these images
5. The networking layer establishes connections between VMs
6. The state management system keeps track of the global system state

## Security Model

The system uses a mix of security mechanisms:

- WireGuard for secure network communications
- BFT consensus for distributed decision making
- Cryptographic signatures for message verification
- Isolated VM environments for workload security

## Deployment Architecture

Formation can be deployed in various configurations:

- Single-node local testing environment
- Multi-node local development setup
- Full distributed deployment for production environments

The system requires specific network configurations, including bridge interfaces and port forwarding, to enable proper communication between components.

## Custom Virtual Machines

VMs in Formation are configured using "Formfiles" that specify VM parameters such as:

- Kernel and rootfs configurations
- Memory and CPU allocations
- Network settings
- Custom command line parameters
- Console configuration

These specifications are used by the form-pack system to build appropriate VM images.

## Relation to Cloud Hypervisor

Formation builds upon Cloud Hypervisor, providing an extended service layer with distributed capabilities. While Cloud Hypervisor is focused on VM technology, Formation wraps this with networking, coordination, and distributed state management to create a complete fog computing platform. 