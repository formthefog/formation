# Formation Protocol API Reference

This section provides comprehensive documentation for the Formation Protocol's API endpoints. The Formation protocol consists of several components, each with its own API for different aspects of the system.

## API Components

The Formation Protocol exposes APIs through the following components:

### VMM Service API

The [VMM Service API](./vmm/index.md) provides endpoints for managing virtual machines (VMs) on the Formation cloud. This includes creating, starting, stopping, and deleting VMs, as well as querying VM status and listing active VMs.

### State Service API

The [State Service API](./state/index.md) manages the global state of the Formation cloud, including users, CIDRs (Classless Inter-Domain Routing), associations, DNS records, instances, nodes, and accounts. This API is responsible for maintaining consistency across the distributed system.

### P2P Service API

The [P2P Service API](./p2p/index.md) handles peer-to-peer communication between Formation nodes. It provides endpoints for message queuing, topic subscription, and node discovery.

### DNS Service API

The [DNS Service API](./dns/index.md) manages domain name resolution within the Formation cloud, allowing instances to be addressed by domain names rather than just IP addresses.

### Formnet API

The [Formnet API](./formnet/index.md) manages the network layer of the Formation protocol, handling WireGuard-based encrypted connections between nodes and instances.

## OpenAPI Specifications

For developers who prefer standard API specifications, we provide [OpenAPI specifications](./openapi/index.md) for all our services. These specifications can be used with tools like Swagger UI, Postman, or OpenAPI Generator to facilitate integration with your applications.

## Authentication

Most API endpoints require authentication using one of the following methods:

1. **Ethereum Wallet Signatures**: For user-facing APIs
2. **Node Identity Verification**: For node-to-node communication
3. **API Keys**: For programmatic access (when applicable)

See each specific API section for details on required authentication methods.

## API Versions

The Formation Protocol is under active development. All current APIs are considered v1 and may change as the protocol evolves. Future versions will maintain backward compatibility where possible.

## Making API Requests

All APIs accept and return JSON-formatted data unless otherwise specified. HTTP status codes are used to indicate success or failure of requests.

## Rate Limiting

API endpoints may implement rate limiting to prevent abuse. Rate limits, when applicable, are documented in the respective API sections.

## API Guidelines for Users

Please note that most POST, PUT, and DELETE endpoints are intended for internal use only. As a user of the Formation Protocol, you should primarily interact with the GET endpoints which provide read-only access to the system state. The primary exceptions to this rule are:

1. **P2P Service API**: Users can publish messages to the message queue
2. **Inference Engine API**: Users can make direct inference requests to the network (coming soon)

Attempting to use internal-only endpoints may result in authentication errors or service disruption.

## Next Steps

Choose an API component from the navigation to explore its endpoints, or return to the [Developer Documentation](../developer/getting-started/index.md) for guides on using these APIs. 