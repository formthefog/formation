# MCP Server Implementation Plan for Formation Network

## Overall Status: Phase 1 (100% Complete)

Current status:
- ✅ Core infrastructure is implemented and functional
- ✅ VM management tools are fully implemented (status, control, create, list, delete)
- ✅ Operations repository for long-running tasks is complete
- ✅ API endpoint structure is in place
- ✅ Automated cleanup for expired operations is working
- ✅ Authentication and authorization system is implemented with JWT tokens
  - ✅ Keypair management and signature verification working
  - ✅ Token-based authentication with proper validation 
  - ✅ Authorization middleware protecting endpoints
- ✅ Pack build and ship functionality implemented
  - ✅ Support for Formfile-based workload definitions
  - ✅ Integration with queue system for build and deployment operations
  - ✅ Documentation for pack tools
- ✅ API Documentation and Client SDK
  - ✅ Comprehensive OpenAPI specification
  - ✅ Python client library with robust error handling
  - ✅ Documentation and example code

Phase 1 of the MCP server implementation is now complete. Future phases will include:
- Network configuration tools
- Metrics and monitoring functionality
- Event system for workload state changes
- Resource optimization recommendations
- Agent policy framework
- Advanced logging and monitoring

## Table of Contents

1. [Overview](#1-overview)
2. [Architecture](#2-architecture)
   - 2.1 [Server Architecture](#21-server-architecture)
   - 2.2 [API Design](#22-api-design)
   - 2.3 [Authentication](#23-authentication)
   - 2.4 [Workload Management](#24-workload-management)
   - 2.5 [API Documentation and Client SDK](#25-api-documentation-and-client-sdk)
3. [Implementation Details](#3-implementation-details)
   - 3.1 [Core Framework](#31-core-framework)
   - 3.2 [Authentication Implementation](#32-authentication-implementation)
   - 3.3 [Tool Registry Implementation](#33-tool-registry-implementation)
   - 3.4 [Operations Repository Implementation](#34-operations-repository-implementation)
   - 3.5 [Pack Management Implementation](#35-pack-management-implementation)
   - 3.6 [API Documentation and Client SDK Implementation](#36-api-documentation-and-client-sdk-implementation)

## 1. Overview

The Model Context Protocol (MCP) server is a critical component of the Formation Network ecosystem, designed to enable AI agents and developers to manage workloads through a standardized API. This implementation plan documents the completed Phase 1 work and outlines future development phases.

### 1.1 Purpose and Goals

The primary purpose of the MCP server is to provide a structured interface for workload management operations through a tool-based API following the Model Context Protocol standard. Key goals include:

- Enabling AI agents to autonomously manage workloads on the Formation Network
- Providing developers with programmatic access to Formation infrastructure
- Ensuring secure, authenticated access to resources
- Offering a standardized API for client library development
- Supporting long-running operations with status tracking

### 1.2 Implementation Approach

The implementation follows a phased approach:

- **Phase 1 (Completed):** Core infrastructure, VM management tools, Pack Build/Ship tools, authentication, and API documentation
- **Phase 2 (Planned):** Network configuration, metrics and monitoring, event system, resource optimization
- **Phase 3 (Future):** Agent policy framework, advanced logging, ecosystem integration

### 1.3 Component Overview

The MCP server consists of several key components:

- **API Server:** Rust-based Actix Web server exposing RESTful endpoints
- **Tool Registry:** System for registering and discovering available tools
- **Authentication System:** JWT-based authentication with Ed25519 signature verification
- **Operations Repository:** Storage and management of long-running operations
- **Queue Integration:** RabbitMQ-based message processing for async operations
- **API Documentation:** OpenAPI specification for all endpoints
- **Client Libraries:** Starting with a Python reference implementation

## 2. Architecture

### 2.1 Server Architecture

The MCP server is built using a modular architecture with the following key components:

- **Web Server Layer:** Actix Web framework providing HTTP endpoints
- **Authentication Middleware:** JWT token validation and request signature verification
- **Tool Registry:** Central registry of available tools with metadata
- **Tool Execution Engine:** Processing tool requests and managing execution
- **Operations Store:** Repository for tracking long-running operations
- **Queue Integration:** RabbitMQ client for publishing messages to worker processes
- **State Store Integration:** Connection to Formation state database
- **API Documentation:** Built-in OpenAPI documentation generation

The server follows a request-response pattern for simple operations and an operation-based pattern for long-running tasks, where clients can poll for operation status updates.

### 2.2 API Design

The API is designed around the Model Context Protocol principles, featuring:

- **RESTful Endpoints:** Following REST principles for resource management
- **Tool-based Interface:** All functionality exposed as discrete tools
- **JSON Request/Response:** Structured data exchange using JSON
- **Comprehensive Error Handling:** Detailed error responses with status codes
- **Long-running Operations:** Asynchronous processing with operation tracking
- **Secure Authentication:** JWT-based token system with signature verification

Key API endpoints include:
- `/api/auth/*` - Authentication endpoints
- `/api/tools` - Tool discovery and metadata
- `/api/tools/{tool_id}` - Tool execution
- `/api/operations/{operation_id}` - Operation status and management

### 2.3 Authentication

The authentication system provides secure access to MCP resources through:

- **JWT Tokens:** JSON Web Tokens for authenticated sessions
- **Ed25519 Signatures:** Cryptographic request validation using Ed25519 keys
- **Token Expiration:** Automatic expiration and refresh mechanisms
- **Permission Model:** Role-based access control for different operations
- **Chain of Trust:** Verification through Formation account ownership

The authentication flow consists of:
1. Client generates a keypair (Ed25519)
2. Client sends the public key to the server with a signature
3. Server verifies the signature and issues a JWT token
4. Client includes the JWT token in subsequent requests
5. Server validates the token and permissions for each request

### 2.4 Workload Management

The MCP server provides tools for managing workloads through the Formation pack system. This includes:

#### 2.4.1 Pack Build Tool

The Pack Build Tool allows AI agents and users to build workloads from Formfile specifications. It supports:

- Defining workloads using Formfile format (JSON/YAML)
- Including context files for the build
- Specifying resource requirements
- Configuring network settings
- Setting environment variables and exposed ports

#### 2.4.2 Pack Ship Tool

The Pack Ship Tool enables deploying built workloads to Formation instances. It supports:

- Deploying workloads by build ID
- Configuring VM resources for the deployment
- Specifying network settings
- Tracking deployment status

### 2.5 API Documentation and Client SDK

The MCP server provides comprehensive API documentation and client libraries to facilitate integration:

#### 2.5.1 OpenAPI Specification

An OpenAPI 3.0 specification documents all API endpoints, request/response formats, and authentication requirements. This enables:

- Interactive exploration of the API
- Automatic client generation in various languages
- Standardized documentation format for developers

#### 2.5.2 Python Client Library

A Python client library simplifies interaction with the MCP server, providing:

- Structured interface to all MCP functionality
- Strong typing with dataclasses for request/response objects
- Comprehensive error handling
- Authentication management
- Long-running operation support
- Convenience methods for common operations

## 3. Implementation Details

### 3.1 Core Framework

The core MCP server framework is implemented with the following components:

- **Server and Routing:** Using Actix Web for HTTP request handling and routing
- **Configuration:** Environment-based configuration with sensible defaults
- **Logging:** Structured logging with filtering and formatting
- **Error Handling:** Centralized error handling with detailed error responses
- **Metrics:** Basic telemetry for server performance monitoring
- **Health Checks:** Endpoints for monitoring server health

### 3.2 Authentication Implementation

The authentication system is implemented with:

- **JWT Library:** Using a robust JWT implementation for token management
- **Ed25519 Verification:** Cryptographic signature verification
- **Middleware Integration:** Actix middleware for request authentication
- **Token Storage:** Secure storage of issued tokens
- **User Management:** Integration with Formation account system

### 3.3 Tool Registry Implementation

The tool registry design includes:

- **Tool Registration:** API for registering tools with the server
- **Tool Discovery:** Endpoints for discovering available tools
- **Tool Metadata:** Detailed information about tool parameters and outputs
- **Tool Versioning:** Support for tool versioning and compatibility
- **Tool Categories:** Grouping of tools by functionality

### 3.4 Operations Repository Implementation

The operations repository manages long-running tasks:

- **Operation Storage:** Persistent storage of operation state
- **Status Updates:** Mechanism for updating operation status
- **Result Storage:** Storage of operation results
- **Expiration:** Automatic cleanup of expired operations
- **Querying:** Efficient querying of operations by various criteria

### 3.5 Pack Management Implementation

#### 3.5.1 Formfile Structure

The Formfile structure defines how workloads are built and configured:

```json
{
  "from": "base-image",
  "name": "workload-name",
  "run": ["command1", "command2"],
  "include": ["file1", "file2"],
  "env": {"KEY": "VALUE"},
  "expose": [port1, port2],
  "entrypoint": "command",
  "resources": {
    "vcpus": count,
    "memory_mb": size,
    "disk_gb": size
  },
  "network": {
    "join_formnet": boolean
  },
  "metadata": {"key": "value"}
}
```

#### 3.5.2 Build Process

The build process follows these steps:

1. Client submits a Formfile and context files to the Pack Build Tool
2. The tool validates the Formfile format and content
3. The request is sent to the queue system with the topic "pack.build"
4. The pack manager processes the build request
5. A build ID is returned to the client for tracking

#### 3.5.3 Deployment Process

The deployment process follows these steps:

1. Client submits a build ID and configuration to the Pack Ship Tool
2. The tool validates the request parameters
3. The request is sent to the queue system with the topic "pack.ship"
4. The pack manager deploys the workload to a Formation instance
5. A deployment ID is returned to the client for tracking

### 3.6 API Documentation and Client SDK Implementation

#### 3.6.1 OpenAPI Specification

The OpenAPI specification is implemented as a YAML file that includes:

- Authentication endpoints
- Tool discovery and execution endpoints
- Operation status endpoints
- Detailed schema definitions for all request and response objects
- Security schemes for authentication
- Example requests and responses
- Comprehensive descriptions of all API components

The specification follows OpenAPI 3.0.3 standards and can be used with tools like Swagger UI, Redoc, and Stoplight Studio for interactive exploration.

#### 3.6.2 Python Client Library

The Python client library provides a structured interface to the MCP server with:

- `FormationMCPClient` class for managing server communication
- Dataclasses for structured request and response handling
- Exception hierarchy for detailed error handling
- Authentication and token management
- Automatic polling for long-running operations
- Helper methods for common workload management tasks
- Type annotations for better IDE integration
- Comprehensive documentation and examples

The library is designed to be both easy to use for simple tasks and flexible enough for advanced use cases, making it accessible to both developers and AI agents. 