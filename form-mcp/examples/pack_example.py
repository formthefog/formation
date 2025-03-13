#!/usr/bin/env python3
"""
Example script demonstrating the use of the Formation MCP server's pack tools.

This script shows how to:
1. Build a workload using the form_pack_build tool
2. Deploy the workload using the form_pack_ship tool
"""

import requests
import json
import time
import sys
import argparse

# Default MCP server URL
DEFAULT_MCP_URL = "http://localhost:3010"

def login(mcp_url, address, signed_message, signature):
    """Log in to the MCP server and get an authentication token."""
    login_url = f"{mcp_url}/api/auth/login"
    login_data = {
        "address": address,
        "signed_message": signed_message,
        "signature": signature
    }
    
    response = requests.post(login_url, json=login_data)
    if response.status_code != 200:
        print(f"Login failed: {response.text}")
        sys.exit(1)
    
    result = response.json()
    if result.get("status") != "success":
        print(f"Login failed: {result.get('message')}")
        sys.exit(1)
    
    return result["data"]["token"]

def execute_tool(mcp_url, token, tool_name, parameters):
    """Execute a tool on the MCP server."""
    tool_url = f"{mcp_url}/api/tools/{tool_name}"
    headers = {"Authorization": f"Bearer {token}"}
    tool_data = {"parameters": parameters}
    
    response = requests.post(tool_url, headers=headers, json=tool_data)
    if response.status_code != 200:
        print(f"Tool execution failed: {response.text}")
        sys.exit(1)
    
    result = response.json()
    if result.get("status") != "success":
        print(f"Tool execution failed: {result.get('message')}")
        sys.exit(1)
    
    return result["data"]

def check_operation(mcp_url, token, operation_id):
    """Check the status of a long-running operation."""
    operation_url = f"{mcp_url}/api/operations/{operation_id}"
    headers = {"Authorization": f"Bearer {token}"}
    
    while True:
        response = requests.get(operation_url, headers=headers)
        if response.status_code != 200:
            print(f"Failed to check operation: {response.text}")
            sys.exit(1)
        
        result = response.json()
        if result.get("status") != "success":
            print(f"Failed to check operation: {result.get('message')}")
            sys.exit(1)
        
        operation = result["data"]
        if operation["status"] == "completed":
            return operation["result"]
        elif operation["status"] == "failed":
            print(f"Operation failed: {operation.get('error')}")
            sys.exit(1)
        
        print(f"Operation status: {operation['status']}, progress: {operation.get('progress', 'unknown')}")
        time.sleep(5)

def build_workload(mcp_url, token, formfile_content, context_files=None):
    """Build a workload using the form_pack_build tool."""
    print("Building workload...")
    
    parameters = {
        "formfile_content": formfile_content
    }
    
    if context_files:
        parameters["context_files"] = context_files
    
    result = execute_tool(mcp_url, token, "form_pack_build", parameters)
    
    # If the tool returns an operation ID, wait for it to complete
    if "operation_id" in result:
        print(f"Build operation started with ID: {result['operation_id']}")
        result = check_operation(mcp_url, token, result["operation_id"])
    
    print(f"Build completed with ID: {result['build_id']}")
    return result["build_id"]

def deploy_workload(mcp_url, token, build_id, instance_name, vm_config=None):
    """Deploy a workload using the form_pack_ship tool."""
    print(f"Deploying workload with build ID {build_id}...")
    
    parameters = {
        "build_id": build_id,
        "instance_name": instance_name
    }
    
    if vm_config:
        parameters["vm_config"] = vm_config
    
    result = execute_tool(mcp_url, token, "form_pack_ship", parameters)
    
    # If the tool returns an operation ID, wait for it to complete
    if "operation_id" in result:
        print(f"Deployment operation started with ID: {result['operation_id']}")
        result = check_operation(mcp_url, token, result["operation_id"])
    
    print(f"Deployment completed with ID: {result['deploy_id']}")
    return result

def main():
    parser = argparse.ArgumentParser(description="Example script for using Formation MCP pack tools")
    parser.add_argument("--mcp-url", default=DEFAULT_MCP_URL, help="MCP server URL")
    parser.add_argument("--address", required=True, help="User address for authentication")
    parser.add_argument("--signed-message", required=True, help="Signed message for authentication")
    parser.add_argument("--signature", required=True, help="Signature for authentication")
    
    args = parser.parse_args()
    
    # Log in to the MCP server
    token = login(args.mcp_url, args.address, args.signed_message, args.signature)
    print("Successfully logged in")
    
    # Example Formfile for a simple Python web server
    formfile_content = json.dumps({
        "from": "ubuntu:22.04",
        "name": "python-web-server",
        "run": [
            "apt-get update",
            "apt-get install -y python3"
        ],
        "include": ["server.py"],
        "env": {
            "PORT": "8080"
        },
        "expose": [8080],
        "entrypoint": "python3 server.py",
        "resources": {
            "vcpus": 1,
            "memory_mb": 512,
            "disk_gb": 5
        },
        "network": {
            "join_formnet": True
        }
    })
    
    # Example context files
    context_files = {
        "server.py": """
import http.server
import socketserver
import os

PORT = int(os.environ.get('PORT', 8080))

class Handler(http.server.SimpleHTTPRequestHandler):
    def do_GET(self):
        self.send_response(200)
        self.send_header('Content-type', 'text/html')
        self.end_headers()
        self.wfile.write(b'Hello from Formation!')

with socketserver.TCPServer(("", PORT), Handler) as httpd:
    print(f"Serving at port {PORT}")
    httpd.serve_forever()
"""
    }
    
    # Build the workload
    build_id = build_workload(args.mcp_url, token, formfile_content, context_files)
    
    # VM configuration for deployment
    vm_config = {
        "vcpus": 1,
        "memory_mb": 1024,
        "network": {
            "join_formnet": True
        }
    }
    
    # Deploy the workload
    deploy_result = deploy_workload(args.mcp_url, token, build_id, "python-web-server", vm_config)
    
    print("\nWorkload deployment summary:")
    print(f"  Build ID: {build_id}")
    print(f"  Deploy ID: {deploy_result['deploy_id']}")
    print(f"  Status: {deploy_result['details']['status']}")
    if deploy_result['details']['instance_id']:
        print(f"  Instance ID: {deploy_result['details']['instance_id']}")
    
    print("\nYou can check the status of your instance using the vm_status tool.")

if __name__ == "__main__":
    main() 