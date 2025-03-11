# Virtual Anycast Implementation Plan for Formation Network

## 1. Introduction and Overview

### 1.1 Purpose

This document outlines a comprehensive plan for implementing a virtual Anycast routing solution for the Formation network. The goal is to enable all nodes in the network to appear as if they share a common IP address, allowing users to connect to the nearest available node automatically without requiring specific bootstrap nodes, but without the need for an actual ASN or public IP block.

### 1.2 Current Architecture

The Formation network currently uses a peer-to-peer architecture with the following components:

1. **formnet**: A WireGuard-based P2P VPN tunnel that connects nodes in the network
2. **Bootstrap Nodes**: Initial entry points for new nodes to join the network
3. **NAT Traversal**: Mechanisms to establish connections through NAT, including relay functionality
4. **Peer Discovery**: Methods for nodes to discover and connect to other nodes in the network

### 1.3 Proposed Architecture with Virtual Anycast

The proposed architecture will primarily use DNS-based routing with geographic awareness and health-based filtering:

1. **DNS-Based Geographic Routing**: Use DNS with low TTL values to route users to geographically close entry points
2. **Health-Based Routing**: Automatically update DNS based on node health information
3. **Seamless Failover**: If a node fails, traffic will be automatically rerouted to another node
4. **Improved Reliability**: No single point of failure for network entry 

*Note: While a private BGP overlay was originally considered as part of this plan, our implementation focus is on the DNS-based approach which provides the core functionality needed.*

*For a high-level overview of this architecture, see the [High-Level Architecture Diagram](diagrams/high_level_architecture.md).*

## 2. Technical Background

### 2.1 DNS-Based Routing

DNS-based routing uses the Domain Name System to direct users to appropriate servers:

- **GeoDNS**: Routes users to servers based on geographic location
- **Health Checks**: Removes unhealthy servers from DNS records
- **Low TTL**: Enables quick updates to routing by setting short cache times
- **Multiple A/AAAA Records**: Allows for multiple potential endpoints

### 2.2 Private BGP Overlay

Border Gateway Protocol (BGP) can be implemented within a private network:

- **Private ASNs**: Use ASNs from the private range (64512-65534)
- **Virtual Anycast**: Multiple nodes can advertise the same private IP addresses
- **Route Propagation**: Routing information is shared between nodes
- **Policy-Based Routing**: Traffic can be directed based on custom policies

### 2.3 Integration Points with Formation Network

The virtual Anycast implementation will integrate with the Formation network at several key points:

1. **Bootstrap Process**: Replace or augment the current bootstrap mechanism with DNS-based discovery
2. **Node Registration**: Allow nodes to register themselves as virtual Anycast endpoints
3. **Health Monitoring**: Ensure nodes only advertise routes when healthy
4. **Failover Handling**: Manage graceful failover between nodes 

### 2.4 Architecture Diagrams

Detailed architecture diagrams are available in the `diagrams` directory as Markdown files with embedded Mermaid diagrams:

1. **[High-Level Architecture](diagrams/high_level_architecture.md)**: Overview of the virtual Anycast system architecture, showing how DNS-based routing and the private BGP overlay work together.

2. **[DNS Routing Flow](diagrams/dns_routing_flow.md)**: Illustrates the DNS-based routing flow, including GeoDNS routing, health checks, and how clients connect to the nearest healthy node.

3. **[BGP Overlay Design](diagrams/bgp_overlay_design.md)**: Shows the private BGP network topology, including ASN allocation, route propagation, and virtual Anycast IP allocation.

4. **[Health Monitoring System](diagrams/health_monitoring_system.md)**: Depicts the health monitoring architecture, including health checks, reporting flow, and how health status affects routing decisions.

5. **[Integration Points](diagrams/integration_points.md)**: Visualizes how the virtual Anycast system integrates with the existing Formation network components.

These diagrams can be viewed in any Markdown viewer that supports Mermaid diagrams, including GitHub and many modern text editors.

## 3. Detailed Implementation Plan

### 3.1 DNS-Based Routing Implementation

#### 3.1.1 Form-DNS Extension for GeoDNS

