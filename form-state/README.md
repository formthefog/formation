# form-state Service

This directory contains the Formation State service, which manages configuration, state, and provides API services for the Formation platform, including authentication, marketplace, and billing features.

## Overview

The form-state service provides:

1. Centralized state management for the Formation platform
2. Authentication and authorization for users and services
3. API endpoints for configuration management
4. Service discovery and registration
5. Marketplace functionality for AI agents and models
6. Billing and usage tracking

## Building the Service

### Prerequisites

- Rust toolchain (1.58 or newer)
- Docker (if building containerized version)
- Formation base image (for containerized version)
- SQLite development libraries (`libsqlite3-dev`)

### Build Steps

#### Local Build

```bash
# Build the service
cargo build --release --bin form-state

# Run tests
cargo test --package form-state
```

#### Docker Build

```bash
# From the project root
docker build -t formation/form-state:latest -f form-state/Dockerfile .

# Or using the Makefile
cd docker
make form-state
```

## Configuration

The service can be configured using:

1. Configuration file (default: `/etc/formation/state/default.conf`)
2. Environment variables
3. Command line arguments

### Key Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `STATE_CONFIG_PATH` | Path to configuration file | `/etc/formation/state/default.conf` |
| `STATE_LOG_LEVEL` | Logging level (debug, info, warn, error) | `info` |
| `STATE_API_PORT` | API port to listen on | `3004` |
| `STATE_DB_PATH` | SQLite database path | `/var/lib/formation/db/state.db` |
| `AUTH_MODE` | Authentication mode (development, production) | `production` |
| `MARKETPLACE_ENABLED` | Enable marketplace functionality | `true` |
| `BILLING_ENABLED` | Enable billing functionality | `true` |
| `API_KEYS_ENABLED` | Enable API key authentication | `true` |
| `WAIT_FOR` | Comma-separated list of services to wait for (host:port format) | `` |

### Configuration File

See `config/default.conf` for a fully documented example configuration file.

### Authentication Configuration

The service supports JWT-based authentication. A sample environment configuration file is provided in `.env.example`. For production use, customize the following variables:

```
AUTH_JWT_SECRET=change_this_to_a_secure_random_value
AUTH_ADMIN_USERNAME=admin
AUTH_ADMIN_PASSWORD=change_this_to_a_secure_password
```

## Running the Service

### Directly

```bash
form-state --config /path/to/config.conf
```

### Using Docker

```bash
docker run -d \
  --name form-state \
  -p 3004:3004 \
  -v /path/to/config:/etc/formation/state \
  -v /path/to/db:/var/lib/formation/db \
  -v /path/to/marketplace:/var/lib/formation/marketplace \
  formation/form-state:latest
```

### Dependencies

This service has the following optional dependencies:

- `form-dns` - For service discovery
- Other Formation services - For health monitoring and management

## API Documentation

The service provides a RESTful API documented using OpenAPI 3.0. The specification is available at:

- `/var/lib/formation/marketplace/openapi.yaml` 
- `http://localhost:3004/api/docs` (when running)

Key endpoints include:

- `/health` - Service health check
- `/auth/login` - Authentication
- `/services` - Service management
- `/marketplace/agents` - AI agent marketplace

## Database

The service uses SQLite for data storage. The database is automatically initialized when the service starts for the first time. Key database tables include:

- `users` - User accounts
- `services` - Registered services
- `agents` - Marketplace agents
- `transactions` - Billing transactions

## Testing

### Unit Tests

```bash
cargo test --package form-state
```

### API Testing

```bash
# Health check
curl http://localhost:3004/health

# Login (get access token)
curl -X POST http://localhost:3004/auth/login \
  -H "Content-Type: application/json" \
  -d '{"username":"admin","password":"password"}'
```

## Directories

- `/var/lib/formation/db` - Database storage
- `/var/lib/formation/marketplace` - Marketplace storage
- `/etc/formation/state` - Configuration
- `/etc/formation/auth` - Authentication configuration
- `/etc/formation/billing` - Billing configuration

## Troubleshooting

Common issues:

1. **Database initialization fails**: Check permissions on the database directory
2. **Authentication errors**: Verify JWT secret is configured correctly
3. **API not responding**: Check port availability and firewall settings
4. **Marketplace not available**: Ensure the MARKETPLACE_ENABLED flag is set to true 