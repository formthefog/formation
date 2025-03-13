# Formation Network Strategic Roadmap

This document outlines the strategic roadmap for the Formation network, prioritizing features and enhancements based on immediate needs, technical dependencies, and long-term vision.

## Status Summary

**Completed Components:**
- ✅ VM Management Ownership Verification - All VM operations now require signature verification
- ✅ formnet Improvements - Enhanced connectivity, reduced connection times, improved reliability
- ✅ DNS-based Routing - Implemented health-aware GeoDNS for improved connectivity
- ✅ MCP Server Phase 1 - Core framework, VM management tools, workload packaging, authentication, API documentation

**In Progress:**
- Vanity Domain Provisioning - Basic functionality implemented, enhancements in progress
- Economic Infrastructure - Foundational components in place, integration work ongoing

**Upcoming:**
- Stateful Elastic Scaling
- Native P2P AI Inference Engine

## MUST HAVE

### 1. VM Management Ownership Verification (Highest Priority)

Implement signature verification and ownership checks for all VM management commands to ensure only authorized users can perform operations.

**User Stories:**
- As an instance owner, I want my VMs to be protected from unauthorized access
- As a team lead, I want to delegate management permissions to team members
- As an administrator, I want to transfer ownership when team members leave
- As an AI agent, I want to manage instances on behalf of my owner

**Implementation Tasks:**
- [x] Add signature verification to all VM management endpoints
- [x] Update `Instance` structure to include owner address
- [x] Implement authorized users list for each instance
- [x] Create ownership transfer functionality
- [x] Add account structure to `form-state` crate
- [x] Implement account-to-instances relationship tracking
- [x] Update all command handlers to verify signatures
- [x] Create permissions model (owner, authorized user, read-only, etc.)
- [x] Add API endpoints for ownership management
  - [x] `get_account` - Get account details
  - [x] `list_accounts` - List all accounts
  - [x] `create_account` - Create a new account
  - [x] `update_account` - Update existing account
  - [x] `delete_account` - Delete an account
- [x] Update CLI to sign requests with wallet keys

### 2. formnet Improvements

Address critical networking issues to improve reliability, connection speed, and platform compatibility.

**User Stories:**
- As a user, I want peer discovery to happen within seconds, not minutes
- As a user, I want reliable reconnection if I temporarily disconnect
- As a node operator, I want efficient NAT traversal for my nodes

**Implementation Tasks:**
- [x] Reduce NAT traversal step interval from 5 seconds to 1 second
- [x] Expand candidate endpoint limit from 10 to 30
- [x] Remove unnecessary startup delay
- [x] Increase endpoint refresher frequency from 10 seconds to 3 seconds
- [x] Implement parallel endpoint testing (3 endpoints per peer simultaneously)
- [x] Add connection success caching for faster reconnection
- [x] Implement smart endpoint prioritization for remote connections
- [x] Add connection health checks and automatic retry
- [x] Research and implement decentralized TURN server approach
- [x] Add connection quality metrics and automated retry logic
- [x] Relay tests
- [x] Endpoint collection and prioritization tests
- [x] Data throughput tests
- [x] Direct connection tests
- [x] Security and edge case tests
- [x] Scalability tests (concurrent connections, large peer networks)

### 3. Vanity Domain Provisioning

Complete and enhance the existing vanity domain system to provide users with friendly access to their instances.

**User Stories:**
- As a user, I want to SSH into my instance using a memorable domain name
- As a user, I want to publish services on a custom subdomain
- As a developer, I want to access my instances without knowing specific IP addresses
- As a team, we want consistent naming across all our instances

**Implementation Tasks:**
- [x] Integrate existing `form-cli dns` commands with instance creation flow
- [x] Enhance `form-dns` and `form-rplb` crates for better reliability
- [x] Implement automatic DNS provisioning on instance creation
- [~] Create DNS management UI/CLI for users (CLI exists, UI not implemented)
- [~] Implement wildcard certificate support for user domains
- [x] Add domain verification for custom domains
- [~] Create optional domain templates for organizations (nice to have)
- [~] Implement DNS propagation checking (nice to have)
- [~] Add support for DNS record TTL management (basic implementation exists)
- [~] Create integration with VM networking configuration (future improvement)
- [~] Add unit and integration tests for DNS components (future improvement)
- [x] Implement `form-cli dns add` command (implemented)
- [x] Implement `form-cli dns update` command (implemented)
- [x] Implement `form-cli dns remove` command (implemented)
- [x] Document DNS features and usage for users
- [x] Create technical documentation for DNS architecture

