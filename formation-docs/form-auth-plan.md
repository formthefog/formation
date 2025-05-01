# Authentication Standardization Plan

## Overview

This document outlines the plan to standardize authentication across Formation services by creating a dedicated `form-auth` crate. Currently, we have inconsistent implementations between services like `form-state` and `form-pack`, which creates maintenance challenges and potential security issues.

## Current State Analysis

### Authentication Types
- **JWT/JWK Auth**: Used for account creation and core identity verification
- **API Keys**: Used for service-to-service communication and programmatic access
- **No ECDSA**: Not currently implemented but desired

### Implementation Differences
- **form-state** and **form-pack** have similar but inconsistent implementations
- Different error handling and response formats
- Slightly different middleware chains
- Shared concepts but different code

## Proposed Architecture

### Create `form-auth` Crate
A dedicated authentication crate with these modules:

1. **Core Module**
   - `AuthInfo` - Common authentication info representation
   - `AuthError` - Standardized error types
   - `AuthConfig` - Configuration options

2. **Providers Module**
   - `JwtProvider` - JWT validation and claims handling
   - `ApiKeyProvider` - API key validation and scope handling
   - `EcdsaProvider` - New provider for cryptographic signatures
   - `ProviderRegistry` - Registry pattern to manage multiple providers

3. **Middleware Module**
   - `AuthMiddleware` - Single middleware that tries providers in sequence
   - Common error response handling

4. **Client Module**
   - `AuthClient` - Make authenticated calls to other services
   - Support for forwarding credentials

## Implementation Approach

1. **Extract & Standardize**
   - Extract existing JWT and API key implementations
   - Create consistent interfaces and error handling
   - Standardize on response types and error codes

2. **Add ECDSA Authentication**
   - Implement signature verification
   - Define standards for key formats and claims

3. **Create Registry Pattern**
   - Allow services to register which auth providers they support
   - Configure priority and fallback behavior

4. **Implement Unified Middleware**
   - Single middleware that tries multiple auth methods
   - Consistent error responses across services

## Project Structure

```
form-workspace/
├── form-auth/                      # New crate
│   ├── src/
│   │   ├── core.rs                 # Common types and traits
│   │   ├── providers/              # Auth providers
│   │   │   ├── jwt.rs              # JWT provider
│   │   │   ├── api_key.rs          # API key provider
│   │   │   ├── ecdsa.rs            # ECDSA provider
│   │   │   └── registry.rs         # Provider registry
│   │   ├── middleware.rs           # Unified middleware
│   │   ├── client.rs               # Auth client for services
│   │   └── lib.rs                  # Public exports
│   └── Cargo.toml
├── form-pack/                      # Updated to use form-auth
└── form-state/                     # Updated to use form-auth
```

## Detailed Implementation Subtasks

### Phase 1: Setup and Core Implementation

1. **Create `form-auth` crate structure**
   - Create directory and basic file structure
   - Set up Cargo.toml with dependencies
   - Add crate to workspace

2. **Define core authentication types**
   - Implement `AuthInfo` trait and struct
   - Create standardized `AuthError` enum
   - Build `AuthConfig` struct for configuration

3. **Create provider interface**
   - Define `AuthProvider` trait
   - Implement provider registration mechanism
   - Add provider priority/fallback system

### Phase 2: Implement Authentication Providers

4. **Implement JWT provider**
   - Extract JWT logic from existing services
   - Create standardized JWT validation flow
   - Implement claims extraction and verification

5. **Implement API key provider**
   - Extract API key logic from existing services
   - Standardize API key validation
   - Implement scope-based permission checks

6. **Create provider registry**
   - Implement registry pattern for providers
   - Add configuration options for enabled providers
   - Create fallback chain mechanism

### Phase 3: Middleware and Client Implementation

7. **Implement unified middleware**
   - Create single middleware that uses provider registry
   - Standardize request/response flow
   - Add bypass options for health endpoints

8. **Build authentication client**
   - Implement client for service-to-service calls
   - Add credential forwarding capabilities
   - Create helpers for common authentication patterns

9. **Add error handling and responses**
   - Standardize error response format
   - Implement consistent status codes
   - Create helpful error messages

### Phase 4: Testing and Integration

10. **Create comprehensive test suite**
    - Unit tests for each provider
    - Integration tests for middleware
    - Mock services for client testing

11. **Update form-pack to use form-auth**
    - Replace current authentication with form-auth
    - Migrate middleware to new unified approach
    - Update service-to-service calls

12. **Update form-state to use form-auth**
    - Apply same migration process
    - Ensure consistent behavior with form-pack
    - Fix any compatibility issues

### Phase 5: Advanced Features and Optimization

13. **Implement ECDSA provider**
    - Add cryptographic signature verification
    - Create key management utilities
    - Document ECDSA token format

14. **Add caching layer**
    - Implement token/key caching
    - Add cache invalidation mechanism
    - Optimize performance

15. **Enhance documentation**
    - Create comprehensive API docs
    - Add usage examples
    - Document configuration options

### Phase 6: Finalization

16. **Performance testing and optimization**
    - Benchmark authentication flow
    - Identify and fix bottlenecks
    - Optimize critical paths

17. **Security audit**
    - Review authentication flow for vulnerabilities
    - Check for common security issues
    - Document security considerations

18. **Final integration and testing**
    - End-to-end testing across services
    - Verify all authentication paths
    - Document any known limitations

## Benefits

- **Consistency**: Same auth behavior across all services
- **Flexibility**: Easy to add new auth methods
- **Maintenance**: Single codebase for auth logic
- **Security**: Standardized security practices
- **Interoperability**: Services can reliably communicate with shared auth

## Migration Strategy

1. Create the new `form-auth` crate
2. Implement core functionality with tests
3. Update one service (form-pack) to use the new crate
4. Fix any issues and refine the API
5. Update remaining services (form-state)
6. Add ECDSA support after migration is complete 