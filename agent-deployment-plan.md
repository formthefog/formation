# Agent Deployment and Automatic Registration Plan

## Overview
This document outlines the implementation plan for deploying agents and enabling automatic registration of agents, nodes, and models in the Formation platform. The goal is to streamline the deployment process for devnet setups and provide a seamless experience for developers.

## Current State
- Docker-compose setup is complete for single-node and multi-node devnet deployment
- Manual registration is required for agents, nodes, and models
- Automated registration exists for users (formnet peers) and instances (VMs)

## Implementation Goals
1. Automatic agent registration from formfile configuration
2. Streamlined image building and deployment process
3. Vanity domain name assignment and routing
4. Auth bypass for trusted devnet nodes

## Implementation Plan

### 1. Formfile Integration for Automatic Registration

#### Tasks:
- [ ] Extend the formfile schema to include registration metadata
- [ ] Implement a parser in form-state to extract registration data from formfile
- [ ] Create an API endpoint in form-state for automatic registration
- [ ] Add formfile validation to ensure required fields are present

#### Technical Details:
- Extend formfile to include:
  ```yaml
  registration:
    agent_name: "my-agent"
    model_name: "my-model"
    description: "Agent description"
    routing:
      internal_domain: "agent.internal"
      external_domain: "agent.formation"
  ```
- Form-state will process this information during image deployment

### 2. Image Building and Deployment Pipeline

#### Tasks:
- [ ] Enhance form-pack to extract registration info from formfile
- [ ] Implement communication between form-pack and form-state
- [ ] Create deployment workflow automation script
- [ ] Add status tracking for deployment process

#### Technical Details:
- Flow: form-pack builds image → extracts formfile → communicates with form-state → VMM launches instance
- Implement hooks in the image build process to trigger registration

### 3. Networking and Domain Name Management

#### Tasks:
- [ ] Implement vanity domain registration in form-dns
- [ ] Set up routing between external domains and internal formnet IPs
- [ ] Create API for domain name management
- [ ] Implement lookup and routing for agent domains

#### Technical Details:
- Form-dns will need to be updated to handle vanity domains
- Add mapping between formnet IPs and domain names
- Implement DNS record creation during agent registration

### 4. Auth Bypass for Trusted Nodes

#### Tasks:
- [ ] Implement trusted node identification in form-state
- [ ] Create configuration for specifying trusted nodes
- [ ] Add auth bypass logic for requests from trusted nodes
- [ ] Implement security measures to prevent misuse

#### Technical Details:
- Add environment variable for trusted node IPs or identifiers
- Modify auth middleware to check for trusted node requests
- Add logging for all bypassed auth requests for security

### 5. Integration Testing

#### Tasks:
- [ ] Create test fixtures for automatic registration
- [ ] Test image building and deployment pipeline
- [ ] Verify domain name assignment and routing
- [ ] Test multi-node network with automatic registration

## Implementation Timeline

### Phase 1: Core Registration Features (2 weeks)
- Formfile extension
- Basic registration endpoints
- Auth bypass for trusted nodes

### Phase 2: Image Deployment Automation (2 weeks)
- Integration between form-pack and form-state
- Deployment workflow script
- Status tracking

### Phase 3: DNS and Routing (1 week)
- Vanity domain implementation
- Routing setup
- DNS record management

### Phase 4: Testing and Documentation (1 week)
- Integration testing
- Documentation update
- Developer guides

## Success Criteria
1. An agent can be deployed from formfile with zero manual registration steps
2. Vanity domains work correctly both inside and outside formnet
3. Trusted nodes can perform operations without auth barriers in devnet
4. The entire workflow is documented and reproducible 