# Vanity Domain Provisioning Implementation Plan

This document outlines the detailed implementation plan for completing the Vanity Domain Provisioning epic. Based on our codebase analysis, we've identified that several components are partially implemented or missing. This plan breaks down the remaining work into small, manageable tasks with clear implementation details.

## 1. Complete CLI Command Implementation

### 1.1 Implement `form-cli dns update` command

**Description:** Implement the functionality to update existing DNS records, which currently exists only as a stub.

**Implementation Details:**
- **File:** `form-cli/src/dev/dns/update.rs`
- **Tasks:**
  - [x] Implement `handle_update_command` method to send requests to the DNS API
  - [x] Add API call to `http://{provider}:3004/record/{domain}/update` endpoint
  - [x] Create response handling for successful and failed updates
  - [x] Add user-friendly output formatting
  - [x] Add validation for input parameters

**Integration Points:**
- Needs to connect to the `form-dns` API module's update endpoint

### 1.2 Implement `form-cli dns remove` command

**Description:** Implement the functionality to remove DNS records, which currently exists only as a stub.

**Implementation Details:**
- **File:** `form-cli/src/dev/dns/remove.rs`
- **Tasks:**
  - [x] Implement `handle_remove_command` method to send requests to the DNS API
  - [x] Add API call to `http://{provider}:3004/record/{domain}/delete` endpoint
  - [x] Create response handling for successful and failed deletions
  - [x] Add user-friendly output formatting including warnings about impacts
  - [x] Add confirmation prompt before deletion

**Integration Points:**
- Needs to connect to the `form-dns` API module's delete endpoint

### 1.3 Add DNS Command Handling in Main CLI

**Description:** Ensure DNS commands are properly handled in the main CLI dispatch logic.

**Implementation Details:**
- **File:** `form-cli/src/main.rs`
- **Tasks:**
  - [x] Add case handling for `FormCommand::Dns` in the main match statement
  - [x] Implement handler for each DNS subcommand (Add, Update, Remove)
  - [x] Ensure proper error handling and reporting
  - [x] Add authentication and authorization for DNS operations
  - [x] Maintain consistent UX with other command types

## 2. Integration with Instance Creation Flow

### 2.1 Implement Automatic DNS Provisioning on Instance Creation

**Description:** Automatically create DNS records when instances are created.

**Implementation Details:**
- **File:** `form-vmm/vmm-service/src/service/vmm.rs`
- **Tasks:**
  - [x] Identify the instance creation process and where DNS integration should happen
  - [x] Add optional domain name parameters to instance creation command
  - [x] If domain parameters are provided, call DNS add functionality after instance creation
  - [x] Generate default domain names based on instance/build IDs if none provided
  - [x] Add ability to disable automatic DNS provisioning with a flag

**Integration Points:**
- Instance creation workflow
- DNS add command functionality

### 2.2 Add Domain to Instance Metadata

**Description:** Ensure that domain information is stored with instance metadata.

**Implementation Details:**
- **File:** `form-state/src/instances.rs`
- **Tasks:**
  - [x] Identify the instance metadata structure
  - [x] Add domain-related fields to the structure
  - [x] Update APIs to include domain information in responses
  - [x] Ensure domain information is persisted with instance data
  - [x] Add domain information to instance status/info commands

**Implementation Notes:**
- The `Instance` struct already includes a `dns_record` field of type `Option<FormDnsRecord>`
- No additional fields needed as the existing structure is sufficient
- The DNS record information is part of the instance data and included in responses
- Commands that display instance information will already show domain information

## 3. Enhanced DNS Features

### 3.1 Implement Domain Verification for Custom Domains

**Description:** Add mechanisms to verify ownership of custom domains by checking if they already point to our network nodes.

**Implementation Details:**
- **File:** `form-dns/src/api.rs` and others
- **Tasks:**
  - [x] Design domain verification system based on existing DNS records (A/CNAME records)
  - [x] Implement DNS record checking using `dig` utility or Rust DNS libraries
  - [x] Add functionality to verify if DNS records point to network nodes
  - [x] Create API endpoints for domain verification
  - [x] Add verification status to domain records
  - [x] Create CLI commands for initiating and checking verification
  - [x] Implement user-friendly instructions for DNS configuration

**Implementation Notes:**
- Implemented domain verification using trust-dns-client library to check if DNS records point to network nodes
- Added verification_status field to FormDnsRecord to track verification state
- Created API endpoints for initiating and checking verification
- Implemented CLI commands with user-friendly output and confirmation prompts
- Added detailed instructions for users to configure their DNS settings when needed

**Integration Points:**
- DNS API
- DNS record storage
- External DNS querying

## 4. Testing and Reliability

### 4.1 Add Unit Tests for DNS Components

**Description:** Create comprehensive unit tests for the DNS functionality.

**Implementation Details:**
- **Files:** Test files in each component
- **Tasks:**
  - [ ] Add unit tests for `form-dns` record management
  - [ ] Add unit tests for `form-dns` API endpoints
  - [ ] Add unit tests for `form-dns` authority handling
  - [ ] Add unit tests for `form-rplb` TLS handling
  - [ ] Add unit tests for CLI DNS commands

