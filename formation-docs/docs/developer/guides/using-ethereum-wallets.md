# Using Ethereum Wallets with Formation

Formation uses Ethereum-compatible wallets for authentication and authorization. This guide explains how to use Ethereum wallets with the Formation platform for secure application deployment and management.

## Why Ethereum Wallets?

Formation leverages Ethereum wallet technology for several key reasons:

1. **Strong Security**: Cryptographic authentication using public/private key pairs
2. **Ownership Verification**: Clear chain of ownership for resources
3. **Compatibility**: Works with the broader web3 ecosystem
4. **Authorization Framework**: Granular permission management 

## Supported Wallet Types

Formation supports the following wallet types:

1. **Private Key**: Direct use of Ethereum private keys
2. **Mnemonic Phrases**: BIP39 compliant seed phrases (12 or 24 words)
3. **Keystore Files**: Encrypted JSON files containing private keys

## Setting Up Your Wallet

### Option 1: Create a New Wallet

If you don't have an Ethereum wallet, you can create one through the Formation CLI:

```bash
form kit init
```

Select "Create new wallet" when prompted. This will generate a new wallet and configure Formation to use it.

### Option 2: Import an Existing Private Key

To use an existing private key:

```bash
form kit init
```

Select "Import from Private Key" when prompted and enter your private key.

Alternatively, you can specify your private key directly in commands:

```bash
form pack build --private-key <your-private-key>
```

### Option 3: Import a Mnemonic Phrase

To use a mnemonic phrase (seed phrase):

```bash
form kit init
```

Select "Import from Mnemonic Phrase" when prompted and enter your 12 or 24 word phrase.

Alternatively, you can specify your mnemonic directly in commands:

```bash
form pack build --mnemonic "word1 word2 word3 ... word12"
```

### Option 4: Use a Keystore File

For improved security, you can use an encrypted keystore file:

```bash
form kit init
```

When prompted for keystore path, specify the location where you want to store your encrypted keys.

To use an existing keystore:

```bash
form pack build --keyfile /path/to/keystore
```

## Security Best Practices

### Protecting Your Private Keys

1. **Never share your private key or mnemonic phrase** with anyone
2. **Store backup copies securely**, preferably offline
3. **Use strong, unique passwords** for keystores
4. **Consider hardware wallets** for maximum security (future support planned)

### Command Line Security

When using private keys or mnemonics in commands:

1. **Avoid using them in commands** when possible, as they may be saved in your shell history
2. **Use keystore files instead** with encryption for better security
3. **Clear your command history** after entering sensitive information:
   ```bash
   history -c
   ```

## Using Wallets for Formation Operations

### Configuration 

Your wallet configuration is stored in `~/.config/form/config.json` by default. You can specify a different location:

```bash
export FORMKIT=/path/to/config.json
```

### Building and Deploying

When building and deploying applications, Formation uses your wallet to sign requests:

```bash
# Using configured wallet
form pack build

# Explicitly specifying private key
form pack build --private-key <your-private-key>

# Using mnemonic
form pack build --mnemonic "your mnemonic phrase"

# Using keystore file
form pack build --keyfile /path/to/keystore
```

### Managing Instances

Instance management also requires authentication:

```bash
# Using configured wallet
form manage start --build-id <your-build-id>

# Explicitly specifying private key
form manage start --build-id <your-build-id> --private-key <your-private-key>
```

## Authorization and Ownership

### Instance Ownership

The wallet used to create an instance becomes its owner. Owners have full control over their instances, including:

- Starting and stopping the instance
- Modifying configuration
- Deleting the instance
- Granting access to other users

### Transferring Ownership

You can transfer instance ownership to another Ethereum address:

```bash
form manage transfer-ownership --build-id <your-build-id> --to <recipient-address>
```

### Granting Access

*(Coming soon)* Formation will support granting specific permissions to other users:

```bash
form manage add-authorization --build-id <your-build-id> --address <ethereum-address> --level <permission-level>
```

## Advanced Wallet Configuration

### Custom Derivation Paths

For advanced users with hierarchical deterministic wallets:

```bash
form kit init --mnemonic "your mnemonic phrase" --derivation-path "m/44'/60'/0'/0/0"
```

### Multiple Wallet Profiles

You can maintain multiple wallet configurations for different purposes:

```bash
# Create a profile
FORMKIT=~/.config/form/profile1.json form kit init

# Use a specific profile
FORMKIT=~/.config/form/profile1.json form pack build
```

## Troubleshooting

### Authentication Issues

If you're experiencing authentication issues:

1. **Verify your wallet configuration**:
   ```bash
   cat ~/.config/form/config.json
   ```

2. **Check your wallet address**:
   ```bash
   form wallet info
   ```

3. **Ensure the correct wallet is being used**:
   ```bash
   form wallet current
   ```

### Common Error Messages

#### "Unauthorized: Address not authorized"

- Ensure you're using the same wallet that created the instance
- Verify that ownership hasn't been transferred

#### "Invalid signature"

- Check that your private key or mnemonic is correct
- Verify the format of your key input

#### "Failed to load keystore"

- Confirm the keystore file path is correct
- Ensure the keystore password is correct

## Integration with Web3 Tools

Formation's Ethereum wallet compatibility means you can use it with other Web3 tools and services:

- **MetaMask**: Export your private key from MetaMask to use with Formation
- **Hardware Wallets**: Support coming in future versions
- **Web3 Frameworks**: Integration with ethers.js, web3.js libraries

## Next Steps

- Explore [Writing Effective Formfiles](./writing-formfiles.md) to deploy your applications
- Learn about [Managing Your Instances](./managing-instances.md) once deployed
- Review [Formation SDK Integration](./formation-sdk.md) for programmatic access 