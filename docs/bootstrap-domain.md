# Bootstrap Domain Integration

This document explains how to use the new bootstrap domain feature in the Formation Network.

## Overview

The bootstrap domain feature allows nodes to join the Formation Network using a domain name instead of specific IP addresses. This provides several benefits:

- **Geographic Proximity**: Nodes automatically connect to the nearest healthy bootstrap node
- **Fault Tolerance**: If some bootstrap nodes are unhealthy, they are automatically filtered out
- **Simplified Configuration**: Users only need to remember a single domain name
- **Centralized Management**: Bootstrap nodes can be added or removed without changing client configuration

## How It Works

When a node attempts to join the Formation Network using a bootstrap domain:

1. The domain is resolved through DNS, using our GeoDNS system
2. The GeoDNS system returns the IP addresses of healthy bootstrap nodes, prioritizing those closest to the client
3. The node tries to connect to these bootstrap nodes in order
4. If all bootstrap nodes are unhealthy, all IPs are returned to avoid service disruption

## Using the Bootstrap Domain

### Command Line Interface

To join the Formation Network using the bootstrap domain:

```bash
form-net operator join --bootstrap-domain bootstrap.formation.cloud --signing-key <your-key>
```

You can also combine the bootstrap domain with specific bootstrap nodes:

```bash
form-net operator join --bootstrap-domain bootstrap.formation.cloud --bootstraps 198.51.100.2:51820 --signing-key <your-key>
```

To leave the network:

```bash
form-net operator leave --bootstrap-domain bootstrap.formation.cloud --signing-key <your-key>
```

### Configuration File

You can also specify the bootstrap domain in your operator configuration file:

```json
{
  "bootstrap_domain": "bootstrap.formation.cloud",
  "bootstrap_nodes": [
    "198.51.100.2:51820"
  ],
  "secret_key": "<your-key>"
}
```

## Fallback Mechanism

The system implements multiple fallback mechanisms to ensure reliability:

1. If domain resolution fails, the system falls back to directly specified bootstrap nodes
2. If all bootstrap nodes are unhealthy, all IPs are returned anyway to avoid complete service disruption
3. If no bootstrap nodes are reachable, the node can initialize as a new bootstrap node

## For Administrators

If you're administering the Formation Network:

### Setting Up the Bootstrap Domain

1. Configure your DNS to point `bootstrap.formation.cloud` to your bootstrap nodes
2. Ensure the form-dns service is running and configured with the bootstrap domain
3. Add bootstrap nodes to the form-dns configuration

### Health Monitoring

The system automatically monitors the health of bootstrap nodes:

- Nodes that fail to respond to health checks are marked as unhealthy
- Unhealthy nodes are filtered out of DNS responses
- If all nodes become unhealthy, all IPs are returned to avoid service disruption

### TTL Configuration

Consider adjusting TTL (Time To Live) values:

- Lower TTL values (30-60 seconds) allow faster failover but increase DNS traffic
- Higher TTL values (300-600 seconds) reduce DNS traffic but slow down failover
- You can dynamically adjust TTLs based on network stability

## Troubleshooting

If you're having issues with the bootstrap domain:

1. **Resolution Failures**: Check if the domain resolves correctly using `nslookup bootstrap.formation.cloud`
2. **Connection Issues**: Verify that the bootstrap nodes are reachable using `ping` or `telnet`
3. **Health Check Failures**: Check if your bootstrap nodes are marked as unhealthy in the form-dns logs
4. **DNS Caching**: Some DNS servers might cache results longer than the TTL; try flushing your DNS cache

## Next Steps

Future enhancements planned for the bootstrap domain feature:

- Dynamic TTL adjustment based on node health status
- Regional health degradation handling
- Integration with the broader health monitoring system
- Advanced metrics and observability

For more information, see the [Virtual Anycast Implementation Plan](virtual_anycast_implementation_plan.md). 