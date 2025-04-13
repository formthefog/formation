# AI Marketplace Flow Diagrams

This document contains high-level flow diagrams for the Formation AI Agent and Model Marketplace, illustrating how developers can register and monetize their AI models/agents, and how users can discover, select, and deploy them.

## Developer Registration and Monetization Flow

```mermaid
flowchart TB
    subgraph "Developer Journey"
        A[Developer Creates AI Model/Agent] --> B[Package with Formation Templates]
        B --> C[Upload to Formation Network]
        C --> D[Register in Marketplace]
        
        D --> E{Set Monetization Parameters}
        E --> F[Set Usage Tracking]
        E --> G[Configure Royalty Percentage]
        E --> H[Define Pricing Model]
        
        F & G & H --> I[Publish to Marketplace]
        I --> J[Monitor Usage & Analytics]
        J --> K[Receive Royalties]
        
        L[Update Model/Agent] --> I
    end
    
    subgraph "Formation Protocol"
        M[form-state CRDT Registry] --- D
        N[form-pack Templates] --- B
        O[form-vmm Deployment] --- C
        P[form-usage-events] --- J
        Q[Royalty Calculation System] --- K
    end
    
    style A fill:#d4f1f9,stroke:#05668d
    style I fill:#c1e1c1,stroke:#05668d
    style J fill:#c1e1c1,stroke:#05668d
    style K fill:#c1e1c1,stroke:#05668d
```

### Detail: Model/Agent Registration Process

```mermaid
sequenceDiagram
    participant Dev as Developer
    participant CLI as form-cli
    participant Registry as Marketplace Registry (form-state)
    participant Pack as form-pack
    participant P2P as form-p2p
    
    Dev->>CLI: Register model/agent with metadata
    CLI->>Registry: Submit model/agent data
    Registry->>Registry: Validate metadata
    Registry->>Registry: Generate unique ID
    Registry->>P2P: Broadcast registration event
    P2P->>Registry: Sync across network nodes
    Registry-->>CLI: Registration confirmation
    
    Dev->>CLI: Upload model/agent assets
    CLI->>Pack: Submit Formfile template
    Pack->>Pack: Build VM image
    Pack->>P2P: Distribute build across network
    Pack-->>CLI: Distribution complete
    CLI-->>Dev: Registration & upload complete
    
    note over Registry,P2P: Model/Agent metadata replicated via CRDT
    note over Pack,P2P: Model/Agent builds stored on responsible nodes
```

### Detail: Revenue & Royalty System

```mermaid
flowchart LR
    subgraph "Usage Tracking"
        A[User Deploys Model/Agent] --> B[form-usage-events Collector]
        B --> C[Usage Metrics in form-state]
    end
    
    subgraph "Royalty Processing"
        C --> D[Usage Aggregation]
        D --> E[Apply Royalty Percentage]
        E --> F[Calculate Creator Earnings]
        F --> G[Process Payments]
        G --> H[Developer Receives Royalties]
    end
    
    style A fill:#d4f1f9,stroke:#05668d
    style H fill:#c1e1c1,stroke:#05668d
```

## User Discovery and Deployment Flow

```mermaid
flowchart TB
    subgraph "User Journey"
        A[User Accesses Marketplace] --> B[Browse/Search for AI Solutions]
        
        B --> C[Select Agent]
        B --> D[Select Model]
        
        C --> E[Configure Agent Settings]
        D --> F[Configure Model Settings]
        
        E & F --> G[Combine Agent & Model]
        G --> H[Deploy Combined Solution]
        H --> I[Use AI Solution]
        I --> J[Pay for Usage]
    end
    
    subgraph "Formation Protocol"
        K[Marketplace Registry] --- B
        L[form-pack Deployment] --- G
        M[form-vmm Execution] --- H
        N[form-inference API] --- I
        O[Usage Tracking] --- J
    end
    
    style A fill:#d4f1f9,stroke:#05668d
    style I fill:#c1e1c1,stroke:#05668d
    style J fill:#ffcccc,stroke:#05668d
```

### Detail: Agent & Model Selection Process

```mermaid
flowchart TD
    A[User Need Identification] --> B[Search Marketplace]
    
    B --> C{Choose Approach}
    C --> D[Select Agent First]
    C --> E[Select Model First]
    C --> F[Choose Pre-combined Solution]
    
    D --> G[Filter Compatible Models]
    G --> H[Select Specific Model]
    D & H --> I[Configure Agent-Model Pairing]
    
    E --> J[Filter Agents Compatible with Model]
    J --> K[Select Specific Agent]
    E & K --> I
    
    F --> I
    
    I --> L[Customize Configuration]
    L --> M[Specify Resource Requirements]
    M --> N[Deploy to Formation Network]
    
    style A fill:#d4f1f9,stroke:#05668d
    style N fill:#c1e1c1,stroke:#05668d
```

### Detail: Deployment Sequence

```mermaid
sequenceDiagram
    participant User
    participant CLI as form-cli
    participant State as form-state
    participant Pack as form-pack
    participant VMM as form-vmm
    participant Inference as form-inference
    
    User->>CLI: Deploy agent with model
    CLI->>State: Query model & agent metadata
    State-->>CLI: Return metadata
    CLI->>Pack: Request build preparation
    Pack->>Pack: Prepare deployment package
    Pack-->>CLI: Deployment ready
    
    CLI->>VMM: Request VM deployment
    VMM->>VMM: Create and configure VM
    VMM->>VMM: Apply resource requirements
    VMM->>Inference: Register inference endpoint
    VMM-->>CLI: Deployment complete
    
    CLI-->>User: Deployment success with endpoint info
    User->>Inference: Interact with agent/model
    
    note over State,Pack: Registry provides metadata and build templates
    note over VMM,Inference: Deployed agent connects to model via form-inference
```

## AI Marketplace Architecture Overview

```mermaid
flowchart TB
    subgraph "Formation Core Infrastructure"
        VMM[form-vmm: VM Management]
        Net[form-net: Network Layer]
        P2P[form-p2p: Node Communication]
        State[form-state: Distributed Datastore]
        Pack[form-pack: VM Image Building]
    end
    
    subgraph "AI Marketplace Extensions"
        Registry[AI Marketplace Registry]
        Templates[AI-Specific Build Templates]
        Inference[form-inference: AI Inference Protocol]
        Usage[Usage Tracking & Royalties]
        Discovery[Model & Agent Discovery]
    end
    
    subgraph "User Interfaces"
        CLI[Command Line Interface]
        API[API Access]
        WebUI[Web Dashboard]
    end
    
    %% Core Infrastructure Connections
    State <--> P2P
    VMM <--> P2P
    Net <--> P2P
    Pack <--> P2P
    
    %% Marketplace Extension Connections
    Registry <--> State
    Templates <--> Pack
    Inference <--> VMM
    Usage <--> State
    Discovery <--> Registry
    
    %% User Interface Connections
    CLI --> Registry
    CLI --> Templates
    CLI --> Inference
    API --> Registry
    API --> Inference
    WebUI --> Discovery
    WebUI --> Registry
    
    style Registry fill:#f9d4f9,stroke:#05668d
    style Templates fill:#f9d4f9,stroke:#05668d
    style Inference fill:#f9d4f9,stroke:#05668d
    style Usage fill:#f9d4f9,stroke:#05668d
    style Discovery fill:#f9d4f9,stroke:#05668d
```

This diagram shows how the AI Marketplace components (in purple) extend the core Formation infrastructure, providing AI-specific functionality while building on the existing distributed computing capabilities. 
