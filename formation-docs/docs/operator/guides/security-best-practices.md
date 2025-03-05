# Security Best Practices for Formation Operators

This guide provides comprehensive security recommendations for operating Formation nodes. Following these security best practices will help you protect your infrastructure, maintain high availability, and ensure the security of your users' workloads.

## Host System Security

### Operating System Hardening

1. **Keep the system updated**:
   ```bash
   # For Ubuntu/Debian systems
   sudo apt update && sudo apt upgrade -y
   
   # For RHEL/CentOS systems
   sudo yum update -y
   ```

2. **Enable automatic security updates**:
   ```bash
   # For Ubuntu/Debian systems
   sudo apt install unattended-upgrades
   sudo dpkg-reconfigure -plow unattended-upgrades
   
   # For RHEL/CentOS systems
   sudo yum install yum-cron
   sudo systemctl enable yum-cron
   sudo systemctl start yum-cron
   ```

3. **Use a minimal installation**:
   - Only install required packages
   - Remove unnecessary services and software
   - Disable unused kernel modules

4. **Enable a firewall**:
   ```bash
   # For UFW (Ubuntu)
   sudo ufw default deny incoming
   sudo ufw default allow outgoing
   sudo ufw allow ssh
   sudo ufw allow 9372/tcp  # Formnet port
   sudo ufw allow 51820/udp  # WireGuard port
   sudo ufw enable
   
   # For firewalld (RHEL/CentOS)
   sudo firewall-cmd --permanent --add-service=ssh
   sudo firewall-cmd --permanent --add-port=9372/tcp  # Formnet port
   sudo firewall-cmd --permanent --add-port=51820/udp  # WireGuard port
   sudo firewall-cmd --reload
   ```

5. **Enable SELinux or AppArmor**:
   ```bash
   # Check SELinux status (RHEL/CentOS)
   sestatus
   
   # Check AppArmor status (Ubuntu)
   sudo apparmor_status
   ```

6. **Apply the principle of least privilege**:
   - Run services with minimal required permissions
   - Use separate user accounts for different services
   - Apply capabilities instead of running services as root

### User and Authentication Security

1. **Use SSH key-based authentication only**:
   ```bash
   sudo nano /etc/ssh/sshd_config
   ```
   Add or modify these lines:
   ```
   PasswordAuthentication no
   ChallengeResponseAuthentication no
   PubkeyAuthentication yes
   ```
   Restart SSH:
   ```bash
   sudo systemctl restart sshd
   ```

2. **Disable root login**:
   ```bash
   sudo nano /etc/ssh/sshd_config
   ```
   Add or modify this line:
   ```
   PermitRootLogin no
   ```
   Restart SSH:
   ```bash
   sudo systemctl restart sshd
   ```

3. **Implement fail2ban to prevent brute force attacks**:
   ```bash
   sudo apt install fail2ban
   sudo cp /etc/fail2ban/jail.conf /etc/fail2ban/jail.local
   sudo systemctl enable fail2ban
   sudo systemctl start fail2ban
   ```

4. **Use strong password policies**:
   ```bash
   sudo apt install libpam-pwquality
   sudo nano /etc/security/pwquality.conf
   ```
   Example settings:
   ```
   minlen = 12
   minclass = 3
   maxrepeat = 2
   enforce_for_root
   ```

5. **Implement 2FA for SSH**:
   ```bash
   sudo apt install libpam-google-authenticator
   google-authenticator
   sudo nano /etc/pam.d/sshd
   ```
   Add this line:
   ```
   auth required pam_google_authenticator.so
   ```
   Edit SSH config:
   ```bash
   sudo nano /etc/ssh/sshd_config
   ```
   Add or modify this line:
   ```
   ChallengeResponseAuthentication yes
   ```
   Restart SSH:
   ```bash
   sudo systemctl restart sshd
   ```

### File System Security

1. **Enable disk encryption**:
   - For new installations, use full disk encryption (LUKS)
   - For existing systems, encrypt sensitive partitions

2. **Set up proper file permissions**:
   ```bash
   # Secure Formation configuration directory
   chmod 700 ~/.config/form
   chmod 600 ~/.config/form/form-config.json
   chmod 600 ~/.config/form/keystore.json
   
   # Secure log files
   sudo chmod 640 /var/log/formation/*.log
   ```

3. **Enable file system auditing**:
   ```bash
   # For Ubuntu/Debian
   sudo apt install auditd
   sudo systemctl enable auditd
   sudo systemctl start auditd
   
   # Configure important file monitoring
   sudo auditctl -w /etc/passwd -p wa -k identity
   sudo auditctl -w /etc/group -p wa -k identity
   sudo auditctl -w ~/.config/form/form-config.json -p wa -k form-config
   ```

