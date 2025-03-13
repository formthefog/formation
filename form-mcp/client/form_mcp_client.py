#!/usr/bin/env python3
"""
Formation MCP Client Library

A client library for interacting with the Formation Model Context Protocol (MCP) server.
This library provides a structured way to use the MCP server's tools, with a focus on
workload packaging and deployment functionality.
"""

import requests
import json
import time
import logging
from typing import Dict, List, Optional, Any, Union
from enum import Enum
from dataclasses import dataclass
import urllib.parse

# Configure logging
logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s - %(name)s - %(levelname)s - %(message)s'
)
logger = logging.getLogger('form_mcp_client')

class MCPClientError(Exception):
    """Base exception for MCP client errors."""
    pass

class AuthenticationError(MCPClientError):
    """Exception raised for authentication failures."""
    pass

class ToolExecutionError(MCPClientError):
    """Exception raised when a tool execution fails."""
    pass

class OperationError(MCPClientError):
    """Exception raised when an operation fails or times out."""
    pass

class ToolNotFoundError(MCPClientError):
    """Exception raised when a requested tool is not found."""
    pass

class ParameterError(MCPClientError):
    """Exception raised when invalid parameters are provided."""
    pass

class ApiError(MCPClientError):
    """Exception raised for API-related errors."""
    def __init__(self, message: str, status_code: int, response_body: str = None):
        self.status_code = status_code
        self.response_body = response_body
        super().__init__(f"{message} (Status code: {status_code})")

class OperationStatus(Enum):
    """Enum representing the possible states of a long-running operation."""
    QUEUED = "queued"
    RUNNING = "running"
    COMPLETED = "completed"
    FAILED = "failed"
    CANCELLED = "cancelled"

class BuildStatus(Enum):
    """Enum representing the possible states of a build operation."""
    CREATED = "created"
    BUILDING = "building"
    SUCCEEDED = "succeeded"
    FAILED = "failed"

class DeploymentStatus(Enum):
    """Enum representing the possible states of a deployment operation."""
    CREATED = "created"
    DEPLOYING = "deploying"
    RUNNING = "running"
    FAILED = "failed"
    STOPPED = "stopped"

@dataclass
class Formfile:
    """Representation of a Formfile for workload building."""
    from_image: str
    name: Optional[str] = None
    run: Optional[List[str]] = None
    include: Optional[List[str]] = None
    env: Optional[Dict[str, str]] = None
    expose: Optional[List[int]] = None
    entrypoint: Optional[str] = None
    resources: Optional[Dict[str, Any]] = None
    network: Optional[Dict[str, Any]] = None
    metadata: Optional[Dict[str, str]] = None
    
    def to_dict(self) -> Dict[str, Any]:
        """Convert the Formfile to a dictionary."""
        result = {"from": self.from_image}
        
        if self.name:
            result["name"] = self.name
        if self.run:
            result["run"] = self.run
        if self.include:
            result["include"] = self.include
        if self.env:
            result["env"] = self.env
        if self.expose:
            result["expose"] = self.expose
        if self.entrypoint:
            result["entrypoint"] = self.entrypoint
        if self.resources:
            result["resources"] = self.resources
        if self.network:
            result["network"] = self.network
        if self.metadata:
            result["metadata"] = self.metadata
            
        return result
    
    def to_json(self) -> str:
        """Convert the Formfile to a JSON string."""
        return json.dumps(self.to_dict())

@dataclass
class VMConfig:
    """VM configuration for workload deployment."""
    vcpus: Optional[int] = None
    memory_mb: Optional[int] = None
    network: Optional[Dict[str, Any]] = None
    metadata: Optional[Dict[str, Any]] = None
    
    def to_dict(self) -> Dict[str, Any]:
        """Convert the VM configuration to a dictionary."""
        result = {}
        
        if self.vcpus is not None:
            result["vcpus"] = self.vcpus
        if self.memory_mb is not None:
            result["memory_mb"] = self.memory_mb
        if self.network:
            result["network"] = self.network
        if self.metadata:
            result["metadata"] = self.metadata
            
        return result

@dataclass
class BuildResult:
    """Result of a build operation."""
    build_id: str
    status: str
    message: Optional[str] = None
    operation_id: Optional[str] = None
    details: Optional[Dict[str, Any]] = None

