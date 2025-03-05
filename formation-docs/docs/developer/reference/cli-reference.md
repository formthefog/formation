# Formation CLI Reference

This document provides a comprehensive reference for all available commands in the Formation Command Line Interface (CLI). The Formation CLI is the primary tool for developers to interact with the Formation platform, enabling building, deploying, and managing applications.

## Global Options

These options can be used with any command:

```
--help, -h       Show help for the command
--version, -v    Display version information
--json           Output in JSON format
--debug          Enable debug logging
--config PATH    Specify an alternate config file (default: ~/.config/form/config.json)
```

## Core Commands

### form

**Description**: The base command for the Formation CLI.

**Usage**:
```
form [global options] command [command options] [arguments...]
```

**Subcommands**:
- `deploy` - Deploy an application
- `build` - Build an application without deploying
- `instance` - Manage instances
- `wallet` - Manage Ethereum wallet
- `config` - Manage configuration
- `manage` - Manage Formation cloud connection
- `kit` - Access simplified workflows (for beginners)
- `pack` - Low-level commands for building and packaging

## Deployment Commands

### form deploy

**Description**: Build and deploy an application in one step.

**Usage**:
```
form deploy [options] [path/to/formfile]
```

**Options**:
- `--name NAME` - Set the instance name (overrides NAME in Formfile)
- `--wait` - Wait for deployment to complete before returning
- `--no-cache` - Build without using cached layers
- `--file, -f PATH` - Path to Formfile (default: Formfile in current directory)
- `--output, -o FORMAT` - Output format: text or json (default: text)

**Example**:
```
form deploy --name web-server ./my-app
```

### form build

**Description**: Build an application without deploying.

**Usage**:
```
form build [options] [path/to/formfile]
```

**Options**:
- `--file, -f PATH` - Path to Formfile (default: Formfile in current directory)
- `--tag NAME:TAG` - Tag to apply to the built image
- `--no-cache` - Build without using cached layers
- `--output, -o FORMAT` - Output format: text or json (default: text)

**Example**:
```
form build --tag my-app:latest ./my-app
```

## Instance Management

### form instance list

**Description**: List all your instances.

**Usage**:
```
form instance list [options]
```

**Options**:
- `--status STATUS` - Filter by status (running, stopped, failed)
- `--output, -o FORMAT` - Output format: text, json, or wide (default: text)
- `--limit N` - Limit number of results
- `--owner ADDRESS` - Filter by Ethereum address of owner

**Example**:
```
form instance list --status running
```

### form instance get

**Description**: Get details about a specific instance.

**Usage**:
```
form instance get [options] INSTANCE_ID
```

**Options**:
- `--output, -o FORMAT` - Output format: text or json (default: text)

**Example**:
```
form instance get i-1234567890abcdef
```

### form instance start

**Description**: Start a stopped instance.

**Usage**:
```
form instance start [options] INSTANCE_ID
```

**Options**:
- `--wait` - Wait for instance to start before returning

**Example**:
```
form instance start --wait i-1234567890abcdef
```

### form instance stop

**Description**: Stop a running instance.

**Usage**:
```
form instance stop [options] INSTANCE_ID
```

**Options**:
- `--wait` - Wait for instance to stop before returning
- `--force` - Force stop the instance (similar to power off)

**Example**:
```
form instance stop i-1234567890abcdef
```

### form instance delete

**Description**: Delete an instance.

**Usage**:
```
form instance delete [options] INSTANCE_ID
```

**Options**:
- `--force` - Force deletion without confirmation
- `--wait` - Wait for deletion to complete before returning

**Example**:
```
form instance delete --force i-1234567890abcdef
```

### form instance logs

**Description**: Fetch logs from an instance.

**Usage**:
```
form instance logs [options] INSTANCE_ID
```

**Options**:
- `--follow, -f` - Follow log output
- `--tail N` - Number of lines to show from the end (default: all)
- `--since DURATION` - Show logs since duration (e.g., 10m, 1h)

**Example**:
```
form instance logs --follow --tail 100 i-1234567890abcdef
```

### form instance ssh

