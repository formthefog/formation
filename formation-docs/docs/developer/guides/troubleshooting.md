# Troubleshooting Formation

This guide provides solutions for common issues you might encounter when working with Formation. Whether you're having trouble with building, deploying, or managing instances, this guide will help you diagnose and resolve problems.

## Build Issues

### Formfile Syntax Errors

**Symptoms**: Build fails with error messages related to Formfile parsing.

**Solutions**:

1. Validate your Formfile:
   ```bash
   form pack validate
   ```

2. Check for common syntax issues:
   - Missing or incorrect instruction keywords
   - Improper quoting of values with spaces
   - Invalid parameters for instructions

3. Review the [Formfile reference](../reference/formfile-reference.md) for correct syntax

### Missing Files or Context Errors

**Symptoms**: Build fails with errors about missing files or incorrect paths.

**Solutions**:

1. Verify that all paths in `COPY` instructions are relative to your build context
2. Check file permissions to ensure Formation can access your files
3. Use `form pack dry-run` to see which files would be included in the build

### Dependency Installation Failures

**Symptoms**: Build fails during package installation.

**Solutions**:

1. Verify internet connectivity for package downloading
2. Check for compatibility between packages
3. Try specifying exact package versions
4. Review build logs for specific error messages:
   ```bash
   form pack status --build-id <your-build-id>
   ```

### Build Timeout

**Symptoms**: Build fails with timeout error or seems to hang indefinitely.

**Solutions**:

1. Check resource-intensive build steps (e.g., large file copying, complex compilations)
2. Optimize your build by:
   - Breaking down complex `RUN` commands
   - Using smaller file sets in `COPY` instructions
   - Implementing multi-stage builds

## Deployment Issues

### Deployment Failures

**Symptoms**: `form pack ship` command fails or instance never becomes available.

**Solutions**:

1. Verify build was successful:
   ```bash
   form pack status --build-id <your-build-id>
   ```

2. Check Formnet connectivity:
   ```bash
   form manage formnet-up
   ```

3. Inspect deployment logs:
   ```bash
   form pack status --build-id <your-build-id> --verbose
   ```

### Instance Not Accessible

**Symptoms**: Instance is reported as running but you can't connect to it.

**Solutions**:

1. Verify instance is running:
   ```bash
   form pack status --build-id <your-build-id>
   ```

2. Check IP addresses:
   ```bash
   form manage get-ip --build-id <your-build-id>
   ```

3. Verify network connectivity:
   ```bash
   ping <instance-ip>
   ```

4. Check if ports are exposed properly in your Formfile

### Resources Unavailable

**Symptoms**: Deployment fails with errors about insufficient resources.

**Solutions**:

1. Reduce resource requirements in your Formfile (VCPU, MEM, DISK)
2. Verify operator nodes have sufficient capacity
3. Check that your allocation limits haven't been exceeded

## Authentication Issues

### Wallet Authentication Failures

**Symptoms**: Commands fail with authentication or signature errors.

**Solutions**:

1. Verify your wallet configuration:
   ```bash
   form wallet info
   ```

2. Reinitialize your configuration:
   ```bash
   form kit init
   ```

3. Try explicitly specifying credentials:
   ```bash
   form pack build --private-key <your-private-key>
   ```

### Permission Errors

**Symptoms**: Commands fail with access denied or insufficient permissions.

**Solutions**:

1. Verify you're using the wallet that owns the instance
2. Check instance ownership:
   ```bash
   form manage info --build-id <your-build-id>
   ```

3. If accessing someone else's instance, request proper authorization

## Formnet Connectivity Issues

### Can't Join Formnet

**Symptoms**: `form manage join` fails or times out.

**Solutions**:

1. Check your internet connection
2. Verify network ports aren't blocked (especially UDP 51820)
3. Try again with verbose logging:
   ```bash
   form manage join --verbose
   ```

### Unstable Formnet Connection

**Symptoms**: Intermittent connectivity or connection drops.

**Solutions**:

1. Restart Formnet:
   ```bash
   form manage formnet-up
   ```

2. Check for network issues:
   ```bash
   ip a show wg0
   ```

3. Verify WireGuard configuration:
   ```bash
   sudo wg show
   ```

## Instance Runtime Issues

### Instance Crashes or Stops

**Symptoms**: Instance repeatedly crashes or stops unexpectedly.

**Solutions**:

1. Check instance status:
   ```bash
   form pack status --build-id <your-build-id>
   ```

