---
id: index
title: Developer Documentation
sidebar_label: Overview
---

# Developer Documentation

Welcome to the Formation Developer Documentation. This comprehensive resource will help you build, deploy, and maintain applications on the Formation cloud platform.

## Documentation Sections

### [Getting Started](./getting-started/index.md)

Start here if you're new to developing on Formation. This section will guide you through initial setup, basic concepts, and your first deployment.

### [Guides](./guides/index.md)

Step-by-step instructions for accomplishing specific development tasks, from writing Formfiles to managing deployments and networking.

### [Reference](./reference/index.md)

Comprehensive technical documentation including complete references for Formfile directives, CLI commands, API endpoints, and configuration options.

### [Tutorials](./tutorials/index.md)

Detailed tutorials that walk you through real-world scenarios such as deploying web applications, databases, and microservices.

### [Examples](./examples/index.md)

Ready-to-use example projects and code snippets that demonstrate best practices and common patterns.

## Key Benefits for Developers

### Global Network

Deploy your applications on a global, decentralized cloud:

- Automatic redundancy across geographic regions
- Low-latency edge deployment capabilities
- Trustless, verifiable infrastructure 
- Confidential computing for sensitive workloads

### Simplified Deployment

Streamlined developer experience:

- Formfile-based declarative deployments
- Integrated CLI for all operations
- Strong authentication and permission model
- Comprehensive monitoring and logging

### Flexible Workloads

Support for diverse application types:

- Web applications and APIs
- Databases and storage solutions
- Machine learning and AI workloads
- Long-running services and batch jobs

### Developer-Focused Tools

Tools and services designed for developers:

- Form-kit SDK for simplified development
- Comprehensive API for programmatic control
- Integration with popular CI/CD platforms
- Detailed documentation and examples

## Getting Started

If you're new to developing on Formation, here's how to get started:

1. **Install the Formation CLI:**
   ```bash
   curl https://dev.formation.cloud/install/form/install.sh | sudo bash
   ```

2. **Initialize your environment:**
   ```bash
   sudo form kit init
   ```

3. **Join Formnet:**
   ```bash
   sudo form manage join
   sudo form manage formnet-up
   ```

4. **Create your first Formfile and deploy:**
   ```bash
   # Create a Formfile
   # Deploy your application
   sudo form pack build
   form pack ship
   ```

For a more detailed walkthrough, visit the [Getting Started](./getting-started/index.md) section.

## Popular Guides

- [Writing Formfiles](./guides/writing-formfiles.md)
- [Managing Instances](./guides/managing-instances.md)
- [Networking in Formation](./guides/networking.md)
- [Using Ethereum Wallets](./guides/using-ethereum-wallets.md)
- [Troubleshooting](./guides/troubleshooting.md)

## Popular References

- [Formfile Reference](./reference/formfile-reference.md)
- [CLI Reference](./reference/cli-reference.md)
- [API Reference](./reference/api-reference.md)
- [Configuration Reference](./reference/configuration-reference.md)

## Application Types

Formation supports various application types and architectures:

- **Web Applications**: Frontend and backend web services
- **APIs and Microservices**: Distributed service architectures
- **Databases**: Relational and NoSQL databases
- **AI/ML Workloads**: Machine learning model training and inference
- **Content Delivery**: Static site hosting and media delivery
- **Background Jobs**: Scheduled and event-driven processing

## Getting Help

If you need assistance developing on Formation:

- Check the [Troubleshooting](./guides/troubleshooting.md) guide
- Search or ask questions in the [Formation Developer Forums](https://forum.formation.cloud/c/developers)
- Join the [Formation Discord](https://discord.gg/formation) developer channel
- Contact developer support at developers@formation.cloud

We're continuously improving our documentation based on developer feedback. If you have suggestions for improvements, please let us know! 