# Microservices Migration Plan

## Phase 1: Service Containerization

### 1.1 Service Identification & Analysis
- [x] 1.1.1 Identify all current services from existing Dockerfiles
  - [x] Document service names and purposes
  - [x] Map service dependencies
  - [x] Document required ports for each service
- [x] 1.1.2 Analyze shared resources
  - [x] Identify shared directories and files
  - [x] Document volume requirements
  - [x] Map inter-service communication patterns
- [x] 1.1.3 Document build requirements for each service
  - [x] List required build dependencies
  - [x] Document runtime dependencies
  - [x] Identify configuration files needed

### 1.2 Base Image Definition
- [x] 1.2.1 Create minimal base image with common dependencies
  - [x] Identify common system packages
  - [x] Create base Dockerfile
  - [x] Test and verify base image
- [x] 1.2.2 Document image extension process for service-specific images
  - [x] Create template for service Dockerfiles
  - [x] Document image inheritance pattern
  - [x] Establish versioning strategy

### 1.3 Service-Specific Dockerfiles
- [x] 1.3.1 Create Dockerfile for form-dns
  - [x] Identify specific dependencies
  - [x] Set up proper directories
  - [x] Configure entry point script
  - [x] Document build process
- [x] 1.3.2 Create Dockerfile for form-state
  - [x] Identify specific dependencies
  - [x] Set up proper directories
  - [x] Configure entry point script
  - [x] Document build process
- [x] 1.3.3 Create Dockerfile for vmm-service
  - [x] Identify specific dependencies
  - [x] Set up proper directories
  - [x] Configure entry point script
  - [x] Document build process
- [x] 1.3.4 Create Dockerfile for form-broker
  - [x] Identify specific dependencies
  - [x] Set up proper directories
  - [x] Configure entry point script
  - [x] Document build process
- [x] 1.3.5 Create Dockerfile for form-pack-manager
  - [x] Identify specific dependencies
  - [x] Set up proper directories
  - [x] Configure entry point script
  - [x] Document build process
- [x] 1.3.6 Create Dockerfile for formnet
  - [x] Identify specific dependencies
  - [x] Set up proper directories
  - [x] Configure entry point script
  - [x] Document build process
- [x] 1.3.7 Create Dockerfile for form-p2p
  - [x] Identify specific dependencies
  - [x] Set up proper directories
  - [x] Configure entry point script
  - [x] Document build process

### 1.4 Build & Test Individual Containers
- [x] 1.4.1 Create build scripts for each service container
  - [x] Write individual build script
  - [x] Document build options
  - [x] Create CI/CD configuration
- [x] 1.4.2 Set up test harness for individual containers
  - [x] Create container health check tests
  - [x] Develop service-specific unit tests
  - [x] Document test procedures
- [x] 1.4.3 Verify each container independently
  - [x] Test form-dns container
  - [x] Test form-state container
  - [x] Test vmm-service container
  - [x] Test form-broker container
  - [x] Test form-pack-manager container
  - [x] Test formnet container
  - [x] Test form-p2p container
  - [x] Test mock-server container (if needed)

### 1.5 Docker Compose Configuration
- [x] 1.5.1 Create initial docker-compose.yml
  - [x] Define service entries
  - [x] Configure networks
  - [x] Set up volumes
  - [x] Define environment variables
- [x] 1.5.2 Configure service dependencies and ordering
  - [x] Define depends_on relationships
  - [x] Configure healthchecks
  - [x] Set startup order
- [x] 1.5.3 Test complete docker-compose deployment
  - [x] Verify all services start correctly
  - [x] Test service intercommunication
  - [x] Verify volume sharing works correctly
  - [x] NOTE: Testing to be performed on Linux machine due to VMM and networking requirements
- [x] 1.5.4 Create docker-compose profiles for different scenarios
  - [x] Development profile
  - [x] Production profile
  - [x] Testing profile

## Phase 1.6: Agent Deployment and Automatic Registration