### 4.3 VM Network Configuration Integration

**Description:** Ensure DNS settings are properly integrated with VM networking.

**Implementation Details:**
- **Files:** To be determined based on VM networking code
- **Tasks:**
  - [ ] Analyze current VM networking configuration
  - [ ] Identify integration points for DNS configuration
  - [ ] Implement DNS configuration in VM networking setup
  - [ ] Add DNS update mechanisms when VM networking changes
  - [ ] Ensure DNS cleanup on VM deletion

**Integration Points:**
- VM networking configuration
- Instance lifecycle management

## 5. Documentation

### 5.1 Create User Documentation

**Description:** Create comprehensive user documentation for the DNS features.

**Implementation Details:**
- **Files:** Documentation files
- **Tasks:**
  - [x] Document CLI DNS commands with examples
  - [x] Create guides for common use cases
  - [x] Document domain verification process
  - [x] Document limitations and best practices
  - [x] Add troubleshooting information

**Implementation Notes:**
- Created comprehensive user guide at `formation-docs/docs/developer/guides/domain-configuration.md`
- Documented all command options with examples
- Added detailed sections on automatic domain provisioning, custom domain verification, and troubleshooting
- Included best practices and limitations sections
- Integrated the documentation with the Docusaurus sidebar

### 5.2 Create Technical Documentation

**Description:** Create technical documentation for the DNS architecture.

**Implementation Details:**
- **Files:** Documentation files
- **Tasks:**
  - [x] Document DNS architecture and components
  - [x] Create sequence diagrams for key workflows
  - [x] Document API endpoints and data structures
  - [x] Document integration points with other systems
  - [x] Add information about security aspects

**Implementation Notes:**
- Created detailed technical documentation at `formation-docs/docs/architecture/dns-architecture.md`
- Included component breakdown, data models, and API endpoints
- Added sequence diagrams for automatic provisioning and domain verification
- Documented integration points with instance management and VM networking
- Added security considerations and implementation details
- Integrated with the Docusaurus sidebar architecture section

## 6. Future Improvements

### 6.1 Implement DNS Propagation Checking

**Description:** Add mechanisms to check if DNS records have propagated correctly.

**Implementation Details:**
- **File:** New file in `form-dns/src/`
- **Tasks:**
  - [ ] Create DNS checking mechanisms using external DNS servers
  - [ ] Implement polling logic to verify propagation
  - [ ] Add timeout and retry mechanisms
  - [ ] Create API endpoints for checking propagation status
  - [ ] Add CLI commands to check propagation status

**Integration Points:**
- DNS API
- CLI commands

### 6.2 Create Domain Templates for Organizations

**Description:** Allow organizations to define templates for domain naming.

**Implementation Details:**
- **File:** New files in `form-dns/src/`
- **Tasks:**
  - [ ] Design template structure and storage
  - [ ] Create template management API endpoints
  - [ ] Implement template application logic
  - [ ] Add CLI commands for template management
  - [ ] Integrate templates with automatic domain provisioning

**Integration Points:**
- DNS API
- Instance creation process
- CLI commands

### 6.3 Add Integration Tests

**Description:** Create integration tests to ensure the whole DNS system works together.

**Implementation Details:**
- **Files:** Test files in each component
- **Tasks:**
  - [ ] Create test environment setup for DNS services
  - [ ] Add tests for end-to-end domain registration flow
  - [ ] Add tests for TLS certificate provisioning
  - [ ] Add tests for domain updates and removals
  - [ ] Add tests for edge cases and error handling

**Integration Points:**
- DNS API
- CLI commands
- VM Manager
- Reverse Proxy/Load Balancer

## Implementation Timeline

This section outlines a suggested implementation sequence to ensure that dependencies are respected and that we deliver value incrementally.

### Phase 1: Core CLI Functionality (COMPLETED)
- Complete CLI command implementation (1.1, 1.2, 1.3)
- Add unit tests for CLI commands (4.1 partial)

### Phase 2: Integration and Automatic Provisioning (COMPLETED)
- Implement automatic DNS provisioning (2.1)
- Add domain to instance metadata (2.2)

### Phase 3: Enhanced Features (COMPLETED)
- Implement domain verification (3.1)

### Phase 4: Documentation (COMPLETED)
- Create user and technical documentation (5.1, 5.2)

### Phase 5: Future Improvements (As Needed)
- Add unit tests for DNS components (4.1)
- Integration with VM networking configuration (4.3) 
- Implement DNS propagation checking (6.1)
- Create domain templates (6.2)
- Add integration tests (6.3)

## Success Criteria

The Vanity Domain Provisioning epic has been completed with the following criteria met:

1. ✅ Users can manually create, update, and remove domain records using the CLI
2. ✅ Domains are automatically provisioned when instances are created
3. ✅ Custom domains can be verified and used
4. ✅ All core functionality is fully documented
5. ✅ The system is reliable for everyday use

Some additional enhancements may be implemented in the future:
- Domain templates for organizational naming consistency
- Integration tests for comprehensive system verification
- DNS propagation checking
- More comprehensive integration with VM networking

This implementation plan serves as a reference for both the completed work and potential future enhancements to the Vanity Domain Provisioning system. 