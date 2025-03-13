#!/usr/bin/env python3
"""
Example script demonstrating the use of the Formation MCP client library.

This script shows how to:
1. Initialize the client
2. Authenticate with the server
3. Build a workload using the Formfile class
4. Deploy the workload with configuration options
5. Handle errors properly
"""

import sys
import json
import argparse
import logging
from form_mcp_client import (
    FormationMCPClient, 
    Formfile, 
    VMConfig, 
    AuthenticationError,
    ToolExecutionError,
    OperationError,
    ApiError
)

# Set up logging
logging.basicConfig(level=logging.INFO)
logger = logging.getLogger(__name__)

# Default MCP server URL
DEFAULT_MCP_URL = "http://localhost:3010"

def main():
    """Main entry point for the example script."""
    parser = argparse.ArgumentParser(description="Example script for using Formation MCP client library")
    parser.add_argument("--mcp-url", default=DEFAULT_MCP_URL, help="MCP server URL")
    parser.add_argument("--address", required=True, help="User address for authentication")
    parser.add_argument("--signed-message", required=True, help="Signed message for authentication")
    parser.add_argument("--signature", required=True, help="Signature for authentication")
    parser.add_argument("--timeout", type=int, default=300, help="Timeout for operations in seconds")
    
    args = parser.parse_args()
    
    try:
        # Initialize the client
        client = FormationMCPClient(base_url=args.mcp_url)
        
        # Authenticate with the MCP server
        logger.info("Authenticating with the MCP server...")
        token = client.login(args.address, args.signed_message, args.signature)
        logger.info("Successfully authenticated")
        
        # Define a workload using the Formfile class
        logger.info("Creating Formfile definition...")
        formfile = Formfile(
            from_image="ubuntu:22.04",
            name="python-web-server",
            run=[
                "apt-get update",
                "apt-get install -y python3"
            ],
            include=["server.py"],
            env={"PORT": "8080"},
            expose=[8080],
            entrypoint="python3 server.py",
            resources={
                "vcpus": 1,
                "memory_mb": 512,
                "disk_gb": 5
            },
            network={"join_formnet": True}
        )
        
        # Define context files
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
        
        # Build options
        build_options = {
            "cache": True,
            "timeout_seconds": 600
        }
        
        # Build the workload
        logger.info("Building workload...")
        try:
            build_result = client.build_workload(
                formfile=formfile,
                context_files=context_files,
                build_options=build_options,
                wait_for_completion=True,
                timeout=args.timeout
            )
            logger.info(f"Build completed with ID: {build_result.build_id}")
        except (ToolExecutionError, OperationError, ApiError) as e:
            logger.error(f"Build failed: {str(e)}")
            sys.exit(1)
        
        # Define VM configuration for deployment
        vm_config = VMConfig(
            vcpus=1,
            memory_mb=1024,
            network={"join_formnet": True}
        )
        
        # Deployment options
        deployment_options = {
            "auto_restart": True,
            "timeout_seconds": 300
        }
        
        # Deploy the workload
        logger.info(f"Deploying workload with build ID {build_result.build_id}...")
        try:
            deploy_result = client.deploy_workload(
                build_id=build_result.build_id,
                instance_name="python-web-server",
                vm_config=vm_config,
                deployment_options=deployment_options,
                wait_for_completion=True,
                timeout=args.timeout
            )
            logger.info(f"Deployment completed with ID: {deploy_result.deploy_id}")
        except (ToolExecutionError, OperationError, ApiError) as e:
            logger.error(f"Deployment failed: {str(e)}")
            sys.exit(1)
        
        # Print deployment summary
        print("\nWorkload deployment summary:")
        print(f"  Build ID: {build_result.build_id}")
        print(f"  Deploy ID: {deploy_result.deploy_id}")
        print(f"  Status: {deploy_result.status}")
        
        if deploy_result.instance_id:
            print(f"  Instance ID: {deploy_result.instance_id}")
        
        if deploy_result.details and "endpoints" in deploy_result.details:
            print("\nEndpoints:")
            for endpoint in deploy_result.details["endpoints"]:
                protocol = endpoint.get("protocol", "http")
                host = endpoint.get("host", "localhost")
                port = endpoint.get("port", 8080)
                print(f"  {protocol}://{host}:{port}")
        
        print("\nYou can check the status of your instance using the vm_status tool.")
        
    except AuthenticationError as e:
        logger.error(f"Authentication failed: {str(e)}")
        sys.exit(1)
    except ApiError as e:
        logger.error(f"API error: {str(e)}")
        if hasattr(e, 'response_body') and e.response_body:
            logger.error(f"Response: {e.response_body}")
        sys.exit(1)
    except Exception as e:
        logger.error(f"Unexpected error: {str(e)}")
        sys.exit(1)

if __name__ == "__main__":
    main() 