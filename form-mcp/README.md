# Formation MCP Server (form-mcp)

A Model Context Protocol (MCP) server implementation for the Formation network, enabling AI agents to manage workload lifecycles securely and efficiently.

## Overview

The MCP server provides a standardized interface for AI agents to interact with and manage Formation network resources. By implementing the MCP specification, it ensures compatibility with various AI clients while providing powerful management capabilities specific to the Formation network.

## Features

- **Tool Registry System**: Register, discover, and execute tools for managing workloads
- **Long-running Operations**: Track and manage the status of asynchronous operations
- **Authentication & Authorization**: Secure access to Formation network management functions
- **VM Lifecycle Management**: Create, control, and monitor VMs
- **Network Configuration**: Manage network settings and connections
- **Metrics & Monitoring**: Collect and analyze resource usage data
- **Policy Enforcement**: Apply safety and security policies to management actions

## Current Status

This project is under active development. Currently implemented:

- [x] Basic project structure and module layout
- [x] Core tool registry data structures
- [x] Registry management functionality
- [x] VM management tools
- [x] Operations tracking system for long-running tasks
- [x] API endpoints for tool execution and operation status
- [ ] Authentication system
- [ ] Metrics and monitoring tools
- [ ] Network management tools

## Architecture

The MCP server follows a modular architecture to separate concerns and enable extensibility:

```
form-mcp/
├── api/              # API endpoints and handlers
├── auth/             # Authentication and authorization
├── tools/            # Tool implementations
│   ├── vm/           # VM management tools
│   ├── network/      # Network management tools
│   └── metrics/      # Metrics and monitoring tools
├── events/           # Event system for notifications
├── models/           # Data models and schemas
│   └── operations/   # Long-running operations tracking
├── config/           # Configuration management
├── billing/          # Billing and payment integration
└── errors/           # Error handling
```

## API Endpoints

### Tools

- `GET /api/tools` - List available tools
- `POST /api/tools/{name}` - Execute a tool

### Operations

- `GET /api/operations/{id}` - Get status of a long-running operation
- `GET /api/operations` - List operations (optionally filtered by user)

## Getting Started

### Prerequisites

- Rust (latest stable version)
- Cargo
- Formation network components

### Building

```bash
cd form-mcp
cargo build
```

### Running

```bash
cargo run
```

### Testing

```bash
cargo test
```

## Contributing

1. Choose a task from the implementation plan
2. Create a branch for your feature
3. Write tests for your feature
4. Implement your feature
5. Submit a pull request

## License

This project is licensed under the [insert license name]. 