### 1.6.1 Formfile Integration for Automatic Registration
- [ ] 1.6.1.1 Extend formfile schema for registration metadata
  - [ ] Add agent name and description fields
  - [ ] Add model metadata fields
  - [ ] Add routing configuration for domains
  - [ ] Define validation rules for schema
- [ ] 1.6.1.2 Implement registration data extraction
  - [ ] Create parser for formfile registration data
  - [ ] Implement validation for required fields
  - [ ] Add error handling for invalid data
  - [ ] Test with various formfile formats

### 1.6.2 Form-state API Enhancements
- [ ] 1.6.2.1 Implement trusted node authentication bypass
  - [ ] Add configuration for trusted node IDs/IPs
  - [ ] Modify auth middleware to check trusted sources
  - [ ] Implement additional logging for bypassed auth
  - [ ] Test security boundaries and edge cases
- [ ] 1.6.2.2 Create automatic agent registration endpoint
  - [ ] Design API endpoint for agent registration
  - [ ] Implement registration logic
  - [ ] Add validation and error handling
  - [ ] Integrate with existing database schema

### 1.6.3 Form-pack Integration
- [ ] 1.6.3.1 Enhance form-pack to extract and use registration info
  - [ ] Add formfile parsing during build
  - [ ] Implement extraction of registration metadata
  - [ ] Add validation for required fields
  - [ ] Test with various formfile configurations
- [ ] 1.6.3.2 Implement form-pack to form-state communication
  - [ ] Create API client for form-state
  - [ ] Implement registration API calls
  - [ ] Add error handling and retries
  - [ ] Test integration points

### 1.6.4 Domain Name and Routing
- [ ] 1.6.4.1 Implement vanity domain registration in form-dns
  - [ ] Add API endpoint for domain registration
  - [ ] Implement domain record creation
  - [ ] Add validation for domain names
  - [ ] Test DNS resolution for registered domains
- [ ] 1.6.4.2 Set up formnet routing for agent domains
  - [ ] Implement IP to domain mapping
  - [ ] Create routing configuration
  - [ ] Test internal and external routing
  - [ ] Document routing architecture

### 1.6.5 Integration Testing
- [ ] 1.6.5.1 Test end-to-end agent deployment
  - [ ] Create test agents with formfiles
  - [ ] Test automatic registration
  - [ ] Verify domain assignment
  - [ ] Test routing and connectivity
- [ ] 1.6.5.2 Test multi-node deployment
  - [ ] Set up multi-node test environment
  - [ ] Test agent deployment across nodes
  - [ ] Verify cross-node communication
  - [ ] Document multi-node deployment process

### 1.6.6 Entity Relationship Implementation
- [x] 1.6.6.1 Implement localhost auth bypass
  - [x] Add localhost detection in API key auth middleware
  - [x] Create helper methods for dummy auth objects
  - [x] Update middleware to bypass authentication for local requests
  - [x] Add security logging
- [x] 1.6.6.2 Extend Formfile parser
  - [x] Add DESCRIPTION directive
  - [x] Add MODEL directive
  - [ ] Add DOMAINS directive
  - [x] Extract existing metadata
- [x] 1.6.6.3 Create API client in form-pack
  - [x] Implement form-state client
  - [x] Add agent creation function
  - [x] Add agent update function
  - [x] Handle authentication and errors
- [x] 1.6.6.4 Update build process
  - [x] Add agent registration step
  - [x] Update agent with build ID
  - [x] Test build flow end-to-end
- [x] 1.6.6.5 Enhance deployment process
  - [x] Update instance with agent ID
  - [x] Update agent with instance ID
  - [x] Link account to instance and agent
  - [ ] Link to DNS records
  - [x] Document deployment flow

## Phase 2: Admin Tool Development

### 2.1 API Design & Planning ✅
- [x] 2.1.1 Define requirements for admin tool
  - [x] List required management capabilities
  - [x] Define user roles and permissions
  - [x] Document security requirements
- [x] 2.1.2 Design admin API specification
  - [x] Define API endpoints
  - [x] Document request/response formats
  - [x] Create OpenAPI/Swagger specification
  - [x] Define error handling strategy
