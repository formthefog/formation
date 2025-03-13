"""
Formation MCP Client Library

A client library for interacting with the Formation Model Context Protocol (MCP) server.
"""

from .form_mcp_client import (
    FormationMCPClient,
    Formfile,
    VMConfig,
    BuildResult,
    DeploymentResult,
    MCPClientError,
    AuthenticationError,
    ToolExecutionError,
    OperationError,
    ToolNotFoundError,
    ParameterError,
    ApiError,
    OperationStatus,
    BuildStatus,
    DeploymentStatus,
)

__all__ = [
    'FormationMCPClient',
    'Formfile',
    'VMConfig',
    'BuildResult',
    'DeploymentResult',
    'MCPClientError',
    'AuthenticationError',
    'ToolExecutionError',
    'OperationError',
    'ToolNotFoundError',
    'ParameterError',
    'ApiError',
    'OperationStatus',
    'BuildStatus',
    'DeploymentStatus',
]

__version__ = "0.1.0" 