# Formation System - C4 Component Diagrams (Level 3)

This file contains component-level diagrams for the main containers in the Formation system.

## form-vmm Components

```mermaid
flowchart TD
    subgraph form-vmm["form-vmm"]
        VMLifecycle["VM Lifecycle\nManager"]
        HypervisorInterface["Hypervisor\nInterface"]
        DeviceManager["Device\nManager"]
        VMAllocator["VM\nAllocator"]
        
        VMLifecycle <--> HypervisorInterface
        HypervisorInterface <--> DeviceManager
        DeviceManager <--> VMAllocator
    end
    
    classDef component fill:#85bbf0,stroke:#5d82a8,color:black
    classDef boundary fill:none,stroke:#666666,stroke-dasharray:5 5
    
    class VMLifecycle,HypervisorInterface,DeviceManager,VMAllocator component
    class form-vmm boundary
```

## form-net Components

```mermaid
flowchart TD
    subgraph form-net["form-net"]
        WireGuard["WireGuard\nInterface"]
        NetConfig["Network\nConfiguration"]
        DNS["DNS\nService"]
        HostConfig["Host\nConfiguration"]
        
        WireGuard <--> NetConfig
        NetConfig <--> DNS
        DNS <--> HostConfig
    end
    
    classDef component fill:#85bbf0,stroke:#5d82a8,color:black
    classDef boundary fill:none,stroke:#666666,stroke-dasharray:5 5
    
    class WireGuard,NetConfig,DNS,HostConfig component
    class form-net boundary
```

## form-p2p Components

```mermaid
flowchart TD
    subgraph form-p2p["form-p2p"]
        PeerDiscovery["Peer\nDiscovery"]
        MessageRouter["Message\nRouter"]
        EventPublisher["Event\nPublisher"]
        EventSubscriber["Event\nSubscriber"]
        
        PeerDiscovery <--> MessageRouter
        MessageRouter <--> EventPublisher
        EventPublisher <--> EventSubscriber
    end
    
    classDef component fill:#85bbf0,stroke:#5d82a8,color:black
    classDef boundary fill:none,stroke:#666666,stroke-dasharray:5 5
    
    class PeerDiscovery,MessageRouter,EventPublisher,EventSubscriber component
    class form-p2p boundary
```

## form-state Components

```mermaid
flowchart TD
    subgraph form-state["form-state"]
        StateStore["State\nStore"]
        ConsensusManager["Consensus\nManager"]
        CRDT["CRDT\nImplementation"]
        
        StateStore <--> ConsensusManager
        ConsensusManager <--> CRDT
    end
    
    classDef component fill:#85bbf0,stroke:#5d82a8,color:black
    classDef boundary fill:none,stroke:#666666,stroke-dasharray:5 5
    
    class StateStore,ConsensusManager,CRDT component
    class form-state boundary
```

## Description

These component diagrams illustrate the internal structure of the main containers in the Formation system:

1. **form-vmm** - The Virtual Machine Manager with components for:
   - VM lifecycle management
   - Hypervisor interface
   - Device management
   - VM resource allocation

2. **form-net** - The Networking layer with components for:
   - WireGuard interface management
   - Network configuration
   - DNS services
   - Host network configuration

3. **form-p2p** - The Peer-to-Peer messaging layer with components for:
   - Peer discovery
   - Message routing
   - Event publishing
   - Event subscription

4. **form-state** - The State Management layer with components for:
   - State storage
   - Consensus management
   - CRDT (Conflict-free Replicated Data Type) implementation 