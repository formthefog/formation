# Form DNS Service Setup

## Overview

The Form DNS service provides DNS resolution for the Formation platform. It integrates with form-state for service discovery and health tracking.

## Requirements

* form-state service running and accessible (default: http://localhost:3004)
* Host networking mode (to bind to ports and interact with system DNS)
* Privileged mode (to modify system DNS settings)
* Access to /var/run/dbus and /etc/resolv.conf

## Running with Docker

### Method 1: Using the run script

```bash
# First make sure form-state is running
./scripts/docker/run-form-state.sh

# Then run form-dns
./scripts/docker/run-form-dns.sh
```

### Method 2: Using Docker Compose

```bash
# Start both form-state and form-dns
docker-compose up form-state form-dns
```

## Environment Variables

The form-dns service understands the following environment variables:

* `DNS_LOG_LEVEL` - Log verbosity level (default: info)
* `DNS_PORT` - Port to listen on for DNS requests (default: 53)
* `STATE_URL` - URL of the form-state service (default: http://localhost:3004)
* `WAIT_FOR_STATE` - Whether to wait for form-state to be available (default: true)

## Verifying the Service

After starting the service, you can verify it's running correctly with:

```bash
# Test DNS resolution
dig @localhost formation

# Or query a specific domain
dig @localhost example.com
```

## Troubleshooting

If the service fails to start:

1. Ensure form-state is running and accessible
2. Check that port 53 is not already in use by another DNS server
3. Verify the container has privileged access
4. Check logs with `docker logs formation-dns`
5. Ensure /var/run/dbus and /etc/resolv.conf are accessible

Most issues are related to networking or system permissions, as form-dns needs to modify system DNS settings. 