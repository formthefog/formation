# Operator Configuration Reference

This document provides a comprehensive reference for all configuration options available to Formation operators. Configuration is managed through the Form Config Wizard and stored as JSON in the configuration file.

## Configuration File Location

The primary configuration file for operators is located at:

```
~/.config/form/form-config.json
```

## Core Configuration Sections

The configuration file is divided into several sections, each controlling different aspects of the operator node.

### 1. General Settings

```json
{
  "general": {
    "node_name": "my-operator-node",
    "operator_address": "0x1234567890abcdef1234567890abcdef12345678",
    "region": "us-east",
    "contact_email": "operator@example.com",
    "log_level": "info",
    "data_directory": "/var/lib/formation",
    "metrics_enabled": true,
    "auto_update": true,
    "update_channel": "stable"
  }
}
```

| Setting | Type | Description | Default |
|---------|------|-------------|---------|
| `node_name` | String | Unique name for your operator node | Generated UUID |
| `operator_address` | String | Ethereum address of the operator | From wallet |
| `region` | String | Geographic region of the node | Detected |
| `contact_email` | String | Email for operational notifications | Required |
| `log_level` | String | Logging verbosity (debug, info, warn, error) | "info" |
| `data_directory` | String | Directory for storing Formation data | "/var/lib/formation" |
| `metrics_enabled` | Boolean | Enable Prometheus metrics collection | true |
| `auto_update` | Boolean | Automatically update Formation software | true |
| `update_channel` | String | Update channel (stable, beta, edge) | "stable" |

### 2. Hardware Configuration

```json
{
  "hardware": {
    "total_cpus": 16,
    "reserved_cpus": 2,
    "total_memory_mb": 32768,
    "reserved_memory_mb": 4096,
    "total_disk_gb": 1000,
    "reserved_disk_gb": 100,
    "gpu_enabled": true,
    "gpu_devices": [
      {
        "id": "GPU-1234567890abcdef",
        "name": "NVIDIA GeForce RTX 3080",
        "memory_mb": 10240,
        "compute_units": 68
      }
    ],
    "network_bandwidth_mbps": 1000,
    "overcommit_ratio": 1.2
  }
}
```

| Setting | Type | Description | Default |
|---------|------|-------------|---------|
| `total_cpus` | Integer | Total CPU cores available | Auto-detected |
| `reserved_cpus` | Integer | CPU cores reserved for system use | 2 |
| `total_memory_mb` | Integer | Total memory in MB | Auto-detected |
| `reserved_memory_mb` | Integer | Memory reserved for system use in MB | 4096 |
| `total_disk_gb` | Integer | Total disk space in GB | Auto-detected |
| `reserved_disk_gb` | Integer | Disk space reserved for system use in GB | 100 |
| `gpu_enabled` | Boolean | Whether GPU passthrough is enabled | false |
| `gpu_devices` | Array | List of available GPU devices | Auto-detected |
| `network_bandwidth_mbps` | Integer | Network bandwidth in Mbps | Auto-detected |
| `overcommit_ratio` | Float | Resource overcommitment ratio | 1.0 |

### 3. Network Configuration

```json
{
  "network": {
    "formnet_enabled": true,
    "formnet_port": 9372,
    "wireguard_port": 51820,
    "public_address": "203.0.113.10",
    "external_dns": "node123.operator.example.com",
    "allowed_networks": ["10.0.0.0/8", "172.16.0.0/12", "192.168.0.0/16"],
    "bandwidth_limit_mbps": 500,
    "wireguard_mtu": 1420,
    "tcp_keepalive_seconds": 60,
    "use_ipv6": true,
    "firewall_enabled": true
  }
}
```

| Setting | Type | Description | Default |
|---------|------|-------------|---------|
| `formnet_enabled` | Boolean | Enable Formation network connectivity | true |
| `formnet_port` | Integer | Port for Formation network services | 9372 |
| `wireguard_port` | Integer | Port for WireGuard VPN | 51820 |
| `public_address` | String | Public IP address of the node | Auto-detected |
| `external_dns` | String | DNS hostname for the node | Generated |
| `allowed_networks` | Array | CIDR blocks allowed to connect | All private networks |
| `bandwidth_limit_mbps` | Integer | Maximum outbound bandwidth in Mbps | 0 (unlimited) |
| `wireguard_mtu` | Integer | MTU for WireGuard interface | 1420 |
| `tcp_keepalive_seconds` | Integer | TCP keepalive interval | 60 |
| `use_ipv6` | Boolean | Enable IPv6 connectivity | false |
| `firewall_enabled` | Boolean | Enable built-in firewall | true |

