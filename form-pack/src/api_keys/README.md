# API Keys Module for Form Pack

This module provides API key-based authentication for the Form Pack service, similar to what exists in Form State.

## Features

1. API key validation
2. Scope-based access control
3. Inter-service API key forwarding

## Usage

### Middleware

The API key middleware can be applied to routes:

```rust
use axum::middleware;
use crate::api_keys::{api_key_auth_middleware, ApiKeyClient};

// Initialize API key client
let api_key_client = Arc::new(ApiKeyClient::from_env());

// Apply middleware to routes
let app = Router::new()
    .route("/some-protected-route", post(my_handler))
    .with_state(api_key_client.clone())
    .route_layer(middleware::from_fn_with_state(
        api_key_client,
        api_key_auth_middleware
    ));
```

### Request Handlers

In your request handlers, you can access the API key information:

```rust
use axum::Extension;
use crate::api_keys::ApiKeyAuth;

async fn my_handler(
    Extension(api_key_auth): Extension<ApiKeyAuth>,
    // other parameters...
) -> impl IntoResponse {
    // Access API key information
    let key_id = api_key_auth.key_id;
    let account_id = api_key_auth.account_id;
    
    // Rest of handler
}
```

### Making Authenticated Inter-service Calls

To make an authenticated call to another service using an API key:

```rust
use crate::api_keys::client::ApiKeyClient;

async fn call_another_service(api_key_client: &ApiKeyClient, api_key: &str) {
    let response = api_key_client.call_service_with_api_key(
        reqwest::Method::GET,
        "https://other-service/api/endpoint",
        api_key,
        Some(serde_json::json!({ "key": "value" })),
    ).await.unwrap();
    
    // Process response
}
```

## Dual Authentication Support

Form Pack supports both JWT and API key authentication. Requests will be authenticated if either:

1. A valid JWT token is provided in the `Authorization: Bearer <token>` header
2. A valid API key is provided in the `X-API-Key: <api_key>` header

The authentication information will be passed along to other services when making inter-service calls.

## Configuration

Set the following environment variables:

```
API_KEY_SERVICE_URL=https://api.formation.dev/api-keys
``` 