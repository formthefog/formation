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
- [x] Extend Account with comprehensive usage tracking
  - [x] Add token consumption tracking structure
  - [x] Track agent invocations and request counts
  - [x] Implement time-windowed usage stats
  - [x] Store per-model and per-agent metrics within Account
- [x] Create account-based usage recording API
  - [x] Implement `record_token_usage(model_id, token_count)`
  - [x] Implement `record_agent_usage(agent_id, operation_type)`
  - [x] Add atomic increment operations for thread safety
  - [x] Create usage aggregation methods
- [x] Implement credit management within Account
  - [x] Set up credit tracking and balance management
  - [x] Implement credit transaction history
  - [x] Create methods for credit consumption and addition
  - [x] Add reporting for available credits

### 6. Add Eligibility Enforcement
- [x] Implement account-based eligibility checking
  - [x] Create `can_use_tokens(model_id, token_count)` method on Account
  - [x] Create `can_hire_agent(agent_id)` method on Account 
  - [x] Implement credit checking for operations
- [x] Create plan-based limits
  - [x] Define subscription tiers and their quotas (agent slots and credits)
  - [x] Implement tier-based eligibility rules
  - [x] Create usage projection utilities
- [x] Set up rejection handling
  - [x] Create standardized responses for insufficient credits
  - [x] Add upgrade prompts in limit exceeded responses
  - [x] Implement graceful handling for users at their limits

## Phase 4: Mock Datastore Server

### 7. Create Devnet Execution Mode
- [x] Implement feature flagging system
  - [x] Add `devnet` Cargo feature flag
  - [x] Create conditional compilation paths throughout the codebase
  - [x] Configure default feature settings in `Cargo.toml`
- [x] Refactor core components for devnet mode
  - [x] Create mock versions of critical services that depend on distributed components
  - [x] Implement in-memory storage for devnet mode
  - [x] Add configuration options to enable/disable message queue integration

### 8. Split API and Queue Processing Components
- [x] Refactor `run` function in `api.rs`
  - [x] Extract API server into separate `run_api` function
  - [x] Move queue processing loop into dedicated `run_queue_reader` function
  - [x] Ensure both components can be started independently
- [x] Update `main.rs` with execution modes
  - [x] Add compilation flag to control queue behavior
  - [x] Create configuration for message queue connection in production
  - [x] Add logging to indicate devnet/production mode

### 9. Implement Conditional Write-to-Queue
- [x] Refactor `write_to_queue` method in `datastore.rs`
  - [x] Add conditional execution based on compile-time feature flag
  - [x] Create no-op implementation for devnet mode
  - [x] Log write operations in devnet mode without actual queue writes
- [x] Modify message handlers for devnet
  - [x] Create direct application of operations in devnet mode
  - [x] Bypass queue for immediate state changes in single-instance mode
  - [x] Maintain operational parity between modes

### 10. Create Mock Services
- [x] Implement mock model and agent services
  - [x] Create simple mock server with built-in capabilities for testing
  - [x] Leverage existing DataStore initialization
  - [x] Avoid complex mock data structure for simplicity
- [x] Add mock data initialization
  - [x] Utilize built-in DataStore initialization
  - [x] Leverage existing functionality in main.rs
  - [x] Create simple, clean mock server implementation

### 11. Add Testing and Development Tools
- [x] Create development script
  - [x] Implement `scripts/run_mock_server.sh` for easy local testing
  - [x] Add options for JWT configuration
  - [x] Create parameter passing for various configurations
- [x] Add documentation for devnet mode
  - [x] Document usage in script help text
  - [x] Create quick-start guide via command-line arguments
  - [x] Include common development configuration options

## Phase 5: API Enhancements and API Key Management

### 12. Upgrade Existing API Endpoints
- [x] Refactor model endpoints
  - [x] Add auth middleware to all protected routes
  - [x] Integrate account-based usage tracking
  - [x] Add eligibility checking before processing requests
- [x] Refactor agent endpoints
  - [x] Secure all protected operations
  - [x] Add account-based hiring slot verification
  - [x] Implement usage metering via account methods
- [x] Update API response formats
  - [x] Add credit balance to relevant responses
  - [x] Include subscription status in response metadata
  - [x] Standardize error responses for eligibility failures

### 13. Create Account and Usage Management APIs
- [x] Add subscription status endpoints
  - [x] Create endpoints for viewing current plan
  - [x] Implement endpoints to check available credits/slots
  - [x] Add usage history access
- [x] Create usage reporting APIs
  - [x] Add endpoints for usage statistics
  - [x] Implement credit consumption history
  - [x] Create usage forecasting endpoints
- [x] Implement data reception endpoints
  - [x] Add handlers for receiving subscription updates from frontend
  - [x] Create credit balance update handlers
  - [x] Implement event processing for plan changes

