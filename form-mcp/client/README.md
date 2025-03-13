# Formation MCP Client

A Python client library for interacting with the Formation Model Context Protocol (MCP) server. This library provides a structured way to use the MCP server's tools, with a focus on workload packaging and deployment functionality.

## Features

- **Authentication**: Secure authentication with the MCP server
- **Tool Discovery**: List available tools and their parameters
- **Tool Execution**: Execute tools with proper parameter validation
- **Long-running Operations**: Track and wait for operation completion
- **Error Handling**: Structured exceptions and error handling
- **Workload Management**: Build and deploy workloads using Formfiles
- **Type Hinting**: Full type annotations for better IDE integration

## Installation

Currently, the client library is distributed with the Formation MCP server. You can use it directly from the repository:

```bash
# Navigate to the client directory
cd form-mcp/client

# Install dependencies
pip install -r requirements.txt
```

## Usage

### Basic Usage

```python
from form_mcp_client import FormationMCPClient

# Initialize the client
client = FormationMCPClient(base_url="http://localhost:3010")

# Authenticate
token = client.login(
    address="0x1234567890abcdef1234567890abcdef12345678",
    signed_message="I want to authenticate with the MCP server",
    signature="0xabcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890"
)

# List available tools
tools = client.list_tools()
for tool in tools:
    print(f"Tool: {tool['name']} - {tool['description']}")

# Execute a simple tool
result = client.execute_tool("vm_list", {"all_users": False})
print(f"VM list: {result}")
```

### Building and Deploying Workloads

```python
from form_mcp_client import FormationMCPClient, Formfile, VMConfig

# Initialize the client and authenticate
client = FormationMCPClient(base_url="http://localhost:3010")
client.login(address, signed_message, signature)

# Create a Formfile definition
formfile = Formfile(
    from_image="ubuntu:22.04",
    name="my-app",
    run=["apt-get update", "apt-get install -y python3"],
    include=["app.py"],
    env={"PORT": "8080"},
    expose=[8080],
    entrypoint="python3 app.py",
    resources={
        "vcpus": 1,
        "memory_mb": 512,
        "disk_gb": 5
    },
    network={"join_formnet": True}
)

# Context files
context_files = {
    "app.py": """
import http.server
import socketserver
import os

PORT = int(os.environ.get('PORT', 8080))

with socketserver.TCPServer(("", PORT), http.server.SimpleHTTPRequestHandler) as httpd:
    print(f"Serving at port {PORT}")
    httpd.serve_forever()
"""
}

# Build the workload
build_result = client.build_workload(
    formfile=formfile,
    context_files=context_files,
    wait_for_completion=True
)

# Define VM configuration for deployment
vm_config = VMConfig(
    vcpus=1,
    memory_mb=1024,
    network={"join_formnet": True}
)

# Deploy the workload
deploy_result = client.deploy_workload(
    build_id=build_result.build_id,
    instance_name="my-app-instance",
    vm_config=vm_config,
    wait_for_completion=True
)

# Print deployment information
print(f"Build ID: {build_result.build_id}")
print(f"Deploy ID: {deploy_result.deploy_id}")
print(f"Instance ID: {deploy_result.instance_id}")
```

### Error Handling

```python
from form_mcp_client import (
    FormationMCPClient,
    AuthenticationError,
    ToolExecutionError,
    OperationError,
    ApiError
)

client = FormationMCPClient()

try:
    # Attempt to authenticate
    client.login(address, signed_message, signature)
    
    # Attempt to execute a tool
    result = client.execute_tool("form_pack_build", {...})
    
except AuthenticationError as e:
    print(f"Authentication failed: {str(e)}")
except ToolExecutionError as e:
    print(f"Tool execution failed: {str(e)}")
except OperationError as e:
    print(f"Operation failed: {str(e)}")
except ApiError as e:
    print(f"API error (status {e.status_code}): {str(e)}")
    if hasattr(e, 'response_body') and e.response_body:
        print(f"Response: {e.response_body}")
except Exception as e:
    print(f"Unexpected error: {str(e)}")
```

## Example Script

The repository includes an example script (`example.py`) that demonstrates the full workflow of building and deploying a workload using the client library:

```bash
# Run the example script
python example.py \
  --address 0x1234567890abcdef1234567890abcdef12345678 \
  --signed-message "I want to authenticate with the MCP server" \
  --signature 0xabcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890
```

## API Reference

### Main Client Class

#### `FormationMCPClient(base_url="http://localhost:3010", token=None)`

Initialize the MCP client.

### Authentication Methods

#### `login(address, signed_message, signature)`

Authenticate with the MCP server and return the token.

#### `validate_token()`

Validate the current token with the server.

### Tool Methods

#### `list_tools(category=None)`

List available tools, optionally filtered by category.

#### `execute_tool(tool_name, parameters, context=None)`

Execute a tool with the given parameters and context.

### Operation Methods

#### `get_operation_status(operation_id)`

Get the status of a long-running operation.

#### `wait_for_operation(operation_id, check_interval=5, timeout=None)`

Wait for an operation to complete and return the result.

### Workload Methods

#### `build_workload(formfile, context_files=None, build_options=None, wait_for_completion=True, timeout=None)`

Build a workload using the form_pack_build tool.

#### `deploy_workload(build_id, instance_name, vm_config=None, target_node=None, deployment_options=None, wait_for_completion=True, timeout=None)`

Deploy a workload using the form_pack_ship tool.

#### `get_build_status(build_id)`

Get the status of a build.

#### `get_deployment_status(deployment_id)`

Get the status of a deployment.

## Data Classes

### `Formfile`

Representation of a Formfile for workload building.

### `VMConfig`

VM configuration for workload deployment.

### `BuildResult`

Result of a build operation.

### `DeploymentResult`

Result of a deployment operation.

## Exceptions

### `MCPClientError`

Base exception for MCP client errors.

### `AuthenticationError`

Exception raised for authentication failures.

### `ToolExecutionError`

Exception raised when a tool execution fails.

### `OperationError`

Exception raised when an operation fails or times out.

### `ToolNotFoundError`

Exception raised when a requested tool is not found.

### `ParameterError`

Exception raised when invalid parameters are provided.

### `ApiError`

Exception raised for API-related errors.

## License

This project is licensed under the same license as the Formation Network. 