**Status: COMPLETED** - Core functionality implemented, with some optional enhancements planned for future releases.

### 4. Economic Infrastructure

Implement event-driven resource usage tracking with frequent event emission and minimal state retention.

**User Stories:**
- As a user, I want my resource usage to be accurately measured and tracked
- As a node provider, I want to be paid automatically for resources provided
- As a developer, I want programmatic access to usage data
- As a platform operator, I want to integrate billing with external systems

**Implementation Tasks (Compute Backend):**
- [x] Implement resource usage measurement system
  - [x] Track CPU, memory, storage, network, and GPU usage per VM
  - [x] Create efficient point-in-time metrics collection
  - [x] Implement minimal short-term buffer for latest metrics only
- [x] Create usage event emission system
  - [x] Design lightweight usage event schema with essential properties
  - [x] Implement reliable event emission every 30 seconds
  - [x] Build retry mechanisms for reliability
  - [x] Implement circuit breaker for destination outages
- [x] Implement stateless threshold detection
  - [x] Create configurable resource usage thresholds from external source
  - [x] Build real-time threshold checking against current metrics
  - [x] Implement notification event emission for threshold violations
- [x] Develop minimal API layer
  - [x] Implement RESTful API for current usage data (no history)
  - [x] Build health check endpoints for monitoring system components
  - [x] Create API documentation with examples and usage guidelines
  - [x] Create webhook registration for real-time usage events
- [ ] Future enhancements
  - [ ] Testing and validation
  - [ ] Deployment and operations
  - [ ] Authentication and authorization
  - [ ] Filtering parameters for VM-specific metrics
  - [ ] Integration with account service
  - [ ] Dead-letter queue for unprocessable events
  - [ ] Batching for failed events
  - [ ] Event emission monitoring dashboard
  - [ ] Advanced data retention policies

**Status: COMPLETED** - Core functionality implemented, with future enhancements planned for subsequent releases.

**Integration Tasks (Other Teams):**
- [ ] Implement Usage Database for historical data storage and aggregation
- [ ] Design and implement credit token on Ethereum (Blockchain Team)
- [ ] Create fiat on-ramp for purchasing credits (User Portal Team)
- [ ] Implement wallet abstraction layer for non-crypto users (User Portal Team)
- [ ] Build automated billing system against tokenized credits (Billing Service Team)
- [ ] Design deposit and withdrawal mechanisms (Blockchain Team)
- [ ] Create reporting and analytics for usage and billing (Admin Dashboard Team)
- [ ] Implement resource pricing mechanism (Admin Dashboard Team)
- [ ] Build invoice generation system (Billing Service Team)

### 5. BGP/Anycast Routing

Implement advanced routing for seamless access to the network without requiring specific bootstrap nodes.

**User Stories:**
- As a user, I want to connect to the nearest available node automatically
- As an operator, I want to share network traffic across my nodes
- As a developer, I want consistent access points regardless of network changes
- As a user, I want reliable network connectivity even during node failures

**Implementation Tasks:**
- [x] Design BGP/Anycast routing architecture
- [x] Implement DNS-based routing with geolocation support
- [x] Create health tracking for IP addresses
- [x] Implement filtering of unhealthy IPs in DNS responses
- [x] Enhance bootstrap process to use GeoDNS and health-aware DNS

**Future Enhancements:**
- [ ] Implement variable TTL adjustment based on node health status
- [ ] Advanced caching strategies for DNS responses
- [ ] Regional-specific health degradation handling
- [ ] Integrate DNS health metrics with form-node-metrics for unified observability
- [ ] Create dashboard visualizations for DNS health filtering operations
- [ ] Implement BGP session management for nodes
  - [x] Create test environment with multiple virtual nodes for BGP testing
  - [ ] Evaluate and select BGP daemon
  - [ ] Implement BGP configuration generation
