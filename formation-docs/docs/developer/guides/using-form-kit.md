# Using Form-kit

Form-kit is a simplified interface for working with the Formation cloud, designed especially for beginners and those who prefer a streamlined experience. This guide explains how to get started with Form-kit and use it to deploy and manage applications on Formation.

## What is Form-kit?

Form-kit is a component of the Formation CLI that provides:

- Simplified authentication setup
- Guided workflows for common tasks
- Automated network configuration
- Pre-configured templates and examples

Form-kit is an excellent starting point for new Formation developers or teams looking to simplify their workflow.

## Getting Started with Form-kit

### Installation

Form-kit is included with the Formation CLI. To install:

```bash
curl https://dev.formation.cloud/install/form/install.sh | sudo bash
```

### Initialization

Initialize Form-kit with:

```bash
sudo form kit init
```

This interactive wizard will:

1. Guide you through creating or importing a wallet
2. Configure your keystore location
3. Set up provider settings
4. Configure Formnet (Formation's P2P network) participation

The wizard saves your configuration in `~/.config/form/config.json` by default.

## Core Form-kit Features

### Wallet Management

Form-kit simplifies wallet management by handling key generation, storage, and encryption:

```bash
# View your current wallet information
form wallet info

# Generate a new wallet
form wallet generate

# Import an existing wallet
form wallet import --private-key <your-private-key>
# or
form wallet import --mnemonic "your mnemonic phrase"
```

### Network Management

Form-kit handles Formnet connectivity:

```bash
# Join the Formation cloud
form manage join

# Start/maintain Formnet connectivity
form manage formnet-up

# Check network status
form manage formnet-status
```

## Working with Templates

### Listing Available Templates

```bash
form kit templates list
```

### Creating a Project from a Template

```bash
form kit templates create --template web-server --name my-web-app
```

This creates a new project directory with a pre-configured Formfile and any necessary application files.

## Development Workflow with Form-kit

### 1. Create a New Project

Create a new project directory and initialize it:

```bash
mkdir my-project
cd my-project
form kit project init
```

The wizard will guide you through creating a basic Formfile and project structure.

### 2. Customize Your Project

Edit the generated Formfile and add your application code to the project directory.

### 3. Build and Deploy

Form-kit simplifies the build and deployment process:

```bash
# Build your application
form pack build

# Deploy your application
form pack ship
```

### 4. Manage Your Deployment

Manage your deployed instance:

```bash
# Get instance IP address
form manage get-ip --build-id <your-build-id>

# Check instance status
form pack status --build-id <your-build-id>
```

## Form-kit vs. Direct Ethereum Integration

Form-kit provides a simplified workflow compared to directly using Ethereum wallets:

| Feature | Form-kit | Direct Ethereum Integration |
|---------|----------|----------------------------|
| Setup complexity | Simple guided wizard | Requires wallet management knowledge |
| Authentication | Automated key handling | Manual key management |
| Learning curve | Gentle, ideal for beginners | Steeper, better for Web3 developers |
| Flexibility | Streamlined but more limited | Maximum control and options |
| Security | Good with reasonable defaults | Depends on user's security practices |

## Form-kit Configuration

Your Form-kit configuration is stored in `~/.config/form/config.json` and includes:

- Wallet information
- Network settings
- Default parameters

You can edit this file directly or use the CLI to update settings:

```bash
# View current configuration
form kit config show

# Update a configuration value
form kit config set --key "network.provider" --value "new-provider-url"
```

## Security Considerations

Form-kit provides sensible security defaults, but consider these best practices:

1. **Keystore encryption**: Always use encryption when storing your keystore
2. **Regular backups**: Keep secure backups of your configuration
3. **Password management**: Use a strong, unique password for keystore encryption
4. **Environment separation**: Use different wallets for development and production

## Templates and Examples

Form-kit comes with several templates to help you get started quickly:

### Basic Web Server

```bash
form kit templates create --template web-server --name my-web-app
```

Creates a simple NGINX-based web server.

### Node.js Application

```bash
form kit templates create --template nodejs --name my-node-app
```

Sets up a Node.js application with Express.

### Database Server

```bash
form kit templates create --template database --name my-database
```

Configures a PostgreSQL database server.

## Troubleshooting Form-kit

### Common Issues

#### Initialization Failures

If Form-kit initialization fails:

```bash
# Try again with debug output
sudo form kit init --debug
```

#### Network Connection Issues

If you're having trouble connecting to Formnet:

```bash
# Ensure Formnet is running
form manage formnet-up

# Restart Formnet connection
form manage formnet-restart
```

#### Configuration Problems

For configuration issues:

```bash
# Reset Form-kit configuration
rm ~/.config/form/config.json
form kit init
```

## Extending Form-kit

While Form-kit provides a simplified interface, you can still access all the power of Formation:

1. **Combine with advanced commands**: Use Form-kit for setup, then use direct commands for advanced operations

2. **Custom templates**: Create your own templates for frequently used configurations:
   ```bash
   form kit templates save --name my-template --directory ./my-config
   ```

3. **Scripting**: Use Form-kit commands in shell scripts for automation

## Next Steps

- Learn about [Writing Effective Formfiles](./writing-formfiles.md) to customize your deployments
- Explore [Managing Your Instances](./managing-instances.md) once deployed
- Consider [Using Ethereum Wallets](./using-ethereum-wallets.md) for direct wallet integration when you're ready for more advanced options 