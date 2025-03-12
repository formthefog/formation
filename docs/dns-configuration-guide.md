# Formation Network: Authoritative DNS Configuration Guide

This guide provides step-by-step instructions for configuring `form-dns` as the authoritative name server for `bootstrap.formation.cloud` and `network.formation.cloud` domains. Following these steps will ensure your Formation Network can properly manage DNS resolution for bootstrap node discovery and network services.

## Table of Contents

1. [Prerequisites](#prerequisites)
2. [Domain Registrar Configuration](#domain-registrar-configuration)
3. [DNS Server Infrastructure Setup](#dns-server-infrastructure-setup)
4. [form-dns Configuration](#form-dns-configuration)
5. [Testing DNS Configuration](#testing-dns-configuration)
6. [Bootstrap Node Registration](#bootstrap-node-registration)
7. [Maintaining DNS Records](#maintaining-dns-records)
8. [Troubleshooting](#troubleshooting)

## Prerequisites

Before proceeding, ensure you have:

- Access to the domain registrar's control panel for `formation.cloud`
- Server(s) with public IP addresses to run `form-dns` instances
- Root/administrative access on these servers
- `form-dns` built and ready to deploy
- Basic knowledge of DNS concepts

## Domain Registrar Configuration

### Step 1: Create Nameserver Records

At your domain registrar (e.g., Namecheap, GoDaddy, AWS Route 53), you'll need to create nameserver records for your authoritative DNS servers.

1. Log in to your domain registrar's control panel
2. Navigate to the DNS settings for `formation.cloud`
3. Create at least two nameserver (NS) records:
   ```
   ns1.formation.cloud  →  [Public IP of your first DNS server]
   ns2.formation.cloud  →  [Public IP of your second DNS server]
   ```

### Step 2: Set Up Glue Records

Glue records are required to resolve the circular dependency between your domain and nameservers.

1. Find the glue record section in your registrar's control panel
2. Add glue records for your nameservers:
   ```
   ns1.formation.cloud  →  [Public IP of your first DNS server]
   ns2.formation.cloud  →  [Public IP of your second DNS server]
   ```

### Step 3: Create Subdomain Delegation

Create DNS delegation for your specific subdomains to point to your authoritative nameservers.

1. In the DNS settings for `formation.cloud`, add NS records for your subdomains:
   ```
   bootstrap.formation.cloud  →  ns1.formation.cloud
   bootstrap.formation.cloud  →  ns2.formation.cloud
   network.formation.cloud    →  ns1.formation.cloud
   network.formation.cloud    →  ns2.formation.cloud
   ```

### Step 4: Set TTL Values

Set appropriate Time-To-Live (TTL) values for your DNS records:

1. For nameserver records, set a high TTL (e.g., 86400 seconds/24 hours)
2. For subdomain delegations, set a medium TTL (e.g., 3600-14400 seconds/1-4 hours)

## DNS Server Infrastructure Setup

### Step 1: Prepare Server Infrastructure

On each server that will run as a nameserver:

1. Ensure the server has a static public IP address
2. Open the following ports in your firewall:
   - UDP port 53 (DNS)
   - TCP port 53 (DNS zone transfers)
   - TCP port 5353 (form-dns API)

```bash
# Example using UFW (Ubuntu)
sudo ufw allow 53/udp
sudo ufw allow 53/tcp
sudo ufw allow 5353/tcp
```

### Step 2: Install Required Dependencies

Install necessary dependencies for `form-dns`:

```bash
# Update package lists
sudo apt update

# Install dependencies
sudo apt install -y build-essential libssl-dev pkg-config
```

### Step 3: Configure System DNS Service

Either disable the system DNS service (if it exists) or configure it to listen on a different port:

```bash
# For systemd-resolved
sudo systemctl stop systemd-resolved
sudo systemctl disable systemd-resolved

# For bind9
sudo systemctl stop bind9
sudo systemctl disable bind9
```

## form-dns Configuration

### Step 1: Create Configuration Directory

Create a directory structure for form-dns:

```bash
sudo mkdir -p /etc/formation/dns/zones
sudo mkdir -p /etc/formation/geo
```

### Step 2: Set Up MaxMind Geolocation Database (Optional)

Download and configure MaxMind GeoLite2 database for geolocation-based DNS responses:

```bash
# Download GeoLite2 City database (you need to create a MaxMind account for this)
cd /etc/formation/geo
wget https://download.maxmind.com/app/geoip_download?edition_id=GeoLite2-City&license_key=YOUR_LICENSE_KEY&suffix=tar.gz -O geolite2-city.tar.gz
tar -xzvf geolite2-city.tar.gz
cp GeoLite2-City_*/GeoLite2-City.mmdb .
```

### Step 3: Configure form-dns Service

Create a systemd service file for form-dns:

```bash
sudo nano /etc/systemd/system/form-dns.service
```

Add the following content:

```ini
[Unit]
Description=Formation DNS Service
After=network.target

[Service]
ExecStart=/usr/local/bin/form-dns --config-path /etc/formation/dns/config.json
Restart=always
User=root
Group=root
WorkingDirectory=/etc/formation/dns

[Install]
WantedBy=multi-user.target
```

### Step 4: Create form-dns Configuration

Create the main configuration file:

```bash
sudo nano /etc/formation/dns/config.json
```

Add the following content (adjust as needed):

```json
{
  "listen_addr": "0.0.0.0",
  "port": 53,
  "api_port": 5353,
  "records_file": "/etc/formation/dns/records.json",
  "geo_database": "/etc/formation/geo/GeoLite2-City.mmdb",
  "upstream_dns": ["1.1.1.1", "8.8.8.8"],
  "zones": [
    {
      "name": "bootstrap.formation.cloud",
      "type": "primary",
      "ttl": 60
    },
    {
      "name": "network.formation.cloud",
      "type": "primary",
      "ttl": 300
    }
  ],
  "health_check": {
    "enabled": true,
    "interval": 30,
    "timeout": 5,
    "unhealthy_threshold": 3
  }
}
```

### Step 5: Initialize Records

Create the initial records file:

```bash
sudo nano /etc/formation/dns/records.json
```

Add the following content:

```json
{
  "records": [
    {
      "domain": "bootstrap.formation.cloud",
      "record_type": "A",
      "public_ip": [],
      "formnet_ip": [],
      "ttl": 60,
      "verification_status": "Verified"
    },
    {
      "domain": "network.formation.cloud",
      "record_type": "A",
      "public_ip": [],
      "formnet_ip": [],
      "ttl": 300,
      "verification_status": "Verified"
    }
  ],
  "servers": []
}
```

### Step 6: Deploy and Start form-dns

Copy the built `form-dns` binary to the system path and start the service:

```bash
# Assuming form-dns binary is in your current directory
sudo cp form-dns /usr/local/bin/
sudo chmod +x /usr/local/bin/form-dns

# Reload systemd and start service
sudo systemctl daemon-reload
sudo systemctl enable form-dns
sudo systemctl start form-dns
```

## Testing DNS Configuration

### Step 1: Verify Local DNS Service

Check if the form-dns service is running properly:

```bash
sudo systemctl status form-dns
sudo ss -tulpn | grep 53
```

### Step 2: Test Local Resolution

Test DNS resolution on the local server:

```bash
# Install dig if not available
sudo apt install -y dnsutils

# Test local resolution
dig @localhost bootstrap.formation.cloud
dig @localhost network.formation.cloud
```

### Step 3: Test Public Resolution

From another machine, test if your DNS servers are accessible:

```bash
# Test resolution against your nameservers
dig @ns1.formation.cloud bootstrap.formation.cloud
dig @ns2.formation.cloud bootstrap.formation.cloud

# Test domain delegation
dig bootstrap.formation.cloud
dig network.formation.cloud
```

### Step 4: Test DNS Propagation

DNS changes can take time to propagate. Use online DNS propagation checkers:

- https://www.whatsmydns.net/
- https://dnschecker.org/

Check for your `bootstrap.formation.cloud` and `network.formation.cloud` domains.

## Bootstrap Node Registration

### Step 1: Register Initial Bootstrap Nodes

Use the `form-config-wizard` tool to register bootstrap nodes:

```bash
# Register a bootstrap node
form-config-wizard bootstrap add --node-id node1 --ip 203.0.113.1 --region us-east --api http://localhost:5353

# Register additional bootstrap nodes
form-config-wizard bootstrap add --node-id node2 --ip 203.0.113.2 --region eu-west --api http://localhost:5353
```

### Step 2: Verify Bootstrap Node Registration

List the registered bootstrap nodes:

```bash
form-config-wizard bootstrap list --api http://localhost:5353
```

### Step 3: Test Bootstrap Domain Resolution

Verify that bootstrap nodes are resolvable:

```bash
dig bootstrap.formation.cloud
```

## Maintaining DNS Records

### Adding New Bootstrap Nodes

To add a new bootstrap node:

```bash
form-config-wizard bootstrap add --node-id <node-id> --ip <public-ip> --region <region> --api http://localhost:5353
```

### Removing Bootstrap Nodes

To remove a bootstrap node:

```bash
form-config-wizard bootstrap remove --ip <public-ip> --api http://localhost:5353
```

### Monitoring Health Status

Check the health status of bootstrap nodes:

```bash
form-config-wizard bootstrap list --api http://localhost:5353
```

### Backup DNS Records

Regularly backup your DNS records:

```bash
sudo cp /etc/formation/dns/records.json /etc/formation/dns/records.json.backup-$(date +%Y%m%d)
```

## Troubleshooting

### Common Issues and Solutions

#### DNS Resolution Failures

**Issue**: Unable to resolve `bootstrap.formation.cloud` or `network.formation.cloud`

**Solutions**:
1. Verify the `form-dns` service is running:
   ```bash
   sudo systemctl status form-dns
   ```

2. Check logs for errors:
   ```bash
   sudo journalctl -u form-dns -n 100
   ```

3. Verify DNS port is open:
   ```bash
   sudo ss -tulpn | grep 53
   ```

4. Test local resolution:
   ```bash
   dig @localhost bootstrap.formation.cloud
   ```

5. Check domain delegation:
   ```bash
   dig +trace bootstrap.formation.cloud
   ```

#### Domain Registrar Configuration Issues

**Issue**: Nameserver or glue record setup problems

**Solutions**:
1. Verify nameserver records at your registrar
2. Ensure glue records have the correct IP addresses
3. Check WHOIS data to confirm nameserver changes are active:
   ```bash
   whois formation.cloud | grep -i "name server"
   ```
4. Wait up to 48 hours for DNS propagation

#### Health Check Failures

**Issue**: Bootstrap nodes marked as unhealthy

**Solutions**:
1. Verify bootstrap nodes are online and accessible
2. Check `form-dns` health check configuration
3. Test connectivity to bootstrap nodes:
   ```bash
   curl -v telnet://<bootstrap-node-ip>:51820
   ```
4. Review health check logs:
   ```bash
   sudo journalctl -u form-dns | grep "health"
   ```

#### API Access Issues

**Issue**: Unable to register or manage bootstrap nodes

**Solutions**:
1. Verify API port is open:
   ```bash
   sudo ss -tulpn | grep 5353
   ```
2. Check API endpoint connectivity:
   ```bash
   curl -v http://localhost:5353/api/bootstrap/list
   ```
3. Review API logs for errors:
   ```bash
   sudo journalctl -u form-dns | grep "api"
   ```

#### TTL Issues

**Issue**: DNS changes not propagating quickly enough

**Solutions**:
1. Adjust TTL settings in your form-dns configuration
2. For bootstrap nodes, use a lower TTL (30-60 seconds)
3. Wait for previous TTL to expire before changes take effect
4. Flush DNS resolver cache on client systems:
   ```bash
   # Linux
   sudo systemd-resolve --flush-caches
   
   # Windows
   ipconfig /flushdns
   
   # macOS
   sudo killall -HUP mDNSResponder
   ```

#### Geolocation Issues

**Issue**: Clients not connecting to geographically closer bootstrap nodes

**Solutions**:
1. Verify the MaxMind database is correctly installed and configured
2. Check if geolocation is enabled in form-dns configuration
3. Test geolocation resolution with a known client IP:
   ```bash
   # Assuming you have a tool that lets you specify source IP
   dig @localhost bootstrap.formation.cloud -b <client-ip>
   ```
4. Update the MaxMind database if it's outdated

### Diagnostic Tools

- **dig**: For DNS queries and troubleshooting
  ```bash
  # Basic query
  dig bootstrap.formation.cloud
  
  # Trace query path
  dig +trace bootstrap.formation.cloud
  
  # Short output
  dig +short bootstrap.formation.cloud
  ```

- **nslookup**: Alternative DNS lookup tool
  ```bash
  nslookup bootstrap.formation.cloud
  ```

- **whois**: Check domain registration information
  ```bash
  whois formation.cloud
  ```

- **curl**: Test API endpoints
  ```bash
  curl -s http://localhost:5353/api/bootstrap/list | jq
  ```

- **tcpdump**: Capture DNS traffic for detailed analysis
  ```bash
  sudo tcpdump -i any port 53
  ```

---

## Next Steps

After successfully configuring your authoritative DNS for bootstrap domains, consider:

1. Setting up DNS redundancy with multiple form-dns instances
2. Implementing automatic health checks and failover
3. Adding DNSSEC for enhanced security
4. Creating a monitoring system for your DNS infrastructure
5. Establishing a regular backup schedule for DNS configuration

By following this guide, you should have a fully functional authoritative DNS setup for your Formation Network bootstrap domains, enabling reliable node discovery and network operation. 