- [ ] Create anycast IP allocation system
- [ ] Build route advertisement and propagation system
- [ ] Implement health checks for routing decisions
- [ ] Create failover mechanisms for automated recovery
- [ ] Design and implement traffic distribution logic
- [ ] Build monitoring for routing infrastructure
- [ ] Implement border router configuration management
- [ ] Create documentation for network operators

**Status: COMPLETED** - Core DNS-based routing functionality implemented, with BGP overlay and additional enhancements planned for future releases.

### 6. MCP Server for Workload Lifecycle Management

Implement management control plane to enable agents and AI to manage workload lifecycles following the Model Context Protocol standard.

**User Stories:**
- As an AI agent, I want to deploy and manage workloads autonomously
- As a developer, I want to automate scaling based on application metrics
- As an operator, I want centralized management of distributed workloads
- As a user, I want intelligent resource optimization for my workloads

**Implementation Tasks:**
- [x] Design MCP server architecture and API
- [x] Choose appropriate language and framework (Rust with Actix Web)
- [x] Set up project structure and basic module layout
- [x] Implement tool registry and execution system
  - [x] Create core data structures for tool registry
  - [x] Implement basic registry management functionality
  - [x] Develop VM management tools
    - [x] VM Create Tool - Provisioning new VMs with customizable configurations
    - [x] VM Status Tool - Retrieving status information about existing VMs
    - [x] VM Control Tool - Managing lifecycle operations (start, stop, restart)
    - [x] VM List Tool - Listing available VMs with filtering capabilities
    - [x] VM Delete Tool - Removing VMs when no longer needed
  - [x] Implement workload packaging and deployment tools
    - [x] Pack Build Tool - Building workloads from Formfile specifications
    - [x] Pack Ship Tool - Deploying built workloads to Formation instances
- [x] Create authentication and authorization system
  - [x] Implement JWT-based authentication
  - [x] Add signature verification for requests
  - [x] Create permission-based authorization
- [x] Implement API documentation and client libraries
  - [x] Create comprehensive OpenAPI specification
  - [x] Develop Python client library with error handling
  - [x] Document client usage with examples

**Future Improvements (Phase 2):**
- Build network configuration tools
- Create metrics and monitoring tools
- Build event system for workload state changes
- Create resource optimization recommendations
- Design and implement agent policy framework
- Build logging and monitoring for agent actions

**Status: COMPLETED FOR PHASE 1** - Core framework implementation, VM management tools, and workload packaging tools completed. VM tools provide full lifecycle management including creation, status checking, control operations, listing, and deletion. Pack tools enable building and deploying workloads using Formfile specifications. All tools interact properly with the state datastore and message queue system, with robust error handling and security checks. API is fully documented with OpenAPI specification and a Python client library is available for developers. Future phases will focus on implementing network configuration tools, metrics/monitoring capabilities, and advanced features.

### 7. Stateful Elastic Scaling

Enable dynamic scaling of compute resources and storage without losing application state.

**User Stories:**
- As a user, I want to scale my VM's resources up or down based on demand
- As a developer, I want to add more storage to my instance without downtime
- As an application owner, I want to add more CPU/RAM during peak periods
- As a user, I want my application state preserved during scaling operations

**Implementation Tasks:**
- [ ] Implement hot-add capabilities for CPU, memory, and storage
- [ ] Create seamless storage migration between tiers
- [ ] Build resource monitoring and recommendation system
- [ ] Implement state preservation during scaling operations
- [ ] Create automated scaling policies framework
- [ ] Develop scaling scheduler for time-based operations
- [ ] Build API for programmatic scaling operations
- [ ] Implement quota and limit management for scaling
- [ ] Create testing framework for scaling operations
- [ ] Build rollback mechanisms for failed scaling operations

### 8. Native P2P AI Inference Engine

Build a distributed AI inference engine with OpenAI/Anthropic compatible APIs for efficient model serving.

**User Stories:**
- As a developer, I want to use industry-standard APIs for AI inference
- As a user, I want distributed inference to handle large models efficiently
- As a user, I want to share compute resources for inference tasks
- As a model provider, I want to deploy my models to the distributed network

