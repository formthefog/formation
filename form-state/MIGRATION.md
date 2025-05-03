# Authentication Migration Guide

This document outlines the migration from JWT and API key authentication to signature-based authentication.

## Overview

The Formation platform is transitioning from using JWT and API key authentication to signature-based authentication based on ECDSA signatures. This change provides several benefits:

1. **Better security**: Signature-based authentication is more secure and resistant to token theft.
2. **Simplified auth model**: One unified authentication mechanism for all services.
3. **Better compatibility with blockchain ecosystem**: ECDSA signatures are widely used in blockchain applications.

## Deprecated Modules

The following modules have been marked as deprecated and will be removed in a future version:

- `form-state/src/auth/*`: JWT authentication
- `form-state/src/api_keys/*`: API key authentication

## New Signature-Based Authentication

The new authentication system is implemented in `form-state/src/signature_auth.rs`.

### Key Features

- Uses ECDSA signatures based on secp256k1 (compatible with Ethereum and other blockchains)
- Signature verification with timestamp validation
- Account association based on public key

## Migration Steps for API Consumers

If you're consuming the API, follow these steps to migrate:

1. Generate an ECDSA key pair (secp256k1)
2. Register your public key with the system by creating an account
3. For each API request:
   - Generate a timestamp
   - Create a message to sign (typically the request body or path)
   - Sign the message with your private key
   - Include the signature, recovery ID, and timestamp in the request headers:
     - `X-Signature`: The hex-encoded signature
     - `X-Recovery-ID`: The recovery ID (typically 0 or 1)
     - `X-Timestamp`: The timestamp used for signing

## Migration Steps for Developers

If you're working on the codebase, follow these steps to migrate code that uses the old authentication:

1. Replace imports from `crate::auth` with imports from `crate::signature_auth`
2. Update route handlers to accept `SignatureAuth` instead of `JwtClaims` or `ApiKeyAuth`
3. Remove JWT/API key validation logic and use signature validation

Example:

```rust
// Old code
pub async fn my_handler(
    auth: ApiKeyAuth,
    // ...
) -> impl IntoResponse {
    // ...
}

// New code
pub async fn my_handler(
    auth: crate::signature_auth::SignatureAuth,
    // ...
) -> impl IntoResponse {
    // ...
}
```

## Testing

You can test the new authentication system using the provided test script:

```bash
./test_api_with_registration.sh
```

This script:
1. Generates a key pair
2. Registers an account with the public key
3. Signs and sends test requests to protected endpoints

## Timeline

- **Current phase**: Dual support for both authentication methods
- **Next release**: JWT and API key auth marked as deprecated
- **Future release**: Complete removal of deprecated authentication modules

## Questions

If you have questions or need help with the migration, please contact the Formation team. 