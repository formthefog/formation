# Formation System - C4 Container Diagram (Level 2)

This diagram shows the major containers (components) of the Formation system and their relationships.

```mermaid
flowchart TD
    subgraph FormationSystem["Formation System"]
        CLI["form-cli\n(CLI/UI)"]
        VMM["form-vmm\n(VM Manager)"]
        Net["form-net\n(Networking)"]
        Pack["form-pack\n(Image Manager)"]
        P2P["form-p2p\n(Message Queue/P2P Layer)"]
        State["form-state\n(State Manager)"]
        
        CLI <--> VMM
        VMM <--> Net
        Net <--> Pack
        
        CLI <--> P2P
        VMM <--> P2P
        Net <--> P2P
        Pack <--> P2P
        
        P2P <--> State
    end
    
    User["User/Developer"] <--> CLI
    
    classDef container fill:#1168bd,stroke:#0b4884,color:white
    classDef person fill:#08427b,stroke:#052e56,color:white
    classDef boundary fill:none,stroke:#666666,stroke-dasharray:5 5
    
    class CLI,VMM,Net,Pack,P2P,State container
    class User person
    class FormationSystem boundary
```

## Description

This container diagram illustrates the main components of the Formation system:

- **form-cli**: The command-line interface and user interaction layer
- **form-vmm**: Virtual machine manager based on Cloud Hypervisor
- **form-net**: Network management based on WireGuard
- **form-pack**: VM image management and packaging
- **form-p2p**: Peer-to-peer messaging system that acts as a message queue
- **form-state**: Distributed state management with BFT-CRDT

The arrows show the communication paths between components, primarily flowing through the form-p2p layer which serves as the central messaging bus for the system. Components can also communicate directly via API calls when immediate responses are needed. 