- [x] 2.1.3 Design authentication system
  - [x] Select authentication method
  - [x] Define token format and lifecycle
  - [x] Document key management
  - [x] Plan authorization mechanism
- [x] 2.1.4 Plan service discovery implementation
  - [x] Select service discovery approach
  - [x] Document service registration process
  - [x] Define health check requirements
  - [x] Plan fallback mechanisms

### 2.2 Service Modifications
- [ ] 2.2.1 Add startup conditioning to form-dns
  - [ ] Implement wait-for-command mode
  - [ ] Add configuration reload capability
  - [ ] Implement status endpoints
- [ ] 2.2.2 Add startup conditioning to form-state
  - [ ] Implement wait-for-command mode
  - [ ] Add configuration reload capability
  - [ ] Implement status endpoints
- [ ] 2.2.3 Add startup conditioning to vmm-service
  - [ ] Implement wait-for-command mode
  - [ ] Add configuration reload capability
  - [ ] Implement status endpoints
- [ ] 2.2.4 Add startup conditioning to form-broker
  - [ ] Implement wait-for-command mode
  - [ ] Add configuration reload capability
  - [ ] Implement status endpoints
- [ ] 2.2.5 Add startup conditioning to form-pack-manager
  - [ ] Implement wait-for-command mode
  - [ ] Add configuration reload capability
  - [ ] Implement status endpoints
- [ ] 2.2.6 Add startup conditioning to formnet
  - [ ] Implement wait-for-command mode
  - [ ] Add configuration reload capability
  - [ ] Implement status endpoints
- [ ] 2.2.7 Add startup conditioning to form-p2p
  - [ ] Implement wait-for-command mode
  - [ ] Add configuration reload capability
  - [ ] Implement status endpoints
- [ ] 2.2.8 Add authentication verification to all services
  - [ ] Implement token validation
  - [ ] Add request signing verification
  - [ ] Configure access controls
  - [ ] Document security model

### 2.3 Admin Tool Backend Implementation
- [ ] 2.3.1 Create admin service core
  - [ ] Set up project structure
  - [ ] Implement basic REST framework
  - [ ] Set up database models
  - [ ] Implement logging system
- [ ] 2.3.2 Implement authentication system
  - [ ] Create user management
  - [ ] Implement token issuance
  - [ ] Add role-based access control
  - [ ] Set up key management
- [ ] 2.3.3 Implement service discovery
  - [ ] Create service registry
  - [ ] Implement health checks
  - [ ] Add service tagging
  - [ ] Create status dashboard data endpoints
- [ ] 2.3.4 Implement service control endpoints
  - [ ] Add start/stop capabilities
  - [ ] Implement configuration management
  - [ ] Create connection management
  - [ ] Add resource allocation controls

### 2.4 Admin Tool Frontend Implementation
- [ ] 2.4.1 Design admin UI
  - [ ] Create wireframes
  - [ ] Design component system
  - [ ] Plan navigation structure
  - [ ] Define responsive layouts
- [ ] 2.4.2 Implement core UI framework
  - [ ] Set up project structure
  - [ ] Implement authentication screens
  - [ ] Create main navigation
  - [ ] Add dashboard components
- [ ] 2.4.3 Create service management UI
  - [ ] Implement service status view
  - [ ] Add service control interface
  - [ ] Create configuration editor
  - [ ] Implement logs viewer
- [ ] 2.4.4 Implement monitoring dashboards
  - [ ] Create system overview
  - [ ] Add resource utilization charts
  - [ ] Implement service metrics displays
  - [ ] Add alerting configuration

### 2.5 Testing & Documentation
- [ ] 2.5.1 Create test suite for admin backend
  - [ ] Write unit tests
  - [ ] Implement integration tests
  - [ ] Create system tests
  - [ ] Document test coverage
- [ ] 2.5.2 Create test suite for admin frontend
  - [ ] Write component tests
  - [ ] Implement E2E tests
  - [ ] Add visual regression tests
  - [ ] Document test coverage
- [ ] 2.5.3 Document admin tool
  - [ ] Create API documentation
  - [ ] Write user manual
  - [ ] Document system architecture
  - [ ] Create operations guide