### 4. Blockchain Configuration

```json
{
  "blockchain": {
    "network": "mainnet",
    "custom_rpc_url": "",
    "staking_amount": "1000000000000000000",
    "payment_address": "0x1234567890abcdef1234567890abcdef12345678",
    "keystore_path": "~/.config/form/keystore.json",
    "auto_claim_rewards": true,
    "claim_threshold": "100000000000000000",
    "gas_price_strategy": "medium",
    "max_gas_price_gwei": 100
  }
}
```

| Setting | Type | Description | Default |
|---------|------|-------------|---------|
| `network` | String | Blockchain network (mainnet, testnet) | "mainnet" |
| `custom_rpc_url` | String | Custom Ethereum RPC endpoint | "" |
| `staking_amount` | String | Amount staked in wei | 1 ETH in wei |
| `payment_address` | String | Address to receive payments | Same as operator address |
| `keystore_path` | String | Path to Ethereum keystore file | "~/.config/form/keystore.json" |
| `auto_claim_rewards` | Boolean | Automatically claim rewards | true |
| `claim_threshold` | String | Minimum reward to claim in wei | 0.1 ETH in wei |
| `gas_price_strategy` | String | Gas price strategy (low, medium, high) | "medium" |
| `max_gas_price_gwei` | Integer | Maximum gas price in Gwei | 100 |

### 5. Security Configuration

```json
{
  "security": {
    "ssh_public_key": "ssh-rsa AAAAB3NzaC1...",
    "firewall_rules": [
      {
        "port": 22,
        "protocol": "tcp",
        "allow": ["203.0.113.0/24"]
      }
    ],
    "fail2ban_enabled": true,
    "secure_boot_required": false,
    "disk_encryption": true,
    "audit_logging": true,
    "enable_secure_enclaves": false,
    "auto_security_updates": true,
    "whitelist_only_mode": false
  }
}
```

| Setting | Type | Description | Default |
|---------|------|-------------|---------|
| `ssh_public_key` | String | SSH public key for operator access | Required |
| `firewall_rules` | Array | Custom firewall rules | [] |
| `fail2ban_enabled` | Boolean | Enable fail2ban for brute force protection | true |
| `secure_boot_required` | Boolean | Require secure boot for instances | false |
| `disk_encryption` | Boolean | Enable disk encryption | true |
| `audit_logging` | Boolean | Enable detailed audit logging | true |
| `enable_secure_enclaves` | Boolean | Enable secure enclaves for instances | false |
| `auto_security_updates` | Boolean | Apply security updates automatically | true |
| `whitelist_only_mode` | Boolean | Only allow whitelisted images | false |

### 6. VM Configuration

```json
{
  "vm": {
    "hypervisor": "kvm",
    "default_cpu_model": "host",
    "enable_nested_virtualization": false,
    "default_memory_ballooning": true,
    "default_disk_format": "qcow2",
    "default_network_model": "virtio",
    "cpu_pinning_enabled": true,
    "use_hugepages": true,
    "hugepages_size_mb": 1024,
    "default_vm_storage_path": "/var/lib/formation/vms",
    "vm_image_cache_size_gb": 50,
    "default_emulator_path": "/usr/bin/qemu-system-x86_64"
  }
}
```

| Setting | Type | Description | Default |
|---------|------|-------------|---------|
| `hypervisor` | String | Hypervisor type (kvm, xen, etc.) | "kvm" |
| `default_cpu_model` | String | Default CPU model to expose to VMs | "host" |
| `enable_nested_virtualization` | Boolean | Allow nested virtualization | false |
| `default_memory_ballooning` | Boolean | Enable memory ballooning | true |
| `default_disk_format` | String | Default VM disk format | "qcow2" |
| `default_network_model` | String | Default VM network device model | "virtio" |
| `cpu_pinning_enabled` | Boolean | Pin VM vCPUs to physical CPUs | false |
| `use_hugepages` | Boolean | Use hugepages for VM memory | false |
| `hugepages_size_mb` | Integer | Hugepage size in MB | 1024 |
| `default_vm_storage_path` | String | Path for VM storage | "/var/lib/formation/vms" |
| `vm_image_cache_size_gb` | Integer | Size of VM image cache in GB | 50 |
| `default_emulator_path` | String | Path to hypervisor emulator | Auto-detected |

