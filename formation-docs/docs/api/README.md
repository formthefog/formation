# Formation API Documentation Updates

This file outlines the documentation updates that are still needed to ensure all API documentation is accurate and complete.

## Current Status

- ✅ State Service API documentation updated to reflect actual codebase
- ✅ OpenAPI specification created for State Service API
- ✅ API index updated to include OpenAPI references and user guidelines

## Remaining Tasks

### API Documentation Updates

- [ ] Review and update VMM Service API documentation
- [ ] Review and update DNS Service API documentation
- [ ] Review and update Formnet API documentation
- [ ] Review and update P2P Service API documentation
- [ ] Focus all documentation on GET endpoints with exceptions for P2P and upcoming Inference Engine
- [ ] Create OpenAPI specifications for all remaining services:
  - [ ] VMM Service API
  - [ ] DNS Service API
  - [ ] Formnet API
  - [ ] P2P Service API

### Operator Documentation Updates

- [ ] Verify installation instructions in Getting Started guide
- [ ] Complete deployment guide for full walkthrough
- [ ] Verify hardware requirements match current needs
- [ ] Complete troubleshooting guide
- [ ] Update guides for network configuration
- [ ] Complete operator maintenance procedures
- [ ] Update staking requirements and procedures
- [ ] Create detailed guide for joining the live network
- [ ] Document Inference Engine integration

### Inference Engine Documentation

- [ ] Create API documentation for the Inference Engine (after integration)
- [ ] Document user-facing endpoints for making inference requests
- [ ] Create OpenAPI specification for Inference Engine API
- [ ] Document integration with existing Formation systems

## Documentation Guidelines

When updating documentation, please ensure:

1. All API endpoints match the actual implementation in the codebase
2. Focus on GET endpoints for most services (except where noted)
3. Provide clear examples of request/response formats
4. Include authentication requirements
5. Create or update OpenAPI specifications to match implementation
6. Update example code in SDK Integration sections
7. Review deployment guides for accuracy with the current version

## Validation Process

Each documentation update should be validated by:

1. Comparing with the actual codebase implementation
2. Testing described endpoints against a running service
3. Walking through deployment steps in a fresh environment
4. Peer review by at least one other developer 