**Description**: Connect to an instance via SSH.

**Usage**:
```
form instance ssh [options] INSTANCE_ID [COMMAND]
```

**Options**:
- `--user, -u NAME` - Username for SSH connection
- `--identity, -i FILE` - Identity file for SSH connection
- `--port, -p PORT` - Port for SSH connection (default: 22)

**Example**:
```
form instance ssh i-1234567890abcdef
form instance ssh i-1234567890abcdef "tail -f /var/log/nginx/error.log"
```

## Wallet Management

### form wallet info

**Description**: Display information about the current wallet.

**Usage**:
```
form wallet info [options]
```

**Options**:
- `--output, -o FORMAT` - Output format: text or json (default: text)

**Example**:
```
form wallet info
```

### form wallet generate

**Description**: Generate a new Ethereum wallet.

**Usage**:
```
form wallet generate [options]
```

**Options**:
- `--output, -o FORMAT` - Output format: text or json (default: text)
- `--save` - Save the wallet to the configuration file
- `--keystore PATH` - Path to save the keystore file

**Example**:
```
form wallet generate --save
```

### form wallet import

**Description**: Import an existing Ethereum wallet.

**Usage**:
```
form wallet import [options]
```

**Options**:
- `--private-key KEY` - Private key to import
- `--mnemonic PHRASE` - Mnemonic phrase to import
- `--keystore PATH` - Path to keystore file
- `--save` - Save the wallet to the configuration file

**Example**:
```
form wallet import --mnemonic "word1 word2 ... word12" --save
```

### form wallet transfer

**Description**: Transfer ownership of an instance.

**Usage**:
```
form wallet transfer [options] INSTANCE_ID ETHEREUM_ADDRESS
```

**Options**:
- `--force` - Force transfer without confirmation

**Example**:
```
form wallet transfer i-1234567890abcdef 0x1234567890abcdef1234567890abcdef12345678
```

## Network Management

### form manage join

**Description**: Join the Formation cloud.

**Usage**:
```
form manage join [options]
```

**Options**:
- `--network NAME` - Network to join (default: mainnet)
- `--force` - Force rejoin if already connected

**Example**:
```
form manage join --network testnet
```

### form manage formnet-status

**Description**: Check the status of the Formation cloud connection.

**Usage**:
```
form manage formnet-status [options]
```

**Options**:
- `--output, -o FORMAT` - Output format: text or json (default: text)

**Example**:
```
form manage formnet-status
```

### form manage formnet-up

**Description**: Restart the Formation cloud connection.

**Usage**:
```
form manage formnet-up [options]
```

**Options**:
- `--force` - Force restart even if already connected

**Example**:
```
form manage formnet-up --force
```

## Configuration Management

### form config get

**Description**: Get configuration values.

**Usage**:
```
form config get [options] [KEY]
```

**Options**:
- `--output, -o FORMAT` - Output format: text or json (default: text)

**Example**:
```
form config get network.endpoint
```

### form config set

**Description**: Set configuration values.

**Usage**:
```
form config set [options] KEY VALUE
```

**Options**:
- `--global` - Set in global config instead of local

**Example**:
```
form config set network.endpoint https://formation.example.com
```

### form config list

**Description**: List all configuration values.

**Usage**:
```
form config list [options]
```

**Options**:
- `--output, -o FORMAT` - Output format: text or json (default: text)

**Example**:
```
form config list
```

## Form-kit Commands (Simplified Workflows)

### form kit init

**Description**: Initialize a new Formation project with guided setup.

**Usage**:
```
form kit init [options]
```

**Options**:
- `--output, -o FORMAT` - Output format: text or json (default: text)
- `--quick` - Skip confirmations and use defaults

**Example**:
```
form kit init
```

### form kit templates list

**Description**: List available project templates.

**Usage**:
```
form kit templates list [options]
```

**Options**:
- `--output, -o FORMAT` - Output format: text or json (default: text)
- `--category CATEGORY` - Filter by category

**Example**:
```
form kit templates list --category webserver
```

### form kit templates create

**Description**: Create a new project from a template.