- [ ] 2.5.4 Perform security audit
  - [ ] Conduct threat modeling
  - [ ] Perform penetration testing
  - [ ] Check compliance requirements
  - [ ] Document security practices

## Phase 3: Marketplace Integration

### 3.1 API Standardization
  - [ ] Specify performance requirements
  - [ ] Document error handling expectations
- [ ] 3.1.2 Design `run_task` API
  - [ ] Define parameters
  - [ ] Document response format
  - [ ] Specify error handling
  - [ ] Add validation requirements
- [ ] 3.1.3 Design `submit_task` API
  - [ ] Define parameters
  - [ ] Document response format
  - [ ] Specify error handling
  - [ ] Add validation requirements
- [ ] 3.1.4 Create reference implementation
  - [ ] Implement example agent
  - [ ] Document code patterns
  - [ ] Create starter templates
  - [ ] Add deployment examples

### 3.2 Validation System
- [ ] 3.2.1 Design validation workflow
  - [ ] Define validation criteria
  - [ ] Create validation checklist
  - [ ] Document review process
  - [ ] Establish approval workflow
- [ ] 3.2.2 Implement manual validation tools
  - [ ] Create validation dashboard
  - [ ] Implement review system
  - [ ] Add approval workflow
  - [ ] Create validation reports
- [ ] 3.2.3 Design automated validation
  - [ ] Define test scenarios
  - [ ] Create conformance tests
  - [ ] Design performance tests
  - [ ] Plan security validation
- [ ] 3.2.4 Document marketplace requirements
  - [ ] Create developer guide
  - [ ] Write submission manual
  - [ ] Document best practices
  - [ ] Create troubleshooting guide

### 3.3 Agent Onboarding Process
- [ ] 3.3.1 Design agent submission system
  - [ ] Define submission workflow
  - [ ] Plan versioning system
  - [ ] Design review process
  - [ ] Create update mechanism
- [ ] 3.3.2 Implement marketplace dashboard
  - [ ] Create agent management UI
  - [ ] Add submission interface
  - [ ] Implement review tools
  - [ ] Create analytics dashboard
- [ ] 3.3.3 Develop agent deployment system
  - [ ] Create containerization tools
  - [ ] Implement resource allocation
  - [ ] Add scaling capabilities
  - [ ] Design failover mechanisms
- [ ] 3.3.4 Create marketplace discovery
  - [ ] Implement search functionality
  - [ ] Add categorization system
  - [ ] Create recommendation engine
  - [ ] Design agent discovery API

## Build Documentation

### form-broker Service Build Process

The form-broker service is built using a multi-stage Docker build process:

1. **Build Stage**:
   - Uses `rust:1.75-slim-bullseye` as the base image
   - Installs necessary build dependencies: pkg-config, libssl-dev, git, ca-certificates
   - Uses a two-step build process to leverage Docker cache for dependencies:
     - First copies only Cargo.toml files and builds dummy source
     - Then copies actual source code and builds the final binary
   - Builds the form-broker binary with `cargo build --release --package form-broker`

2. **Runtime Stage**:
   - Uses `debian:bullseye-slim` as the minimal base image
   - Installs only required runtime dependencies: ca-certificates, libssl1.1
   - Creates necessary directories: /etc/formation/broker, /var/lib/formation/broker
   - Copies the compiled binary from the build stage
   - Copies configuration and entrypoint script
   - Configures a non-root user (formation) for improved security
   - Exposes necessary ports: 3005 (API), 5672 (AMQP), 1883 (MQTT)
   - Sets up volumes for persistent data and configuration

3. **Build Command**:
   ```bash
   docker build -t formation/form-broker:latest -f form-broker/Dockerfile .
   ```

4. **Runtime Configuration**:
   - The entrypoint script (`entrypoint.sh`) handles dynamic configuration generation
   - Environment variables can override default settings
   - Configuration is stored in `/etc/formation/broker/default.conf`
   - Service exposes multiple protocol endpoints: HTTP API, AMQP, and MQTT

### form-pack-manager Service Build Process

