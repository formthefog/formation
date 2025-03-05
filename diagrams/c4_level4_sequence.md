# Formation System - C4 Sequence Diagram (Level 4)

This diagram shows the dynamic flow of data and commands through the Formation system.

```mermaid
sequenceDiagram
    participant User as User/Developer
    participant CLI as form-cli
    participant P2P as form-p2p
    participant VMM as form-vmm
    participant State as form-state
    participant Pack as form-pack
    participant Net as form-net
    
    User->>CLI: 1. Submit CLI Command
    CLI->>P2P: 2. Publish P2P Event
    P2P->>VMM: 3. Forward VM Event
    VMM->>State: 4. Update State
    VMM->>Pack: 5. Request VM Image
    Pack-->>VMM: 5a. Return VM Image
    VMM->>Net: 6. Configure VM Network
    Net->>P2P: 7. Publish Network Events
    P2P->>State: 8. Forward Events to Update State
    VMM-->>P2P: 9. Publish VM Ready Event
    P2P-->>CLI: 10. Forward Event to CLI
    CLI-->>User: 11. Display Results
```

## Description

This sequence diagram illustrates the typical flow of creating and configuring a VM in the Formation system:

1. The user submits a command through the CLI
2. The CLI publishes an event to the P2P layer
3. The P2P layer forwards the VM-related event to the VMM
4. The VMM updates the system state with the pending VM creation
5. The VMM requests a VM image from the form-pack component
6. The form-pack component returns the requested VM image
7. The VMM configures networking for the VM via the form-net component
8. The form-net component publishes network events to the P2P layer
9. The P2P layer forwards events to update the system state
10. The VMM publishes a "VM Ready" event to the P2P layer when the VM is ready
11. The P2P layer forwards the event to the CLI
12. The CLI displays the results to the user

This diagram shows both synchronous calls (solid arrows) and asynchronous message returns (dashed arrows). 