4. **Regularly verify file integrity**:
   ```bash
   # Install AIDE
   sudo apt install aide
   
   # Initialize the database
   sudo aideinit
   
   # Move the initial database to the correct location
   sudo mv /var/lib/aide/aide.db.new /var/lib/aide/aide.db
   
   # Set up a daily check
   sudo bash -c 'echo "0 3 * * * root /usr/bin/aide.wrapper --check" > /etc/cron.d/aide'
   ```

## Network Security

### Formnet and WireGuard Security

1. **Configure secure network settings in form-config.json**:
   ```json
   {
     "network": {
       "formnet_enabled": true,
       "formnet_port": 9372,
       "wireguard_port": 51820,
       "public_address": "YOUR_PUBLIC_IP",
       "allowed_networks": ["10.0.0.0/8", "172.16.0.0/12", "192.168.0.0/16"],
       "bandwidth_limit_mbps": 500,
       "firewall_enabled": true
     }
   }
   ```

2. **Restrict WireGuard endpoints**:
   ```bash
   # Add allowed IPs to WireGuard configuration
   sudo wg set wg0 allowed-ips 10.0.0.0/8,172.16.0.0/12,192.168.0.0/16
   ```

3. **Enable TCP keepalive settings**:
   ```bash
   # Add to /etc/sysctl.conf
   net.ipv4.tcp_keepalive_time = 60
   net.ipv4.tcp_keepalive_intvl = 10
   net.ipv4.tcp_keepalive_probes = 6
   
   # Apply changes
   sudo sysctl -p
   ```

4. **Set up network monitoring**:
   ```bash
   # Install netdata for real-time monitoring
   bash <(curl -Ss https://my-netdata.io/kickstart.sh)
   
   # Or install Prometheus node exporter
   sudo apt install prometheus-node-exporter
   ```

### DDoS Protection

1. **Configure kernel parameters for DDoS mitigation**:
   ```bash
   # Add to /etc/sysctl.conf
   net.ipv4.tcp_syncookies = 1
   net.ipv4.tcp_max_syn_backlog = 2048
   net.ipv4.tcp_synack_retries = 2
   net.ipv4.tcp_syn_retries = 5
   net.ipv4.icmp_echo_ignore_broadcasts = 1
   
   # Apply changes
   sudo sysctl -p
   ```

2. **Install and configure fail2ban for HTTP/API protection**:
   ```bash
   # Create custom jail for Formation API
   sudo nano /etc/fail2ban/jail.d/formation-api.conf
   ```
   Add:
   ```
   [formation-api]
   enabled = true
   port = 9372
   filter = formation-api
   logpath = /var/log/formation/api.log
   maxretry = 5
   bantime = 3600
   ```

3. **Consider using a CDN or DDoS protection service**:
   - Cloudflare
   - AWS Shield
   - Google Cloud Armor

4. **Set up rate limiting on your reverse proxy**:
   ```bash
   # For Nginx
   sudo apt install nginx
   ```
   Edit your Nginx configuration:
   ```
   http {
     limit_req_zone $binary_remote_addr zone=formation:10m rate=10r/s;
     
     server {
       location /api/ {
         limit_req zone=formation burst=20 nodelay;
         proxy_pass http://localhost:9372;
       }
     }
   }
   ```

## Virtualization Security

### KVM/QEMU Security

1. **Keep hypervisor software updated**:
   ```bash
   sudo apt update
   sudo apt install --only-upgrade qemu-kvm libvirt-daemon-system
   ```

2. **Secure VM isolation**:
   ```bash
   # Edit libvirt configuration
   sudo nano /etc/libvirt/qemu.conf
   ```
   Add or modify:
   ```
   security_driver = "apparmor"
   namespaces = [ "mount", "network" ]
   user = "formation"
   group = "formation"
   dynamic_ownership = 1
   seccomp_sandbox = 1
   ```

3. **Configure secure storage for VM images**:
   ```bash
   # Create a separate volume for VM storage
   sudo mkdir -p /var/lib/formation/vms
   sudo chmod 700 /var/lib/formation/vms
   
   # Set proper ownership
   sudo chown formation:formation /var/lib/formation/vms
   ```

4. **Enable sVirt/SELinux for VM isolation**:
   - This provides additional isolation between VMs
   - Ensures VMs can only access their own resources

5. **Disable unnecessary device access**:
   ```json
   {
     "vm": {
       "device_passthrough": {
         "usb": false,
         "pci": false,
         "audio": false
       }
     }
   }
   ```