The form-pack-manager service is built using a multi-stage Docker build process:

1. **Build Stage**:
   - Uses `rust:1.75-slim-bullseye` as the base image
   - Installs necessary build dependencies: pkg-config, libssl-dev, git, ca-certificates
   - Uses a two-step build process to leverage Docker cache for dependencies:
     - First copies only Cargo.toml files and builds dummy source for all dependencies
     - Then copies actual source code and builds the final binary
   - Builds the form-pack-manager binary with `cargo build --release --bin form-pack-manager`

2. **Runtime Stage**:
   - Uses `debian:bullseye-slim` as the minimal base image
   - Installs only required runtime dependencies: ca-certificates, libssl1.1
   - Creates necessary directories: /etc/formation/pack-manager, /var/lib/formation/pack-manager
   - Copies the compiled binary from the build stage
   - Creates and configures an entrypoint script to handle configuration management
   - Configures a non-root user (formation) for improved security
   - Exposes API port 8080
   - Sets up volumes for persistent data and configuration

3. **Build Command**:
   ```bash
   docker build -t formation/form-pack-manager:latest -f form-pack/Dockerfile .
   ```

4. **Runtime Configuration**:
   - The entrypoint script handles dynamic configuration generation
   - Environment variables can override default settings:
     - PACK_MANAGER_PORT: API port (default: 8080)
     - PACK_MANAGER_INTERFACE: Network interface (default: All)
     - PACK_MANAGER_CONFIG_PATH: Path to config file (default: /etc/formation/pack-manager/config.json)
     - PACK_MANAGER_ENCRYPTED: Whether config is encrypted (default: true)
     - PACK_MANAGER_PASSWORD: Password for encrypted config (must be provided)
   - A default configuration is created if none exists
   - Requires a configuration file with encryption keys

### formnet Service Build Process

The formnet service is built using a multi-stage Docker build process that includes WireGuard integration:

1. **Rust Build Stage**:
   - Uses `rust:1.75-slim-bullseye` as the base image
   - Installs necessary build dependencies: pkg-config, libsqlite3-dev, libssl-dev, git, ca-certificates, clang, libclang-dev, build-essential
   - Uses a two-step build process to leverage Docker cache for dependencies:
     - First copies only Cargo.toml files and builds dummy source for all dependencies
     - Then copies actual source code and builds the final binaries
   - Builds both the formnet-server and formnet-client binaries with `cargo build --release`

2. **WireGuard Build Stage**:
   - Uses `golang:1.22-bullseye` as the base image for building WireGuard
   - Downloads and compiles wireguard-go from source
   - This provides the necessary WireGuard functionality for the networking components

3. **Runtime Stage**:
   - Uses `debian:bullseye-slim` as the minimal base image
   - Installs only required runtime dependencies: ca-certificates, libsqlite3-0, iproute2, iputils-ping
   - Creates necessary directories: /etc/formnet, /var/lib/formnet, /var/log/formnet
   - Copies the compiled binaries and WireGuard from the build stages
   - Creates and configures an entrypoint script to handle network setup and configuration
   - Sets necessary Linux capabilities for network operations
   - Configures a non-root user (formation) for improved security
   - Exposes ports 8080 (API server) and 51820/udp (WireGuard)
   - Sets up volumes for persistent data and configuration

4. **Build Command**:
   ```bash
   docker build -t formation/formnet:latest -f form-net/Dockerfile .
   ```

5. **Runtime Configuration**:
   - The entrypoint script handles dynamic configuration and network setup
   - Environment variables can override default settings:
     - FORMNET_SERVER_PORT: API server port (default: 8080)
     - FORMNET_DATA_DIR: Data directory (default: /var/lib/formnet)
     - FORMNET_CONFIG_DIR: Configuration directory (default: /etc/formnet)
     - FORMNET_NETWORK_NAME: Network name (default: formnet)
     - FORMNET_EXTERNAL_ENDPOINT: External endpoint for WireGuard (default: auto)
     - FORMNET_LISTEN_PORT: WireGuard port (default: 51820)
     - FORMNET_LOG_LEVEL: Logging level (default: info)
   - A new network is automatically created if no configuration exists
   - Supports both server and client operations in a single container

