# Formation

A vertically integrated 2-sided marketplace for AI Agents & Models to power the Age of Autonomy.

---

## Table of Contents

- [Overview](#overview)
- [Marketplace Features](#marketplace-features)
- [Deployment Architecture](#deployment-architecture)
- [Docker Configuration](#docker-configuration)
- [Special Thanks](#special-thanks)
- [Contributing](#contributing)
- [Pre-release Notice](#pre-release-notice)
- [Prerequisites & Setup](#prerequisites--setup)
  - [System Requirements](#system-requirements)
  - [System Dependencies](#system-dependencies)
  - [Rust Toolchain & Docker](#rust-toolchain--docker)
- [Network Configuration](#network-configuration)
  - [Configuring Your Local Network](#configuring-your-local-network)
  - [Setting Up a Bridge Interface](#setting-up-a-bridge-interface)
- [Running a Node](#running-a-node)
  - [Single Node Local Test (Docker)](#single-node-local-test-docker)
  - [Multinode Local Test (Docker)](#multinode-local-test-docker)
- [Joining the Official Developer Network](#joining-the-official-developer-network)
- [Formation Development Getting Started](#formation-development-guide)
  - [Getting Started](#getting-started)
  - [Core Workflow](#core-workflow)
  - [Writing Formfiles](#writing-formfiles)
  - [Advanced Topics](#advanced-topics)
  - [Troubleshooting](#troubleshooting)
- [AI Marketplace Development](#ai-marketplace-development)
  - [Creating AI Assets for the Formation Marketplace](#creating-ai-assets-for-the-formation-marketplace)
  - [Containerization Options for Agents](#containerization-options-for-agents)
- [Marketplace Deployment Process](#marketplace-deployment-process)

## Overview

Formation is a **Vertically Integrated 2-Sided Marketplace for AI Agents & Models** that enables:

1. **AI Creators** to publish, monetize, and distribute their AI models and agents
2. **AI Consumers** to discover, deploy, and utilize these AI capabilities through a unified platform

Built on a foundation of trustless, confidential computing, Formation manages the entire stack from infrastructure to marketplace features:

- **Vertically Integrated**: Controls every layer from low-level VM provisioning and networking to high-level marketplace functions including discovery, billing, and deployment
- **2-Sided Marketplace**: Connects AI providers with users through structured authentication, access controls, and usage-based billing
- **Fog Computing Infrastructure**: Leverages a distributed network of nodes for resilient, decentralized computation without central points of failure

This comprehensive platform serves as the foundation for the next generation of AI applications and autonomous systems. The core infrastructure of Formation is deployed as a suite of services, currently including `form-state` for state management, `form-dns` for network routing, `form-net` for secure mesh networking, `form-vmm` for virtual machine management, and `form-pack-manager` for agent and model packaging. These services work in concert to provide the platform's capabilities and are typically deployed using Docker Compose for ease of setup.

## Marketplace Features

The Formation marketplace provides comprehensive infrastructure for AI model and agent distribution:

### For AI Creators
- **Model & Agent Publishing**: Structured registration process with detailed metadata, versioning, and documentation
- **Monetization Options**: Flexible pricing models including subscription-based access, pay-per-use, and token-based billing
- **Usage Analytics**: Track model performance, adoption, and revenue across customers
- **Access Control**: Define private or public accessibility for models and agents with granular permissions

### For AI Consumers
- **Discovery & Deployment**: Find and deploy AI capabilities with standardized interfaces
- **Subscription Management**: Tiered subscription plans with varying levels of resource access
- **API Key Management**: Generate and manage API keys for programmatic access with configurable permissions
- **Usage Tracking**: Monitor token consumption, agent usage, and credit balances

### Core Technology
- **Authentication**: ECDSA based authentication means no API keys needed, and all interactions are verifiable. 
- **Billing Integration**: Flexible payment options with both Subscription based billing and Usage-based metering with credit system and Stripe integration
- **Resource Eligibility**: Automated enforcement of plan limitations and quota management
- **API Access**: Comprehensive API for programmatic interaction with all marketplace components

## Deployment Architecture

Formation's current deployment model revolves around a set of core microservices orchestrated via Docker Compose. These services provide the foundational capabilities for the AI marketplace:

- **`form-state`**: Manages the overall state of the network, including user accounts, instance information, and marketplace data. It serves as the central source of truth.
- **`form-dns`**: Provides DNS resolution within the Formation network, enabling service discovery and human-readable names for instances and services.
- **`form-net`**: Establishes a secure WireGuard-based mesh network (Formnet) for communication between nodes, instances, and users.
- **`form-vmm`**: The Virtual Machine Monitor responsible for creating, running, and managing the lifecycle of virtual machines that host AI agents and models.
- **`form-pack-manager`**: Handles the packaging of AI models and agents from `Formfile` definitions into runnable VM images.

These components are designed to work together to deliver a seamless experience for developing, deploying, and utilizing AI assets.

## Special Thanks

> "If I have seen further, it is by standing on the shoulders of giants." – Sir Isaac Newton

This project builds upon the impressive work of [Rust-VMM](https://github.com/rust-vmm), [Cloud-Hypervisor](https://github.com/cloud-hypervisor/cloud-hypervisor), [Innernet](https://github.com/tonarino/innernet), and [DStack-VM](https://github.com/amiller/dstack-vm).

---

## Contributing

As an open source project, we welcome your contributions. Before submitting changes, please review the [CONTRIBUTING](./CONTRIBUTING.md) file in its entirety.

---
## Pre-release Notice

> **WARNING:**  
> Formation is in early development. Although it is nearing production readiness, no guarantees are provided. Please report any issues in this repository.

---

## Prerequisites & Setup

### System Requirements

- **Operating System:** Ubuntu 22.04 LTS (or a compatible Linux distribution).
- **For local development & testing with Docker Compose:**
    - **Minimum:** 4+ CPU cores, 8GB RAM, 50GB+ free disk space.
    - **Recommended:** 8+ CPU cores, 16GB+ RAM, 100GB+ free disk space.
- **For participating as a full Formation Network provider node:**
    - Significant resources are required (e.g., 32+ physical cores, 64GB+ RAM, 8TB+ storage). Please refer to future documentation or contact the team for details on provider node requirements.

### System Dependencies

Install required packages for running the Dockerized services and network setup scripts:
```bash
sudo apt update
sudo apt install -y curl bridge-utils # dnsmasq is installed by the network validation script
```

   - `curl`: For downloading Docker installation scripts and other resources.
   - `bridge-utils`: For creating and managing network bridges (e.g., `br0`) required by Formation.
   - `dnsmasq`: Provides DHCP and DNS services for VMs; installed by the `scripts/validate-network-config.sh` if not present.
   - *(Note: If building services from source, additional dependencies like `build-essential`, `pkg-config`, `libssl-dev`, `libudev-dev`, `protobuf-compiler`, and `libsqlite3-dev` will be required.)*

### Rust Toolchain & Docker

Ensure Rust is installed and updated:
```bash
# install
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# source env
. "$HOME/.cargo/env"

# update
rustup update
```
Verify `cc` is installed:
```bash
which cc
```
If missing, install via `build-essential`.

#### **Install Docker**:
   ```bash
   sudo apt install -y apt-transport-https ca-certificates curl software-properties-common
   curl -fsSL https://download.docker.com/linux/ubuntu/gpg | sudo apt-key add -
   sudo add-apt-repository "deb [arch=amd64] https://download.docker.com/linux/ubuntu $(lsb_release -cs) stable"
   sudo apt update
   sudo apt install -y docker-ce
   sudo usermod -aG docker $USER
   ```

### Network Preparation

**Crucial:** Proper network configuration is essential *before* launching Formation services. This ensures that virtual machines can access the internet and that Formnet (the internal mesh network) functions correctly.

**1. Port Forwarding (If behind a NAT/Router):**

Ensure the following ports are open and forwarded on your router/firewall to the machine hosting Formation, if you intend to access services or instances from outside your local network or if specific Formation features require external reachability:

*   **`3002/tcp`**: `form-vmm` API
*   **`3003/tcp`**: `form-pack-manager` API
*   **`3004/tcp`**: `form-state` API
*   **`51820/udp`**: `form-net` (WireGuard for Formnet)

*(Note: Port 53333 for `form-p2p` is not required for the default `docker-compose` setup at this time.)*

**2. Bridge Interface (`br0`) and Local Networking Setup:**

Formation requires a network bridge named `br0` for VMs to connect to your local network and access the internet.

*   **Recommended Method: Using the Validation Script**

    The easiest way to configure the bridge, NAT, and `dnsmasq` (for VM IP assignment) is to run the provided script. From the root of the repository:
    ```bash
    sudo bash scripts/validate-network-config.sh
    ```
    This script will attempt to find an unused private IP range, create `br0`, set up NAT, and configure `dnsmasq`. Review the script's output for any errors.

*   **Manual Setup (Alternative/Advanced)**

    If you prefer or need to set up the network manually, follow these steps. These are condensed from the script's logic and original documentation:

    1.  **Find an available IP range:** Use a private range like `192.168.x.x` or `172.16.x.x`. Avoid `10.x.x.x` as Formnet may use it.
    2.  **Install `bridge-utils` and `dnsmasq`** (if not already installed by the script or dependency steps):
        ```bash
        sudo apt-get update && sudo apt-get upgrade
        sudo apt-get install bridge-utils dnsmasq
        ```
    3.  **Create the bridge:**
        ```bash
        sudo brctl addbr br0
        ```
    4.  **Assign an IP address to the bridge** (e.g., for `192.168.100.0/24` range):
        ```bash
        sudo ip addr add 192.168.100.1/24 dev br0
        ```
    5.  **Set the bridge interface up:**
        ```bash
        sudo ip link set br0 up
        ```
        *(Note: `ip addr show br0` may still show `DOWN` until a device is attached, this is normal.)*
    6.  **Enable IP forwarding:**
        ```bash
        sudo sysctl -w net.ipv4.ip_forward=1
        ```
    7.  **Add NAT rule using `iptables`** (replace `192.168.100.0/24` with your chosen range, and `eth0` with your main internet-facing interface if different):
        ```bash
        # First, find your main internet-facing interface (e.g., eth0, enpXsY)
        # export MAIN_IFACE=$(ip route show default | awk '{print $5}' | head -n1)
        # sudo iptables -t nat -A POSTROUTING -s 192.168.100.0/24 -o $MAIN_IFACE -j MASQUERADE
        # Example assuming 192.168.100.0/24 range. Adjust your main interface if not eth0.
        sudo iptables -t nat -A POSTROUTING -s 192.168.100.0/24 ! -o br0 -j MASQUERADE
        ```
        *To make this persistent, you might need `iptables-persistent`.* 
    8.  **Configure `dnsmasq` for `br0`:**
        Create or edit `/etc/dnsmasq.d/br0.conf` (adjust IP ranges as needed):
        ```ini
        interface=br0
        port=0 # Listen on all available ports for DNS, set to 53 if you want to restrict it to only br0
        dhcp-range=192.168.100.10,192.168.100.250,24h
        dhcp-option=option:router,192.168.100.1
        dhcp-option=option:dns-server,192.168.100.1,8.8.8.8,1.1.1.1 # Use bridge as DNS, then fallback
        # Ensure your host system's /etc/resolv.conf doesn't point to 127.0.0.53 if systemd-resolved is running dnsmasq
        # or if you run into issues, consider setting `port=0` in /etc/dnsmasq.conf and let dnsmasq run on port 53 for br0 only.
        ```
    9.  **Restart `dnsmasq`:**
        ```bash
        sudo systemctl restart dnsmasq
        ```
    10. **(Optional) Test with a network namespace:** (Commands from original README can be used here for verification).

---

## Deploying Core Services with Docker Compose

The primary method for deploying the Formation core services for local development and testing is by using the provided `docker-compose.yml` file. This file defines and configures all the necessary services: `form-state`, `form-dns`, `form-net`, `form-vmm`, and `form-pack-manager`.

**Steps:**

1.  **Clone the Repository (if you haven't already):**
    ```bash
    git clone https://github.com/formthefog/formation.git # Replace with the actual repository URL if different
    cd formation
    ```

2.  **Ensure Network Prerequisites are Met:**
    Before proceeding, make sure you have completed all steps in the "Network Preparation" section under "Prerequisites & Setup". This includes setting up the `br0` bridge interface and ensuring necessary ports are available.

3.  **Configure Environment Variables:**
    The `docker-compose.yml` file uses environment variables for sensitive or configurable paths. You'll need to create a `.env` file in the same directory as the `docker-compose.yml` file (usually the project root) or ensure these variables are set in your shell environment.

    Create a file named `.env` with the following content, adjusting paths and passwords as necessary:

    ```env
    # Path to your operator configuration file (e.g., .operator-config.json)
    # This file contains cryptographic keys and operator settings.
    # Ensure this file exists. For a first-time setup, you might need to use a tool like form-config-wizard (if available and updated)
    # or manually create a properly structured JSON file according to the expected format.
    # Example: SECRET_PATH=/home/user/.config/formation/.operator-config.json
    SECRET_PATH=/path/to/your/.operator-config.json

    # Password to encrypt/decrypt sensitive information in the operator config.
    PASSWORD=your-strong-encryption-password

    # Optional: You can set other environment variables used by docker-compose services here if needed.
    # Refer to the docker-compose.yml for variables like DYNAMIC_JWKS_URL, FORMNET_LOG_LEVEL etc.
    ```
    *   **Important for `SECRET_PATH`**: The `.operator-config.json` file is critical. It contains your node's identity (cryptographic keys) and operational settings. Without a valid configuration file pointed to by `SECRET_PATH`, core services like `form-state` and `form-net` may fail to initialize correctly or will operate with default/insecure settings if they generate placeholders.
    *   **For `PASSWORD`**: Choose a strong password. This is used by services to decrypt sensitive parts of the operator configuration if it's encrypted.

4.  **Start the Services:**
    Navigate to the directory containing `docker-compose.yml` and run:
    ```bash
    docker-compose up -d
    ```
    This command will pull the necessary Docker images (if not already present) from Docker Hub (e.g., `formationai/*`) and start all services in detached mode (`-d`).

5.  **Verify the Services:**
    *   Check that all containers are running:
        ```bash
        docker-compose ps
        ```
        You should see containers for `formation-state`, `formation-dns`, `formation-network` (for `form-net`), `formation-vmm`, and `formation-pack-manager` in an "Up" or "healthy" state.

    *   Check the logs for any errors:
        ```bash
        docker-compose logs -f # View logs for all services
        docker-compose logs -f form-state # View logs for a specific service
        ```

    *   **Check Health Endpoints:** Most services expose a health check endpoint:
        *   `form-state`: `curl http://localhost:3004/health`
        *   `form-dns`: The `docker-compose.yml` uses `dig @localhost -p 5453 formation +short`. Manual check might involve querying through it.
        *   `form-net`: `curl http://localhost:8080/health` (as its API server `FORMNET_SERVER_PORT` defaults to 8080).
        *   `vmm-service` (`formation-vmm` container): `curl http://localhost:3002/health`
        *   `form-pack-manager`: `curl http://localhost:3003/health`

        A successful response (often JSON with a "Healthy" status) indicates the service is operational.

Once these steps are completed, the Formation core services should be running and ready for interaction.

## Deploying Your First Agent (via API)

Once the core Formation services are running, you can deploy an "agent." In Formation, an agent is essentially a program or service running inside a dedicated Virtual Machine (VM) instance. This guide demonstrates how to deploy a basic agent using `curl` to interact with the Formation API. The `form-cli` tool is under development and will provide a more streamlined interface in the future.

### Agent and Formfile Concepts

*   **Agent:** An AI model, a web service, or any other application you want to run securely and managed within the Formation network. It's packaged and executed within a VM.
*   **`Formfile`:** A text file that defines how your agent is built and configured. Similar to a `Dockerfile`, it specifies:
    *   Base operating system components.
    *   System resources required (vCPUs, memory, disk).
    *   Files to be copied into the VM (e.g., your agent's code, models, configuration).
    *   Build commands to set up the environment (e.g., installing dependencies).
    *   User accounts and SSH access.
    *   The `ENTRYPOINT` or command that starts your agent's service when the VM boots.
*   **`form-pack-manager`:** This core service is responsible for taking your `Formfile` and project files, building a VM image from them, and storing it for deployment by `form-vmm`.

For this first example, we'll assume a very simple `Formfile` that runs a basic web server. The actual content of the agent's code or a complex `Formfile` is beyond this initial guide, but understanding the concept is key.

### Creating the Agent Instance

To deploy an agent, you send a request to the `form-state` service, which then coordinates with `form-pack-manager` to build the image (if not already built from a previous identical request) and `form-vmm` to launch the instance.

**Endpoint:** `POST http://localhost:3004/instance/create`

**Request Body (`CreateInstanceRequest`):**
A JSON object containing:
*   `formfile` (string): The complete content of your `Formfile`.
*   `name` (string): A unique name for this agent build. This name can be used to identify and manage instances of this agent type. The combination of the owner (derived from your signature) and this name typically forms a unique `build_id`.

**Authentication:**
All Formation API endpoints that modify state or access private resources are protected. The `/instance/create` endpoint uses ECDSA signature-based authentication. You need to:
1.  Generate an Ethereum-style keypair. The public key (address) will be the owner of the instance.
2.  Construct the JSON payload for the `CreateInstanceRequest`.
3.  Hash the JSON payload string (e.g., using Keccak256 or SHA256).
4.  Sign the resulting hash using your private key.
5.  Include the hash, signature, and recovery ID in the request headers.

**Headers for Authentication:**
*   `Content-Type: application/json`
*   `X-Message: <The hash (e.g., Keccak256 or SHA256) of the JSON payload string>`
*   `X-Signature: <hex-encoded signature of the hash specified in X-Message>`
*   `X-Recovery-Id: <recovery_id (0, 1, 2, or 3) as a string>`

**Example `curl` Request:**

First, create an example `Formfile` in your current directory. Let's name it `Formfile.my-simple-agent`:

```formfile
NAME my-simple-agent
VCPU 1
MEM 512
DISK 5

USER username:agentuser passwd:somepassword ssh_authorized_keys:"your-ssh-public-key"

INSTALL python3

# Create a dummy app directory for the COPY command to succeed
RUN mkdir -p /app
COPY ./app /app 
WORKDIR /app
RUN echo "from http.server import SimpleHTTPRequestHandler, HTTPServer; server_address = ('', 8000); httpd = HTTPServer(server_address, SimpleHTTPRequestHandler); print('Server running on port 8000...'); httpd.serve_forever()" > /app/server.py

EXPOSE 8000
ENTRYPOINT ["python3", "server.py"]
```
*(Note: In a real scenario, `./app` would contain your agent's code. The `RUN mkdir -p /app` is included so the `COPY` command doesn't fail if an empty `./app` directory is used for this example.)*

Now, use the following bash script to send the request. This script uses `jq` to construct the JSON payload and `sha256sum` to hash it for the `X-Message` header (replace with Keccak256 or the appropriate hashing algorithm if specified by the API).

```bash
# 1. Define the path to your Formfile
FORMFILE_PATH="Formfile.my-simple-agent"

# 2. Read Formfile content
FORMFILE_CONTENT=$(cat "${FORMFILE_PATH}")

# 3. Define your agent build name
AGENT_BUILD_NAME="my-first-agent-from-file"

# 4. Construct the JSON payload string using jq (recommended for robustness)
#    jq handles escaping of special characters within FORMFILE_CONTENT.
JSON_PAYLOAD=$(jq -n --arg ff "$FORMFILE_CONTENT" --arg name "$AGENT_BUILD_NAME" \
                 '{formfile: $ff, name: $name}')

# 5. Hash the JSON_PAYLOAD for the X-Message header
#    Replace sha256sum with the correct hashing algorithm (e.g., keccak256sum -l | awk '{print $1}')
#    if required by the API, and adjust prefix if needed (e.g., "0x").
HASH_OF_PAYLOAD=$(echo -n "${JSON_PAYLOAD}" | sha256sum | awk '{print $1}')
X_MESSAGE_HEADER_CONTENT="0x${HASH_OF_PAYLOAD}" # Assuming 0x prefix for the hash

# 6. Sign the HASH_OF_PAYLOAD (i.e., the value that will be in X-Message header)
#    This is a placeholder for actual signing.
#    Replace <...> placeholders with your actual signature data.
#    The signature must be over the exact string provided in X-Message.
MESSAGE_TO_SIGN="${X_MESSAGE_HEADER_CONTENT}" 
SIGNATURE_HEX="<your-hex-encoded-signature-of-MESSAGE_TO_SIGN>"
RECOVERY_ID_STR="<your-recovery-id-as-a-string-e.g.-0-or-1>"

# 7. Make the API call
curl -X POST http://localhost:3004/instance/create \\
     -H "Content-Type: application/json" \\
     -H "X-Message: ${X_MESSAGE_HEADER_CONTENT}" \\
     -H "X-Signature: ${SIGNATURE_HEX}" \\
     -H "X-Recovery-Id: ${RECOVERY_ID_STR}" \\
     -d "${JSON_PAYLOAD}"
```

**Important Notes on the Example:**
*   **`Formfile.my-simple-agent`**: Ensure this file exists in the directory where you run the script and that its content is valid.
*   **`jq` for JSON Construction**: Using `jq` (as shown in the primary example for `JSON_PAYLOAD`) is highly recommended. It correctly handles escaping special characters from the `Formfile` content when embedding it into the JSON string. If `jq` is not available, manual string manipulation for escaping is fragile.
*   **Hashing for `X-Message`**: The `X-Message` header must contain the hash of the JSON payload. The example uses `sha256sum` and prepends "0x"; verify the exact hashing algorithm (e.g., Keccak256) and format required by the Formation API.
*   **Signature Generation**: The `X-Signature` must be a signature of the exact hash string sent in the `X-Message` header. Use appropriate cryptographic libraries or tools for this.

**Expected Response:**
A successful request will return a JSON object, possibly including an `instanceId` or `buildId` and a status indicating that the instance creation process has started. The exact structure may vary.

```json
{
  "instanceId": "0xabcdef0123456789...", 
  "buildId": "<instance-build-id>",
  "status": "REQUEST_RECEIVED",
  "message": "Instance creation request received and is being processed."
}
```
*(The `instanceId` is typically a unique identifier for this specific deployment attempt or resulting instance, while `buildId` refers to the version of the agent defined by the `name` and `formfile` content.)*

### Checking Agent Status and Boot Completion

After requesting instance creation, you'll want to check its status. The build process and VM boot can take some time.

**Endpoints:**
*   To get a specific instance by its unique `instanceId` (obtained from the create response or other listings):
    `GET http://localhost:3004/instance/:instance_id/get`
*   To get (potentially multiple) instances associated with a `build_id` (the `name` you provided during creation):
    `GET http://localhost:3004/instance/:build_id/get_by_build_id`

**Authentication:**
These GET endpoints also require ECDSA signature authentication, similar to the create request.
1.  The message to sign for a GET request is typically the request path string itself (e.g., `/instance/your_instance_id_here/get`).
2.  Include `X-Message` (the request path), `X-Signature`, and `X-Recovery-Id` in the headers.

**Example `curl` Request (to get by `instanceId`):

Assume `INSTANCE_ID` is the ID you received from the creation step or from listing instances.

```bash
# 1. Define the instance ID you want to query
INSTANCE_ID="0xabcdef0123456789..." # Replace with the actual instance ID

# 2. Define the request path (this is what you will sign)
REQUEST_PATH="/instance/${INSTANCE_ID}/get"

# 3. Sign the REQUEST_PATH string (placeholder for actual signing)
#    Obtain these values using an appropriate ECDSA signing tool/library.
#    The X-Message header MUST be this exact REQUEST_PATH string.
SIGNED_MESSAGE_PATH="${REQUEST_PATH}"
SIGNATURE_HEX_GET="<your-hex-encoded-signature-of-SIGNED_MESSAGE_PATH>"
RECOVERY_ID_STR_GET="<your-recovery-id-as-a-string-e.g.-0-or-1>"

# 4. Make the API call
curl -X GET "http://localhost:3004${REQUEST_PATH}" \\
     -H "X-Message: ${SIGNED_MESSAGE_PATH}" \\
     -H "X-Signature: ${SIGNATURE_HEX_GET}" \\
     -H "X-Recovery-Id: ${RECOVERY_ID_STR_GET}"
```

**Expected Response:**
A JSON object describing the instance, including its status, IP address (once assigned), and other metadata.

```json
{
  "instanceId": "0xabcdef0123456789...",
  "buildId": "<instance-build-id>",
  "ownerAddress": "0xYourAddress...",
  "status": "RUNNING", // Could be PENDING_BUILD, BUILDING, STARTING, RUNNING, FAILED, STOPPED, etc.
  "formnetIp": "10.x.x.x", // Assigned once the VM boots and joins Formnet
  "createdAt": "2023-10-27T10:00:00Z",
  "updatedAt": "2023-10-27T10:05:00Z"
  // ... other fields like resources, node_id where it's running, etc.
}
```
You would poll one of these endpoints until the `status` is "RUNNING" (or your desired state) and `formnetIp` is populated if network access is needed.

### Interacting with Your Deployed Agent

Once your agent instance's status is "RUNNING", you don't typically interact with it directly via its Formnet IP from outside the network if you are an end-user. Instead, `form-state` acts as an authenticated gateway, proxying your requests to the agent running securely within the Formnet.

One common way to interact is by sending a JSON payload to your agent through a specific `form-state` API endpoint, such as `/agents/:agent_id/hire`. This endpoint takes your payload, authenticates your request, and then `form-state` forwards the payload to the correct agent instance over Formnet.

**Endpoint for Agent Interaction (Example via `form-state`):**
`POST http://localhost:3004/agents/:agent_id/hire`

*   `:agent_id`: This is the identifier for your agent. It could be the `buildId` (the `name` you provided during instance creation) or another ID assigned upon agent registration.

**Request Body:**
The request body is a JSON payload that you want to send to your agent. The agent's application (defined by its `ENTRYPOINT` in the `Formfile`) must be designed to receive and process this payload.

**Authentication:**
This `form-state` endpoint is protected by ECDSA signature authentication, identical to the `/instance/create` call:
1.  The message to sign is the exact JSON payload string of your request body.
2.  Include `X-Message` (the JSON payload string), `X-Signature` (hex-encoded signature), and `X-Recovery-Id` (as a string) in the request headers.

**Example `curl` Request (sending a task to an agent via `form-state`):

Assuming:
*   `AGENT_ID` is "my-first-agent-from-file".
*   Your agent is designed to process a JSON payload (e.g., a Flask/FastAPI app rather than a simple static file server). Let's imagine it expects a task.
*   You have generated the necessary signature for the payload.

```bash
# 1. Define the Agent ID
AGENT_ID="my-first-agent-from-file" # Replace with your agent's actual buildId or registered agent ID

# 2. Construct the JSON payload intended for your agent
AGENT_JSON_PAYLOAD='{"task": "translate_text", "source_language": "en", "target_language": "es", "text": "Hello, world!"}'

# 3. Sign the AGENT_JSON_PAYLOAD string (placeholder for actual signing)
#    - The X-Message header MUST be this exact AGENT_JSON_PAYLOAD string.
#    - Obtain these values using an appropriate ECDSA signing tool/library.
SIGNED_AGENT_MESSAGE="${AGENT_JSON_PAYLOAD}"
SIGNATURE_HEX_HIRE="<your-hex-encoded-signature-of-SIGNED_AGENT_MESSAGE>"
RECOVERY_ID_STR_HIRE="<your-recovery-id-as-a-string>" # e.g., "0" or "1"

# 4. Make the API call to form-state
curl -X POST "http://localhost:3004/agents/${AGENT_ID}/run_task" \\
     -H "Content-Type: application/json" \\
     -H "X-Message: ${SIGNED_AGENT_MESSAGE}" \\
     -H "X-Signature: ${SIGNATURE_HEX_HIRE}" \\
     -H "X-Recovery-Id: ${RECOVERY_ID_STR_HIRE}" \\
     -d "${AGENT_JSON_PAYLOAD}"
```

**Expected Response from `form-state`:**
`form-state` will proxy the request to your agent and stream the agent's response directly back to you. Therefore, the structure and content of the JSON response you receive will be whatever your agent application sends.

For example, if your agent was an LLM designed for text summarization and received a payload like:
`{"text_to_summarize": "A very long article about renewable energy...", "max_length": 100}`

A realistic proxied response from your agent, returned via `form-state`, might look like this:

```json
{
  "summary": "Renewable energy sources are pivotal for sustainable development, offering environmental benefits and energy independence. Key technologies include solar, wind, and hydro power, with ongoing research focusing on efficiency and storage solutions.",
  "original_char_count": 5832,
  "summary_char_count": 285,
  "model_used": "form-summarizer-xl-v2.1",
  "processing_time_seconds": 3.75
}
```
Or, if your agent performed a question-answering task based on a given context, the response might be:
```json
{
  "question": "What are the main advantages of decentralization?",
  "answer": "The main advantages of decentralization include increased resilience against single points of failure, enhanced censorship resistance, and often greater transparency and user autonomy.",
  "confidence": 0.92,
  "sources_consulted": ["document_A.pdf", "internal_knowledge_base_v3"]
}
```

**Important Considerations for Agent Interaction:**
*   **Agent Application Design:** Your agent's `ENTRYPOINT` application needs to be a service (e.g., a web server using Flask, FastAPI, Node.js Express, etc.) capable of receiving the JSON payload (or other content types as configured) from `form-state`, processing it, and returning a JSON response (or other appropriate content type).
*   **Streaming and Timeouts:** Since `form-state` streams the response, be mindful of potential timeouts. Long-running agent tasks should be designed with appropriate client-side and server-side timeout handling, or consider asynchronous patterns if the platform supports them for such tasks.
*   **Interaction Patterns:** The `/agents/:id/hire` endpoint implies a general way to invoke an agent. The specifics of how your agent interprets the payload and what endpoints it exposes internally are up to your agent's design. `form-state` facilitates authenticated access and proxies the communication. Always refer to the latest Formation API documentation for any specific requirements or additional supported interaction patterns (e.g., different endpoints for different types of agent interactions).

This revised interaction model, where `form-state` serves as an authenticated gateway that streams the agent's response, is crucial for the security and architecture of the Formation network.

# Formation Development Guide

> **Note on `form` CLI Usage:** The `form` Command Line Interface (CLI) described in parts of this guide is currently under active development and alignment with the latest API changes and authentication mechanisms. While it represents the intended future user experience, the most stable way to interact with Formation at this time is through direct API calls as demonstrated in the "Deploying Your First Agent (via API)" section. Information regarding `form` CLI commands should be considered preliminary.

Formation is a platform for building, deploying, and managing verifiable confidential VPS instances in the Formation network. This guide will walk you through the core development workflow and key concepts.

## Table of Contents
- [Getting Started](#getting-started)
- [Core Workflow](#core-workflow)
- [Writing Formfiles](#writing-formfiles)
- [Advanced Topics](#advanced-topics)
- [Troubleshooting](#troubleshooting)

## Getting Started

Formation uses a CLI tool called `form` to manage the entire development workflow. Before you begin development, you'll need to install and configure the Formation CLI.

### Installing Form

To install the official Formation CLI, run the following command:

```bash
curl https://dev.formation.cloud/install/form/install.sh | sudo bash
```

This script will download and install the latest version of the Formation CLI. The installation requires root privileges to ensure proper system integration.

### Initial Setup

1. Install the Formation CLI (installation instructions coming soon)

2. Initialize your development environment:
```bash
sudo form kit init
```

This launches an interactive wizard that will:
- Create or import a wallet for signing requests
- Set up your keystore location and configuration
- Configure your provider settings
- Set up your Formnet participation preferences

The wizard saves your configuration in `~/.config/form/config.json` by default.

#### Be sure to add one of the 2 hosts (or both):
<hr>
host 1: 3.214.9.18
<br>
host 2: 44.218.128.162
<hr>

### Joining Formnet

Formnet is Formation's peer-to-peer network that enables secure communication between instances. If you didn't join during initialization, you can join with:

```bash
sudo form manage join
sudo form manage formnet-up
```

The `formnet-up` command starts a background process that maintains your peer connections. This must be running to access your instances.

## Core Workflow

### 1. Create Your Formfile

Every Formation project needs a `Formfile` in its root directory. The Formfile defines your instance configuration and build process. See the [Writing Formfiles](#writing-formfiles) section for details.

### 2. Build Your Instance

From your project root directory:

```bash
sudo form pack build
```

This command:
- Validates your Formfile
- Creates a build context from your project
- Generates a unique build ID
- Initiates the build process

You'll receive a build ID that you'll use to track your build status:

```bash
form pack status --build-id <your-build-id>
```

### 3. Deploy Your Instance

Once your build succeeds, deploy it with:

```bash
form pack ship
```

This command must be run from the same directory as your original build.

### 4. Access Your Instance

Formation automatically creates redundant instances for reliability. Get their addresses with:

```bash
form manage get-ip --build-id <your-build-id>
```

Once you have an IP, access your instance via SSH:

```bash
ssh <username>@<formnet-ip>
```

Note: SSH access requires:
- Active Formnet membership
- Running `formnet-up` process
- Valid SSH key configured in your Formfile

## Writing Formfiles

A Formfile defines your instance configuration and build process. Here's the anatomy of a Formfile:

## Formfile Reference

A Formfile consists of several types of instructions that define your instance configuration and build process. Let's examine each component in detail.

### Build Instructions

#### RUN Command
The RUN instruction executes commands in the image as root during the build phase. Use this for any system-level configuration or setup tasks.

```
RUN apt-get update
RUN echo "custom_setting=value" >> /etc/system.conf
```

Multiple commands can be chained using && to create a single layer:
```
RUN apt-get update && apt-get install -y curl && rm -rf /var/lib/apt/lists/*
```

#### COPY Command
COPY transfers files from your build context to the instance. The build context is the directory containing your Formfile. The files will be placed in a temporary artifacts directory, archived, and then copied to your specified WORKDIR.

```
COPY ./app /app
COPY ./config.json /etc/myapp/config.json
```

If no source is specified, all files from the current directory will be copied.

#### INSTALL Command
INSTALL provides a simplified way to install system packages using apt-get. While this could be done with RUN, INSTALL handles update, installation, and cleanup automatically.

```
INSTALL nginx python3 postgresql
```

#### ENV Command
ENV sets environment variables with specific scopes. Unlike traditional Docker ENV instructions, Formation's ENV requires a scope specification.

Scopes can be:
- system: System-wide variables
- user:<username>: User-specific variables
- service:<service-name>: Service-specific variables

```
ENV --scope=system PATH=/usr/local/bin:$PATH
ENV --scope=user:webdev DB_PASSWORD=secret
ENV --scope=service:nginx NGINX_PORT=80
```

#### ENTRYPOINT Command
ENTRYPOINT specifies the command that runs when your instance starts. It can be specified in two formats:

JSON array format (recommended):
```
ENTRYPOINT ["nginx", "-g", "daemon off;"]
```

Shell format:
```
ENTRYPOINT nginx -g "daemon off;"
```

#### EXPOSE Command
EXPOSE documents the ports your application uses. While it doesn't actually publish the ports, it serves as documentation and may be used by Formation's networking layer.

```
EXPOSE 80 443 8080
```

### User Configuration

The USER instruction in a Formfile supports comprehensive user account configuration. Here are all available options:

```
USER username:myuser \
     passwd:mypassword \
     ssh_authorized_keys:"ssh-rsa AAAA... user@host" \
     lock_passwd:false \
     sudo:true \
     shell:/bin/bash \
     ssh_pwauth:true \
     disable_root:true \
     chpasswd_expire:true \
     groups:docker,sudo
```

Configuration Options:

username (Required)
- Must start with a lowercase letter or underscore
- Can contain lowercase letters, numbers, underscores, or hyphens
- Maximum length of 32 characters

passwd (Required)
- Sets the user's password
- Will be appropriately hashed during instance creation
- Should meet your security requirements

ssh_authorized_keys (Optional)
- List of SSH public keys for remote access
- Multiple keys can be provided as a comma-separated list
- Required for SSH access to your instance

lock_passwd (Optional, default: false)
- When true, prevents password-based login
- Useful when enforcing SSH-only access

sudo (Optional, default: false)
- Grants sudo privileges to the user
- When true, adds user to sudo group

shell (Optional, default: /bin/bash)
- Specifies the user's login shell
- Must be an absolute path

ssh_pwauth (Optional, default: true)
- Enables or disables SSH password authentication
- Consider setting to false when using SSH keys exclusively

disable_root (Optional, default: true)
- Controls whether root login is disabled
- Best practice is to leave enabled and use sudo

chpasswd_expire (Optional, default: true)
- When true, forces password change on first login
- Useful for generating secure initial passwords

groups (Optional)
- Additional groups for the user
- Provided as comma-separated list
- Common groups: docker, sudo, users

### Required Fields

- `NAME`: Identifier for your instance (auto-generated if omitted)
- `USER`: At least one user configuration
- System Resources:
  - `VCPU`: 1-128 cores (default: 1)
  - `MEM`: 512-256000 MB (default: 512)
  - `DISK`: 5-65535 GB

### User Configuration

The `USER` directive supports multiple options:

```
USER username:myuser \
     passwd:mypassword \
     sudo:true \
     ssh_authorized_keys:"ssh-rsa ..." \
     lock_passwd:false \
     shell:/bin/bash \
     ssh_pwauth:true \
     disable_root:true \
     groups:docker,users
```

### Example: Simple Web Server

```
NAME hello-server

USER username:webdev passwd:webpass123 sudo:true ssh_authorized_keys:"ssh-rsa ..."

VCPU 2
MEM 2048
DISK 5

COPY ./app /app
INSTALL python3

WORKDIR /app
ENTRYPOINT ["python3", "server.py"]
```

## Advanced Topics

### Resource Limits

Development Network Limits:
- VCPU: Max 2 cores
- Memory: 512-4096 MB
- Disk: Max 5 GB

These limits will be higher on testnet and mainnet.

### Nginx Configuration

Formation instances come with a pre-installed nginx server. Your configuration needs will depend on your deployment architecture.

#### Option 1: Using the System Nginx

For simple deployments, you can replace the default nginx configuration:

```
COPY ./my-nginx.conf /etc/nginx/nginx.conf
RUN sudo systemctl restart nginx
```

This approach works well when:
- Your application doesn't use containerized nginx
- You need a simple reverse proxy or static file server
- You want to maintain the standard system service

#### Option 2: Containerized Nginx with Docker Networking

When using docker-compose or container deployments that rely on Docker's internal networking (e.g., using `proxy_pass http://container-name`), you'll need to manage the system nginx service. There are two approaches:

You can manage the system nginx service directly in your Formfile using the `RUN` command:

1. To stop nginx for the current session:
   ```
   RUN sudo systemctl stop nginx
   ```

2. To permanently disable nginx on boot:
   ```
   RUN sudo systemctl stop nginx && sudo systemctl disable nginx
   ```

Including these commands in your Formfile automates the service management as part of your deployment.

Important Considerations:
- If your nginx configuration uses Docker container names in `proxy_pass` directives, you must use a containerized nginx instance
- The system nginx service must be stopped to avoid port conflicts
- Even with `disable`, you may need to SSH into the instance after initial deployment
- Future updates to Formation may provide more automated solutions for this workflow

Example docker-compose nginx configuration:
```yaml
services:
  nginx:
    image: nginx:latest
    ports:
      - "80:80"
    volumes:
      - ./nginx.conf:/etc/nginx/nginx.conf
    depends_on:
      - app
```

With corresponding nginx.conf:
```nginx
http {
    upstream app {
        server app:3000;  # Docker network allows using container name
    }
    server {
        listen 80;
        location / {
            proxy_pass http://app;
        }
    }
}
```

These container-based deployments require careful consideration of service orchestration and may need additional deployment steps.

### Build Context

The build context is determined by the directory containing your Formfile. All `COPY` commands are relative to this directory.

## Troubleshooting

### Common Issues

1. Cannot SSH into instance
   - Verify `formnet-up` is running
   - Confirm you've joined Formnet
   - Check your SSH key configuration

2. Build fails
   - Verify your resource requests are within limits
   - Check your Formfile syntax
   - Ensure all copied files exist in your build context

3. Deployment issues
   - Confirm you're in the same directory as your build
   - Verify your network connection
   - Check your provider status

### Getting Help

Join our community:
- GitHub: github.com/formthefog/formation
- Twitter: @formthefog


## Roadmap

#### **COMING SOON**

<hr>

## AI Marketplace Development

### Creating AI Assets for the Formation Marketplace

The Formation marketplace enables developers to create, publish, and monetize AI models and agents. While the user interface simplifies this process, understanding the underlying Formfile structure is valuable for advanced customization.

#### Containerization Options for Agents

Formation supports multiple deployment patterns for AI agents within VM instances:

1. **Docker Container Deployment**:
```
NAME containerized-agent

USER username:aiagent passwd:securepass sudo:true ssh_authorized_keys:"ssh-rsa ..."

VCPU 2
MEM 4096
DISK 20

INSTALL docker.io docker-compose

RUN systemctl enable docker
RUN systemctl start docker

COPY ./agent-container /app
WORKDIR /app

ENTRYPOINT ["docker", "run", "--name", "agent-service", "-p", "8080:8080", "agent-image:latest"]
```

2. **Docker Compose from Git Repository**:
```
NAME git-compose-agent

USER username:aiagent passwd:securepass sudo:true ssh_authorized_keys:"ssh-rsa ..."

VCPU 2
MEM 4096
DISK 20

INSTALL docker.io docker-compose git

RUN systemctl enable docker
RUN systemctl start docker

WORKDIR /app
RUN git clone https://github.com/your-org/your-agent-repo.git .

ENTRYPOINT ["docker-compose", "up", "-d"]
```

3. **Native Execution from Source**:
```
NAME native-agent

USER username:aiagent passwd:securepass sudo:true ssh_authorized_keys:"ssh-rsa ..."

VCPU 2
MEM 4096
DISK 20

COPY ./agent /app/agent
INSTALL python3 python3-pip

WORKDIR /app
RUN pip install -r agent/requirements.txt

ENTRYPOINT ["python3", "agent/main.py"]
```

#### Example Model Formfile

```
NAME llm-model-deployment

USER username:aidev passwd:securepass sudo:true ssh_authorized_keys:"ssh-rsa ..."

VCPU 4
MEM 16384
DISK 50

COPY ./model /app/model
INSTALL python3 python3-pip

WORKDIR /app
RUN pip install -r model/requirements.txt

ENTRYPOINT ["python3", "model/serve.py"]
```

AI assets deployed through the marketplace will automatically integrate with Formation's billing, authentication, and access control systems.

#### Marketplace Deployment Process

> **Note:** The web interface and streamlined marketplace deployment process described below are part of the planned future functionality and are currently under development.

While developers will primarily interact with the Formation marketplace through our web interface, understanding the underlying deployment process is helpful:

1. **Asset Creation**: Developers build their AI model or agent using their preferred tools and frameworks
2. **Asset Registration**: Through the web interface, details about the model/agent are provided (capabilities, resource needs, pricing)
3. **Asset Packaging**: Behind the scenes, the system generates a Formfile and prepares the asset for deployment
4. **Deployment**: The asset is deployed as a secure VM instance with appropriate networking
5. **Publication**: Once deployed and verified, the asset becomes available in the marketplace
6. **Monetization**: Users can discover and use the asset based on the specified pricing model

This streamlined process handles all the complexity of deployment, security, and billing infrastructure automatically.

<hr>

## Project Roadmap

This section outlines the current status and future direction of the Formation project.

### Production Ready

*   **Core Services Suite:**
    *   `form-state`: For robust state management, accounts, and instance tracking.
    *   `form-dns`: For internal network DNS resolution.
    *   `form-net`: Secure WireGuard-based mesh networking (Formnet).
    *   `form-vmm`: Virtual Machine Management for agent instances.
    *   `form-pack-manager`: Packaging agents from `Formfile` definitions.
*   **Deployment:**
    *   Simplified core service deployment using `docker-compose`.
    *   Network setup script (`scripts/validate-network-config.sh`) for bridge and local network preparation.
*   **Basic Agent Lifecycle (via API):**
    *   API-driven creation of agent instances using `Formfile`s.
    *   API-driven status checking of instances.
    *   Interaction with deployed agents over the Formnet network.
*   **Authentication:**
    *   ECDSA signature-based authentication for core API interactions.

### Under Construction (Nearing Production Readiness)

*   **Enhanced API Functionality:**
    *   More comprehensive instance management features via API (e.g., stop, start, delete instances with robust ownership checks).
    *   Refined API responses and error handling.
*   **Marketplace Foundations in `form-state`:**
    *   User account management features.
    *   Basic structures for agent/model registration and discovery (further development ongoing).
*   **`form-cli` Tool:**
    *   Initial version available but requires further development, testing, and alignment with current API authentication and workflows before being recommended for general use.
*   **Monitoring (Initial Stages):**
    *   `form-node-metrics` and `form-vm-metrics` components exist; integration into the main operational flow and data exposure are under development.
*   **Developer Experience:**
    *   Improved documentation for `Formfile` creation and advanced agent deployment patterns.
    *   Clearer guides for cryptographic key management and API signature generation.

### Planned (Future Work)

*   **Full Marketplace Functionality:**
    *   Complete agent/model publishing workflows for AI Creators.
    *   Monetization features (subscriptions, pay-per-use, credit systems).
    *   Discovery mechanisms for AI Consumers.
    *   User-facing dashboard/UI for marketplace interaction.
*   **`form-p2p` Integration:**
    *   Transition to a fully event-driven architecture using the `form-p2p` message queue for all inter-service communication, enhancing resilience and scalability. (Currently, some direct API calls or other queue mechanisms might be in use).
*   **Advanced Networking:**
    *   `form-bgp` integration for more complex network routing scenarios and external network peering.
    *   Simplified external access to agents/instances.
*   **Enhanced Security:**
    *   Integration with Trusted Execution Environments (TEEs) and Hardware Security Modules (HSMs).
    *   More granular access control and permissions.
*   **Scalability and Reliability:**
    *   Automated instance scaling based on demand.
    *   High-availability configurations for core services.
    *   Advanced load balancing.
*   **Broader AI/Agent Support:**
    *   Pre-packaged environments and templates for common AI frameworks.
    *   Support for a wider variety of agent types and use cases.
*   **Comprehensive Documentation Site:**
    *   A dedicated documentation website with in-depth guides, API references, tutorials, and architectural overviews.

This roadmap is subject to change as development progresses and community feedback is incorporated.

<hr>
