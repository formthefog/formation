# Formation

A public verifiable and self-replicating protocol for trustless, confidential virtual private servers (VPS) coordinating as a Fog Compute network to power the Age of Autonomy.

---

## Table of Contents

- [Overview](#overview)
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

## Overview

#### A public verfiable and self-replicating protocol for trustless confidential virtual private servers (VPS), coordinating as a Fog Compute network to power the Age of Autonomy.

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

- **Operating System:** Ubuntu 22.04
- **Resources:** Minimum of 32 physical cores, 64 GB of RAM, and at least 8TB of storage (full devnet participation).

### System Dependencies

Install required packages:
```bash
sudo apt update
sudo apt install -y build-essential bridge-utils kmod pkg-config libssl-dev libudev-dev protobuf-compiler libsqlite3-dev
   ```

   - `build-essential`: Required for compiling code.
   - `bridge-utils`: For setting up the network bridge.
   - `kmod`: For managing kernel modules.
   - `pkg-config`: For finding library dependencies.
   - `libssl-dev`: SSL library for cryptography.
   - `libudev-dev`: Library required by HIDAPI.
   - `protobuf-compiler`: Protocol Buffers compiler.
   - `libsqlite3-dev`: Required for SQLite support.

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

Install Docker and ensure it is running. Refer to [Docker's Ubuntu installation guide.](https://docs.docker.com/engine/install/ubuntu/).

---
## Network Configuration
### Configuring Your Local Network
Before launching Formation, ensure that the necessary ports are open/forwarded:

**3002:** `form-vmm` API

**3003:** `form-pack-manager` API

**51820:** `formnet` interface

**3004:** for `form-state` API (BFT-CRDT Globally Replicated Datastore)

**53333** for `form-p2p` BFT-CRDT Message queue.

Search for router/ISP-specific instructions on port forwarding if needed.

### Setting Up a Bridge Interface

Formation requires a primary bridge interface named `br0`. If not already set up, follow these steps:


Ensure that you have a `br0` primary bridge interface set up on you machine with an IP address
on a valid private IP range with a default route. You will also need to setup NAT.

By running this script, assuming you do not have significant amounts of customized networking on the 
machine you are running Formation on, you should get a properly configured network and be able to move forward.

If you do not follow these steps, it is very likely that VPS instances launched by Formation
will not have internet access, will not be able to join formnet, and will not be able to be accessed by 
the developer that owns and manages the instances.

If you do have significant amounts of custom networking set up on the machine you
are running formation on, or simply would rather manually set up the bridge network
here is how:

1. Find an available IP range within the `192.168.x.x` range or the `172.16.x.x` range 
(do not use the `10.x.x.x` range as formnet uses this range for the P2P Mesh Network).

2. Install bridge-utils

```bash
sudo apt-get update 
sudo apt-get upgrade

sudo apt-get install bridge-utils
```

3. create the bridge network

```bash
sudo brctl addbr br0
```

4. give it an IP address within the range you selected assuming you selected `192.168.100.0/24` 

```bash
sudo ip addr add 192.168.100.1/24
```

5. set the bridge to `up`

```bash
sudo ip link set br0 up
```

running `ip addr show br0` may still show that br0 is `DOWN`, however, this is because
bridges stay `DOWN` until something is attached to them. This is expected

6. ensure ip forwarding iss enabled

```bash
sudo sysctl -w net.ipv4.ip_forward=1 >/dev/null
```

7. add a nat `POSTROUTING` rule to `iptables` 

```bash
sudo iptables -t nat -A POSTROUTING -s 192.168.100.1/24
```

8. install dnsmasq

```bash
sudo apt-get update
sudo apt-get upgrade

sudo apt-get install dnsmasq
```

Beware, this may throw an error, as you likely have a DNS resolver running already
do not fret, this is expected, just move on.

9. setup dnsmasq

```bash
sudo mkdir -p /etc/dnsmasq.d
sudo cat > /etc/dnsmasq.d/br0.conf <<EOF
interface=br0
port=0
dhcp-range=192.168.100.10,192.168.100.250,24h
dhcp-option=6,8.8.8.8,8.8.4.4,1.1.1.1
EOF
```

Replace the range with your selected range.

10. restart dnsmasq

```bash
sudo systemctl restart dnsmasq
```

11. OPTIONAL test in a local network namespace

```bash
sudo ip netns add testns
sudo ip link add veth-host type veth peer name veth-ns
sudo ip link set veth-host master br0
sudo ip link set veth-host up
sudo ip link set veth-ns netns testns
sudo ip netns exec testns ip addr add 192.168.100.5
sudo ip netns exec testns ip link set veth-ns up
sudo ip netns exec testns ip link set lo up
sudo ip netns exec testns ip route add default via 192.168.100.0
sudo ip netns exec testns ping -c 3 -W 8.8.8.8
sudo ip netns del testns
```

<hr>

There are a few different ways that you can run a Formation node and participate in the network. For full documentation see our [Official Docs](docs.formation.cloud), and navigate to the **Operators** section.

The easiest way to get started is to simply run our Docker image in privileged mode with --network=host.

First, you will need to build an Operator config file, from within the formation repo run:

```bash
cargo build --release --bin form-config-wizard

form-config-wizard 
```

This will walk you through a wizard, which will ask a series of questions. If you a running a single node developer network
or joining the official devnet using the official docker images, you will likely want to select all of the defaults.

Currently, ports related to service apis are hardcoded in many places throughout the system components, and as such,
we as you for the time being to only use the default ports, unless you are actively contributing to the project
and making changes to this default.

The reason for this, for now, is that different identification information does not currently include service ports for external facing services,
and in many cases, right now, due to the age of the project, we haven't gotten around to making internal service API ports configurable.
if you would like to contribute to this, please see [#16](https://github.com/formthefog/formation/issues/16)

Second, you will need to pull the official images:

```bash
# Pull formation-minimal 

# First you will need to pull the 
docker pull cryptonomikhan/formation-minimal:v0.1.0

# Retag it
docker tag cryptonomikhan/formation-minimal:v0.1.0 formation-minimal

# Pull form-build-server
docker pull cryptonomikhan/form-build-server:v0.1.0

# Retag it
docker tag cryptonomikhan/form-build-server:v0.1.0 form-build-server
```

Then you can run it, ensure you use the `--privileged` flag `--network=host` and
provide it with the necessary devices and volumes (`/lib/modules` & `/var/run/docker.sock`)

The **Formation** docker image requires that it be run in *privileged* mode, and while privileged mode is outside the scope of this particular document, we highly suggest you take the time to understand the implications of such. 

It also requires that you provide the `kvm` device and other devices, as it 
needs access to host hypervisor and devices to run virtual machines, some of 
these are likely able to be left out, but as best practice, and for the sake 
of thoroughness and the reduction of operator headaches, we suggest that you do 
provide the devices, as well as the hosts kernel modules to the container, 
as the Formation Virtual Machine Manager, Monitors and Hypervisor 
relies on KVM & other devices under the hood. 

Lastly, for now, we highly suggest your run it with the host network. 
The way that Formation provisions developers secure, confidential access 
to their instances is over a private VPN tunnel mesh network that runs 
wireguard under the hood. Configuring public access to the mesh network 
over the docker bridge network is still experimental, and you are likely to 
run into some headaches as a result. If you're looking to contribute to the 
project, and have expertise in container networking, linux networking, and 
would like to help make this process simpler so that the Formation node 
image can run without the host network access, please see the 
**Contributing to Formation** section above.

Running the image as described above will bootstrap you into an unofficial developer network. To join the official devnet please join our discord, navigate to the **Operators channel** and reach out to the core team there for more information on how to participate. 

### Run Single Node Local Test
<hr>

For a single node local test, we provide the `formation-minimal` docker image.

`formation-minimal`, unlike the `formation` image, does not provide the form-p2p
service, and therefore is not able to connect to the broader network. Further, `formation-minimal` does not register the node running it with the orchestration smart contract, and is therefore unable to be bootstrapped into the network, or verify the workloads it is responsible for. 

Nonetheless, it is valuable, as an operator or developer, to run locally for testing purposes, and makes the process of contributing to the protocol easier, faster, and more convenient. It also provides a minimal test environment for app developers planning to deploy to the broader network, and allows them to test their applications in a more production like scenario before deploying.

Similar to the formation image, you do need to provide `formation-minimal` with `kvm`, it does need to run in privilege mode, it still needs access to the docker socket, however, given that it is designed for a local test network it may not be necessary to run it on the host network, unless you plan on attempting to access instances or applications from a different machine or network.

To run a single local formation node, run the following command:

```bash
docker run --rm --privileged --network=host \
    --device=/dev/kvm \
    --device=/dev/vhost-net \
    --device=/dev/null \
    --device=/dev/zero \
    --device=/dev/random \
    --device=/dev/urandom \
    -v /lib/modules:/lib/modules:ro \
    -v /var/run/docker.sock:/var/run/docker.sock \
    -v /path/to/operator/config \ 
    -e SECRET_PATH=/path/to/config \
    -e PASSWORD=<your-encryption-password> \
    --mount type=tmpfs,destination=/dev/hugepages,tmpfs-mode=1770 \
    -dit formation-minimal

```

<hr>

### Run Multinode Local Test 

<hr>

To run multiple local nodes, effectively building a localized test network, you will want to run the complete `formation` image, but you will have to do so on the docker bridge network or a custom docker network, which means that in order to gain access to instances, applications and nodes on the network from outside the local machine & network, you will need to set up port forwarding on the router, and customized rules in the local machines networking configuration.

<hr>

##### Join Official Developer Network

<hr>

To join the official developer network, we suggest you avoid using the docker images, and instead run the full suite of services on a clean ubuntu 22.04 installation, on hardware with a bare minimum of 32 physical cores, 64 GB of RAM, and at least 8TB of storage, though much more storage is preferred in order to enable larger workloads and applications with significantly larger data requirements. 

To participate as an official developer network node, join our [discord]() navigate
to the operator channel, and let our core team know you would like to participate as a devnet node operator. The devnet operator cohort is small, and is compensated, however there is a significant 
time commitment as the protocol will be iterated on, stopped and restarted frequently, and there will be coordinated live test scenarios that devnet operators will be asked to participate in,
all in an effort to ensure the network is hardened and ready for a more public and open testnet phase as the protocol will be iterated on, stopped and restarted frequently, and there will be coordinated live test scenarios that devnet operators will be asked to participate in,
all in an effort to ensure the network is hardened and ready for a more public and open testnet phase..

<hr>

# Formation Development Guide

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