### Guest VM Security

1. **Apply security profiles to guest VMs**:
   ```bash
   # Create a security profile for VMs
   sudo nano /etc/apparmor.d/libvirt/TEMPLATE
   ```

2. **Use secure boot for guest VMs**:
   ```json
   {
     "security": {
       "secure_boot_required": true
     }
   }
   ```

3. **Implement VM resource limits**:
   ```json
   {
     "resource_pools": [
       {
         "name": "standard",
         "max_instances": 10,
         "cpus": 16,
         "memory_mb": 32768,
         "disk_gb": 500
       }
     ]
   }
   ```

4. **Enable memory protections**:
   - Enable IOMMU for DMA protection
   - Configure KSM (Kernel Same-page Merging) securely

## Wallet Security

### Keystore Protection

1. **Use encrypted keystore files**:
   ```bash
   # Ensure keystore is encrypted
   form wallet inspect --keystore ~/.config/form/keystore.json
   
   # If needed, regenerate with stronger encryption
   form wallet regenerate-keystore --keystore ~/.config/form/keystore.json --encryption-level high
   ```

2. **Secure keystore access permissions**:
   ```bash
   chmod 600 ~/.config/form/keystore.json
   ```

3. **Use a strong password for the keystore**:
   - At least 16 characters
   - Mix of uppercase, lowercase, numbers, and special characters
   - Not based on dictionary words

4. **Consider hardware security modules (HSMs)**:
   - YubiKey
   - Ledger
   - Trezor

### Key Management

1. **Create a backup of your keystore**:
   ```bash
   # Create an encrypted backup
   openssl enc -aes-256-cbc -salt -in ~/.config/form/keystore.json -out ~/keystore-backup.enc
   
   # Store the backup securely off-site
   ```

2. **Split control with multisig wallets**:
   - Use a multisig wallet for high-value operations
   - Distribute signing authority across multiple team members

3. **Implement key rotation procedure**:
   - Document the process for rotating keys
   - Practice key rotation in a test environment
   - Schedule regular key rotations

4. **Monitor your address for unusual activity**:
   - Set up alerts for transactions
   - Use blockchain explorers to verify activity

## Monitoring and Auditing

### Security Monitoring

1. **Set up centralized logging**:
   ```bash
   # Install Filebeat
   curl -L -O https://artifacts.elastic.co/downloads/beats/filebeat/filebeat-7.17.0-amd64.deb
   sudo dpkg -i filebeat-7.17.0-amd64.deb
   
   # Configure Filebeat to collect Formation logs
   sudo nano /etc/filebeat/filebeat.yml
   ```
   Add:
   ```yaml
   filebeat.inputs:
   - type: log
     enabled: true
     paths:
       - /var/log/formation/*.log
   
   output.elasticsearch:
     hosts: ["your-elasticsearch-host:9200"]
   ```

2. **Configure security alerts**:
   ```json
   {
     "monitoring": {
       "alerting_enabled": true,
       "alert_thresholds": {
         "cpu_usage_percent": 90,
         "memory_usage_percent": 90,
         "disk_usage_percent": 85,
         "network_saturation_percent": 80,
         "failed_login_attempts": 5
       },
       "alert_channels": [
         {
           "type": "email",
           "address": "security@example.com"
         },
         {
           "type": "webhook",
           "url": "https://alerts.example.com/webhook"
         }
       ]
     }
   }
   ```

3. **Implement intrusion detection**:
   ```bash
   # Install OSSEC
   wget https://github.com/ossec/ossec-hids/archive/3.7.0.tar.gz
   tar -xzf 3.7.0.tar.gz
   cd ossec-hids-3.7.0
   ./install.sh
   ```

4. **Set up periodic security scans**:
   ```bash
   # Install Lynis for security auditing
   sudo apt install lynis
   
   # Run a system scan
   sudo lynis audit system
   
   # Set up weekly scan via cron
   echo "0 2 * * 0 root /usr/bin/lynis audit system --cronjob" | sudo tee /etc/cron.d/lynis
   ```

### Auditing and Compliance

1. **Enable comprehensive audit logging**:
   ```json
   {
     "security": {
       "audit_logging": true
     }
   }
   ```

2. **Configure logs retention policy**:
   ```json
   {
     "monitoring": {
       "log_retention_days": 90
     }
   }
   ```

3. **Set up regular security audits**:
   - Schedule quarterly security reviews
   - Document findings and remediation steps
   - Implement changes based on audit findings

4. **Implement a security incident response plan**:
   - Document procedures for security incidents
   - Assign roles and responsibilities
   - Practice incident response scenarios

## Secure Operations

