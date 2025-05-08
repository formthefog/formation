# Simplified Localhost Auth Bypass

## Overview

This document outlines a minimal implementation approach for adding localhost authentication bypass to form-state. The goal is to allow services running on the same machine to communicate without requiring API keys in development environments.

## Implementation Approach

### 1. Modify API Key Auth Middleware

The only change needed is to modify the `api_key_auth_middleware` function in `src/api_keys/middleware.rs` to check for localhost requests first:

```rust
pub async fn api_key_auth_middleware(
    State(state): State<Arc<Mutex<DataStore>>>,
    request: Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    // Check if request is from localhost - bypass auth if it is
    if is_localhost_request(&request) {
        log::info!("Localhost detected, bypassing API key authentication");
        return Ok(next.run(request).await);
    }
    
    // Normal authentication flow for non-localhost requests
    // ... (existing code remains unchanged) ...
}
```

We can use the existing `is_localhost_request` function from `api.rs` by making it public and importing it:

```rust
// In api.rs - change from private to public
pub fn is_localhost_request(req: &Request<Body>) -> bool {
    // ... existing implementation ...
}

// In middleware.rs - add import
use crate::api::is_localhost_request;
```

## Implementation Steps

1. Make `is_localhost_request` function public in `src/api.rs`
2. Import the function in `src/api_keys/middleware.rs`
3. Update `api_key_auth_middleware` to check for localhost requests first and bypass auth if found
4. Test the implementation

## Testing Commands

Once implemented, test with:

```bash
# Should work without API key when called from localhost
curl -v http://localhost:3004/api/v1/agents
```

This simple approach should be sufficient for development purposes. If we encounter issues with handlers that expect auth objects in the request extensions, we can address those specifically later. 