# Integration Points

This diagram visualizes how the virtual Anycast system integrates with the existing Formation network components, leveraging the form-dns, form-rplb, and form-state infrastructure.

```mermaid
graph TB
    subgraph "Virtual Anycast System"
        GEO[Geographic Resolution]
        BGP[Private BGP Overlay]
        HM[Health Monitoring]
        IPAM[Anycast IP Manager]
    end
    
    subgraph "Existing Formation Network"
        FORM[formnet Core]
        WG[WireGuard Service]
        JOIN[Join Process]
        BOOT[Bootstrap Process]
        P2P[P2P Discovery]
        NAT[NAT Traversal]
        RELAY[Relay Services]
        FDNS[form-dns]
        FRPLB[form-rplb]
        FSTATE[form-state]
        FMETRICS[form-node-metrics]
    end
    
    %% Integration connections
    GEO -->|Enhances| FDNS
    FDNS -->|Uses| GEO
    
    BGP -->|Optimizes| P2P
    P2P -->|Leverages| BGP
    
    HM -->|Extends| FMETRICS
    FMETRICS -->|Provides data to| HM
    
    IPAM -->|Assigns IPs to| WG
    BGP -->|Integrates with| WG
    
    FSTATE -->|Stores| GEO
    FSTATE -->|Stores| BGP
    
    WG -->|Uses| BGP
    JOIN -->|Uses| FDNS
    BOOT -->|Enhanced by| IPAM
    P2P -->|Enhanced by| IPAM
    
    FRPLB -->|Uses| GEO
    FRPLB -->|Routes based on| HM
    
    %% Configuration flows
    CONFIG[Configuration Service] -->|Configures| GEO
    CONFIG -->|Configures| BGP
    CONFIG -->|Configures| HM
    CONFIG -->|Configures| IPAM
    CONFIG -->|Configures| FORM
    
    %% User flows
    USER[User/Client] -->|Connects via| FDNS
    USER -->|Uses| WG
    USER -->|Joins via| JOIN
    
    %% Integration with core functionality
    FORM -->|Core Network| WG
    FORM -->|Bootstrap Process| BOOT
    FORM -->|P2P Connectivity| P2P
    FORM -->|NAT Traversal| NAT
    FORM -->|Relay Mechanism| RELAY
    
    classDef virtual fill:#bbf,stroke:#333,stroke-width:2px;
    classDef existing fill:#bfb,stroke:#333,stroke-width:2px;
    classDef config fill:#f96,stroke:#333,stroke-width:2px;
    classDef user fill:#c9f,stroke:#333,stroke-width:2px;
    
    class GEO,BGP,HM,IPAM virtual;
    class FORM,WG,JOIN,BOOT,P2P,NAT,RELAY,FDNS,FRPLB,FSTATE,FMETRICS existing;
    class CONFIG config;
    class USER user;
``` 