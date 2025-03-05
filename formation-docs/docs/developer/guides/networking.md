# Networking in Formation

Networking is a critical aspect of deploying applications on the Formation platform. This guide explains Formation's networking architecture, how to configure network settings for your instances, and how to troubleshoot common networking issues.

## Formation Network Architecture

Formation uses a layered networking approach to provide secure, flexible connectivity:

### Formnet

Formnet is Formation's overlay network built on WireGuard. It provides:

- Encrypted connections between all nodes and instances
- Peer-to-peer connectivity
- NAT traversal
- Automatic routing configuration

### Instance Networking

Each Formation instance gets:

- A private IP address on the Formnet network
- Optional DNS hostname
- Configurable network capabilities through the Formfile

## Connecting to Formnet

### Joining Formnet

Before you can deploy or access instances, you need to join Formnet:

```bash
form manage join
```

This command:
1. Generates WireGuard keys
2. Connects to the Formation network
3. Configures your local routing table

### Maintaining Formnet Connection

To maintain your connection to Formnet:

```bash
form manage formnet-up
```

This is a long-running command that keeps your Formnet connection active by:
- Refreshing peer connections
- Updating routing tables
- Maintaining the network interface

You may want to run this in a separate terminal or as a background service.

### Checking Formnet Status

To check your Formnet connection status:

```bash
form manage formnet-status
```

## Network Configuration in Formfiles

### Exposing Ports

To make services accessible, expose ports in your Formfile:

```
NAME web-server
VCPU 2
MEM 2048
DISK 10
INSTALL nginx
EXPOSE 80 443
```

The `EXPOSE` directive makes these ports available on the network interface.

### Environment Variables

You can configure network-related environment variables:

```
ENV SERVER_HOST=0.0.0.0
ENV SERVER_PORT=3000
```

### Custom Network Commands

For more complex network configurations, use `RUN` commands:

```
RUN iptables -A INPUT -p tcp --dport 80 -j ACCEPT
```

## DNS and Hostnames

### Default Hostnames

Each instance receives a default hostname based on its build ID:

```
<build-id>.formation.local
```

For example, if your build ID is `build-123abc`, your instance's hostname will be `build-123abc.formation.local`.

### Custom Domain Names

You can associate custom domain names with your instances:

```bash
form dns add --domain myapp.example.com --build-id <your-build-id>
```

### Vanity Domains

For development and testing, you can use Formation's vanity domains:

```bash
form dns vanity --name myapp --build-id <your-build-id>
```

This creates a domain like `myapp.formation.cloud` pointing to your instance.

## Network Services

### Creating a Web Server

Here's a complete Formfile example for a web server:

```
NAME web-server
USER username:webdev passwd:webpass123 sudo:true

VCPU 2
MEM 2048
DISK 10

INSTALL nginx

RUN echo "server { \
    listen 80 default_server; \
    root /var/www/html; \
    index index.html; \
    location / { \
        try_files \$uri \$uri/ =404; \
    } \
}" > /etc/nginx/sites-available/default

COPY ./www /var/www/html

EXPOSE 80
```

### Database Server with Network Access

Example Formfile for a PostgreSQL server:

```
NAME postgres-db
USER username:pguser passwd:pgpass123 sudo:true

VCPU 4
MEM 4096
DISK 50

INSTALL postgresql

RUN echo "listen_addresses = '*'" >> /etc/postgresql/14/main/postgresql.conf
RUN echo "host all all 0.0.0.0/0 md5" >> /etc/postgresql/14/main/pg_hba.conf

EXPOSE 5432
```

## Connecting Instances

### Service Discovery

Instances on Formnet can discover each other using their hostnames:

```bash
# From inside one instance, connect to another
curl http://another-instance.formation.local
```

### Creating a Network of Services

To create a multi-service application, deploy separate instances and have them communicate over Formnet.

Example architecture:
- Web frontend instance (exposes port 80)
- API service instance (exposes port 3000)
- Database instance (exposes port 5432)

Configure them to connect to each other using Formation hostnames or IP addresses.

## Advanced Networking

### Port Forwarding via SSH

To access services without exposing them publicly, use SSH port forwarding:

```bash
ssh -L 8080:localhost:80 username@<formnet-ip>
```

This forwards your local port 8080 to port 80 on the instance.

### Creating a VPN Connection

Since Formnet is built on WireGuard, you can use it as a VPN to access your instances securely:

```bash
form manage formnet-up
```

Once connected, you can access all Formnet IP addresses directly.

### Network Namespaces and Isolation

Formation instances run in their own network namespaces, providing isolation from each other. Communication between instances must go through exposed network interfaces.

## Troubleshooting Network Issues

### Common Networking Problems

#### Cannot Connect to Formnet

1. Check that Formnet is running:
   ```bash
   form manage formnet-status
   ```

2. Restart the Formnet connection:
   ```bash
   form manage formnet-up
   ```

3. Verify your network configuration:
   ```bash
   ip a show wg0
   ```

#### Cannot Access Instance

1. Verify the instance is running:
   ```bash
   form pack status --build-id <your-build-id>
   ```

2. Ensure you have the correct IP address:
   ```bash
   form manage get-ip --build-id <your-build-id>
   ```

3. Check connectivity:
   ```bash
   ping <instance-ip>
   ```

#### Cannot Access Exposed Service

1. Verify the port is exposed in your Formfile
2. Check if the service is running inside the instance:
   ```bash
   ssh username@<formnet-ip> 'ps aux | grep <service-name>'
   ```
3. Test connectivity internally:
   ```bash
   ssh username@<formnet-ip> 'curl localhost:<port>'
   ```

### Network Debugging Tools

From inside your instance, use standard Linux networking tools:

- `ip a`: View network interfaces
- `ss -tuln`: List listening ports
- `dig`, `nslookup`: DNS queries
- `traceroute`: Trace network path
- `tcpdump`: Capture network traffic

### Logs and Diagnostics

For network-related logs:

1. Formnet logs:
   ```bash
   sudo journalctl -u formnet
   ```

2. On your instance:
   ```bash
   sudo tail -f /var/log/syslog
   ```

## Network Security Best Practices

1. **Limit exposed ports**: Only expose ports that need to be accessible

2. **Use SSH keys instead of passwords**: For more secure instance access

3. **Implement proper firewall rules**: Use `iptables` or `ufw` in your Formfile

4. **Regular security audits**: Periodically review your network configuration

5. **Keep software updated**: Regular updates for security patches

## Next Steps

- Explore [Using Form-kit](./using-form-kit.md) for simplified deployment
- Learn about [Troubleshooting](./troubleshooting.md) techniques
- Review the [Formation SDK](./formation-sdk.md) for programmatic control 