- **Task**: Extend the existing `form-dns` infrastructure with GeoDNS capabilities
- **Subtasks**:
  - [x] Analyze the current DNS authority implementation in `form-dns`
  - [x] Extend the DNS resolution logic to include client location awareness
  - [x] Implement response selection based on geographic proximity

*For details on the DNS routing architecture, see [DNS Routing Flow Diagram](diagrams/dns_routing_flow.md).*

#### 3.1.2 Geographic Resolution Implementation

- **Task**: Implement geographic-based DNS resolution in `form-dns`
- **Subtasks**:
  - [x] Create IP geolocation database integration or service
  - [x] Implement logic to determine client location from DNS queries

#### 3.1.3 Health-Based DNS Updates

- **Task**: Enhance `form-dns` to use health data from `form-node-metrics` and `form-state`
- **Subtasks**:
  - [x] Design health data integration structure for tracking unhealthy node IPs
  - [x] Implement health status tracking for individual IP addresses
  - [x] Modify DNS resolution to filter unhealthy IPs from responses
  - [x] Add observability for health-based DNS filtering decisions

**Future Enhancements**:
  - Implement variable TTL adjustment based on node health status
  - Advanced caching strategies for improved performance
  - Regional-specific health degradation handling
  - Integrate DNS health metrics with form-node-metrics for unified observability
  - Create dashboard visualizations for DNS health filtering operations

### 3.2 Private BGP Overlay Implementation (Optional)

*Note: This section is maintained for reference purposes only. Our current implementation focuses on the DNS-based approach described in section 3.1, which provides the necessary functionality without the complexity of a BGP overlay.*

#### 3.2.1 BGP Daemon Selection and Integration

- **Task**: Select and integrate a BGP daemon for the private network
- **Subtasks**:
  - Evaluate BGP daemon options (BIRD, FRRouting, GoBGP, etc.)
  - Create Rust bindings or integration points for the selected daemon
  - Implement configuration generation for the BGP daemon
  - Develop monitoring and management interfaces

*For a detailed view of the BGP network topology, see [BGP Overlay Design Diagram](diagrams/bgp_overlay_design.md).*

#### 3.2.2 Private BGP Network Design

- **Task**: Design the private BGP network topology
- **Subtasks**:
  - Allocate private ASNs to nodes
  - Design IP addressing scheme for the overlay network
  - Create routing policies for traffic optimization
  - Implement security measures for the BGP overlay

#### 3.2.3 Virtual Anycast Implementation

- **Task**: Implement virtual Anycast within the private network
- **Subtasks**:
  - Create virtual Anycast IP allocation system
  - Implement route advertisement based on node health
  - Develop route withdrawal on node failure
  - Build monitoring for virtual Anycast routing

### 3.3 Health Monitoring System

#### 3.3.1 Health Metrics Integration

- **Task**: Integrate with existing health monitoring system in `form-node-metrics` and `form-state`
- **Subtasks**:
  - Analyze the existing `NodeMetrics` and `Node` data structures
  - Identify which metrics are relevant for routing decisions
  - Extend the existing metrics system to include network connectivity and routing-specific metrics
  - Create adapter interfaces to the existing monitoring system

*For a comprehensive view of the health monitoring architecture, see [Health Monitoring System Diagram](diagrams/health_monitoring_system.md).*

#### 3.3.2 Routing-Specific Health Metrics

- **Task**: Implement additional metrics specific to routing health
- **Subtasks**:
  - Add BGP session status metrics
  - Measure route propagation times
  - Monitor latency between nodes
  - Track DNS resolution times from various geographic locations

#### 3.3.3 Health-Based Routing Decisions

- **Task**: Implement routing decisions based on health status
- **Subtasks**:
  - Create health threshold configuration for routing decisions
  - Implement automatic route updates based on health metrics
  - Develop gradual degradation handling
  - Build recovery procedures for failed nodes

### 3.4 Integration with Formation Network

#### 3.4.1 Bootstrap Process Enhancement

- **Task**: Enhance the bootstrap process to use DNS-based discovery
- **Subtasks**:
  - Modify the bootstrap node discovery to use DNS
  - Update the join process to handle DNS-based bootstrapping
  - Implement fallback mechanisms for backward compatibility
  - Create documentation for the new bootstrap process