### 7. Pricing Configuration

```json
{
  "pricing": {
    "currency": "USD",
    "vcpu_price_per_hour": 0.02,
    "memory_price_per_gb_hour": 0.005,
    "disk_price_per_gb_hour": 0.0001,
    "gpu_price_per_hour": 0.5,
    "network_price_per_gb": 0.05,
    "minimum_price_per_instance_hour": 0.01,
    "offer_discounts": true,
    "volume_discount_thresholds": [
      {
        "hours": 720,
        "discount_percentage": 10
      },
      {
        "hours": 4320,
        "discount_percentage": 20
      }
    ],
    "custom_pricing_enabled": false,
    "custom_pricing_tiers": []
  }
}
```

| Setting | Type | Description | Default |
|---------|------|-------------|---------|
| `currency` | String | Pricing currency code | "USD" |
| `vcpu_price_per_hour` | Float | Price per vCPU hour | 0.02 |
| `memory_price_per_gb_hour` | Float | Price per GB of RAM per hour | 0.005 |
| `disk_price_per_gb_hour` | Float | Price per GB of disk per hour | 0.0001 |
| `gpu_price_per_hour` | Float | Price per GPU hour | 0.5 |
| `network_price_per_gb` | Float | Price per GB of network traffic | 0.05 |
| `minimum_price_per_instance_hour` | Float | Minimum instance price per hour | 0.01 |
| `offer_discounts` | Boolean | Offer volume discounts | true |
| `volume_discount_thresholds` | Array | Volume discount tiers | See example |
| `custom_pricing_enabled` | Boolean | Enable custom pricing tiers | false |
| `custom_pricing_tiers` | Array | Custom pricing tier definitions | [] |

### 8. Performance Settings

```json
{
  "performance": {
    "io_scheduler": "mq-deadline",
    "cpu_governor": "performance",
    "swappiness": 10,
    "dirty_ratio": 20,
    "dirty_background_ratio": 10,
    "transparent_hugepages": "madvise",
    "kernel_same_page_merging": true,
    "numa_balancing": true,
    "io_priority": {
      "enabled": true,
      "high_priority_instances": []
    },
    "network_tuning": {
      "enabled": true,
      "tcp_bbr": true,
      "tcp_window_scaling": true
    }
  }
}
```

| Setting | Type | Description | Default |
|---------|------|-------------|---------|
| `io_scheduler` | String | I/O scheduler (cfq, mq-deadline, etc.) | "mq-deadline" |
| `cpu_governor` | String | CPU governor (performance, powersave, etc.) | "performance" |
| `swappiness` | Integer | VM swappiness (0-100) | 10 |
| `dirty_ratio` | Integer | Dirty ratio for disk writes | 20 |
| `dirty_background_ratio` | Integer | Background dirty ratio | 10 |
| `transparent_hugepages` | String | THP setting (always, madvise, never) | "madvise" |
| `kernel_same_page_merging` | Boolean | Enable KSM for memory deduplication | true |
| `numa_balancing` | Boolean | Enable NUMA balancing | true |
| `io_priority` | Object | I/O priority settings | See example |
| `network_tuning` | Object | Network performance tuning | See example |

### 9. Monitoring and Reporting

```json
{
  "monitoring": {
    "prometheus_enabled": true,
    "prometheus_port": 9100,
    "node_exporter_enabled": true,
    "alerting_enabled": true,
    "alert_thresholds": {
      "cpu_usage_percent": 90,
      "memory_usage_percent": 90,
      "disk_usage_percent": 85,
      "network_saturation_percent": 80
    },
    "alert_channels": [
      {
        "type": "email",
        "address": "alerts@example.com"
      }
    ],
    "telemetry_opt_in": true,
    "dashboard_enabled": true,
    "log_retention_days": 30
  }
}
```