2. Review application logs through SSH:
   ```bash
   ssh <username>@<formnet-ip> 'cat /var/log/syslog'
   ```

3. Check resource usage - instance might be running out of memory or CPU

### Service Not Starting

**Symptoms**: Instance is running but application service doesn't start.

**Solutions**:

1. SSH into instance and check service status:
   ```bash
   ssh <username>@<formnet-ip> 'systemctl status <service-name>'
   ```

2. Review service logs:
   ```bash
   ssh <username>@<formnet-ip> 'journalctl -u <service-name>'
   ```

3. Verify that the `ENTRYPOINT` in your Formfile is correct

### Configuration Issues

**Symptoms**: Application starts but behaves incorrectly.

**Solutions**:

1. SSH into instance and check configuration files
2. Verify environment variables:
   ```bash
   ssh <username>@<formnet-ip> 'env | grep MY_VAR'
   ```

3. Check file permissions for configuration files

## Updating and Redeploying

### Can't Update Instance

**Symptoms**: Need to update running instance with new code or configuration.

**Solutions**:

1. For small changes, SSH in and modify files directly
2. For significant changes:
   - Update your Formfile
   - Rebuild with `form pack build`
   - Deploy with `form pack ship`
   - Transfer data or configuration from old instance if needed

### Lost Build ID

**Symptoms**: Can't manage instance because build ID is lost.

**Solutions**:

1. List your instances:
   ```bash
   form manage list
   ```

2. Look for the instance by name or other identifying information
3. In the future, store build IDs securely after deployment

## DNS and Domain Issues

### Domain Not Resolving

**Symptoms**: Custom or vanity domain doesn't point to your instance.

**Solutions**:

1. Verify domain was properly added:
   ```bash
   form dns list
   ```

2. Add or update domain:
   ```bash
   form dns add --domain <domain-name> --build-id <your-build-id>
   ```

3. Check DNS propagation (may take time)

### Cannot Access Via Hostname

**Symptoms**: Can access instance via IP but not hostname.

**Solutions**:

1. Verify your local DNS resolution
2. Check Formnet DNS configuration
3. Try adding the hostname to your `/etc/hosts` file

## CLI and Tool Issues

### CLI Command Fails

**Symptoms**: Formation CLI command exits with error.

**Solutions**:

1. Check command syntax and arguments
2. Run with verbose flag for more information:
   ```bash
   form <command> --verbose
   ```

3. Verify CLI is up to date:
   ```bash
   form --version
   ```

### Missing or Outdated Dependencies

**Symptoms**: CLI fails with errors about missing dependencies.

**Solutions**:

1. Reinstall Formation CLI:
   ```bash
   curl https://dev.formation.cloud/install/form/install.sh | sudo bash
   ```

2. Update system packages:
   ```bash
   sudo apt update && sudo apt upgrade
   ```

## Gathering Information for Support

If you're unable to resolve an issue, collect the following information to help Formation support assist you:

1. **Build ID and logs**:
   ```bash
   form pack status --build-id <your-build-id> --verbose
   ```

2. **Configuration**:
   ```bash
   form kit config show
   ```

3. **Network status**:
   ```bash
   form manage formnet-status
   ip a show wg0
   ```

4. **System information**:
   ```bash
   uname -a
   form --version
   ```

## Getting Help

If you've tried the troubleshooting steps and still have issues:

1. **Join Discord**: Visit our [Discord server](https://discord.gg/formation) and ask in the #chewing-glass channel
2. **GitHub Issues**: Submit an issue on our [GitHub repository](https://github.com/formthefog/formation)
3. **Contact Support**: Email support@formation.cloud

## Common Error Messages

### "Failed to build context"

This usually means there's an issue with your project files or Formfile. Verify all referenced files exist and are accessible.

### "Authentication failed"

This indicates an issue with your wallet authentication. Check your wallet configuration and make sure you're using the correct credentials.

### "No route to host"

This suggests a Formnet connectivity issue. Make sure Formnet is running with `form manage formnet-up`.

### "Insufficient resources"

Your deployment requires more resources than available. Try reducing the resource specifications in your Formfile.

### "Build timeout exceeded"

Your build is taking too long. Try to optimize your build steps or increase the timeout if available.

### "Failed to execute entrypoint command"

The ENTRYPOINT specified in your Formfile failed to run. SSH into the instance and check for specific errors in application logs. 