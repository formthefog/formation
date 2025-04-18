# Formation Platform Environment Variables

This document outlines the environment variables required for each Formation service.

## Core Services

### form-state

The state service manages the system's state database and requires the following environment variables:

| Variable | Description | Default | Required |
|----------|-------------|---------|----------|
| `DB_PATH` | Path to the state database | `/var/lib/formation/db/formation.db` | Yes |
| `SECRET_PATH` | Path to the operator configuration JSON file | `/etc/formation/operator-config.json` | Yes |
| `PASSWORD` | Password used to decrypt the operator configuration | `formation-password` | Yes |
| `DEV_MODE` | Enable development mode | `true` | No |
| `AUTH_MODE` | Authentication mode | `development` | No |
| `DYNAMIC_JWKS_URL` | URL for JSON Web Key Set | `https://app.dynamic.xyz/api/v0/sdk/3f53e601-17c7-419b-8a13-4c5e25c0bde9/.well-known/jwks` | Yes |

Example `.env` file entries:
```
# Required for form-state
SECRET_PATH=/path/to/your/operator-config.json
PASSWORD=your-secure-password
DYNAMIC_JWKS_URL=https://your-jwks-provider.com/jwks
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

### formnet

The network service provides virtual network capabilities for the platform:

| Variable | Description | Default | Required |
|----------|-------------|---------|----------|
| `FORMNET_LOG_LEVEL` | Log level for formnet service | `debug` | No |
| `RUST_LOG` | Rust logging level | `debug` | No |
| `RUST_BACKTRACE` | Rust backtrace settings | `1` | No |
| `FORMNET_CONFIG_DIR` | Directory for formnet configuration | `/etc/formnet` | Yes |
| `FORMNET_DATA_DIR` | Directory for formnet data storage | `/var/lib/formnet` | Yes |
| `FORMNET_NETWORK_NAME` | Name of the virtual network | `formnet` | No |
| `FORMNET_SERVER_PORT` | Port for the formnet API server | `8080` | No |
| `FORMNET_LISTEN_PORT` | Port for WireGuard VPN | `51820` | No |
| `FORMNET_EXTERNAL_ENDPOINT` | External endpoint for WireGuard | `auto` | No |
| `STATE_URL` | URL for the state service | `http://localhost:3004` | Yes |

Example `.env` file entries:
```
# Optional formnet customization
FORMNET_NETWORK_NAME=myformnet
FORMNET_LOG_LEVEL=debug
FORMNET_EXTERNAL_ENDPOINT=auto
```

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

### formnet
- `net-data:/var/lib/formnet` - Network configuration and data
- `./secrets:/etc/formnet` - Configuration and secret files
- `${HOME}/.config/formation/certs:${HOME}/.config/formation/certs` - Certificate storage

## Running with docker-compose

When using docker-compose, you can create a `.env` file in the same directory as your `docker-compose.yml` file with the required environment variables:

```
# .env file example
# Required for form-state
SECRET_PATH=/etc/formation/operator-config.json
PASSWORD=your-secure-password
DYNAMIC_JWKS_URL=https://your-jwks-provider.com/jwks

# Optional formnet configuration
FORMNET_LOG_LEVEL=debug
FORMNET_NETWORK_NAME=formnet
FORMNET_EXTERNAL_ENDPOINT=auto
```

Then start the services with:

```bash
docker-compose up -d
``` 