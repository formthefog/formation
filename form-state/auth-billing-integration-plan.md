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
- [ ] Implement feature flagging system
  - [ ] Add `devnet` Cargo feature flag
  - [ ] Create conditional compilation paths throughout the codebase
  - [ ] Configure default feature settings in `Cargo.toml`
- [ ] Refactor core components for devnet mode
  - [ ] Create mock versions of critical services that depend on distributed components
  - [ ] Implement in-memory storage for devnet mode
  - [ ] Add configuration options to enable/disable message queue integration

### 8. Split API and Queue Processing Components
- [ ] Refactor `run` function in `api.rs`
  - [ ] Extract API server into separate `run_api` function
  - [ ] Move queue processing loop into dedicated `run_queue_reader` function
  - [ ] Ensure both components can be started independently
- [ ] Update `main.rs` with execution modes
  - [ ] Add command-line flag to control execution mode (api-only, queue-only, or both)
  - [ ] Create configuration for message queue connection in production
  - [ ] Add logic to conditionally start components based on mode

### 9. Implement Conditional Write-to-Queue
- [ ] Refactor `write_to_queue` method in `datastore.rs`
  - [ ] Add conditional execution based on runtime mode or compile-time feature flag
  - [ ] Create no-op implementation for devnet mode
  - [ ] Log write operations in devnet mode without actual queue writes
- [ ] Modify message handlers for devnet
  - [ ] Create direct application of operations in devnet mode
  - [ ] Bypass queue for immediate state changes in single-instance mode
  - [ ] Maintain operational parity between modes

### 10. Create Mock Services
- [ ] Implement mock model and agent services
  - [ ] Create `mock_models.rs` with sample AI models
  - [ ] Create `mock_agents.rs` with example agent configurations
  - [ ] Implement deterministic response generation for testing
- [ ] Add mock data initialization
  - [ ] Create function to populate datastore with mock entities
  - [ ] Add sample accounts with different permissions
  - [ ] Create realistic test dataset for development

### 11. Add Testing and Development Tools
- [ ] Create development script
  - [ ] Implement `scripts/run_devnet.sh` for easy local testing
  - [ ] Add options for data persistence between runs
  - [ ] Create parameter passing for various configurations
- [ ] Add documentation for devnet mode
  - [ ] Document all devnet features and limitations
  - [ ] Create quick-start guide for local development
  - [ ] Add examples of common development workflows

## Phase 5: API Enhancements and API Key Management

### 12. Upgrade Existing API Endpoints
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

### 13. Create Account and Usage Management APIs
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

### 14. Define API Key Infrastructure
- [ ] Design API key data structures
  - [ ] Create `ApiKey` struct with name, key ID, hashed secret, creation date, expiration date, and permissions
  - [ ] Define API key permission scopes (read-only, read-write, admin, etc.)
  - [ ] Create API key status enum (active, revoked, expired)
- [ ] Set up API key storage
  - [ ] Extend `Account` struct to store associated API keys
  - [ ] Implement hash-based storage for API key secrets (never store in plaintext)
  - [ ] Create database table/CRDT structure for persistent storage

### 15. Implement API Key Generation
- [ ] Create secure key generation system
  - [ ] Implement cryptographically secure random generation for API key secrets
  - [ ] Design key format with prefixes for identification (e.g., `fs_live_` for production keys)
  - [ ] Generate unique key IDs that are separate from the secret parts
- [ ] Develop key issuance endpoints
  - [ ] Create API endpoint for generating new API keys (`POST /api-keys/create`)
  - [ ] Add required parameters (name, permissions, expiration)
  - [ ] Limit the number of API keys per account based on tier
- [ ] Implement one-time secret display
  - [ ] Set up secure one-time transmission of the secret key
  - [ ] Store only the hashed version in the database
  - [ ] Add clear warnings about the inability to retrieve the secret later

### 16. Create API Key Authentication and Management
- [ ] Implement API key authentication middleware
  - [ ] Create an extraction method for API key from Authorization header
  - [ ] Support both Bearer token format and custom X-API-Key header
  - [ ] Verify API key using time-constant comparison
- [ ] Integrate with existing auth system
  - [ ] Set up API key middleware as an alternative to JWT
  - [ ] Create middleware chain to try both auth methods
  - [ ] Prioritize API key auth for programmatic endpoints
- [ ] Add rate limiting for API keys
  - [ ] Implement per-key rate limiting middleware
  - [ ] Set up tiered rate limits based on account subscription
  - [ ] Add headers for rate limit status in responses
- [ ] Create API key management endpoints
  - [ ] Implement listing all keys for an account (`GET /api-keys`)
  - [ ] Add endpoint to view a specific key's metadata (`GET /api-keys/:id`)
  - [ ] Create endpoints for updating key metadata (`PATCH /api-keys/:id`)
- [ ] Implement key revocation system
  - [ ] Add endpoint to revoke API keys (`DELETE /api-keys/:id`)
  - [ ] Create key rotation endpoint for seamless key updates
  - [ ] Build temporary dual-auth periods during rotation
- [ ] Add audit logging for API key usage
  - [ ] Create `ApiKeyEvent` struct for tracking key usage
  - [ ] Log all API key actions (creation, revocation, usage)
  - [ ] Implement API key usage reporting

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