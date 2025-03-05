# Formation System - C4 Context Diagram (Level 1)

This diagram shows the Formation system in context, including its relationships with users and external systems.

```mermaid
flowchart TD
    User["User/Developer\n(CLI User/API Consumer)"]
    Formation["Formation System\n(Fog Compute Network)"]
    Hardware["Hardware/Host System"]
    CloudHypervisor["Cloud-Hypervisor\n(Dependency)"]

    User <-->|VM Management,\nNetwork Config| Formation
    Formation -->|VM Execution,\nNetwork Traffic| Hardware
    CloudHypervisor <-->|Virtualization\nServices| Hardware

    classDef system fill:#1168bd,stroke:#0b4884,color:white
    classDef person fill:#08427b,stroke:#052e56,color:white
    classDef external fill:#999999,stroke:#6b6b6b,color:white

    class Formation system
    class User person
    class Hardware,CloudHypervisor external
```

## Description

This context diagram illustrates:

- **User/Developer**: Interacts with the Formation system through CLI commands and API calls
- **Formation System**: The core fog compute network that manages virtual machines and networking
- **Hardware/Host System**: The underlying physical infrastructure that hosts Formation
- **Cloud-Hypervisor**: The dependency that Formation uses for VM management

All communication between components is shown with annotated arrows that describe the nature of the interaction. 