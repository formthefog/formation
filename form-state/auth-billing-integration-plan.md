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

## Phase 3: Billing Integration (Frontend-Driven)

### 4. Create Subscription Data Storage
- [x] Design subscription data structures
  - [x] Define SubscriptionInfo, SubscriptionTier, and SubscriptionStatus types
  - [x] Create BillingConfig for default pricing and limits
  - [x] Implement serialization/deserialization for storage
- [x] Create API endpoints for receiving subscription data
  - [x] Implement endpoint for subscription status updates
  - [x] Create handlers for credit purchases
  - [x] Add validation for incoming subscription data
- [x] Set up secure reception of billing data
  - [x] Add validation of incoming requests
  - [x] Implement proper authorization checks on billing endpoints
  - [x] Store received subscription data in accounts
  - [x] Note: All payment processing and Stripe interactions happen exclusively in the frontend

### 5. Implement Usage Tracking (Account-Centric)
- [ ] Extend Account with comprehensive usage tracking
  - [ ] Add token consumption tracking structure
  - [ ] Track agent invocations and request counts
  - [ ] Implement time-windowed usage stats
  - [ ] Store per-model and per-agent metrics within Account
- [ ] Create account-based usage recording API
  - [ ] Implement `record_token_usage(model_id, token_count)`
  - [ ] Implement `record_agent_usage(agent_id, operation_type)`
  - [ ] Add atomic increment operations for thread safety
  - [ ] Create usage aggregation methods
- [ ] Implement credit management within Account
  - [ ] Set up credit tracking and balance management
  - [ ] Implement credit transaction history
  - [ ] Create methods for credit consumption and addition
  - [ ] Add reporting for available credits

### 6. Add Eligibility Enforcement
- [ ] Implement account-based eligibility checking
  - [ ] Create `can_use_tokens(model_id, token_count)` method on Account
  - [ ] Create `can_hire_agent(agent_id)` method on Account 
  - [ ] Implement credit checking for operations
- [ ] Create plan-based limits
  - [ ] Define subscription tiers and their quotas (agent slots and credits)
  - [ ] Implement tier-based eligibility rules
  - [ ] Create usage projection utilities
- [ ] Set up rejection handling
  - [ ] Create standardized responses for insufficient credits
  - [ ] Add upgrade prompts in limit exceeded responses
  - [ ] Implement graceful handling for users at their limits

## Phase 4: API Enhancements

### 7. Upgrade Existing API Endpoints
- [ ] Refactor model endpoints
  - [ ] Add auth middleware to all protected routes
  - [ ] Integrate account-based usage tracking
  - [ ] Add eligibility checking before processing requests
- [ ] Refactor agent endpoints
  - [ ] Secure all protected operations
  - [ ] Add account-based hiring slot verification
  - [ ] Implement usage metering via account methods
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
- [ ] Implement data reception endpoints
  - [ ] Add handlers for receiving subscription updates from frontend
  - [ ] Create credit balance update handlers
  - [ ] Implement event processing for plan changes

## Phase 5: Testing and Documentation

### 9. Create Comprehensive Tests
- [ ] Create authentication unit tests
  - [ ] Test JWT validation with mock tokens
  - [ ] Test role-based access control
  - [ ] Test extractors and middleware components
- [ ] Implement billing integration tests
  - [ ] Test account-based usage tracking accuracy
  - [ ] Verify quota enforcement through account methods
  - [ ] Test data reception endpoints with mock subscription data
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
  - [ ] Explain frontend-driven billing model
  - [ ] Document API endpoints for receiving billing data 
  - [ ] Outline usage tracking and eligibility enforcement
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