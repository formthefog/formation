# Authentication for FormPack

This directory contains authentication modules for the FormPack service.

## Authentication Methods

FormPack now supports three authentication methods:

1. **Signature-based Authentication (Recommended)** - ECDSA signatures for secure and decentralized authentication
2. **JWT-based Authentication (Deprecated)** - Traditional JWT token-based authentication
3. **API Key Authentication (Deprecated)** - Simple API key-based authentication

The signature-based authentication is recommended for all new integrations, as the JWT and API Key methods will be removed in a future version.

## Using Signature-based Authentication

### Configuration

Configure signature authentication by setting these environment variables:

```
# Comma-separated list of authorized public keys (hex-encoded)
AUTH_PUBKEYS=046a04c1f05384c734c5dbe48f9df93b2234fb92534fb4f10f6e53dace81c12b52781cb8172225da30d4d6a8e06de1e52db0749ad41cdfa36a5dbb281703c0e430

# Comma-separated list of paths that bypass auth (default: "/health,/ping")
AUTH_BYPASS_PATHS=/health,/ping
```

### Client-Side Signing

Clients need to:

1. Create a message hash from the request data (typically the endpoint path)
2. Sign the hash with their ECDSA private key
3. Add the following headers to their request:
   - `X-Signature`: Hex-encoded signature (64 bytes)
   - `X-Recovery-ID`: Hex-encoded recovery ID (1 byte, usually "00" or "01")
   - `X-Timestamp`: Current UNIX timestamp in seconds

### Example Client Code

```rust
use k256::{
    ecdsa::{SigningKey, signature::Signer},
    SecretKey,
};
use sha2::{Sha256, Digest};
use std::time::{SystemTime, UNIX_EPOCH};
use hex;

// Load private key
let private_key = SigningKey::from_bytes(&[/* your key bytes */])?;

// Get current timestamp
let timestamp = SystemTime::now()
    .duration_since(UNIX_EPOCH)
    .unwrap()
    .as_secs();

// Endpoint path (used as message)
let path = "/build";

// Create hash to sign
let mut hasher = Sha256::new();
hasher.update(path.as_bytes());
hasher.update(timestamp.to_string().as_bytes());
let hash = hasher.finalize();

// Sign the hash
let signature = private_key.sign(&hash);
let signature_bytes = signature.to_bytes();
let recid = 0; // Use appropriate recovery ID

// Add to headers
let headers = reqwest::header::HeaderMap::new();
headers.insert("X-Signature", hex::encode(signature_bytes).parse().unwrap());
headers.insert("X-Recovery-ID", format!("{:02x}", recid).parse().unwrap());
headers.insert("X-Timestamp", timestamp.to_string().parse().unwrap());

// Make request with headers
let client = reqwest::Client::new();
let response = client.post("https://api.formation.fi/build")
    .headers(headers)
    .send()
    .await?;
```

## Legacy Authentication Methods (Deprecated)

### JWT Authentication

JWT authentication uses traditional token-based authentication with JWKs for validation.

Environment variables:
- `JWT_URL`: URL for JWKS endpoint
- `JWT_AUDIENCE`: Expected audience in JWT claims
- `JWT_ISSUER`: Expected issuer in JWT claims

### API Key Authentication

API key authentication uses simple keys for authentication.

Environment variables:
- `API_KEY_TEST`: API key for testing
- `API_KEY_PREFIX`: Required prefix for API keys (default: "form-")

## Integration Example

For examples of how to use signature-based auth in your service, see [FormAuth Documentation](../../form-auth/README.md). 