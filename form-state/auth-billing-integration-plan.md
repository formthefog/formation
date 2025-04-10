# Authentication and Billing Integration Plan

This document outlines the step-by-step plan for integrating authentication (using Dynamic Auth with JWT) and billing (using Stripe) into the Form-State marketplace for agents and models.

## Phase 1: Authentication Setup

### 1. Create JWT Authentication Module
- [x] Add jwt-authorizer dependency to Cargo.toml
- [x] Configure JWKS URL from Dynamic Auth
  - [x] Set up environment variable for JWKS URL
  - [x] Add configuration module to load and validate environment variables
- [x] Create custom JWT claims struct
  - [x] Define project field for project-scoping
  - [x] Define role field for role-based access
  - [x] Add any additional required claims
- [x] Implement JWT verification utilities
  - [x] Set up key refreshing and caching
  - [x] Configure token validation parameters (exp, iss, aud)

### 2. Implement Authentication Middleware
- [x] Create Axum middleware for JWT validation
  - [x] Extract Bearer token from Authorization header
  - [x] Validate token against JWKS endpoint
  - [x] Handle validation errors with appropriate status codes
- [x] Store validated claims in request extensions
  - [x] Set up extractors for accessing claims in handlers
  - [x] Create helper functions for common claim operations
- [x] Organize routes into public and protected groups
  - [x] Create public route group (no auth required)
  - [x] Create protected route group with JWT middleware

## Phase 2: Authorization Implementation

### 3. Define Role-Based Access System
- [x] Create UserRole enum
  - [x] Define Admin, Developer, and User roles
  - [x] Implement serialization/deserialization for roles
- [x] Implement role-based extractors
  - [x] Create AdminOnly extractor
  - [x] Create DeveloperOrAdmin extractor
  - [x] Create custom rejection handlers for unauthorized access
- [x] Set up role validation functions
  - [x] Create helpers to check role permissions for specific operations

## Phase 3: Stripe Billing Integration

### 4. Set Up Stripe Client for Verification
- [ ] Add Stripe crate dependencies
  - [ ] Select appropriate Stripe library (async-stripe recommended)
  - [ ] Configure with environment variables
- [ ] Create Stripe client configuration
  - [ ] Set up API key handling for verification purposes
  - [ ] Configure webhooks for subscription status updates
- [ ] Implement subscription status tracking
  - [ ] Create endpoints to verify subscription status
  - [ ] Set up database for tracking subscription status and tier
  - [ ] Implement mechanism to receive subscription status updates from frontend/Stripe
  - [ ] Note: All payment processing and method management will happen via Stripe and frontend

### 5. Implement Usage Tracking
- [ ] Extend ModelState with usage tracking
  - [ ] Add counters for token consumption
  - [ ] Track request counts and other metrics
  - [ ] Implement atomic increment operations
- [ ] Extend AgentState with usage tracking
  - [ ] Track agent invocations and runtime
  - [ ] Track how many agents are currently hired
  - [ ] Implement time-windowed usage stats
- [ ] Create usage reporting system
  - [ ] Set up credit deduction for token usage
  - [ ] Implement background task for usage aggregation
  - [ ] Track remaining credits for pay-as-you-go users

### 6. Add Eligibility Enforcement
- [ ] Implement eligibility checking middleware
  - [ ] Check available credits before processing token consumption
  - [ ] Verify available agent slots before hiring
  - [ ] Enforce subscription tier limits
- [ ] Create plan-based limits
  - [ ] Define subscription tiers and their quotas (agent slots and credits)
  - [ ] Implement eligibility checking before processing requests
  - [ ] Create usage projection utilities
- [ ] Set up rejection handling
  - [ ] Create standardized responses for insufficient credits
  - [ ] Add upgrade prompts in limit exceeded responses
  - [ ] Implement graceful handling for users at their limits

## Phase 4: API Enhancements

### 7. Upgrade Existing API Endpoints
- [ ] Refactor model endpoints
  - [ ] Add auth middleware to all protected routes
  - [ ] Integrate usage tracking and credit deduction
  - [ ] Add eligibility checking before processing requests
- [ ] Refactor agent endpoints
  - [ ] Secure all protected operations
  - [ ] Add hiring slot verification
  - [ ] Implement usage metering for agent calls
- [ ] Update API response formats
  - [ ] Add credit balance to relevant responses
  - [ ] Include subscription status in response metadata
  - [ ] Standardize error responses for eligibility failures

### 8. Create Account and Usage Management APIs
- [ ] Add subscription status endpoints
  - [ ] Create endpoints for viewing current plan
  - [ ] Implement endpoints to check available credits/slots
  - [ ] Add usage history access
- [ ] Create usage reporting APIs
  - [ ] Add endpoints for usage statistics
  - [ ] Implement credit consumption history
  - [ ] Create usage forecasting endpoints
- [ ] Implement webhook handlers
  - [ ] Add handlers for subscription status changes
  - [ ] Create credit balance update handlers
  - [ ] Implement event processing for plan changes

## Phase 5: Testing and Documentation

### 9. Create Comprehensive Tests
- [ ] Create authentication unit tests
  - [ ] Test JWT validation with mock tokens
  - [ ] Test role-based access control
  - [ ] Test extractors and middleware components
- [ ] Implement billing integration tests
  - [ ] Test usage tracking accuracy
  - [ ] Verify quota enforcement
  - [ ] Test Stripe API integration with test mode
- [ ] Set up end-to-end testing
  - [ ] Create test scenarios covering auth and billing
  - [ ] Test rate limiting and quota enforcement
  - [ ] Verify usage reporting accuracy

### 10. Update Documentation
- [ ] Add authentication documentation
  - [ ] Document token requirements and format
  - [ ] Create examples for authenticated requests
  - [ ] Document error codes and troubleshooting
- [ ] Create billing integration docs
  - [ ] Document subscription plans and features
  - [ ] Explain usage-based billing model
  - [ ] Create integration examples for client applications
- [ ] Update API reference
  - [ ] Add auth and billing parameters to all endpoints
  - [ ] Document rate limits and quota constraints
  - [ ] Add sample responses for various scenarios 

## Future Enhancements

### Project-Based Resource Control
- [ ] Implement project-resource access control
  - [ ] Create ProjectResourceAccess association model
  - [ ] Implement API to manage project access to resources
  - [ ] Add permission checking to agent and model routes
- [ ] Modify agent operations to verify project access permissions
  - [ ] Add access control to agent route handlers
  - [ ] Implement resource-level permission logic
  - [ ] Add authorized project list to agent responses
- [ ] Modify model operations to verify project access permissions
  - [ ] Add access control to model route handlers
  - [ ] Implement resource-level permission logic
  - [ ] Add authorized project list to model responses
- [ ] Implement cross-project sharing mechanism
  - [ ] Create access control model for resource sharing
  - [ ] Add sharing permissions management API
  - [ ] Implement access validation for shared resources 