**Usage**:
```
form kit templates create [options]
```

**Options**:
- `--template NAME` - Template to use
- `--name NAME` - Name for the new project
- `--output-dir DIR` - Directory to create the project in (default: current directory)
- `--variables KEY=VALUE` - Variables to use in the template

**Example**:
```
form kit templates create --template web-server --name my-website
```

## Low-level Pack Commands

### form pack validate

**Description**: Validate a Formfile without building.

**Usage**:
```
form pack validate [options] [path/to/formfile]
```

**Options**:
- `--file, -f PATH` - Path to Formfile (default: Formfile in current directory)

**Example**:
```
form pack validate ./my-app/Formfile
```

### form pack status

**Description**: Check the status of a build.

**Usage**:
```
form pack status [options]
```

**Options**:
- `--build-id ID` - Build ID to check
- `--output, -o FORMAT` - Output format: text or json (default: text)

**Example**:
```
form pack status --build-id b-1234567890abcdef
```

### form pack push

**Description**: Push a built application to the Formation cloud.

**Usage**:
```
form pack push [options] BUILD_ID
```

**Options**:
- `--wait` - Wait for push to complete before returning

**Example**:
```
form pack push --wait b-1234567890abcdef
```

## Domain Management

### form domain list

**Description**: List domains associated with your instances.

**Usage**:
```
form domain list [options]
```

**Options**:
- `--output, -o FORMAT` - Output format: text or json (default: text)
- `--instance-id ID` - Filter by instance ID

**Example**:
```
form domain list --instance-id i-1234567890abcdef
```

### form domain add

**Description**: Associate a domain with an instance.

**Usage**:
```
form domain add [options] DOMAIN INSTANCE_ID
```

**Options**:
- `--wait` - Wait for DNS propagation check before returning
- `--skip-verification` - Skip DNS verification

**Example**:
```
form domain add example.com i-1234567890abcdef
```

### form domain remove

**Description**: Remove a domain association.

**Usage**:
```
form domain remove [options] DOMAIN
```

**Options**:
- `--force` - Force removal without confirmation

**Example**:
```
form domain remove --force example.com
```

## Examples

### Building and Deploying an Application

```bash
# Initialize a new project from a template
form kit templates create --template web-server --name my-website

# Navigate to the project directory
cd my-website

# Build and deploy the application
form deploy --wait

# List your running instances
form instance list

# Connect to your instance via SSH
form instance ssh i-1234567890abcdef
```

### Managing Instances

```bash
# List all instances
form instance list

# Stop an instance
form instance stop i-1234567890abcdef

# Start an instance
form instance start i-1234567890abcdef

# View logs from an instance
form instance logs --follow i-1234567890abcdef

# Delete an instance
form instance delete i-1234567890abcdef
```

### Working with Wallets

```bash
# Generate a new wallet
form wallet generate --save

# Display wallet information
form wallet info

# Import an existing wallet using a mnemonic phrase
form wallet import --mnemonic "word1 word2 ... word12" --save

# Transfer instance ownership
form wallet transfer i-1234567890abcdef 0x1234567890abcdef1234567890abcdef12345678
```

### Managing Formation Cloud Connection

```bash
# Join the Formation cloud
form manage join

# Check network connection status
form manage formnet-status

# Restart network connection
form manage formnet-up
```

## Environment Variables

The following environment variables can be used to configure the Formation CLI:

- `FORM_CONFIG`: Path to the configuration file
- `FORM_DEBUG`: Enable debug logging when set to any value
- `FORM_JSON`: Output in JSON format when set to any value
- `FORM_KEYSTORE_PASSWORD`: Password for keystore file (use with caution)
- `FORM_NETWORK`: Default network to use (mainnet or testnet)
- `FORM_API_ENDPOINT`: Override the API endpoint

## Exit Codes

The Formation CLI uses the following exit codes:

- `0`: Success
- `1`: General error
- `2`: Command-line argument error
- `3`: Network error
- `4`: Authentication error
- `5`: Resource not found
- `6`: Validation error
- `7`: Permission error
- `8`: Resource already exists
- `9`: Resource in use 