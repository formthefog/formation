# Formation Platform Environment Variables

This document outlines the environment variables required for each Formation service.

## Core Services

### form-state

The state service manages the system's state database and requires the following environment variables:

| Variable | Description | Default | Required |
|----------|-------------|---------|----------|
| `DB_PATH` | Path to the state database | `/var/lib/formation/db/formation.db` | Yes |
| `SECRET_PATH` | Path to the operator configuration JSON file | `/etc/formation/.operator-config.json` | Yes |
| `PASSWORD` | Password used to decrypt the operator configuration | `formation-password` | Yes |
| `DEV_MODE` | Enable development mode | `true` | No |
| `AUTH_MODE` | Authentication mode | `development` | No |
| `DYNAMIC_JWKS_URL` | URL for JSON Web Key Set | `https://app.dynamic.xyz/api/v0/sdk/3f53e601-17c7-419b-8a13-4c5e25c0bde9/.well-known/jwks` | Yes |
| `TRUSTED_OPERATOR_KEYS` | Comma-separated list of trusted operator public keys | Empty | No |
| `ALLOW_INTERNAL_ENDPOINTS` | Whether to enable internal service endpoints | `true` | No |

Example `.env` file entries:
```
# Required for form-state
SECRET_PATH=/path/to/your/operator-config.json
PASSWORD=your-secure-password
DYNAMIC_JWKS_URL=https://your-jwks-provider.com/jwks
TRUSTED_OPERATOR_KEYS=0x1234abcd,0x5678efgh
```

### form-dns

The DNS service provides name resolution for the platform and requires the following environment variables:

| Variable | Description | Default | Required |
|----------|-------------|---------|----------|
| `DNS_LOG_LEVEL` | Log level for DNS service | `trace` | No |
| `RUST_LOG` | Rust logging level | `trace` | No |
| `RUST_BACKTRACE` | Rust backtrace settings | `full` | No |
| `DNS_PORT` | Port for DNS service | `53` | Yes |
| `STATE_URL` | URL for the state service | `http://localhost:3004` | Yes |
| `WAIT_FOR_STATE` | Whether to wait for state service before starting | `true` | No |

Example `.env` file entries:
```
# Required for form-dns
DNS_PORT=53
STATE_URL=http://localhost:3004
```

### form-net

The network service provides virtual network capabilities for the platform:

| Variable | Description | Default | Required |
|----------|-------------|---------|----------|
| `FORMNET_LOG_LEVEL` | Log level for formnet service | `debug` | No |
| `RUST_LOG` | Rust logging level | `debug` | No |
| `RUST_BACKTRACE` | Rust backtrace settings | `1` | No |
| `FORMNET_CONFIG_DIR` | Directory for formnet configuration | `/etc/formation` | Yes |
| `FORMNET_DATA_DIR` | Directory for formnet data storage | `/var/lib/formnet` | Yes |
| `FORMNET_NETWORK_NAME` | Name of the virtual network | `formnet` | No |
| `FORMNET_SERVER_PORT` | Port for the formnet API server | `8080` | No |
| `FORMNET_LISTEN_PORT` | Port for WireGuard VPN | `51820` | No |
| `FORMNET_EXTERNAL_ENDPOINT` | External endpoint for WireGuard | `auto` | No |
| `STATE_URL` | URL for the state service | `http://localhost:3004` | Yes |
| `SECRET_PATH` | Path to the operator configuration JSON file | `/etc/formation/.operator-config.json` | Yes |
| `PASSWORD` | Password used to decrypt the operator configuration | `formation-password` | Yes |
| `API_KEY` | Fallback API key for authentication with form-state | Empty | No |

Example `.env` file entries:
```
# Required for form-net
SECRET_PATH=/etc/formation/.operator-config.json
PASSWORD=your-secure-password

# Optional form-net customization
FORMNET_NETWORK_NAME=myformnet
FORMNET_LOG_LEVEL=debug
FORMNET_EXTERNAL_ENDPOINT=auto
API_KEY=your-api-key-if-needed
```

## Authentication

### Operator Public Key Authentication

The Formation platform uses a secure authentication method between services based on operator public keys:

1. **Setup Process**:
   - Each operator has a unique public/private key pair stored in the operator configuration
   - The form-state service is configured with a list of trusted operator public keys
   - The form-net service extracts the operator's public key from the configuration

2. **Authentication Flow**:
   - When form-net starts, it reads the public key from the operator configuration
   - It registers with form-state by sending its public key in the `X-Formation-Node-Key` header
   - form-state verifies this key against its list of trusted operator keys
   - Subsequent API calls include the same header for authentication

3. **Advantages**:
   - No need to create and manage separate API keys
   - Cryptographically secure - uses same keys used for operator identification
   - Integrates with existing configuration

This approach leverages the existing cryptographic identities in the Formation platform, ensuring that only legitimate operator nodes can participate in the network.

### Legacy API Key Authentication

For backward compatibility or special cases, API key authentication is still supported:

1. Create an API key (if you're not using operator public key authentication)
2. Provide this API key to services via the `API_KEY` environment variable
3. The service will include this API key in its requests to form-state

## Volume Mounts

### form-state
- `state-data:/var/lib/formation/db` - Persistent storage for the state database
- `./secrets:/etc/formation` - Configuration and secret files

### form-dns
- `dns-data:/var/lib/formation/dns` - DNS zone and configuration data
- `./secrets:/etc/formation` - Configuration files
- `/var/run/dbus:/var/run/dbus` - System D-Bus socket
- `/etc/resolv.conf:/etc/resolv.conf` - Host DNS resolver configuration
- `/etc/hosts:/etc/hosts` - Host name resolution file
- `${HOME}/.config/formation/certs:${HOME}/.config/formation/certs` - Certificate storage

### form-net
- `net-data:/var/lib/formnet` - Network configuration and data
- `./secrets:/etc/formation` - Configuration and secret files
- `${HOME}/.config/formation/certs:${HOME}/.config/formation/certs` - Certificate storage

## Running with docker-compose

When using docker-compose, you can create a `.env` file in the same directory as your `docker-compose.yml` file with the required environment variables:

```
# .env file example
# Required for form-state
SECRET_PATH=/etc/formation/.operator-config.json
PASSWORD=your-secure-password
DYNAMIC_JWKS_URL=https://your-jwks-provider.com/jwks
TRUSTED_OPERATOR_KEYS=0x1234abcd,0x5678efgh

# Optional form-net configuration
FORMNET_LOG_LEVEL=debug
FORMNET_NETWORK_NAME=formnet
FORMNET_EXTERNAL_ENDPOINT=auto
```

Then start the services with:

```bash
docker-compose up -d
``` 