**Implementation Tasks:**
- [ ] Design model weight sharding protocol
- [ ] Create compatible API layer (OpenAI/Anthropic)
- [ ] Implement model serving infrastructure
- [ ] Build request routing and load balancing system
- [ ] Create model registry and discovery mechanism
- [ ] Implement efficient local caching of model weights
- [ ] Design inference cluster management
- [ ] Build failover and reliability mechanisms
- [ ] Develop model quantization and optimization tools
- [ ] Create accounting system for inference compute usage
- [ ] Implement security and access control for models

## NICE TO HAVE

### 1. IPv6 Support for Networking Components

Implement IPv6 support to ensure compatibility with modern networks and IPv6-only environments.

**User Stories:**
- As a user, I want to access the network on my IPv6-only connection
- As an operator, I want to future-proof my network with IPv6 support
- As a developer, I want to ensure my services work in all network environments

**Implementation Tasks:**
- [ ] Update socket binding code to support IPv6
- [ ] Enhance endpoint collection and discovery for IPv6
- [ ] Implement dual-stack operation where appropriate
- [ ] Update NAT traversal logic for IPv6 networks
- [ ] Create testing infrastructure for IPv6 verification
- [ ] Update the relay system to support IPv6 endpoints

### 2. Node Metrics Verifiability

Implement cryptographic verification of reported node metrics for trust and transparency.

**User Stories:**
- As a user, I want to verify that node resources are reported accurately
- As an operator, I want to prove my node is reporting metrics honestly
- As a platform, we want to prevent fraudulent resource reporting

**Implementation Tasks:**
- [ ] Design node metrics attestation protocol
- [ ] Implement trusted execution environment for metrics reporting
- [ ] Create verification challenge-response system
- [ ] Build reputation system for node honesty
- [ ] Implement spot-checking of node metrics
- [ ] Create decentralized verification network
- [ ] Design penalties for dishonest reporting
- [ ] Build monitoring dashboard for network-wide metrics

### 3. VM & Node Liveness & Availability Verifiability

Implement systems to verify and prove VM and node uptime and availability.

**User Stories:**
- As a user, I want to verify my VM's availability history
- As an operator, I want to prove my node's uptime for compensation
- As a platform, we want to enforce SLAs based on verifiable metrics
- As a user, I want compensation for downtime based on verified data

**Implementation Tasks:**
- [ ] Design uptime attestation protocol
- [ ] Implement distributed uptime monitoring
- [ ] Create cryptographic proofs of uptime periods
- [ ] Build SLA enforcement based on verified uptime
- [ ] Implement automatic compensation for downtime
- [ ] Create unified availability dashboard
- [ ] Design incentive mechanisms for high availability
- [ ] Build historical availability reporting

## ICEBOX

### 1. Workload Verifiability

**Implementation Tasks:**
- [ ] Research TEE/TPM-based workload verification techniques
- [ ] Design optimistic verification system with session-based limited re-execution
- [ ] Implement TEE attestation for secure workload verification
- [ ] Create challenge-response protocol for optimistic verification
- [ ] Build verification sampling mechanism to minimize overhead
- [ ] Implement secure enclaves for sensitive workload components
- [ ] Design dispute resolution for contested verification results

### 2. VM Metrics Verifiability

**Implementation Tasks:**
- [ ] Extend node metrics verification to VM level
- [ ] Design per-VM TEE-based attestation mechanism
- [ ] Implement guest OS metrics validation through TPM
- [ ] Create secure reporting channel from guest to verification service

### 3. Private Wireguard Mesh Networks

**Implementation Tasks:**
- [ ] Design architecture for per-organization or per-user mesh networks
- [ ] Implement network isolation between different mesh networks
- [ ] Create resource sharing mechanisms across mesh boundaries
- [ ] Build network policy framework for cross-mesh communication
- [ ] Implement management tools for private mesh networks
- [ ] Design authentication and authorization for mesh access

### 4. Eventually Consistent State Root & Message Queue Verifiability

**Implementation Tasks:**
- [ ] Design state root calculation protocol
- [ ] Implement merkle-tree based state verification
- [ ] Create message queue integrity verification system with TEE attestation
- [ ] Build reconciliation mechanism for state inconsistencies
- [ ] Implement optimistic verification with challenge periods

### 5. Operator Registration and Staking

**Implementation Tasks:**
- [ ] Design operator staking mechanism
- [ ] Implement registration and identity verification
- [ ] Create stake slashing conditions and enforcement
- [ ] Build operator reputation system 