| Setting | Type | Description | Default |
|---------|------|-------------|---------|
| `prometheus_enabled` | Boolean | Enable Prometheus metrics | true |
| `prometheus_port` | Integer | Port for Prometheus metrics | 9100 |
| `node_exporter_enabled` | Boolean | Enable node exporter | true |
| `alerting_enabled` | Boolean | Enable alerting | true |
| `alert_thresholds` | Object | Resource thresholds for alerts | See example |
| `alert_channels` | Array | Alert notification channels | See example |
| `telemetry_opt_in` | Boolean | Opt-in to anonymous telemetry | true |
| `dashboard_enabled` | Boolean | Enable local dashboard | true |
| `log_retention_days` | Integer | Days to retain logs | 30 |

### 10. Advanced Settings

```json
{
  "advanced": {
    "backup_enabled": true,
    "backup_schedule": "0 2 * * *",
    "backup_retention_count": 7,
    "maintenance_window": {
      "enabled": true,
      "day_of_week": "sunday",
      "hour": 3,
      "duration_hours": 2
    },
    "custom_hooks": {
      "pre_instance_start": "/path/to/pre-start.sh",
      "post_instance_start": "",
      "pre_instance_stop": "",
      "post_instance_stop": ""
    },
    "environment_variables": {
      "FORMATION_EXTRA_ARGS": "--verbose"
    },
    "experimental_features": {
      "live_migration": false,
      "memory_compression": false
    }
  }
}
```

| Setting | Type | Description | Default |
|---------|------|-------------|---------|
| `backup_enabled` | Boolean | Enable configuration backups | true |
| `backup_schedule` | String | Cron expression for backups | "0 2 * * *" |
| `backup_retention_count` | Integer | Number of backups to retain | 7 |
| `maintenance_window` | Object | Scheduled maintenance window | See example |
| `custom_hooks` | Object | Custom scripts for lifecycle events | See example |
| `environment_variables` | Object | Custom environment variables | {} |
| `experimental_features` | Object | Experimental feature flags | See example |

## Resource Pool Configuration

Operators can configure multiple resource pools to segregate resources for different purposes.

```json
{
  "resource_pools": [
    {
      "name": "standard",
      "description": "Standard compute resources",
      "cpus": 12,
      "memory_mb": 24576,
      "disk_gb": 800,
      "gpu_devices": [],
      "priority": "normal",
      "allow_overcommit": true,
      "max_instances": 0,
      "instance_types": ["general", "cpu-optimized"],
      "is_default": true
    },
    {
      "name": "gpu",
      "description": "GPU compute resources",
      "cpus": 2,
      "memory_mb": 4096,
      "disk_gb": 100,
      "gpu_devices": ["GPU-1234567890abcdef"],
      "priority": "high",
      "allow_overcommit": false,
      "max_instances": 1,
      "instance_types": ["gpu"],
      "is_default": false
    }
  ]
}
```

| Setting | Type | Description | Default |
|---------|------|-------------|---------|
| `name` | String | Pool name (unique) | Required |
| `description` | String | Pool description | "" |
| `cpus` | Integer | Number of CPUs in the pool | Required |
| `memory_mb` | Integer | Memory in MB for the pool | Required |
| `disk_gb` | Integer | Disk space in GB for the pool | Required |
| `gpu_devices` | Array | GPU device IDs in the pool | [] |
| `priority` | String | Pool priority (low, normal, high) | "normal" |
| `allow_overcommit` | Boolean | Allow resource overcommitment | true |
| `max_instances` | Integer | Max instances in pool (0=unlimited) | 0 |
| `instance_types` | Array | Instance types allowed in pool | ["general"] |
| `is_default` | Boolean | Whether this is the default pool | One must be true |

## Instance Type Configuration

Operators can define instance types that users can select when deploying.