### Update Management

1. **Create a staging environment**:
   - Test Formation updates in a non-production environment
   - Verify functionality before applying to production

2. **Configure automated backups before updates**:
   ```json
   {
     "advanced": {
       "backup_enabled": true,
       "backup_before_update": true
     }
   }
   ```

3. **Set up a maintenance window**:
   ```json
   {
     "advanced": {
       "maintenance_window": {
         "enabled": true,
         "day_of_week": "sunday",
         "hour": 3,
         "duration_hours": 2
       }
     }
   }
   ```

4. **Enable rolling updates for minimal downtime**:
   ```json
   {
     "advanced": {
       "update_strategy": "rolling"
     }
   }
   ```

### Backup and Recovery

1. **Schedule regular configuration backups**:
   ```json
   {
     "advanced": {
       "backup_enabled": true,
       "backup_schedule": "0 2 * * *",
       "backup_retention_count": 14
     }
   }
   ```

2. **Test restoration procedures**:
   - Regularly practice restoring from backups
   - Verify backup integrity

3. **Implement off-site backup storage**:
   ```bash
   # Install rclone for cloud storage
   curl https://rclone.org/install.sh | sudo bash
   
   # Configure rclone
   rclone config
   
   # Set up automated backup to cloud storage
   echo "0 3 * * * root /usr/bin/rclone copy /var/lib/formation/backups remote:formation-backups" | sudo tee /etc/cron.d/formation-backup
   ```

4. **Document disaster recovery procedures**:
   - Create step-by-step recovery instructions
   - Keep documentation updated with system changes

## Additional Security Measures

### Physical Security

1. **Secure physical access to servers**:
   - Use access controls for server rooms
   - Implement video surveillance
   - Maintain an access log

2. **Enable TPM/secure boot on hardware**:
   - Verify boot integrity
   - Prevent tampering with boot process

3. **Implement hardware monitoring**:
   - Set up temperature sensors
   - Configure alerts for hardware failures
   - Monitor physical access

### Personnel Security

1. **Implement the principle of least privilege**:
   - Grant minimum required access to operators
   - Use separate accounts for administrative tasks

2. **Provide security training**:
   - Train operators on security procedures
   - Conduct regular security awareness sessions

3. **Document security processes**:
   - Create clear security guidelines
   - Maintain up-to-date documentation

### Compliance and Standards

1. **Follow industry security standards**:
   - ISO 27001
   - NIST Cybersecurity Framework
   - CIS Benchmarks

2. **Create a security policy**:
   - Define security requirements
   - Document security controls
   - Establish compliance checks

3. **Conduct regular penetration testing**:
   - Hire external security consultants
   - Perform internal security testing
   - Address identified vulnerabilities

## Security Checklist

Use this checklist to verify your security implementation:

### System Security
- [ ] Operating system is fully updated
- [ ] Automatic security updates are enabled
- [ ] Firewall is properly configured
- [ ] SELinux/AppArmor is enabled
- [ ] Strong SSH configuration is in place
- [ ] Fail2ban is active
- [ ] File systems have appropriate permissions
- [ ] Disk encryption is enabled
- [ ] File integrity monitoring is configured

### Network Security
- [ ] Network ports are properly restricted
- [ ] DDoS protection measures are in place
- [ ] Network monitoring is active
- [ ] Formnet and WireGuard are securely configured
- [ ] Traffic is rate-limited
- [ ] Network isolation for VMs is implemented

### Virtualization Security
- [ ] Hypervisor is updated
- [ ] VM isolation is properly configured
- [ ] Secure storage for VM images
- [ ] Resource limits are implemented
- [ ] Device access is restricted

### Wallet Security
- [ ] Keystore is encrypted with a strong password
- [ ] Keystore file has appropriate permissions
- [ ] Backups are securely stored
- [ ] Key rotation procedure is documented

### Monitoring and Auditing
- [ ] Centralized logging is implemented
- [ ] Security alerts are configured
- [ ] Intrusion detection is active
- [ ] Regular security scans are scheduled
- [ ] Audit logging is enabled
- [ ] Log retention policy is in place

### Operational Security
- [ ] Update process is documented
- [ ] Backup and recovery procedures are tested
- [ ] Maintenance window is configured
- [ ] Disaster recovery plan is documented
- [ ] Incident response procedure is in place

## Conclusion

Implementing these security best practices will significantly enhance the security posture of your Formation operator node. Remember that security is an ongoing process, not a one-time implementation. Regularly review and update your security measures to address new threats and vulnerabilities.

For additional assistance with security configuration, contact the Formation support team or consult with a cybersecurity professional. 