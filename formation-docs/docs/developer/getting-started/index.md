# Getting Started as a Formation Developer

Welcome to the Formation Developer documentation. This guide will help you start building and deploying applications on the Formation cloud.

## What is Formation for Developers?

Formation provides a global, decentralized network of confidential virtual private servers (VPS) where developers can deploy applications with strong privacy and security guarantees. The network's fog computing architecture enables distributed workloads that can scale dynamically while maintaining data privacy.

## Prerequisites

Before you begin developing on Formation, ensure you have:

1. **Development Environment**:
   - Git
   - A code editor of your choice
   - Familiarity with Linux-based environments

2. **Authentication Methods**:
   - For full access: An Ethereum-compatible wallet with private key or mnemonic
   - For simplified access: Our Form-kit SDK (recommended for beginners)

## Setting Up Your Development Environment

### 1. Install the Formation CLI

The Formation CLI (`form`) is the primary tool for managing your deployments on the network:

```bash
curl https://dev.formation.cloud/install/form/install.sh | sudo bash
```

### 2. Initialize Your Development Environment

Set up your development environment with the CLI:

```bash
sudo form kit init
```

This launches an interactive wizard that will:
- Create or import a wallet for signing requests
- Set up your keystore location and configuration
- Configure your provider settings
- Set up your Formnet participation preferences

The wizard saves your configuration in `~/.config/form/config.json` by default.

### 3. Join Formnet

Formnet is Formation's peer-to-peer network that enables secure communication with your instances:

```bash
sudo form manage join
sudo form manage formnet-up
```

The `formnet-up` command starts a background process that maintains your peer connections. This must be running to access your instances.

## Development Options

There are two primary ways to interact with the Formation cloud as a developer:

### Option 1: Form-kit (Recommended for Beginners)

Form-kit is our SDK that provides a simplified interface for deploying and managing workloads on Formation. Using Form-kit, you can:

- Use a simplified authentication mechanism (no need to manage Ethereum keys directly)
- Deploy pre-configured templates and applications
- Manage your instances through a streamlined API

[Learn more about using Form-kit](../guides/using-form-kit.md)

### Option 2: Direct Ethereum Wallet Integration

For more advanced users or those requiring complete control, you can use your Ethereum wallet directly:

- Sign transactions with your Ethereum private key or mnemonic phrase
- Fine-grained control over all aspects of your deployments
- Integration with existing Ethereum-based systems and workflows

[Learn more about using Ethereum wallets](../guides/using-ethereum-wallets.md)

## Core Development Workflow

### 1. Create Your Formfile

Every Formation project needs a `Formfile` in its root directory. The Formfile defines your instance configuration and build process:

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

[Learn more about writing Formfiles](../guides/writing-formfiles.md)

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

### 4. Access Your Instance

Formation automatically creates redundant instances for reliability. Get their addresses with:

```bash
form manage get-ip --build-id <your-build-id>
```

Once you have an IP, access your instance via SSH:

```bash
ssh <username>@<formnet-ip>
```

## CLI vs UI Options

Formation offers two interfaces for developers:

### Command Line Interface (CLI)

The `form` CLI provides complete access to all Formation features and is ideal for:
- CI/CD integration
- Script automation
- Power users who prefer terminal-based workflows

### Web UI (Coming Soon)

Our web-based UI will provide a graphical interface for:
- Visual management of instances
- Monitoring and analytics
- Simplified deployment workflows for teams

## API Integration

For developers who need to integrate Formation directly into their applications, we provide comprehensive APIs:

- [VMM Service API](../../api/vmm/index.md): Manage virtual machines
- [State Service API](../../api/state/index.md): Access global state
- [P2P Service API](../../api/p2p/index.md): For messaging and events
- [DNS Service API](../../api/dns/index.md): Domain management
- [Formnet API](../../api/formnet/index.md): Network management

These APIs offer programmatic access to all Formation services and can be used to build custom tooling or integrations.

## Next Steps

- [Writing Effective Formfiles](../guides/writing-formfiles.md)
- [Managing Your Instances](../guides/managing-instances.md)
- [Networking in Formation](../guides/networking.md)
- [Debugging and Troubleshooting](../guides/troubleshooting.md)
- [Using the Inference Engine](../../inference-engine/index.md) 