@dataclass
class DeploymentResult:
    """Result of a deployment operation."""
    deploy_id: str
    status: str
    instance_id: Optional[str] = None
    message: Optional[str] = None
    operation_id: Optional[str] = None
    details: Optional[Dict[str, Any]] = None

class FormationMCPClient:
    """
    Client for interacting with the Formation MCP server.
    
    This client provides methods for authenticating with the server and
    executing tools, with a focus on workload packaging and deployment.
    """
    
    def __init__(self, base_url: str = "http://localhost:3010", token: Optional[str] = None):
        """
        Initialize the MCP client.
        
        Args:
            base_url: Base URL of the MCP server
            token: Optional authentication token
        """
        self.base_url = base_url.rstrip('/')
        self.token = token
        self.session = requests.Session()
        
        if token:
            self.session.headers.update({"Authorization": f"Bearer {token}"})
    
    def login(self, address: str, signed_message: str, signature: str) -> str:
        """
        Authenticate with the MCP server.
        
        Args:
            address: User's address or ID
            signed_message: Message signed by the user
            signature: Signature of the signed message
            
        Returns:
            Authentication token
            
        Raises:
            AuthenticationError: If authentication fails
            ApiError: If the API request fails
        """
        login_url = f"{self.base_url}/api/auth/login"
        login_data = {
            "address": address,
            "signed_message": signed_message,
            "signature": signature
        }
        
        try:
            response = self.session.post(login_url, json=login_data)
        except requests.RequestException as e:
            raise ApiError(f"Failed to connect to MCP server: {str(e)}", 0)
        
        if response.status_code != 200:
            raise ApiError("Authentication request failed", response.status_code, response.text)
        
        try:
            result = response.json()
        except json.JSONDecodeError:
            raise ApiError("Invalid JSON response from server", response.status_code, response.text)
        
        if result.get("status") != "success":
            raise AuthenticationError(result.get("message", "Authentication failed"))
        
        self.token = result["data"]["token"]
        self.session.headers.update({"Authorization": f"Bearer {self.token}"})
        
        return self.token
    
    def validate_token(self) -> bool:
        """
        Validate the current token with the MCP server.
        
        Returns:
            True if the token is valid, False otherwise
            
        Raises:
            ApiError: If the API request fails
        """
        if not self.token:
            return False
        
        validate_url = f"{self.base_url}/api/auth/validate"
        validate_data = {"token": self.token}
        
        try:
            response = self.session.post(validate_url, json=validate_data)
        except requests.RequestException as e:
            raise ApiError(f"Failed to connect to MCP server: {str(e)}", 0)
        
        if response.status_code != 200:
            return False
        
        try:
            result = response.json()
        except json.JSONDecodeError:
            return False
        
        return result.get("status") == "success"
    
    def list_tools(self, category: Optional[str] = None) -> List[Dict[str, Any]]:
        """
        List available tools on the MCP server.
        
        Args:
            category: Optional category filter
            
        Returns:
            List of tool definitions
            
        Raises:
            ApiError: If the API request fails
        """
        tools_url = f"{self.base_url}/api/tools"
        if category:
            tools_url += f"?category={urllib.parse.quote(category)}"
        
        try:
            response = self.session.get(tools_url)
        except requests.RequestException as e:
            raise ApiError(f"Failed to connect to MCP server: {str(e)}", 0)
        
        if response.status_code != 200:
            raise ApiError("Failed to list tools", response.status_code, response.text)
        
        try:
            result = response.json()
        except json.JSONDecodeError:
            raise ApiError("Invalid JSON response from server", response.status_code, response.text)
        
        if result.get("status") != "success":
            raise ApiError(result.get("message", "Failed to list tools"), response.status_code, response.text)
        
        return result["data"]["tools"]
    
    def execute_tool(self, tool_name: str, parameters: Dict[str, Any], context: Optional[Dict[str, str]] = None) -> Dict[str, Any]:
        """
        Execute a tool on the MCP server.
        
        Args:
            tool_name: Name of the tool to execute
            parameters: Parameters for the tool
            context: Optional context data
            
        Returns:
            Tool execution result
            
        Raises:
            ToolExecutionError: If tool execution fails
            ToolNotFoundError: If the tool is not found
            ParameterError: If the parameters are invalid
            ApiError: If the API request fails
        """
        tool_url = f"{self.base_url}/api/tools/{tool_name}"
        tool_data = {"parameters": parameters}
        
        if context:
            tool_data["context"] = context
        
        try:
            response = self.session.post(tool_url, json=tool_data)
        except requests.RequestException as e:
            raise ApiError(f"Failed to connect to MCP server: {str(e)}", 0)
        
        if response.status_code == 404:
            raise ToolNotFoundError(f"Tool '{tool_name}' not found")
        
        if response.status_code == 400:
            raise ParameterError(f"Invalid parameters for tool '{tool_name}'")
        
        if response.status_code != 200:
            raise ApiError(f"Tool execution request failed", response.status_code, response.text)
        
        try:
            result = response.json()
        except json.JSONDecodeError:
            raise ApiError("Invalid JSON response from server", response.status_code, response.text)
        
        if result.get("status") != "success":
            raise ToolExecutionError(result.get("message", f"Tool '{tool_name}' execution failed"))
        
        return result["data"]
    
    def get_operation_status(self, operation_id: str) -> Dict[str, Any]:
        """
        Get the status of a long-running operation.
        
        Args:
            operation_id: ID of the operation
            
        Returns:
            Operation status
            
        Raises:
            ApiError: If the API request fails
        """
        operation_url = f"{self.base_url}/api/operations/{operation_id}"
        
        try:
            response = self.session.get(operation_url)
        except requests.RequestException as e:
            raise ApiError(f"Failed to connect to MCP server: {str(e)}", 0)
        
        if response.status_code == 404:
            raise OperationError(f"Operation '{operation_id}' not found")
        
        if response.status_code != 200:
            raise ApiError("Failed to get operation status", response.status_code, response.text)
        
        try:
            result = response.json()
        except json.JSONDecodeError:
            raise ApiError("Invalid JSON response from server", response.status_code, response.text)
        
        if result.get("status") != "success":
            raise OperationError(result.get("message", f"Failed to get status for operation '{operation_id}'"))
        
        return result["data"]
    
    def wait_for_operation(self, operation_id: str, check_interval: int = 5, timeout: Optional[int] = None) -> Dict[str, Any]:
        """
        Wait for an operation to complete.
        
        Args:
            operation_id: ID of the operation
            check_interval: Interval in seconds between status checks
            timeout: Optional timeout in seconds
            
        Returns:
            Operation result
            
        Raises:
            OperationError: If the operation fails or times out
            ApiError: If the API request fails
        """
        start_time = time.time()
        
        while True:
            if timeout and (time.time() - start_time) > timeout:
                raise OperationError(f"Operation '{operation_id}' timed out after {timeout} seconds")
            
            operation = self.get_operation_status(operation_id)
            
            status = operation["status"]
            progress = operation.get("progress")
            
            if OperationStatus(status) == OperationStatus.COMPLETED:
                return operation["result"]
            
            if OperationStatus(status) == OperationStatus.FAILED:
                error_msg = operation.get("error", "Unknown error")
                raise OperationError(f"Operation '{operation_id}' failed: {error_msg}")
            
            logger.info(f"Operation '{operation_id}' status: {status}, progress: {progress or 'unknown'}")
            time.sleep(check_interval)
    
    def build_workload(self, 
                      formfile: Union[Formfile, Dict[str, Any], str], 
                      context_files: Optional[Dict[str, str]] = None,
                      build_options: Optional[Dict[str, Any]] = None,
                      wait_for_completion: bool = True,
                      timeout: Optional[int] = None) -> BuildResult:
        """
        Build a workload using the form_pack_build tool.
        
        Args:
            formfile: Formfile definition (as Formfile object, dict, or JSON string)
            context_files: Optional dictionary of file name to content mappings
            build_options: Optional build configuration options
            wait_for_completion: Whether to wait for the build to complete
            timeout: Optional timeout in seconds for waiting
            
        Returns:
            BuildResult object with build information
            
        Raises:
            ToolExecutionError: If the build fails
            OperationError: If waiting for the build operation fails
            ApiError: If the API request fails
        """
        logger.info("Building workload...")
        
        # Convert formfile to the right format
        if isinstance(formfile, Formfile):
            formfile_content = formfile.to_json()
        elif isinstance(formfile, dict):
            formfile_content = json.dumps(formfile)
        else:
            formfile_content = formfile
        
        parameters = {
            "formfile_content": formfile_content
        }
        
        if context_files:
            parameters["context_files"] = context_files
        
        if build_options:
            parameters["build_options"] = build_options
        
        result = self.execute_tool("form_pack_build", parameters)
        
        build_result = BuildResult(
            build_id=result.get("build_id"),
            status="accepted",
            message=result.get("message"),
            operation_id=result.get("operation_id"),
            details=result
        )
        
        # If the tool returns an operation ID and we're asked to wait, wait for it to complete
        if wait_for_completion and "operation_id" in result:
            logger.info(f"Build operation started with ID: {result['operation_id']}")
            operation_result = self.wait_for_operation(result["operation_id"], timeout=timeout)
            
            # Update the build result with the completed operation data
            build_result.status = operation_result.get("status", build_result.status)
            build_result.details = operation_result
            
        logger.info(f"Build completed with ID: {build_result.build_id}")
        return build_result
    
    def deploy_workload(self, 
                       build_id: str, 
                       instance_name: str,
                       vm_config: Optional[Union[VMConfig, Dict[str, Any]]] = None,
                       target_node: Optional[str] = None,
                       deployment_options: Optional[Dict[str, Any]] = None,
                       wait_for_completion: bool = True,
                       timeout: Optional[int] = None) -> DeploymentResult:
        """
        Deploy a workload using the form_pack_ship tool.
        
        Args:
            build_id: ID of the build to deploy
            instance_name: Name for the instance
            vm_config: Optional VM configuration
            target_node: Optional target node ID
            deployment_options: Optional deployment configuration
            wait_for_completion: Whether to wait for the deployment to complete
            timeout: Optional timeout in seconds for waiting
            
        Returns:
            DeploymentResult object with deployment information
            
        Raises:
            ToolExecutionError: If the deployment fails
            OperationError: If waiting for the deployment operation fails
            ApiError: If the API request fails
        """
        logger.info(f"Deploying workload with build ID {build_id}...")
        
        parameters = {
            "build_id": build_id,
            "instance_name": instance_name
        }
        
        # Convert vm_config to the right format if needed
        if isinstance(vm_config, VMConfig):
            parameters["vm_config"] = vm_config.to_dict()
        elif vm_config:
            parameters["vm_config"] = vm_config
        
        if target_node:
            parameters["target_node"] = target_node
            
        if deployment_options:
            parameters["deployment_options"] = deployment_options
        
        result = self.execute_tool("form_pack_ship", parameters)
        
        deployment_result = DeploymentResult(
            deploy_id=result.get("deployment_id", result.get("deploy_id")),
            status="accepted",
            instance_id=result.get("instance_id"),
            message=result.get("message"),
            operation_id=result.get("operation_id"),
            details=result
        )
        
        # If the tool returns an operation ID and we're asked to wait, wait for it to complete
        if wait_for_completion and "operation_id" in result:
            logger.info(f"Deployment operation started with ID: {result['operation_id']}")
            operation_result = self.wait_for_operation(result["operation_id"], timeout=timeout)
            
            # Update the deployment result with the completed operation data
            deployment_result.status = operation_result.get("status", deployment_result.status)
            deployment_result.instance_id = operation_result.get("instance_id", deployment_result.instance_id)
            deployment_result.details = operation_result
            
        logger.info(f"Deployment completed with ID: {deployment_result.deploy_id}")
        return deployment_result
    
    def get_build_status(self, build_id: str) -> Dict[str, Any]:
        """
        Get the status of a build.
        
        Args:
            build_id: ID of the build
            
        Returns:
            Build status information
            
        Raises:
            ToolExecutionError: If retrieving the status fails
            ApiError: If the API request fails
        """
        parameters = {"build_id": build_id}
        return self.execute_tool("form_pack_build_status", parameters)
    
    def get_deployment_status(self, deployment_id: str) -> Dict[str, Any]:
        """
        Get the status of a deployment.
        
        Args:
            deployment_id: ID of the deployment
            
        Returns:
            Deployment status information
            
        Raises:
            ToolExecutionError: If retrieving the status fails
            ApiError: If the API request fails
        """
        parameters = {"deployment_id": deployment_id}
        return self.execute_tool("form_pack_deployment_status", parameters) 