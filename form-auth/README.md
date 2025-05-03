# Form Auth

ECDSA signature-based authentication for Formation microservices.

## Features

- ECDSA signature verification and public key recovery
- Signature data extraction from HTTP headers or request body
- Axum middleware for easy integration
- Path-based bypass for health checks and other public endpoints
- Configurable via environment variables

## Usage

### Adding to Your Service

```rust
use std::sync::Arc;
use axum::{
    Router,
    routing::get,
    middleware,
};
use form_auth::middleware::{auth_middleware, AuthConfig};

#[tokio::main]
async fn main() {
    // Create auth config
    let auth_config = Arc::new(AuthConfig::from_env());
    
    // Build router with auth middleware
    let app = Router::new()
        .route("/protected", get(protected_handler))
        .route("/health", get(health_handler))
        .layer(middleware::from_fn_with_state(
            auth_config.clone(),
            auth_middleware,
        ));
    
    // Start server
    // ...
}
```

### Using Authentication in Handlers

```rust
use axum::{
    extract::Request,
    response::IntoResponse,
};
use form_auth::middleware::{extract_auth, require_authorized};

async fn protected_handler(req: Request) -> impl IntoResponse {
    // Get authenticated public key info
    // Use extract_auth() if you don't need to check authorization
    let auth = require_authorized(&req).unwrap();
    
    // You can now use the authenticated public key
    format!("Authenticated with public key: {}", auth.public_key_hex)
}
```

### Client-Side Signing

Clients need to:

1. Create a message hash from the request data
2. Sign the hash with their ECDSA private key
3. Send the signature, recovery ID, and timestamp in headers or request body

Example for client-side signing:

```rust
use k256::{
    ecdsa::{SigningKey, signature::Signer},
    SecretKey,
};
use sha2::{Sha256, Digest};
use std::time::{SystemTime, UNIX_EPOCH};
use hex;

// Create a private key (in practice, this would be loaded securely)
let private_key = SigningKey::random(&mut rand::thread_rng());

// Get current timestamp
let timestamp = SystemTime::now()
    .duration_since(UNIX_EPOCH)
    .unwrap()
    .as_secs();

// Create message to sign (typically this would be request data)
let message = "Hello, world!";

// Create hash to sign
let mut hasher = Sha256::new();
hasher.update(message.as_bytes());
hasher.update(timestamp.to_string().as_bytes());
let hash = hasher.finalize();

// Sign the hash
let (signature, recid) = private_key.sign_recoverable(hash.as_slice());

// Format for sending
let signature_hex = hex::encode(signature.to_bytes());
let recovery_id_hex = hex::encode([recid.to_byte()]);

println!("Signature: {}", signature_hex);
println!("Recovery ID: {}", recovery_id_hex);
println!("Timestamp: {}", timestamp);
```

### Environment Variables

- `AUTH_PUBKEYS`: Comma-separated list of authorized public keys (hex-encoded)
- `AUTH_BYPASS_PATHS`: Comma-separated list of paths that bypass auth (default: "/health,/ping")

## Customization

The library is designed to be customized as needed:

1. Change how messages are constructed from requests
2. Modify signature extraction logic
3. Implement custom authorization rules
4. Add additional verification steps

## License

MIT License 