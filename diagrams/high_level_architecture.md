# High-Level Architecture

This diagram provides an overview of the virtual Anycast system architecture, showing how the form-dns GeoDNS implementation and the private BGP overlay work together within the Formation network.

```mermaid
graph TB
    subgraph Internet
        User[User/Client]
    end
    
    subgraph "Formation Network"
        subgraph "GeoDNS Layer (form-dns)"
            GEO[Geographic Resolution]
            HC[Health Integration]
            DNSMGMT[DNS Management]
        end
        
        subgraph "Private BGP Overlay"
            BGP[BGP Daemon]
            VA[Virtual Anycast IP]
            RP[Route Propagation]
            PBR[Policy-Based Routing]
        end
        
        subgraph "Formation Nodes"
            N1[Node 1]
            N2[Node 2]
            N3[Node 3]
            N4[Node 4]
        end
        
        subgraph "Health Monitoring (form-node-metrics)"
            HM[Health Monitor]
            RT[Route Table Updates]
        end
    end
    
    User -->|1. DNS Request| GEO
    GEO -->|2. Return Nearest Healthy Node IP| User
    User -->|3. Connect to Node| N1
    
    HC <-->|Health Status| GEO
    HC -->|Monitor| N1
    HC -->|Monitor| N2
    HC -->|Monitor| N3
    HC -->|Monitor| N4
    
    HM -->|Health Data| HC
    HM -->|Health Status| RT
    N1 -->|Report Health| HM
    N2 -->|Report Health| HM
    N3 -->|Report Health| HM
    N4 -->|Report Health| HM
    
    RT -->|Update Routes| BGP
    BGP <-->|BGP Sessions| N1
    BGP <-->|BGP Sessions| N2
    BGP <-->|BGP Sessions| N3
    BGP <-->|BGP Sessions| N4
    
    VA -->|Assign Anycast IPs| N1
    VA -->|Assign Anycast IPs| N2
    VA -->|Assign Anycast IPs| N3
    VA -->|Assign Anycast IPs| N4
    
    DNSMGMT -->|Configure| GEO
    
    classDef dns fill:#f9f,stroke:#333,stroke-width:2px;
    classDef bgp fill:#bbf,stroke:#333,stroke-width:2px;
    classDef node fill:#bfb,stroke:#333,stroke-width:2px;
    classDef health fill:#fbf,stroke:#333,stroke-width:2px;
    
    class GEO,HC,DNSMGMT dns;
    class BGP,VA,RP,PBR bgp;
    class N1,N2,N3,N4 node;
    class HM,RT health;
``` 