### form-p2p Service Build Process

The form-p2p service is built using a multi-stage Docker build process:

1. **Build Stage**:
   - Uses `rust:1.75-slim-bullseye` as the base image
   - Installs necessary build dependencies: pkg-config, libssl-dev, git, ca-certificates
   - Uses a two-step build process to leverage Docker cache for dependencies:
     - First creates dummy source files for all dependencies
     - Then copies actual source code and builds the final binary
   - Creates a custom binary entrypoint `form-p2p-service.rs` to serve the P2P API
   - Builds the form-p2p-service binary with `cargo build --release`

2. **Runtime Stage**:
   - Uses `debian:bullseye-slim` as the minimal base image
   - Installs only required runtime dependencies: ca-certificates, libssl1.1
   - Creates necessary directories: /etc/formation/p2p, /var/lib/formation/p2p, /var/lib/formation/db
   - Copies the compiled binary from the build stage
   - Creates and configures an entrypoint script to handle service setup
   - Configures a non-root user (formation) for improved security
   - Exposes service port 3006 for P2P communication
   - Sets up volumes for persistent data and database storage

3. **Build Command**:
   ```bash
   docker build -t formation/form-p2p:latest -f form-p2p/Dockerfile .
   ```

4. **Runtime Configuration**:
   - The entrypoint script handles dynamic configuration generation
   - Environment variables can override default settings:
     - P2P_PORT: Service port (default: 3006)
     - P2P_NODE_ID: Unique node identifier (auto-generated if not provided)
     - P2P_PRIVATE_KEY: Private key for signing messages (auto-generated if not provided)
     - P2P_STATE_URL: URL to the state service (default: http://form-state:3004)
     - P2P_LOG_LEVEL: Logging level (default: info)
     - P2P_DATA_DIR: Data directory (default: /var/lib/formation/p2p)
     - P2P_DB_DIR: Database directory (default: /var/lib/formation/db)
   - Data directories are created on startup if they don't exist
   - Service connects to the state service for peer discovery

## Phase 4: Testing & Deployment

### 4.1 Integration Testing
- [ ] 4.1.1 Develop end-to-end test suite
  - [ ] Create test scenarios
  - [ ] Implement automated tests
  - [ ] Set up test environments
  - [ ] Document test procedures
- [ ] 4.1.2 Perform load testing
  - [ ] Define load test scenarios
  - [ ] Implement load test scripts
  - [ ] Set up monitoring
  - [ ] Document performance baselines
- [ ] 4.1.3 Conduct security testing
  - [ ] Perform vulnerability scanning
  - [ ] Conduct penetration testing
  - [ ] Test authentication system
  - [ ] Verify data protection

### 4.2 Documentation
- [ ] 4.2.1 Create system architecture documentation
  - [ ] Document component relationships
  - [ ] Create network diagrams
  - [ ] Document data flows
  - [ ] Create sequence diagrams
- [ ] 4.2.2 Write operations manual
  - [ ] Document deployment procedures
  - [ ] Create troubleshooting guide
  - [ ] Document backup/restore procedures
  - [ ] Add monitoring instructions
- [ ] 4.2.3 Develop developer documentation
  - [ ] Write API documentation
  - [ ] Create plugin development guide
  - [ ] Document extension points
  - [ ] Add code examples

### 4.3 Deployment Strategy
- [ ] 4.3.1 Design deployment pipeline
  - [ ] Define CI/CD workflow
  - [ ] Create infrastructure as code
  - [ ] Document deployment environments
  - [ ] Plan rollback strategy
- [ ] 4.3.2 Implement blue-green deployment
  - [ ] Set up staging environment
  - [ ] Create deployment automation
  - [ ] Implement health checks
  - [ ] Document switchover process
- [ ] 4.3.3 Create monitoring and alerting
  - [ ] Set up metrics collection
  - [ ] Configure dashboards
  - [ ] Define alert thresholds
  - [ ] Create incident response procedures 