*For a visualization of how the virtual Anycast system integrates with existing components, see [Integration Points Diagram](diagrams/integration_points.md).*

#### 3.4.2 Peer Discovery Enhancement

- **Task**: Enhance peer discovery to leverage the private BGP overlay
- **Subtasks**:
  - Update the peer discovery process to use the BGP overlay
  - Modify the peer database to include BGP information
  - Implement optimized peer selection based on BGP metrics
  - Create mechanisms to share peer information across the overlay

#### 3.4.3 Configuration Management

- **Task**: Implement configuration management for the virtual Anycast system
- **Subtasks**:
  - Create configuration schema for DNS and BGP settings
  - Implement configuration validation and error checking
  - Develop configuration distribution mechanisms
  - Create documentation for configuration options

### 3.5 Security Considerations

#### 3.5.1 Private BGP Security

- **Task**: Implement security measures for the private BGP overlay
- **Subtasks**:
  - Implement BGP session authentication
  - Create route filtering and validation
  - Develop monitoring for BGP security events
  - Build access control for BGP configuration

#### 3.5.2 DNS Security

- **Task**: Implement security measures for DNS management
- **Subtasks**:
  - Implement DNSSEC where applicable
  - Create access control for DNS updates
  - Develop audit logging for DNS changes
  - Build monitoring for DNS security events

#### 3.5.3 Access Control

- **Task**: Implement access control for the virtual Anycast system
- **Subtasks**:
  - Create role-based access control for configuration
  - Implement audit logging for all system changes
  - Develop approval workflows for critical changes
  - Create secure storage for credentials 

### 3.6 Monitoring and Management

#### 3.6.1 BGP Monitoring System

- **Task**: Implement a BGP monitoring system for the private overlay
- **Subtasks**:
  - Create real-time monitoring of BGP sessions
  - Implement alerting for BGP session changes
  - Develop visualization for BGP routing information
  - Create historical data storage for BGP events

#### 3.6.2 DNS Monitoring System

- **Task**: Implement a DNS monitoring system
- **Subtasks**:
  - Create real-time monitoring of DNS records
  - Implement alerting for DNS record changes
  - Develop visualization for DNS routing
  - Create historical data storage for DNS events

#### 3.6.3 Management API

- **Task**: Develop a management API for the virtual Anycast system
- **Subtasks**:
  - Create API endpoints for BGP configuration
  - Implement DNS management API
  - Develop node health management API
  - Create documentation for the management API 

## 4. Implementation Phases

The implementation will proceed in logical phases, with each phase building on the previous one. There's no fixed timeline—each component will be developed, tested, and deployed as quickly as possible without compromising quality or stability.

### 4.1 Phase 1: DNS-Based Routing (Primary Focus)

This phase establishes the DNS infrastructure that enables geographic routing and health-based failover, which is our primary approach for virtual Anycast functionality.

#### 4.1.1 DNS Infrastructure Setup
- [x] Extend the existing `form-dns` infrastructure with GeoDNS capabilities
- [x] Implement IP geolocation database integration
- [x] Create health status tracking for IP addresses

#### 4.1.2 Health Monitoring for DNS
- [x] Implement basic health checks for entry nodes
- [x] Create health-based DNS response filtering mechanism
- [x] Implement health status repository
- [ ] Implement variable TTL adjustment based on health status
- [ ] Enhance regional health degradation handling

#### 4.1.3 Testing and Validation
- [x] Test geographic-based DNS routing
- [x] Validate health-based updates
- [x] Verify proximity-based routing accuracy
- [ ] Test variable TTL adjustments

### 4.2 Phase 2: Private BGP Overlay (Optional)

*Note: This phase is considered optional and may be implemented in the future if additional routing capabilities are needed. Our current implementation focuses on the DNS-based approach.*

#### 4.2.1 BGP Infrastructure Setup
- [x] Create test environment with multiple virtual nodes for BGP testing
- Select and integrate BGP daemon
- Design and implement private BGP network
- Create initial virtual Anycast configuration

#### 4.2.2 Health Monitoring for BGP
- Implement health checks for BGP nodes
- Create route advertisement/withdrawal mechanism
- Develop monitoring for BGP routing

