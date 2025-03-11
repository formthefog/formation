# Vanity Domain Provisioning Sequence Diagrams

This document provides sequence diagrams for the key workflows in the Vanity Domain Provisioning feature. These diagrams help visualize how components interact and serve as a reference for implementation.

## 1. Current Implementation: Manual Domain Registration

The following diagram shows the current implemented flow for manually adding a domain:

```mermaid
sequenceDiagram
    participant User
    participant CLI as Form CLI
    participant API as Formation API
    participant DNS as Form DNS Server
    participant RPLB as Reverse Proxy/LB
    participant VM as VM Instances

    User->>CLI: form dns add --domain example.fog --build-id XYZ
    CLI->>API: POST /dns/{domain}/{build_id}/request_vanity
    API->>DNS: Create DNS record via /record/create
    DNS->>DNS: Store record in DnsStore
    DNS->>RPLB: Configure proxy & TLS (if enabled)
    DNS->>API: Return success response
    API->>CLI: Return IP addresses and other info
    CLI->>User: Display success message with IP information
```

## 2. Target Implementation: Automatic Domain Provisioning

The following diagram shows how automatic domain provisioning should work with instance creation:

```mermaid
sequenceDiagram
    participant User
    participant CLI as Form CLI
    participant API as Formation API
    participant VMM as VM Manager
    participant DNS as Form DNS Server
    participant RPLB as Reverse Proxy/LB
    participant VM as VM Instances

    User->>CLI: form manage create --domain auto|custom
    CLI->>API: Create instance request
    API->>VMM: Create VM instances
    VMM->>VMM: Deploy instances with IPs
    VMM->>API: Return instance IPs & metadata
    
    alt Automatic domain requested
        API->>DNS: Create default domain (build-id.fog)
    else Custom domain provided
        API->>DNS: Create custom domain record
    end
    
    DNS->>DNS: Store record in DnsStore
    DNS->>RPLB: Configure proxy & TLS certificate
    DNS->>API: Return domain configuration result
    API->>CLI: Return instance info with domain details
    CLI->>User: Display success with instance & domain info
```

## 3. Domain Update Workflow

This diagram shows the flow for updating an existing domain:

```mermaid
sequenceDiagram
    participant User
    participant CLI as Form CLI
    participant API as Formation API
    participant DNS as Form DNS Server
    participant RPLB as Reverse Proxy/LB

    User->>CLI: form dns update --domain example.fog [options]
    CLI->>API: POST /dns/{domain}/update
    API->>DNS: Update record via /record/{domain}/update
    DNS->>DNS: Modify record in DnsStore
    DNS->>RPLB: Update proxy & TLS configuration
    DNS->>API: Return update result
    API->>CLI: Return updated configuration
    CLI->>User: Display success/failure message
```

## 4. Domain Verification Workflow

This diagram shows the proposed flow for verifying ownership of custom domains:

```mermaid
sequenceDiagram
    participant User
    participant CLI as Form CLI
    participant API as Formation API
    participant DNS as Form DNS Server
    participant ExtDNS as External DNS Servers

    User->>CLI: form dns verify --domain example.com
    CLI->>API: POST /dns/{domain}/initiate_verification
    API->>DNS: Request domain verification
    
    DNS->>ExtDNS: Query domain's A/CNAME records
    ExtDNS->>DNS: Return current DNS records
    
    DNS->>DNS: Check if records point to our network nodes
    
    alt Domain already points to our network
        DNS->>API: Domain ownership verified
        API->>CLI: Return verification success
        CLI->>User: Display successful verification message
    else Domain doesn't point to our network
        DNS->>API: Return verification instructions
        API->>CLI: Return required DNS configuration
        CLI->>User: Display instructions to update DNS records
    end
    
    Note over User,ExtDNS: User updates their domain's DNS settings if needed
    
    User->>CLI: form dns verify --domain example.com --check
    CLI->>API: POST /dns/{domain}/check_verification
    API->>DNS: Re-check domain configuration
    DNS->>ExtDNS: Query domain's A/CNAME records again
    ExtDNS->>DNS: Return updated DNS records
    DNS->>DNS: Verify records point to our network
    DNS->>API: Return verification status
    API->>CLI: Return verification result
    CLI->>User: Display verification success/failure
```

## 5. DNS Propagation Check Workflow

This diagram shows the proposed flow for checking DNS propagation:

```mermaid
sequenceDiagram
    participant User
    participant CLI as Form CLI
    participant API as Formation API
    participant DNS as Form DNS Server
    participant ExtDNS as External DNS Servers

    User->>CLI: form dns check-propagation --domain example.fog
    CLI->>API: GET /dns/{domain}/check_propagation
    API->>DNS: Initiate propagation check
    
    loop For each external DNS server
        DNS->>ExtDNS: Query for domain records
        ExtDNS->>DNS: Return query results
        DNS->>DNS: Compare with expected values
    end
    
    DNS->>API: Return propagation status
    API->>CLI: Return propagation details
    CLI->>User: Display propagation status with details
```

## 6. Domain Templates Workflow

This diagram shows the proposed flow for organization domain templates:

```mermaid
sequenceDiagram
    participant Admin
    participant User
    participant CLI as Form CLI
    participant API as Formation API
    participant DNS as Form DNS Server

    Admin->>CLI: form dns template create --name prod-template
    CLI->>API: POST /dns/templates/create
    API->>DNS: Store template configuration
    DNS->>API: Return template creation status
    API->>CLI: Return template details
    CLI->>Admin: Display template creation success
    
    User->>CLI: form manage create --domain-template prod-template
    CLI->>API: Create instance with template reference
    API->>DNS: Generate domain using template
    DNS->>DNS: Apply template rules to generate domain
    DNS->>API: Return generated domain details
    API->>CLI: Return instance & domain details
    CLI->>User: Display success with generated domain
```

These sequence diagrams provide a clear visualization of how the Vanity Domain Provisioning feature should work in both its current state and its target implementation. They will serve as a guide for implementing the remaining tasks and ensuring proper integration between components. 