### 14. Define API Key Infrastructure
- [x] Design API key data structures
  - [x] Create `ApiKey` struct with name, key ID, hashed secret, creation date, expiration date, and permissions
  - [x] Define API key permission scopes (read-only, read-write, admin, etc.)
  - [x] Create API key status enum (active, revoked, expired)
- [x] Set up API key storage
  - [x] Extend `Account` struct to store associated API keys
  - [x] Implement hash-based storage for API key secrets (never store in plaintext)
  - [x] Create database table/CRDT structure for persistent storage

### 15. Implement API Key Generation
- [x] Create secure key generation system
  - [x] Implement cryptographically secure random generation for API key secrets
  - [x] Design key format with prefixes for identification (e.g., `fs_live_` for production keys)
  - [x] Generate unique key IDs that are separate from the secret parts
- [x] Develop key issuance endpoints
  - [x] Create API endpoint for generating new API keys (`POST /api-keys/create`)
  - [x] Add required parameters (name, permissions, expiration)
  - [x] Limit the number of API keys per account based on tier
- [x] Implement one-time secret display
  - [x] Set up secure one-time transmission of the secret key
  - [x] Store only the hashed version in the database
  - [x] Add clear warnings about the inability to retrieve the secret later

### 16. Create API Key Authentication and Management
- [x] Implement API key authentication middleware
  - [x] Create an extraction method for API key from Authorization header
  - [x] Support both Bearer token format and custom X-API-Key header
  - [x] Verify API key using time-constant comparison
- [x] Integrate with existing auth system
  - [x] Set up API key middleware as an alternative to JWT
  - [x] Create middleware chain to try both auth methods
  - [x] Prioritize API key auth for programmatic endpoints
- [x] Add rate limiting for API keys
  - [x] Implement per-key rate limiting middleware
  - [x] Set up tiered rate limits based on account subscription
  - [x] Add headers for rate limit status in responses
- [x] Create API key management endpoints
  - [x] Implement listing all keys for an account (`GET /api-keys`)
  - [x] Add endpoint to view a specific key's metadata (`GET /api-keys/:id`)
  - [x] Create endpoints for updating key metadata (`PATCH /api-keys/:id`)
- [x] Implement key revocation system
  - [x] Add endpoint to revoke API keys (`DELETE /api-keys/:id`)
  - [x] Create key rotation endpoint for seamless key updates
  - [x] Build temporary dual-auth periods during rotation
- [x] Add audit logging for API key usage
  - [x] Create `ApiKeyEvent` struct for tracking key usage
  - [x] Log all API key actions (creation, revocation, usage)
  - [x] Implement API key usage reporting

## Phase 6: Testing and Documentation

### 17. Create Comprehensive Tests
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

### 18. Update Documentation
- [x] Add authentication documentation
  - [x] Document token requirements and format
    - JWT tokens require the following claims: `sub`, `project_id`, `role`
    - Tokens must be signed with RS256 and verified against JWKS
    - Tokens must include standard claims: `exp`, `iss`, `aud`
  - [x] Create examples for authenticated requests
    - Example: `Authorization: Bearer <jwt_token>`
    - Example: `X-API-Key: <api_key>`
  - [x] Document error codes and troubleshooting
    - 401: Unauthorized - Invalid or missing token/API key
    - 403: Forbidden - Insufficient permissions for the operation
    - 429: Too Many Requests - Rate limit exceeded
- [x] Create billing integration docs
  - [x] Document subscription plans and features
    - Free Tier: Limited credits, 1 agent slot, standard rate limits
    - Pro Tier: More credits, 5 agent slots, higher rate limits
    - Enterprise Tier: Custom credits, unlimited agent slots, custom rate limits
  - [x] Explain frontend-driven billing model
    - Frontend handles all Stripe interactions
    - Backend receives confirmed subscription data via secure endpoints
    - Credit purchases processed through Stripe Checkout
  - [x] Document API endpoints for receiving billing data 
    - POST `/billing/checkout/process`: Process Stripe checkout session
    - POST `/billing/credits/add`: Add credits to an account
    - GET `/billing/subscription`: Get current subscription status
    - GET `/billing/usage`: Get usage statistics
  - [x] Outline usage tracking and eligibility enforcement
    - Token usage tracked per model with cost calculation
    - Agent slots limited by subscription tier
    - Operation eligibility checked before processing requests
    - Credit balance enforced for all billable operations
- [x] Update API reference
  - [x] Add auth and billing parameters to all endpoints
    - All protected endpoints require JWT or API key authentication
    - Model inference endpoints include input/output token parameters
    - Agent hire endpoints check for available slots
  - [x] Document rate limits and quota constraints
    - Rate limits vary by subscription tier and API key scope
    - X-RateLimit-* headers included in all responses
    - Usage quotas enforced based on subscription tier
  - [x] Add sample responses for various scenarios
    - Success responses include remaining credits
    - Error responses for insufficient credits include upgrade options
    - API key management responses include audit information

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