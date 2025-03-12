# Pack Management Tools

This document describes the pack management tools available in the Formation MCP server. These tools allow AI agents and users to build and deploy workloads based on Formfile specifications.

## Overview

The Formation MCP server provides two primary tools for workload packaging and deployment:

1. **Pack Build Tool** (`form_pack_build`): Builds a workload from a Formfile specification
2. **Pack Ship Tool** (`form_pack_ship`): Deploys a built workload package to a Formation instance

These tools integrate with the Formation pack manager to provide a seamless experience for building and deploying containerized workloads.

## Formfile Specification

A Formfile is a JSON or YAML document that describes how to build and configure a workload. It includes:

- Base image information
- Commands to run during build
- Files to include
- Environment variables
- Exposed ports
- Resource requirements
- Network configuration
- Additional metadata

Example Formfile (JSON format):

```json
{
  "from": "ubuntu:22.04",
  "name": "my-workload",
  "run": [
    "apt-get update",
    "apt-get install -y python3 python3-pip",
    "pip install flask"
  ],
  "include": [
    "app.py",
    "requirements.txt"
  ],
  "env": {
    "PORT": "8080",
    "DEBUG": "false"
  },
  "expose": [8080],
  "entrypoint": "python3 app.py",
  "resources": {
    "vcpus": 2,
    "memory_mb": 1024,
    "disk_gb": 10
  },
  "network": {
    "join_formnet": true
  },
  "metadata": {
    "description": "My Flask application",
    "version": "1.0.0"
  }
}
```

## Pack Build Tool

The Pack Build Tool (`form_pack_build`) allows you to build a workload from a Formfile specification.

### Parameters

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `formfile_content` | string | Yes | Content of the Formfile in JSON or YAML format |
| `context_files` | object | No | Map of filename to file content for files to include in the build context |

### Example Usage

```json
{
  "formfile_content": "{\"from\":\"ubuntu:22.04\",\"name\":\"my-workload\",\"run\":[\"apt-get update\",\"apt-get install -y python3\"],\"entrypoint\":\"python3 app.py\"}",
  "context_files": {
    "app.py": "print('Hello, Formation!')"
  }
}
```

### Response

The tool returns a build ID that can be used to track the build process and later deploy the workload:

```json
{
  "status": "success",
  "build_id": "550e8400-e29b-41d4-a716-446655440000",
  "message": "Build request accepted successfully"
}
```

## Pack Ship Tool

The Pack Ship Tool (`form_pack_ship`) allows you to deploy a built workload package to a Formation instance.

### Parameters

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `build_id` | string | Yes | ID of the built package to deploy |
| `instance_name` | string | Yes | Name for the instance running the workload |
| `vm_config` | object | No | Virtual machine configuration for the deployment |

The `vm_config` object can include:

- `vcpus`: Number of virtual CPUs
- `memory_mb`: Memory size in megabytes
- `network`: Network configuration
  - `join_formnet`: Whether to join the formnet network
  - `external_networks`: External networks to connect to
- `metadata`: Additional metadata

### Example Usage

```json
{
  "build_id": "550e8400-e29b-41d4-a716-446655440000",
  "instance_name": "my-workload-instance",
  "vm_config": {
    "vcpus": 2,
    "memory_mb": 2048,
    "network": {
      "join_formnet": true
    }
  }
}
```

### Response

The tool returns a deployment ID and status information:

```json
{
  "status": "success",
  "deploy_id": "7b8e8100-f1a2-43d4-b567-123456789abc",
  "message": "Deployment request queued successfully",
  "details": {
    "deploy_id": "7b8e8100-f1a2-43d4-b567-123456789abc",
    "instance_id": null,
    "status": "queued",
    "message": "Deployment request queued successfully"
  }
}
```

## Integration with Other Tools

The pack tools integrate with the VM management tools:

1. After deploying a workload with `form_pack_ship`, you can use the VM tools to:
   - Check the status of the instance with `vm_status`
   - Control the instance with `vm_control` (start, stop, restart)
   - Delete the instance with `vm_delete`

## Error Handling

Both tools return appropriate error messages when operations fail:

```json
{
  "status": "error",
  "message": "Failed to parse Formfile: invalid syntax at line 3"
}
```

## Security Considerations

- The pack build and ship tools require authentication
- Users can only deploy workloads they have built
- Resource limits can be enforced through the VM configuration

## Future Enhancements

Planned enhancements for the pack tools include:

1. Support for build progress tracking
2. Ability to list available builds
3. Support for build caching
4. Integration with CI/CD pipelines
5. Support for multi-stage builds 