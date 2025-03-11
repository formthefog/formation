# DNS Routing Flow

This diagram illustrates the DNS-based routing flow using the internal form-dns system, including geographic resolution, health checks, and how clients connect to the nearest healthy node.

```mermaid
sequenceDiagram
    participant User as Client/User
    participant DNS as form-dns Authority
    participant GEO as GeoDNS Module
    participant HC as Health Monitor
    participant Node1 as Node 1 (US)
    participant Node2 as Node 2 (EU)
    participant Node3 as Node 3 (Asia)
    
    Note over User, Node3: Initial Setup
    HC->>Node1: Periodic health check
    Node1->>HC: Health status: Healthy
    HC->>Node2: Periodic health check
    Node2->>HC: Health status: Healthy
    HC->>Node3: Periodic health check
    Node3->>HC: Health status: Degraded
    HC->>GEO: Update node health statuses
    GEO->>DNS: Update DNS resolution logic (filter unhealthy nodes)
    
    Note over User, Node3: User Connection (US Region)
    User->>DNS: DNS query for bootstrap.formation.network
    DNS->>GEO: Forward query with client geolocation data
    GEO->>DNS: Return IP of nearest healthy node (Node1)
    DNS->>User: Resolve to Node1 IP
    User->>Node1: Connect to Formation network
    
    Note over User, Node3: User Connection (EU Region)
    User->>DNS: DNS query for bootstrap.formation.network
    DNS->>GEO: Forward query with client geolocation data
    GEO->>DNS: Return IP of nearest healthy node (Node2)
    DNS->>User: Resolve to Node2 IP
    User->>Node2: Connect to Formation network
    
    Note over User, Node3: Node Failure Scenario
    HC->>Node1: Periodic health check
    Node1->>HC: Health status: Failed
    HC->>GEO: Update node health status (Node1 down)
    GEO->>DNS: Update DNS resolution logic (remove Node1)
    DNS->>DNS: Apply TTL settings (60 seconds)
    
    User->>DNS: DNS query for bootstrap.formation.network
    DNS->>GEO: Forward query with client geolocation data
    GEO->>DNS: Return IP of next nearest healthy node (Node2)
    DNS->>User: Resolve to Node2 IP
    User->>Node2: Connect to Formation network
``` 