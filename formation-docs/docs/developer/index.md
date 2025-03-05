# Developer Documentation

Welcome to the Formation Developer Documentation. This comprehensive resource will help you build, deploy, and manage applications on the Formation decentralized cloud platform.

## Documentation Sections

### [Getting Started](./getting-started/index.md)

Start here if you're new to Formation. This section will guide you through setting up your environment, deploying your first application, and understanding the core concepts of the platform.

### [Guides](./guides/index.md)

Step-by-step instructions for accomplishing specific tasks on Formation, from writing effective Formfiles to setting up networking and optimizing resources.

### [Reference](./reference/index.md)

Comprehensive technical documentation including complete references for the Formfile syntax, CLI commands, API endpoints, and configuration options.

### [Tutorials](./tutorials/index.md)

Detailed tutorials that walk you through real-world scenarios and common use cases for deploying various types of applications on Formation.

### [Examples](./examples/index.md)

A collection of example applications and configurations to help you understand best practices and get inspiration for your own projects.

## Key Features for Developers

### Virtual Machine Deployments

Formation provides full virtual machines rather than containers, offering:

- Complete isolation for enhanced security
- Full operating system access
- Flexibility to run any software stack
- Persistent storage by default
- Guaranteed resources

### Ethereum-based Authentication

Secure your deployments with Ethereum wallet authentication:

- Use existing Ethereum wallets
- Cryptographic authentication
- Verifiable ownership
- Web3-native authorization

### Global Networking with Formnet

Connect your applications with Formation's built-in overlay network:

- Secure WireGuard-based networking
- Automatic DNS resolution
- Built-in service discovery
- Public and private networking
- Custom domain support

### Resource Flexibility

Right-size your computing resources:

- Specify exact CPU, memory, and storage requirements
- Pay only for what you use
- Scale resources up or down as needed
- Access specialized hardware like GPUs

### Web3-Native Platform

Built for the decentralized web:

- Tokenized compute resources
- Decentralized governance
- Community-owned infrastructure
- Transparent pricing and execution

## Getting Started

If you're new to Formation, here's how to get started:

1. **Install the Formation CLI:**
   ```bash
   curl https://dev.formation.cloud/install/form/install.sh | sudo bash
   ```

2. **Initialize your environment:**
   ```bash
   form kit init
   ```

3. **Create a new project from a template:**
   ```bash
   form kit templates create --template web-server --name my-first-app
   ```

4. **Deploy your application:**
   ```bash
   cd my-first-app
   form deploy
   ```

For a more detailed walkthrough, visit the [Getting Started](./getting-started/index.md) section.

## Popular Guides

- [Writing Formfiles](./guides/writing-formfiles.md)
- [Managing Instances](./guides/managing-instances.md)
- [Using Ethereum Wallets](./guides/using-ethereum-wallets.md)
- [Networking](./guides/networking.md)
- [Troubleshooting](./guides/troubleshooting.md)

## Popular References

- [Formfile Reference](./reference/formfile-reference.md)
- [CLI Reference](./reference/cli-reference.md)
- [API & SDK Reference](./reference/api-sdk-reference.md)

## Use Cases

Formation is ideal for a wide range of applications:

- **Web Applications**: Deploy full-stack web applications with persistence
- **API Servers**: Run backend services with guaranteed resources
- **Databases**: Deploy database servers with persistent storage
- **Blockchain Nodes**: Run validators and blockchain infrastructure
- **AI/ML Workloads**: Leverage GPU resources for machine learning tasks
- **Development Environments**: Create reproducible development environments
- **Microservices**: Deploy and connect distributed services

## Getting Help

If you need assistance with Formation:

- Check the [Troubleshooting](./guides/troubleshooting.md) guide
- Search or ask questions in the [Formation Forums](https://forum.formation.cloud)
- Join the [Formation Discord](https://discord.gg/formation) community
- Contact support at support@formation.cloud

We're continuously improving our documentation based on developer feedback. If you have suggestions for improvements, please let us know! 