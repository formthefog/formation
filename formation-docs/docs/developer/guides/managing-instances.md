# Managing Your Formation Instances

After deploying your application to the Formation cloud, you'll need to manage its lifecycle. This guide covers essential tasks for managing your Formation instances, including starting, stopping, monitoring, and troubleshooting deployments.

## Instance Lifecycle

Formation instances follow a defined lifecycle:

1. **Building**: Your Formfile is processed and your instance is being built
2. **Deploying**: The built image is being deployed to the network
3. **Running**: Your instance is actively running
4. **Stopped**: Your instance is temporarily halted but still exists
5. **Deleted**: Your instance has been permanently removed

## Basic Instance Management

### Listing Your Instances

To see all your deployed instances:

```bash
form manage list
```

This will show you a list of all your instances along with their build IDs, status, and other relevant information.

### Getting Instance Details

To get detailed information about a specific instance:

```bash
form manage info --build-id <your-build-id>
```

This provides comprehensive information about your instance, including its current status, resource usage, IP addresses, and more.

### Starting an Instance

If you've stopped an instance and want to start it again:

```bash
form manage start --build-id <your-build-id>
```

### Stopping an Instance

To temporarily stop a running instance:

```bash
form manage stop --build-id <your-build-id>
```

By default, this performs a graceful shutdown. If you need to force stop an unresponsive instance, add the `--force` flag:

```bash
form manage stop --build-id <your-build-id> --force
```

### Restarting an Instance

To restart a running instance:

```bash
form manage restart --build-id <your-build-id>
```

### Deleting an Instance

To permanently remove an instance when you no longer need it:

```bash
form manage delete --build-id <your-build-id>
```

**Warning**: This operation is irreversible. All data associated with the instance will be permanently deleted.

## Connecting to Your Instances

### Getting Instance IP Addresses

Formation instances are deployed with redundancy across multiple nodes. To get the IP addresses of all instances for a specific build:

```bash
form manage get-ip --build-id <your-build-id>
```

### SSH Access

If you configured SSH access in your Formfile, you can connect to your instance using:

```bash
ssh <username>@<formnet-ip>
```

Where `<formnet-ip>` is one of the IP addresses returned by the `get-ip` command.

### Port Forwarding

To access services running on your instance through SSH tunneling:

```bash
ssh -L 8080:localhost:80 <username>@<formnet-ip>
```

This example forwards your local port 8080 to port 80 on the instance, allowing you to access the service at `http://localhost:8080`.

## Monitoring Instances

### Checking Instance Status

Get the current status of your instance:

```bash
form pack status --build-id <your-build-id>
```

### Viewing Logs

Formation doesn't currently have a built-in log viewing command, but you can access logs by:

1. SSH into your instance
2. View appropriate log files (e.g., `/var/log/syslog`, application-specific logs)

```bash
ssh <username>@<formnet-ip> 'cat /var/log/syslog'
```

### Monitoring Resource Usage

For real-time resource monitoring, SSH into your instance and use tools like `top`, `htop`, or `dstat`:

```bash
ssh <username>@<formnet-ip> 'htop'
```

## Advanced Instance Management

### Adding Resources

Currently, modifying instance resources after deployment isn't directly supported. To change resources:

1. Update your Formfile with new resource specifications
2. Rebuild and redeploy your application
3. Migrate your data to the new instance

### Committing Changes

To save the current state of your instance as a new image:

```bash
form manage commit --build-id <your-build-id> --name <new-image-name>
```

This is useful for capturing configuration changes made after deployment.

### Transferring Instance Ownership

To transfer ownership of an instance to another account:

```bash
form manage transfer-ownership --build-id <your-build-id> --to <recipient-address>
```

## Domain Management

### Adding a Domain to Your Instance

To associate a domain with your instance:

```bash
form dns add --domain <domain-name> --build-id <your-build-id>
```

### Requesting a Vanity Domain

For testing purposes, you can request a vanity subdomain on the Formation cloud:

```bash
form dns vanity --name <subdomain-name> --build-id <your-build-id>
```

This will create a domain like `<subdomain-name>.formation.cloud` pointing to your instance.

### Removing a Domain

To remove a domain association:

```bash
form dns remove --domain <domain-name> --build-id <your-build-id>
```

## Troubleshooting

### Common Issues

#### Instance Build Failures

1. Check your Formfile for syntax errors
2. Verify that all referenced paths and files exist
3. Review the build logs for specific error messages

```bash
form pack status --build-id <your-build-id>
```

#### Connection Issues

If you can't connect to your instance:

1. Verify the instance is running:
   ```bash
   form pack status --build-id <your-build-id>
   ```

2. Ensure you're connected to Formnet:
   ```bash
   form manage formnet-up
   ```

3. Check that you're using the correct IP address:
   ```bash
   form manage get-ip --build-id <your-build-id>
   ```

#### Formnet Connection Issues

If you're having trouble connecting to Formnet:

1. Ensure the Formnet service is running:
   ```bash
   form manage formnet-up
   ```

2. Rejoin the network if necessary:
   ```bash
   form manage join
   ```

### Recovery Options

#### Instance Unresponsive

If your instance becomes unresponsive:

1. Try to force stop it:
   ```bash
   form manage stop --build-id <your-build-id> --force
   ```

2. Start it again:
   ```bash
   form manage start --build-id <your-build-id>
   ```

#### Data Recovery

For data recovery, if you configured persistent storage:

1. SSH into the instance if possible
2. Copy important data to a secure location
3. If SSH isn't possible, contact Formation support for assistance

## Best Practices

1. **Always use build IDs**: Store your build IDs securely for easy reference.

2. **Regular backups**: Implement a backup strategy for important data stored on your instances.

3. **Stateless design**: Design applications to be stateless where possible, storing persistent data on dedicated storage instances.

4. **Monitoring**: Implement monitoring solutions to keep track of your instance health and performance.

5. **Scaling strategy**: Plan a scaling strategy for when your application needs to grow.

## Next Steps

- Learn about [Using Ethereum Wallets](./using-ethereum-wallets.md) for authentication
- Explore [Networking in Formation](./networking.md) for advanced network configurations
- Discover [Troubleshooting](./troubleshooting.md) techniques for resolving common issues 