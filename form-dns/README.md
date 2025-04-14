# form-dns Service

This directory contains the Formation DNS service, which provides DNS resolution and service discovery for the Formation platform.

## Overview

The form-dns service is a specialized DNS server that:

1. Provides name resolution for Formation services
2. Handles service discovery within the Formation network
3. Resolves platform-specific domains
4. Forwards external DNS queries to upstream servers

## Building the Service

### Prerequisites

- Rust toolchain (1.58 or newer)
- Docker (if building containerized version)
- Formation base image (for containerized version)

### Build Steps

#### Local Build

```bash
# Build the service
cargo build --release --bin form-dns

# Run tests
cargo test --package form-dns
```

#### Docker Build

```bash
# From the project root
docker build -t formation/form-dns:latest -f form-dns/Dockerfile .

# Or using the Makefile
cd docker
make form-dns
```

## Configuration

The service can be configured using:

1. Configuration file (default: `/etc/formation/dns/default.conf`)
2. Environment variables
3. Command line arguments

### Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `DNS_CONFIG_PATH` | Path to configuration file | `/etc/formation/dns/default.conf` |
| `DNS_LOG_LEVEL` | Logging level (debug, info, warn, error) | `info` |
| `DNS_LISTEN_PORT` | Port to listen on | `53` |
| `DNS_CACHE_SIZE` | Size of DNS cache | `1000` |
| `DNS_UPSTREAM_SERVERS` | Comma-separated list of upstream DNS servers | `8.8.8.8,1.1.1.1` |
| `WAIT_FOR` | Comma-separated list of services to wait for (host:port format) | `` |

### Configuration File Format

See `config/default.conf` for a fully documented example configuration file.

## Running the Service

### Directly

```bash
form-dns --config /path/to/config.conf
```

### Using Docker

```bash
docker run -d \
  --name form-dns \
  -p 53:53/udp \
  -p 53:53/tcp \
  -v /path/to/config:/etc/formation/dns \
  -v /path/to/zones:/var/lib/formation/dns/zones \
  formation/form-dns:latest
```

### Dependencies

This service has the following dependencies:

- None (can start independently)

## Testing

### Unit Tests

```bash
cargo test --package form-dns
```

### Integration Testing

```bash
# Test basic DNS resolution
dig @localhost -p 53 example.com

# Test service discovery (if configured)
dig @localhost -p 53 service-name.service.formation.local
```

## Directories

- `/var/lib/formation/dns/zones` - Zone files for authoritative DNS
- `/etc/formation/dns` - Configuration directory

## Troubleshooting

Common issues:

1. **Port 53 already in use**: Ensure no other DNS server is running or change the listening port
2. **Cannot resolve internal services**: Check zone files and service discovery configuration
3. **Configuration file not found**: Verify the path or use environment variables 