# Formation System - Animated Data Flow Diagram

This diagram illustrates the event flow through the Formation system with highlighted paths to simulate animation.

```mermaid
flowchart TD
    User((User))
    
    subgraph FormationSystem["Formation System"]
        CLI["form-cli\n(CLI/UI)"]
        VMM["form-vmm\n(VM Manager)"]
        Net["form-net\n(Networking)"]
        Pack["form-pack\n(Image Manager)"]
        P2P["form-p2p\n(Message Queue/P2P Layer)"]
        State["form-state\n(State Manager)"]
    end
    
    User -->|1. CLI Command| CLI
    CLI -->|2. Publish Event| P2P
    P2P -->|3. VM Event| VMM
    VMM -->|4. State Update| State
    VMM -->|5. Image Request| Pack
    Pack -->|6. Image Delivery| VMM
    VMM -->|7. Network Config| Net
    Net -->|8. Network Events| P2P
    P2P -->|9. State Update Events| State
    
    %% Linkstyles for animation effect
    linkStyle 0 stroke:#ff9900,stroke-width:4px
    linkStyle 1 stroke:#ff9900,stroke-width:4px
    linkStyle 2 stroke:#ff9900,stroke-width:4px
    linkStyle 3 stroke:#ff9900,stroke-width:4px
    linkStyle 4 stroke:#ff9900,stroke-width:4px
    linkStyle 5 stroke:#ff9900,stroke-width:4px
    linkStyle 6 stroke:#ff9900,stroke-width:4px
    linkStyle 7 stroke:#ff9900,stroke-width:4px
    linkStyle 8 stroke:#ff9900,stroke-width:4px
    
    classDef container fill:#1168bd,stroke:#0b4884,color:white
    classDef person fill:#08427b,stroke:#052e56,color:white
    classDef boundary fill:none,stroke:#666666,stroke-dasharray:5 5
    
    class CLI,VMM,Net,Pack,P2P,State container
    class User person
    class FormationSystem boundary
```

## Description

This diagram shows the flow of data through the Formation system with highlighted pathways to simulate animation:

1. The user initiates the process by submitting a CLI command
2. The command is published as an event to the P2P messaging layer
3. The P2P layer routes the event to the VMM (Virtual Machine Manager)
4. The VMM updates the state in the form-state component
5. The VMM requests a VM image from the form-pack component
6. The form-pack component delivers the VM image back to the VMM
7. The VMM configures networking through the form-net component
8. The form-net component sends network events to the P2P layer
9. The P2P layer forwards events to update the system state

The highlighted orange paths (using thicker lines and a distinct color) create a visual representation of the data flow, showing how events and commands propagate through the system. 