```json
{
  "instance_types": [
    {
      "name": "general",
      "description": "General purpose instance",
      "vcpu_range": [1, 8],
      "memory_range_mb": [1024, 16384],
      "disk_range_gb": [10, 500],
      "vcpu_to_memory_ratio": 0,
      "default_vcpus": 2,
      "default_memory_mb": 4096,
      "default_disk_gb": 50,
      "enabled": true
    },
    {
      "name": "cpu-optimized",
      "description": "Compute optimized instance",
      "vcpu_range": [2, 16],
      "memory_range_mb": [2048, 32768],
      "disk_range_gb": [10, 500],
      "vcpu_to_memory_ratio": 1024,
      "default_vcpus": 4,
      "default_memory_mb": 4096,
      "default_disk_gb": 50,
      "enabled": true
    },
    {
      "name": "memory-optimized",
      "description": "Memory optimized instance",
      "vcpu_range": [2, 8],
      "memory_range_mb": [4096, 32768],
      "disk_range_gb": [10, 500],
      "vcpu_to_memory_ratio": 4096,
      "default_vcpus": 2,
      "default_memory_mb": 8192,
      "default_disk_gb": 50,
      "enabled": true
    },
    {
      "name": "gpu",
      "description": "GPU instance",
      "vcpu_range": [2, 8],
      "memory_range_mb": [4096, 16384],
      "disk_range_gb": [50, 500],
      "vcpu_to_memory_ratio": 0,
      "default_vcpus": 4,
      "default_memory_mb": 8192,
      "default_disk_gb": 100,
      "gpu_required": true,
      "enabled": true
    }
  ]
}
```

| Setting | Type | Description | Default |
|---------|------|-------------|---------|
| `name` | String | Instance type name (unique) | Required |
| `description` | String | Instance type description | "" |
| `vcpu_range` | Array | Min/max vCPUs allowed | Required |
| `memory_range_mb` | Array | Min/max memory in MB | Required |
| `disk_range_gb` | Array | Min/max disk in GB | Required |
| `vcpu_to_memory_ratio` | Integer | MB of RAM per vCPU (0=any) | 0 |
| `default_vcpus` | Integer | Default vCPU count | Required |
| `default_memory_mb` | Integer | Default memory in MB | Required |
| `default_disk_gb` | Integer | Default disk in GB | Required |
| `gpu_required` | Boolean | Whether a GPU is required | false |
| `enabled` | Boolean | Whether this type is enabled | true |

## Command-Line Configuration

Many configuration options can also be set via command-line arguments when running the operator service:

```
form-operator-service --log-level=debug --network.formnet_port=9373 --vm.use_hugepages=true
```

## Environment Variables

Configuration can also be controlled through environment variables:

```bash
FORM_LOG_LEVEL=debug
FORM_NETWORK_FORMNET_PORT=9373
FORM_VM_USE_HUGEPAGES=true
```

Environment variable names are derived from the configuration keys by:
1. Converting to uppercase
2. Prefixing with `FORM_`
3. Converting dots to underscores

## Configuration Wizard

The Form Config Wizard provides an interactive way to configure your operator node. Launch it with:

```bash
sudo form config wizard
```

The wizard will guide you through all configuration options, providing explanations and validation.

## Configuration Best Practices

1. **Security First**
   - Always use SSH key authentication
   - Enable firewall rules to limit access
   - Use strong encryption for disk and network

2. **Performance Tuning**
   - Use hugepages for memory-intensive workloads
   - Enable CPU pinning for workloads sensitive to CPU scheduling
   - Configure the I/O scheduler appropriate for your storage

3. **Resource Allocation**
   - Reserve sufficient resources for the host system
   - Create resource pools based on your hardware capabilities
   - Avoid excessive overcommitment

4. **Network Configuration**
   - Ensure your public IP is correctly configured
   - Open required ports in your network firewall
   - Configure bandwidth limits to prevent resource starvation

5. **Monitoring**
   - Enable Prometheus metrics
   - Set up alerting for critical thresholds
   - Regularly check node performance

6. **Maintenance**
   - Configure a maintenance window during low-usage times
   - Enable automatic backups
   - Keep the system updated

## Example Configurations

### Minimal Configuration

```json
{
  "general": {
    "node_name": "basic-node",
    "operator_address": "0x1234567890abcdef1234567890abcdef12345678",
    "contact_email": "operator@example.com"
  },
  "hardware": {},
  "network": {},
  "blockchain": {},
  "security": {
    "ssh_public_key": "ssh-rsa AAAAB3NzaC1..."
  }
}
```

### High-Performance Node

