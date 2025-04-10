# JWT Authentication System

This module provides a complete JWT authentication system with JWKS key refreshing and caching for Form Network services.

## Features

- **Dynamic Configuration**: Configuration via environment variables or programmatically
- **JWKS Key Management**: Automatic fetching and caching of JWKS keys
- **Background Refreshing**: Keys are refreshed in the background based on a configurable interval
- **Token Validation**: Comprehensive JWT validation including signature, expiration, audience, and issuer
- **Role-Based Access Control**: Support for role-based authentication with type-safe extractors
- **Middleware Integration**: Ready-to-use middleware for Axum web applications

## Usage

### Basic Setup

```rust
use axum::{Router, routing::get};
use form_state::auth::{JWKSManager, jwt_auth_middleware};
use std::sync::Arc;

#[tokio::main]
async fn main() {
    // Initialize environment variables (e.g., using dotenv)
    // DYNAMIC_JWKS_URL must be set, others are optional
    
    // Create the JWKS manager
    let jwks_manager = Arc::new(JWKSManager::new());
    
    // Build your application with routes
    let app = Router::new()
        .route("/public", get(public_handler))
        .route("/api/protected", get(protected_handler))
        .layer(axum::middleware::from_fn_with_state(
            jwks_manager.clone(),
            jwt_auth_middleware
        ))
        .with_state(jwks_manager);
    
    // Run your application
    // ...
}
```

### Using the Helper Functions

```rust
use form_state::auth::jwks::{init_jwks_manager, force_jwks_refresh};

#[tokio::main]
async fn main() {
    // Initialize with helper function (creates Arc<JWKSManager>)
    let jwks_manager = init_jwks_manager();
    
    // Force a refresh if needed
    if let Err(e) = force_jwks_refresh(&jwks_manager).await {
        log::error!("Failed to refresh JWKS: {}", e);
    }
    
    // Continue with app setup
    // ...
}
```

### Protected Endpoints with Role-Based Access

```rust
use axum::extract::State;
use form_state::auth::{JwtClaims, AdminClaims, UserRole, verify_role};

// Endpoint requiring any authenticated user
async fn protected_handler(
    claims: JwtClaims
) -> String {
    format!("Hello, authenticated user: {}", claims.0.sub)
}

// Endpoint requiring admin role
async fn admin_handler(
    claims: AdminClaims
) -> String {
    format!("Hello, admin: {}", claims.0.sub)
}

// Manual role checking
async fn developer_handler(
    claims: JwtClaims,
) -> Result<String, StatusCode> {
    // Check if user has Developer role
    verify_role(&claims.0, UserRole::Developer)
        .map_err(|_| StatusCode::FORBIDDEN)?;
    
    Ok(format!("Hello, developer: {}", claims.0.sub))
}
```

## Configuration

The system uses the following environment variables:

- `DYNAMIC_JWKS_URL` (required): URL to fetch JWKS keys
- `DYNAMIC_ISSUER` (optional): Expected issuer in JWT claims
- `DYNAMIC_AUDIENCE` (optional): Expected audience in JWT claims
- `DYNAMIC_JWT_LEEWAY` (optional): Leeway in seconds for time-based validation (default: 60)

## API Reference

Refer to the module documentation for detailed API information on each component:

- `AuthConfig`: Configuration for JWT validation
- `JWKSManager`: Manages JWKS keys, refreshing, and caching
- `DynamicClaims`: JWT claims structure
- `UserRole`: Role-based permission system
- Middleware and extractors: `jwt_auth_middleware`, `JwtClaims`, etc.

A complete example is available in `examples/auth-example.rs`. 