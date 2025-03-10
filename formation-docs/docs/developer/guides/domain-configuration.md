---
id: domain-configuration
title: Domain Configuration Guide
sidebar_label: Domain Configuration
---

# Domain Configuration Guide

This guide explains how to use the DNS management features in Formation to create, update, and manage domain names for your instances.

## Overview

Formation's DNS management allows you to:
- Automatically receive a domain name for each VM instance (e.g., `build-123abc.fog`)
- Assign custom domain names to your instances
- Verify ownership of custom domains
- Update and remove domain records as needed

## Command Reference

### Viewing DNS Records

To list all DNS records associated with your account:

```bash
form dns list
```

### Adding a New Domain

To manually add a domain and point it to an instance:

```bash
form dns add --domain your-domain.fog --build-id BUILD_ID
```

Options:
- `--domain`: The domain name to add (required)
- `--build-id`: The build ID of the instance to associate with the domain (required)
- `--public-ip`: Manually specify a public IP (optional)
- `--formnet-ip`: Manually specify a formnet IP (optional)

### Updating an Existing Domain

To update an existing domain record:

```bash
form dns update --domain your-domain.fog --build-id NEW_BUILD_ID
```

Options:
- `--domain`: The domain name to update (required)
- `--build-id`: The new build ID to associate with the domain (optional)
- `--public-ip`: Update the public IP (optional)
- `--formnet-ip`: Update the formnet IP (optional)

### Removing a Domain

To remove a domain record:

```bash
form dns remove --domain your-domain.fog
```

This will prompt for confirmation before removing the record.

### Verifying a Custom Domain

To verify ownership of a custom domain:

```bash
form dns verify --domain your-external-domain.com
```

## Automatic Domain Provisioning

When you create a new VM instance, a domain name is automatically provisioned in the form `build-id.fog`. This domain will be ready to use as soon as the instance finishes booting.

Example of automatic provisioning:

```bash
form manage create
# Output will include information about the provisioned domain
```

## Working with Custom Domains

### Setting Up a Custom Domain

1. Register a domain name with a domain registrar
2. Add a DNS record at your registrar pointing to your Formation node:
   - For A record: Point to your node's public IP
   - For CNAME record: Point to your auto-generated Formation domain

3. Verify ownership using:
   ```bash
   form dns verify --domain your-custom-domain.com
   ```

4. Link the verified domain to your instance:
   ```bash
   form dns add --domain your-custom-domain.com --build-id BUILD_ID
   ```

### Domain Verification Process

When verifying a custom domain, the system:
1. Checks if your domain's DNS records point to your Formation node
2. Confirms you control the domain by verifying the DNS records match
3. Once verified, allows you to use the domain in the Formation network

## Best Practices

- Use descriptive domain names that help identify your instance's purpose
- Review your DNS records periodically for security and accuracy
- Back up your domain configurations if you have complex setups
- Use separate subdomains for different services running on the same instance

## Limitations

- Currently, wildcard certificates are not fully supported
- Domain propagation time depends on external DNS providers (typically 1-24 hours)
- Maximum domain name length is 253 characters

## Troubleshooting

### Common Issues

#### Domain Not Resolving

1. Check if the domain was added successfully:
   ```bash
   form dns list | grep your-domain.fog
   ```
2. Verify the instance is running:
   ```bash
   form manage list
   ```
3. Ensure DNS propagation has completed (may take time)

#### Verification Failed

1. Ensure your DNS records point to the correct Formation node IP
2. Wait for DNS propagation (typically 1-24 hours)
3. Check for typos in domain names or record values
4. Try verification again after confirming DNS settings

#### Cannot Update Domain

1. Ensure you have permission to modify the domain
2. Check if the domain exists in your account
3. Verify the build ID or instance exists

## Getting Help

If you encounter issues not addressed in this guide:
1. Check the logs: `form-dns.log` and `form-rplb.log`
2. Review error messages from the CLI commands
3. Reach out to the Formation community for support