```json
{
  "general": {
    "node_name": "high-perf-node",
    "operator_address": "0x1234567890abcdef1234567890abcdef12345678",
    "contact_email": "operator@example.com",
    "log_level": "info"
  },
  "hardware": {
    "total_cpus": 64,
    "reserved_cpus": 4,
    "total_memory_mb": 262144,
    "reserved_memory_mb": 8192,
    "total_disk_gb": 2000,
    "reserved_disk_gb": 200,
    "gpu_enabled": true,
    "gpu_devices": [
      {
        "id": "GPU-1234567890abcdef",
        "name": "NVIDIA GeForce RTX 3090",
        "memory_mb": 24576,
        "compute_units": 82
      }
    ],
    "overcommit_ratio": 1.1
  },
  "performance": {
    "io_scheduler": "mq-deadline",
    "cpu_governor": "performance",
    "swappiness": 5,
    "transparent_hugepages": "always",
    "kernel_same_page_merging": true,
    "numa_balancing": true
  },
  "vm": {
    "hypervisor": "kvm",
    "enable_nested_virtualization": true,
    "default_memory_ballooning": true,
    "cpu_pinning_enabled": true,
    "use_hugepages": true,
    "hugepages_size_mb": 1024
  }
}
```

### GPU-Focused Node

```json
{
  "general": {
    "node_name": "gpu-node",
    "operator_address": "0x1234567890abcdef1234567890abcdef12345678",
    "contact_email": "operator@example.com"
  },
  "hardware": {
    "total_cpus": 32,
    "reserved_cpus": 4,
    "total_memory_mb": 131072,
    "reserved_memory_mb": 8192,
    "total_disk_gb": 1000,
    "reserved_disk_gb": 100,
    "gpu_enabled": true,
    "gpu_devices": [
      {
        "id": "GPU-1234567890abc",
        "name": "NVIDIA Tesla V100",
        "memory_mb": 32768,
        "compute_units": 80
      },
      {
        "id": "GPU-456789def",
        "name": "NVIDIA Tesla V100",
        "memory_mb": 32768,
        "compute_units": 80
      }
    ]
  },
  "pricing": {
    "vcpu_price_per_hour": 0.03,
    "memory_price_per_gb_hour": 0.006,
    "disk_price_per_gb_hour": 0.0002,
    "gpu_price_per_hour": 1.5
  },
  "instance_types": [
    {
      "name": "gpu-small",
      "description": "Single GPU instance",
      "vcpu_range": [4, 16],
      "memory_range_mb": [16384, 65536],
      "disk_range_gb": [100, 500],
      "default_vcpus": 8,
      "default_memory_mb": 32768,
      "default_disk_gb": 200,
      "gpu_required": true,
      "enabled": true
    }
  ],
  "resource_pools": [
    {
      "name": "gpu-pool",
      "description": "GPU compute resources",
      "cpus": 28,
      "memory_mb": 122880,
      "disk_gb": 900,
      "gpu_devices": ["GPU-1234567890abc", "GPU-456789def"],
      "priority": "high",
      "allow_overcommit": false,
      "max_instances": 2,
      "instance_types": ["gpu-small"],
      "is_default": true
    }
  ]
}
```

## Troubleshooting Configuration Issues

### Common Problems and Solutions

1. **Configuration File Not Found**
   - Check that the file exists at `~/.config/form/form-config.json`
   - Verify file permissions: `chmod 600 ~/.config/form/form-config.json`

2. **Invalid JSON Format**
   - Use a JSON validator to check syntax
   - Common mistakes include missing commas and unquoted keys

3. **Resource Allocation Issues**
   - Ensure reserved resources are sufficient for the host system
   - Check that total resources don't exceed hardware capabilities

4. **Network Connectivity Problems**
   - Verify public IP is correct and accessible
   - Ensure required ports are open in network firewall
   - Check WireGuard configuration

5. **Blockchain Connectivity Issues**
   - Verify wallet configuration and keystore path
   - Check RPC endpoint connectivity

### Validation

To validate your configuration without applying changes:

```bash
form config validate
```

This will check your configuration for errors and provide detailed messages about any issues found.

### Debugging

To enable debug logging for configuration-related issues:

```bash
form-operator-service --log-level=debug
```

Logs are typically stored in:

```
/var/log/formation/operator.log
``` 