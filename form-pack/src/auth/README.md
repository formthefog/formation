# Authentication Module for Form Pack

This module provides JWT-based authentication for the Form Pack service, similar to what exists in Form State.

## Features

1. JWT token validation
2. Role-based access control
3. Project access verification
4. Inter-service authentication

## Usage

### Middleware

The authentication middleware can be applied to routes:

```rust
use axum::middleware;
use crate::auth::{jwt_auth_middleware, AuthConfig, JwtClient};

// Initialize auth client
let auth_config = AuthConfig::from_env();
let jwt_client = Arc::new(JwtClient::new(auth_config));

// Apply middleware to routes
let app = Router::new()
    .route("/some-protected-route", post(my_handler))
    .with_state(jwt_client.clone())
    .route_layer(middleware::from_fn_with_state(
        jwt_client,
        jwt_auth_middleware
    ));
```

### Request Handlers

In your request handlers, you can access the authenticated user's claims:

```rust
use axum::Extension;
use crate::auth::JwtClaims;

async fn my_handler(
    Extension(claims): Extension<JwtClaims>,
    // other parameters...
) -> impl IntoResponse {
    // Access user information
    let user_id = claims.sub;
    let role = claims.role;
    
    // Check project access
    if claims.has_project_access("project-123") {
        // Allow operation
    }
    
    // Rest of handler
}
```

### Making Authenticated Inter-service Calls

To make an authenticated call to another service:

```rust
use crate::auth::jwt_client::JwtClient;

async fn call_another_service(jwt_client: &JwtClient, auth_token: &str) {
    let response = jwt_client.call_service_with_auth(
        reqwest::Method::GET,
        "https://other-service/api/endpoint",
        auth_token,
        Some(serde_json::json!({ "key": "value" })),
    ).await.unwrap();
    
    // Process response
}
```

## Configuration

Set the following environment variables:

```
JWKS_URL=https://auth.formation.dev/.well-known/jwks.json
AUTH_AUDIENCE=https://api.formation.dev
AUTH_ISSUER=https://auth.formation.dev/
API_GATEWAY_URL=https://api.formation.dev
AUTH_SERVICES_URL=https://auth.formation.dev
``` 