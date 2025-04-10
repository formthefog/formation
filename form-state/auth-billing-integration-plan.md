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
- [ ] Create UserRole enum
  - [ ] Define Admin, Developer, and User roles
  - [ ] Implement serialization/deserialization for roles
- [ ] Implement role-based extractors
  - [ ] Create AdminOnly extractor
  - [ ] Create DeveloperOrAdmin extractor
  - [ ] Create custom rejection handlers for unauthorized access
- [ ] Set up role validation functions
  - [ ] Create helpers to check role permissions for specific operations

### 4. Add Project-Based Resource Control
- [ ] Modify agent operations to verify project ownership
  - [ ] Add project ID to AIAgent struct
  - [ ] Update AgentState to validate project access
  - [ ] Implement project validation in agent routes
- [ ] Modify model operations to verify project ownership
  - [ ] Add project ID to AIModel struct
  - [ ] Update ModelState to validate project access
  - [ ] Implement project validation in model routes
- [ ] Implement cross-project sharing (if required)
  - [ ] Create access control model for shared resources
  - [ ] Add shared access validation to state operations

## Phase 3: Stripe Billing Integration

### 5. Set Up Stripe Client
- [ ] Add Stripe crate dependencies
  - [ ] Select appropriate Stripe library (async-stripe recommended)
  - [ ] Configure with environment variables
- [ ] Create Stripe client configuration
  - [ ] Set up API key handling
  - [ ] Configure webhooks and event handling
- [ ] Implement customer and subscription management
  - [ ] Create function to register new Stripe customers
  - [ ] Implement subscription creation and plan selection
  - [ ] Set up database for tracking customer/subscription IDs

### 6. Implement Usage Tracking
- [ ] Extend ModelState with usage tracking
  - [ ] Add counters for token consumption
  - [ ] Track request counts and other metrics
  - [ ] Implement atomic increment operations
- [ ] Extend AgentState with usage tracking
  - [ ] Track agent invocations and runtime
  - [ ] Add metering for API calls
  - [ ] Implement time-windowed usage stats
- [ ] Create usage reporting system
  - [ ] Set up periodic batch reporting to Stripe
  - [ ] Implement background task for usage aggregation
  - [ ] Add retry logic for failed reporting

### 7. Add Quota Enforcement
- [ ] Implement quota checking middleware
  - [ ] Create usage limits based on subscription tiers
  - [ ] Add project-level quota tracking
  - [ ] Implement token bucket rate limiting if needed
- [ ] Create plan-based limits
  - [ ] Define subscription tiers and their quotas
  - [ ] Implement quota checking before processing requests
  - [ ] Create usage projection utilities
- [ ] Set up rejection handling
  - [ ] Create standardized responses for quota exceeded
  - [ ] Add upgrade prompts in quota exceeded responses
  - [ ] Implement graceful degradation for near-limit usage

## Phase 4: API Enhancements

### 8. Upgrade Existing API Endpoints
- [ ] Refactor model endpoints
  - [ ] Add auth middleware to all protected routes
  - [ ] Integrate usage tracking and reporting
  - [ ] Add project context to requests and responses
- [ ] Refactor agent endpoints
  - [ ] Secure all protected operations
  - [ ] Add billing integration to agent operations
  - [ ] Implement usage metering for agent calls
- [ ] Update API response formats
  - [ ] Add usage information to relevant responses
  - [ ] Include quota status in response metadata
  - [ ] Standardize error responses for auth/billing failures

### 9. Create Admin and Billing Management APIs
- [ ] Add subscription management endpoints
  - [ ] Create endpoints for viewing/changing plans
  - [ ] Implement payment method management
  - [ ] Add invoice and payment history access
- [ ] Create usage reporting APIs
  - [ ] Add endpoints for usage statistics
  - [ ] Implement project-level usage aggregation
  - [ ] Create usage forecasting endpoints
- [ ] Implement Stripe webhook handlers
  - [ ] Add handlers for payment events
  - [ ] Create subscription status change handlers
  - [ ] Implement invoice event processing

## Phase 5: Testing and Documentation

### 10. Create Comprehensive Tests
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

### 11. Update Documentation
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