#### 4.2.3 Testing and Validation
- Test private BGP routing
- Validate virtual Anycast functionality
- Verify failover mechanisms

### 4.3 Phase 3: Integration and Optimization

This phase connects the DNS components with the existing Formation network and adds security and performance enhancements.

#### 4.3.1 Formation Network Integration
- [x] Enhance bootstrap process with DNS discovery
- [ ] Update peer discovery to leverage health-aware DNS
- [ ] Implement configuration management for DNS settings

#### 4.3.2 Security Implementation
- Implement BGP and DNS security measures
- Create access control for the system
- Develop audit logging and monitoring

#### 4.3.3 Optimization and Scaling
- Optimize routing policies
- Enhance failover mechanisms
- Improve scalability of the system

## 5. Implementation Priorities

### 5.1 Must-Have Features

1. **Self-Hosted GeoDNS Implementation**
   - [x] Extend `form-dns` with geographic resolution capabilities
   - [x] Integrate health-based record filtering
   - [ ] Implement variable TTL configuration for optimized failover
   - [ ] Enhanced regional health handling

2. **Health Monitoring Integration**
   - [x] Leverage existing `form-node-metrics` and `form-state` functionality
   - [x] Implement health status tracking for IP addresses
   - [x] Create threshold-based DNS filtering
   - [ ] Implement advanced observability for DNS resolution decisions

3. **Bootstrap Process Enhancement**
   - [x] Enhance bootstrap process to use GeoDNS through `form-dns`
   - [x] Update the join process to leverage health-aware DNS
   - [x] Create documentation for the new bootstrap approach

4. **Basic Security Measures**
   - [ ] Access control for DNS configuration
   - [ ] Secure communication between components
   - [ ] Audit logging for changes

5. **Private BGP Overlay** (Optional)
   - [ ] BGP daemon integration
   - [ ] Virtual Anycast IP allocation
   - [ ] Health-based route advertisement

### 5.2 Future Enhancements

*Note: These features will be considered only after all MUST-HAVE features are fully implemented and stable.*

1. **Advanced DNS Features**
   - DNSSEC implementation
   - Multi-region routing optimization
   - Latency-based routing

2. **Advanced BGP Features**
   - Multiple BGP daemon support
   - Custom routing policies
   - Traffic engineering capabilities

3. **Enhanced Monitoring**
   - Real-time monitoring dashboard
   - Historical routing data analysis
   - Performance metrics collection

4. **Geographic Optimization**
   - Region-specific routing policies
   - Latency-based optimization
   - Traffic distribution based on node capacity

5. **Automated Management**
   - Self-healing capabilities
   - Automatic configuration updates
   - Predictive scaling based on traffic patterns

6. **Global Routing Optimization**
   - Advanced traffic engineering
   - Cross-region optimization
   - Predictive routing based on usage patterns

7. **Integration with Cloud Providers**
   - Cloud-specific DNS and routing optimizations
   - Hybrid cloud/on-premises routing
   - Cloud provider-specific health checks

8. **Advanced Security Features**
   - Threat detection and mitigation
   - Automated security response
   - Advanced access control and auditing

9. **Machine Learning-Based Routing**
   - Predictive routing based on historical data
   - Automatic policy optimization
   - Anomaly detection for routing issues

10. **User-Defined Routing Policies**
    - Custom routing policy creation
    - User-specific routing preferences
    - Application-specific routing optimization

## 6. Implementation Roadmap

### 6.1 Current Status

- Initial planning and architecture design complete
- Key implementation work streams identified
- Current `form-dns`, `form-rplb`, and `form-state` services analyzed for extension
- Geographic proximity-based response selection fully implemented in `form-dns` with support for:
  - Multiple distance weighting strategies (Linear, Quadratic, Logarithmic, Stepped)
  - Region and country biasing
  - Health-aware filtering
  - Configurable selection parameters
- Geographic resolution implementation complete, using a unified proximity-based approach rather than region-specific algorithms
- The implementation ensures protocol consistency across all nodes in the network

### 6.2 Timeline

## 7. Testing Strategy

### 7.1 Unit Testing

1. **DNS Component Tests**
   - Test DNS record management
   - Test health check mechanisms
   - Test DNS update logic

2. **BGP Component Tests**
   - Test BGP configuration generation
   - Test route advertisement and withdrawal
   - Test virtual Anycast IP management

### 7.2 Integration Testing

1. **DNS Integration Tests**
   - Test DNS provider integration
   - Test health-based DNS updates
   - Test DNS failover scenarios

2. **BGP Integration Tests**
   - Test BGP daemon integration
   - Test virtual Anycast routing
   - Test health-based route management

### 7.3 System Testing

1. **Network Simulation Tests**
   - Test in simulated multi-node environment
   - Test various network topologies
   - Test failure scenarios

2. **Performance Tests**
   - Test DNS resolution time
   - Test BGP convergence time
   - Test failover latency

### 7.4 Production Testing

1. **Controlled Rollout**
   - Test with a small subset of production nodes
   - Gradually expand to more nodes
   - Monitor for issues during rollout

2. **Chaos Testing**
   - Test node failures and recovery
   - Test network partition scenarios
   - Test DNS provider failures

## 8. Conclusion

The implementation of virtual Anycast routing in the Formation network will significantly improve the reliability, accessibility, and performance of the network without requiring an ASN or public IP block. By leveraging our existing `form-dns`, `form-rplb`, and `form-state` infrastructure and extending it with virtual Anycast capabilities, we can achieve many of the benefits of true Anycast routing while maintaining complete control over our stack.

This approach has several key advantages:

1. **Full Control**: By using our own DNS and routing infrastructure, we maintain complete control over all components
2. **Integration**: We achieve tighter integration with our existing system components
3. **Simplicity**: No need to manage external DNS provider relationships
4. **Cost Efficiency**: No additional costs for external DNS services

This detailed plan outlines the steps necessary to implement virtual Anycast routing by enhancing our existing components rather than building new systems from scratch or integrating with external services. By following this plan and prioritizing the must-have features, we can deliver a robust routing solution that meets the needs of the Formation network.

The modular approach to implementation allows for incremental development and testing, ensuring that each component works correctly before moving on to the next. The backward compatibility with existing systems ensures that current users will not be disrupted during the transition to the new routing architecture.

## 9. Next Steps

With the plan in place, we can proceed with implementation focusing exclusively on the DNS-based approach for virtual Anycast functionality:

### Form-DNS Enhancements
- [x] Analyze the current DNS authority implementation
- [x] Design and implement GeoDNS extensions for `form-dns`
- [x] Develop IP geolocation capabilities for DNS resolution
- Implement health-based IP filtering:
  - [x] Design data structure for tracking unhealthy node IPs
  - [x] Create IP-level health status tracking system
  - [x] Modify DNS resolution to exclude unhealthy IPs
  - [x] Add observability for health-based DNS filtering decisions (basic logging implemented)
  - [ ] Implement variable TTL adjustment based on health status
  - [ ] Enhance regional health degradation handling
  - [ ] Integrate DNS health metrics with form-node-metrics for unified observability
  - [ ] Create dashboard visualizations for DNS health filtering operations

### Integration with Bootstrap Process
- [x] Enhance bootstrap process to use GeoDNS through `form-dns`
- [x] Update the join process to leverage health-aware DNS
- [x] Create documentation for the new bootstrap approach

### Enhanced Metrics and Observability
- [ ] Improve logging for DNS resolution decisions
- [ ] Add metrics collection for DNS resolution patterns
- [ ] Create a dashboard for DNS health and performance

### DNS Configuration Management
- [ ] Implement a configuration framework for DNS settings
- [ ] Create management API for DNS configuration
- [ ] Develop validation and error checking for DNS settings

### BGP Development Environment (Optional)
- [x] Create test environment with multiple virtual nodes for BGP testing
- [ ] Evaluate BGP daemon options (BIRD, FRRouting, GoBGP)
- [ ] Set up initial BGP configuration templates
- [ ] Develop the virtual Anycast IP allocation mechanism

These work streams can proceed in parallel, with regular integration points to ensure compatibility between components. The focus is on enhancing existing capabilities rather than building new systems from scratch or integrating with external services. 