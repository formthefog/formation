# State Service API

The State Service maintains the globally consistent state of the Formation cloud. It is responsible for managing all network resources including users, DNS records, instances, nodes, and accounts.

## API Overview

The State Service operates on port 3004 by default and provides a BFT-CRDT (Byzantine Fault Tolerant Conflict-free Replicated Data Type) based globally replicated datastore, ensuring consistency across all nodes.

## Authentication

API requests to the State Service require authentication using one of the following methods:
- Ethereum wallet signatures for user-facing endpoints
- Node identity verification for node-to-node communication

## Data Types

The State Service manages several types of data:

- **Users**: Network participants
- **DNS Records**: Domain name mappings
- **Instances**: Virtual machine instances
- **Nodes**: Compute nodes in the network
- **Accounts**: User accounts and permissions

Each data type has its own set of API endpoints for retrieval and querying.

## Response Format

The State API uses consistent response types:

```json
{
  "Success": {
    "Some": { ... }  // Single object response
  }
}
```

or

```json
{
  "Success": {
    "List": [ ... ]  // List of objects response
  }
}
```

or

```json
{
  "Failure": {
    "reason": "Error message"
  }
}
```

## Core Endpoints

### Health Check

```
GET /ping
```

Verifies that the State Service is running and responsive.

**Response**:
```
"healthy"
```

## User Management

Users represent network participants who can access and manage resources.

### Get User

```
GET /user/:id/get
```

Retrieves information about a specific user by ID.

**Response**:
```json
{
  "Success": {
    "Some": {
      "id": "user-123456789",
      "contents": {
        "name": "Alice",
        "ip": "10.0.0.1",
        "cidr_id": "cidr-12345",
        "public_key": "0x1234567890abcdef1234567890abcdef12345678",
        "endpoint": null,
        "persistent_keepalive_interval": null,
        "is_admin": false,
        "is_disabled": false,
        "is_redeemed": true,
        "invite_expires": null,
        "candidates": []
      }
    }
  }
}
```

### Get User by IP

```
GET /user/:ip/get_from_ip
```

Retrieves information about a user based on their IP address.

**Response**:
```json
{
  "Success": {
    "Some": {
      "id": "user-123456789",
      "contents": {
        "name": "Alice",
        "ip": "10.0.0.1",
        "cidr_id": "cidr-12345",
        "public_key": "0x1234567890abcdef1234567890abcdef12345678",
        "endpoint": null,
        "persistent_keepalive_interval": null,
        "is_admin": false,
        "is_disabled": false,
        "is_redeemed": true,
        "invite_expires": null,
        "candidates": []
      }
    }
  }
}
```

### Get All Allowed Users

```
GET /user/:id/get_all_allowed
```

Retrieves all users that are allowed to interact with the specified user.

**Response**:
```json
{
  "Success": {
    "List": [
      {
        "id": "user-123456789",
        "contents": {
          "name": "Alice",
          "ip": "10.0.0.1",
          "cidr_id": "cidr-12345",
          "public_key": "0x1234567890abcdef1234567890abcdef12345678",
          "endpoint": null,
          "persistent_keepalive_interval": null,
          "is_admin": false,
          "is_disabled": false,
          "is_redeemed": true,
          "invite_expires": null,
          "candidates": []
        }
      },
      {
        "id": "user-987654321",
        "contents": {
          "name": "Bob",
          "ip": "10.0.0.2",
          "cidr_id": "cidr-12345",
          "public_key": "0x9876543210fedcba9876543210fedcba98765432",
          "endpoint": null,
          "persistent_keepalive_interval": null,
          "is_admin": false,
          "is_disabled": false,
          "is_redeemed": true,
          "invite_expires": null,
          "candidates": []
        }
      }
    ]
  }
}
```

### List Users

```
GET /user/list
```

Retrieves a list of all users.

**Response**:
```json
{
  "Success": {
    "List": [
      {
        "id": "user-123456789",
        "contents": {
          "name": "Alice",
          "ip": "10.0.0.1",
          "cidr_id": "cidr-12345",
          "public_key": "0x1234567890abcdef1234567890abcdef12345678",
          "endpoint": null,
          "persistent_keepalive_interval": null,
          "is_admin": false,
          "is_disabled": false,
          "is_redeemed": true,
          "invite_expires": null,
          "candidates": []
        }
      },
      {
        "id": "user-987654321",
        "contents": {
          "name": "Bob",
          "ip": "10.0.0.2",
          "cidr_id": "cidr-12345",
          "public_key": "0x9876543210fedcba9876543210fedcba98765432",
          "endpoint": null,
          "persistent_keepalive_interval": null,
          "is_admin": false,
          "is_disabled": false,
          "is_redeemed": true,
          "invite_expires": null,
          "candidates": []
        }
      }
    ]
  }
}
```

### List Admin Users

```
GET /user/list_admin
```

Retrieves a list of all users with administrative privileges.

**Response**:
```json
{
  "Success": {
    "List": [
      {
        "id": "user-123456789",
        "contents": {
          "name": "Alice",
          "ip": "10.0.0.1",
          "cidr_id": "cidr-12345",
          "public_key": "0x1234567890abcdef1234567890abcdef12345678",
          "endpoint": null,
          "persistent_keepalive_interval": null,
          "is_admin": true,
          "is_disabled": false,
          "is_redeemed": true,
          "invite_expires": null,
          "candidates": []
        }
      }
    ]
  }
}
```

### List Users by CIDR

```
GET /user/:cidr/list
```

Retrieves a list of users associated with a specific CIDR.

**Response**:
```json
{
  "Success": {
    "List": [
      {
        "id": "user-123456789",
        "contents": {
          "name": "Alice",
          "ip": "10.0.0.1",
          "cidr_id": "cidr-12345",
          "public_key": "0x1234567890abcdef1234567890abcdef12345678",
          "endpoint": null,
          "persistent_keepalive_interval": null,
          "is_admin": false,
          "is_disabled": false,
          "is_redeemed": true,
          "invite_expires": null,
          "candidates": []
        }
      }
    ]
  }
}
```

## DNS Record Management

DNS records map domain names to IP addresses or other resources.

### Get DNS Record

```
GET /dns/:domain/get
```

Retrieves information about a specific DNS record.

**Response**:
```json
{
  "Success": {
    "Some": {
      "domain": "myapp.formation.cloud",
      "record_type": "A",
      "formnet_ip": ["192.168.100.10:80"],
      "public_ip": ["203.0.113.10:80"],
      "cname_target": null,
      "ttl": 300,
      "ssl_cert": false
    }
  }
}
```

### Get DNS Records by Node IP

```
GET /dns/:node_ip/list
```

Retrieves DNS records associated with a specific node IP.

**Response**:
```json
{
  "Success": {
    "List": [
      {
        "domain": "myapp.formation.cloud",
        "record_type": "A",
        "formnet_ip": ["192.168.100.10:80"],
        "public_ip": ["203.0.113.10:80"],
        "cname_target": null,
        "ttl": 300,
        "ssl_cert": false
      },
      {
        "domain": "api.formation.cloud",
        "record_type": "A",
        "formnet_ip": ["192.168.100.11:80"],
        "public_ip": ["203.0.113.11:80"],
        "cname_target": null,
        "ttl": 300,
        "ssl_cert": false
      }
    ]
  }
}
```

### List DNS Records

```
GET /dns/list
```

Retrieves a list of all DNS records.

**Response**:
```json
{
  "Success": {
    "List": [
      {
        "domain": "myapp.formation.cloud",
        "record_type": "A",
        "formnet_ip": ["192.168.100.10:80"],
        "public_ip": ["203.0.113.10:80"],
        "cname_target": null,
        "ttl": 300,
        "ssl_cert": false
      },
      {
        "domain": "api.formation.cloud",
        "record_type": "A",
        "formnet_ip": ["192.168.100.11:80"],
        "public_ip": ["203.0.113.11:80"],
        "cname_target": null,
        "ttl": 300,
        "ssl_cert": false
      }
    ]
  }
}
```

## Instance Management

Instances represent virtual machines running on the network.

### Get Instance

```
GET /instance/:instance_id/get
```

Retrieves information about a specific instance.

**Response**:
```json
{
  "Success": {
    "Some": {
      "instance_id": "instance-123456789",
      "node_id": "node-123456789",
      "build_id": "build-123456789",
      "instance_owner": "user-123456789",
      "formnet_ip": "10.0.0.5",
      "dns_record": {
        "domain": "myapp.formation.cloud",
        "record_type": "A",
        "formnet_ip": ["10.0.0.5:80"],
        "public_ip": ["203.0.113.10:80"],
        "cname_target": null,
        "ttl": 300,
        "ssl_cert": false
      },
      "created_at": 1677721600,
      "updated_at": 1677721600,
      "last_snapshot": 0,
      "status": "Started",
      "host_region": "us-west-1",
      "resources": {
        "vcpus": 2,
        "memory_mb": 2048,
        "bandwidth_mbps": 100,
        "gpu": null
      },
      "cluster": {
        "members": {},
        "scaling_policy": null,
        "template_instance_id": null,
        "session_affinity_enabled": false
      },
      "formfile": "base64-encoded-formfile-content",
      "snapshots": null,
      "metadata": {
        "tags": ["web", "production"],
        "description": "Production web server",
        "annotations": {
          "deployed_by": "user-123456789",
          "network_id": 1,
          "build_commit": "a1b2c3d4"
        },
        "security": {
          "encryption": {
            "is_encrypted": false,
            "scheme": null
          },
          "tee": false,
          "hsm": false
        },
        "monitoring": {
          "logging_enabled": true,
          "metrics_endpoint": "https://metrics.formation.cloud"
        }
      }
    }
  }
}
```

### Get Instance by Build ID

```
GET /instance/:build_id/get_by_build_id
```

Retrieves information about an instance using its build ID.

**Response**:
```json
{
  "Success": {
    "Some": {
      "instance_id": "instance-123456789",
      "node_id": "node-123456789",
      "build_id": "build-123456789",
      "instance_owner": "user-123456789",
      "formnet_ip": "10.0.0.5",
      "dns_record": {
        "domain": "myapp.formation.cloud",
        "record_type": "A",
        "formnet_ip": ["10.0.0.5:80"],
        "public_ip": ["203.0.113.10:80"],
        "cname_target": null,
        "ttl": 300,
        "ssl_cert": false
      },
      "created_at": 1677721600,
      "updated_at": 1677721600,
      "last_snapshot": 0,
      "status": "Started",
      "host_region": "us-west-1",
      "resources": {
        "vcpus": 2,
        "memory_mb": 2048,
        "bandwidth_mbps": 100,
        "gpu": null
      },
      "cluster": {
        "members": {},
        "scaling_policy": null,
        "template_instance_id": null,
        "session_affinity_enabled": false
      },
      "formfile": "base64-encoded-formfile-content",
      "snapshots": null,
      "metadata": {
        "tags": ["web", "production"],
        "description": "Production web server",
        "annotations": {
          "deployed_by": "user-123456789",
          "network_id": 1,
          "build_commit": "a1b2c3d4"
        },
        "security": {
          "encryption": {
            "is_encrypted": false,
            "scheme": null
          },
          "tee": false,
          "hsm": false
        },
        "monitoring": {
          "logging_enabled": true,
          "metrics_endpoint": "https://metrics.formation.cloud"
        }
      }
    }
  }
}
```

### Get Instance IPs

```
GET /instance/:build_id/get_instance_ips
```

Retrieves IP addresses associated with a specific instance.

**Response**:
```json
{
  "Success": {
    "List": [
      "192.168.100.10",
      "192.168.100.11"
    ]
  }
}
```

### Get Instance Metrics

```
GET /instance/:instance_id/metrics
```

Retrieves performance metrics for a specific instance.

**Response**:
```json
{
  "Success": {
    "Some": {
      "load_avg_1": 0,
      "load_avg_5": 0, 
      "load_avg_15": 0,
      "process_count": 42,
      "disk_read_bytes_per_sec": 1024000,
      "disk_write_bytes_per_sec": 512000,
      "network_in_bytes_per_sec": 1024000,
      "network_out_bytes_per_sec": 512000,
      "cpu_temperature": 45,
      "gpu_temperature": null,
      "power_usage_watts": 120
    }
  }
}
```

### List Instance Metrics

```
GET /instance/list/metrics
```

Retrieves performance metrics for all instances.

**Response**:
```json
{
  "Success": {
    "List": [
      {
        "instance_id": "instance-123456789",
        "load_avg_1": 0,
        "load_avg_5": 0, 
        "load_avg_15": 0,
        "process_count": 42,
        "disk_read_bytes_per_sec": 1024000,
        "disk_write_bytes_per_sec": 512000,
        "network_in_bytes_per_sec": 1024000,
        "network_out_bytes_per_sec": 512000,
        "cpu_temperature": 45,
        "gpu_temperature": null,
        "power_usage_watts": 120
      },
      {
        "instance_id": "instance-987654321",
        "load_avg_1": 0,
        "load_avg_5": 0, 
        "load_avg_15": 0,
        "process_count": 35,
        "disk_read_bytes_per_sec": 2048000,
        "disk_write_bytes_per_sec": 1024000,
        "network_in_bytes_per_sec": 2048000,
        "network_out_bytes_per_sec": 1024000,
        "cpu_temperature": 50,
        "gpu_temperature": null,
        "power_usage_watts": 150
      }
    ]
  }
}
```

### Get Cluster Metrics

```
GET /cluster/:build_id/metrics
```

Retrieves performance metrics for a specific cluster.

**Response**:
```json
{
  "Success": {
    "Some": {
      "cluster_id": "cluster-123456789",
      "members": [
        {
          "instance_id": "instance-123456789",
          "load_avg_1": 0,
          "load_avg_5": 0, 
          "load_avg_15": 0,
          "process_count": 42,
          "disk_read_bytes_per_sec": 1024000,
          "disk_write_bytes_per_sec": 512000,
          "network_in_bytes_per_sec": 1024000,
          "network_out_bytes_per_sec": 512000,
          "cpu_temperature": 45,
          "gpu_temperature": null,
          "power_usage_watts": 120
        },
        {
          "instance_id": "instance-987654321",
          "load_avg_1": 0,
          "load_avg_5": 0, 
          "load_avg_15": 0,
          "process_count": 35,
          "disk_read_bytes_per_sec": 2048000,
          "disk_write_bytes_per_sec": 1024000,
          "network_in_bytes_per_sec": 2048000,
          "network_out_bytes_per_sec": 1024000,
          "cpu_temperature": 50,
          "gpu_temperature": null,
          "power_usage_watts": 150
        }
      ]
    }
  }
}
```

### List Instances

```
GET /instance/list
```

Retrieves a list of all instances.

**Response**:
```json
{
  "Success": {
    "List": [
      {
        "instance_id": "instance-123456789",
        "node_id": "node-123456789",
        "build_id": "build-123456789",
        "instance_owner": "user-123456789",
        "formnet_ip": "10.0.0.5",
        "dns_record": {
          "domain": "myapp.formation.cloud",
          "record_type": "A",
          "formnet_ip": ["10.0.0.5:80"],
          "public_ip": ["203.0.113.10:80"],
          "cname_target": null,
          "ttl": 300,
          "ssl_cert": false
        },
        "created_at": 1677721600,
        "updated_at": 1677721600,
        "last_snapshot": 0,
        "status": "Started",
        "host_region": "us-west-1",
        "resources": {
          "vcpus": 2,
          "memory_mb": 2048,
          "bandwidth_mbps": 100,
          "gpu": null
        },
        "cluster": {
          "members": {},
          "scaling_policy": null,
          "template_instance_id": null,
          "session_affinity_enabled": false
        },
        "formfile": "base64-encoded-formfile-content",
        "snapshots": null,
        "metadata": {
          "tags": ["web", "production"],
          "description": "Production web server",
          "annotations": {
            "deployed_by": "user-123456789",
            "network_id": 1,
            "build_commit": "a1b2c3d4"
          },
          "security": {
            "encryption": {
              "is_encrypted": false,
              "scheme": null
            },
            "tee": false,
            "hsm": false
          },
          "monitoring": {
            "logging_enabled": true,
            "metrics_endpoint": "https://metrics.formation.cloud"
          }
        }
      },
      {
        "instance_id": "instance-987654321",
        "node_id": "node-987654321",
        "build_id": "build-987654321",
        "instance_owner": "user-123456789",
        "formnet_ip": "10.0.0.6",
        "dns_record": {
          "domain": "api.formation.cloud",
          "record_type": "A",
          "formnet_ip": ["10.0.0.6:80"],
          "public_ip": ["203.0.113.11:80"],
          "cname_target": null,
          "ttl": 300,
          "ssl_cert": false
        },
        "created_at": 1677722600,
        "updated_at": 1677722600,
        "last_snapshot": 0,
        "status": "Stopped",
        "host_region": "us-west-1",
        "resources": {
          "vcpus": 4,
          "memory_mb": 4096,
          "bandwidth_mbps": 100,
          "gpu": null
        },
        "cluster": {
          "members": {},
          "scaling_policy": null,
          "template_instance_id": null,
          "session_affinity_enabled": false
        },
        "formfile": "base64-encoded-formfile-content",
        "snapshots": null,
        "metadata": {
          "tags": ["api", "production"],
          "description": "Production API server",
          "annotations": {
            "deployed_by": "user-123456789",
            "network_id": 1,
            "build_commit": "e5f6g7h8"
          },
          "security": {
            "encryption": {
              "is_encrypted": false,
              "scheme": null
            },
            "tee": false,
            "hsm": false
          },
          "monitoring": {
            "logging_enabled": true,
            "metrics_endpoint": "https://metrics.formation.cloud"
          }
        }
      }
    ]
  }
}
```

## Node Management

Nodes represent compute resources in the network.

### Get Node

```
GET /node/:id/get
```

Retrieves information about a specific node.

**Response**:
```json
{
  "Success": {
    "Some": {
      "node_id": "node-123456789",
      "node_owner": "user-123456789",
      "created_at": 1677721600,
      "updated_at": 1677721600,
      "last_heartbeat": 1677723600,
      "host_region": "us-west-1",
      "capabilities": {
        "cpu_model": "Intel(R) Xeon(R) CPU E5-2678 v3 @ 2.50GHz",
        "cpu_cores": 16,
        "total_memory": 32768,
        "total_storage": 1000000000000,
        "gpu_models": [],
        "network_interfaces": [],
        "tpm": null,
        "sgx": null,
        "sev": null,
        "virtualization_type": "BareMetal"
      },
      "capacity": {
        "cpu_total_cores": 16,
        "cpu_available_cores": 12000,
        "memory_total_bytes": 34359738368,
        "memory_available_bytes": 30064771072,
        "storage_total_bytes": 1073741824000,
        "storage_available_bytes": 966367641600,
        "gpu_total_memory_bytes": 0,
        "gpu_available_memory_bytes": 0,
        "network_total_bandwidth": 0,
        "network_available_bandwidth": 0
      },
      "metrics": {
        "load_avg_1": 125,
        "load_avg_5": 150,
        "load_avg_15": 180,
        "process_count": 324,
        "disk_read_bytes_per_sec": 10240000,
        "disk_write_bytes_per_sec": 5120000,
        "network_in_bytes_per_sec": 10240000,
        "network_out_bytes_per_sec": 5120000,
        "cpu_temperature": 55,
        "gpu_temperature": null,
        "power_usage_watts": 220
      },
      "metadata": {
        "tags": ["compute", "production"],
        "description": "Production compute node",
        "annotations": {
          "roles": ["compute", "storage"],
          "datacenter": "dc-west-1"
        },
        "monitoring": {
          "logging_enabled": true,
          "metrics_endpoint": "https://metrics.formation.cloud"
        }
      },
      "host": {
        "Domain": "node1.formation.cloud"
      }
    }
  }
}
```

### Get Node Metrics

```
GET /node/:id/metrics
```

Retrieves performance metrics for a specific node.

**Response**:
```json
{
  "Success": {
    "Some": {
      "load_avg_1": 125,
      "load_avg_5": 150,
      "load_avg_15": 180,
      "process_count": 324,
      "disk_read_bytes_per_sec": 10240000,
      "disk_write_bytes_per_sec": 5120000,
      "network_in_bytes_per_sec": 10240000,
      "network_out_bytes_per_sec": 5120000,
      "cpu_temperature": 55,
      "gpu_temperature": null,
      "power_usage_watts": 220
    }
  }
}
```

### List Node Metrics

```
GET /node/list/metrics
```

Retrieves performance metrics for all nodes.

**Response**:
```json
{
  "Success": {
    "List": [
      {
        "node_id": "node-123456789",
        "load_avg_1": 125,
        "load_avg_5": 150,
        "load_avg_15": 180,
        "process_count": 324,
        "disk_read_bytes_per_sec": 10240000,
        "disk_write_bytes_per_sec": 5120000,
        "network_in_bytes_per_sec": 10240000,
        "network_out_bytes_per_sec": 5120000,
        "cpu_temperature": 55,
        "gpu_temperature": null,
        "power_usage_watts": 220
      },
      {
        "node_id": "node-987654321",
        "load_avg_1": 100,
        "load_avg_5": 120,
        "load_avg_15": 140,
        "process_count": 245,
        "disk_read_bytes_per_sec": 20480000,
        "disk_write_bytes_per_sec": 10240000,
        "network_in_bytes_per_sec": 20480000,
        "network_out_bytes_per_sec": 10240000,
        "cpu_temperature": 50,
        "gpu_temperature": null,
        "power_usage_watts": 200
      }
    ]
  }
}
```

### List Nodes

```
GET /node/list
```

Retrieves a list of all nodes.

**Response**:
```json
{
  "Success": {
    "List": [
      {
        "node_id": "node-123456789",
        "node_owner": "user-123456789",
        "created_at": 1677721600,
        "updated_at": 1677721600,
        "last_heartbeat": 1677723600,
        "host_region": "us-west-1",
        "capabilities": {
          "cpu_model": "Intel(R) Xeon(R) CPU E5-2678 v3 @ 2.50GHz",
          "cpu_cores": 16,
          "total_memory": 32768,
          "total_storage": 1000000000000,
          "gpu_models": [],
          "network_interfaces": [],
          "tpm": null,
          "sgx": null,
          "sev": null,
          "virtualization_type": "BareMetal"
        },
        "capacity": {
          "cpu_total_cores": 16,
          "cpu_available_cores": 12000,
          "memory_total_bytes": 34359738368,
          "memory_available_bytes": 30064771072,
          "storage_total_bytes": 1073741824000,
          "storage_available_bytes": 966367641600,
          "gpu_total_memory_bytes": 0,
          "gpu_available_memory_bytes": 0,
          "network_total_bandwidth": 0,
          "network_available_bandwidth": 0
        },
        "metrics": {
          "load_avg_1": 125,
          "load_avg_5": 150,
          "load_avg_15": 180,
          "process_count": 324,
          "disk_read_bytes_per_sec": 10240000,
          "disk_write_bytes_per_sec": 5120000,
          "network_in_bytes_per_sec": 10240000,
          "network_out_bytes_per_sec": 5120000,
          "cpu_temperature": 55,
          "gpu_temperature": null,
          "power_usage_watts": 220
        },
        "metadata": {
          "tags": ["compute", "production"],
          "description": "Production compute node",
          "annotations": {
            "roles": ["compute", "storage"],
            "datacenter": "dc-west-1"
          },
          "monitoring": {
            "logging_enabled": true,
            "metrics_endpoint": "https://metrics.formation.cloud"
          }
        },
        "host": {
          "Domain": "node1.formation.cloud"
        }
      },
      {
        "node_id": "node-987654321",
        "node_owner": "user-987654321",
        "created_at": 1677722600,
        "updated_at": 1677722600,
        "last_heartbeat": 1677723600,
        "host_region": "us-east-1",
        "capabilities": {
          "cpu_model": "AMD EPYC 7642 48-Core Processor",
          "cpu_cores": 32,
          "total_memory": 65536,
          "total_storage": 2000000000000,
          "gpu_models": [],
          "network_interfaces": [],
          "tpm": null,
          "sgx": null,
          "sev": null,
          "virtualization_type": "BareMetal"
        },
        "capacity": {
          "cpu_total_cores": 32,
          "cpu_available_cores": 24000,
          "memory_total_bytes": 68719476736,
          "memory_available_bytes": 51539607552,
          "storage_total_bytes": 2147483648000,
          "storage_available_bytes": 1932735283200,
          "gpu_total_memory_bytes": 0,
          "gpu_available_memory_bytes": 0,
          "network_total_bandwidth": 0,
          "network_available_bandwidth": 0
        },
        "metrics": {
          "load_avg_1": 100,
          "load_avg_5": 120,
          "load_avg_15": 140,
          "process_count": 245,
          "disk_read_bytes_per_sec": 20480000,
          "disk_write_bytes_per_sec": 10240000,
          "network_in_bytes_per_sec": 20480000,
          "network_out_bytes_per_sec": 10240000,
          "cpu_temperature": 50,
          "gpu_temperature": null,
          "power_usage_watts": 200
        },
        "metadata": {
          "tags": ["compute", "production"],
          "description": "Production compute node",
          "annotations": {
            "roles": ["compute", "storage"],
            "datacenter": "dc-east-1"
          },
          "monitoring": {
            "logging_enabled": true,
            "metrics_endpoint": "https://metrics.formation.cloud"
          }
        },
        "host": {
          "Domain": "node2.formation.cloud"
        }
      }
    ]
  }
}
```

## Account Management

Accounts represent user accounts with authentication and permission information.

### Get Account

```
GET /account/:address/get
```

Retrieves information about a specific account by Ethereum address.

**Response**:
```json
{
  "Success": {
    "Some": {
      "address": "0x1234567890abcdef1234567890abcdef12345678",
      "name": "Alice",
      "owned_instances": [
        "instance-123456789",
        "instance-987654321"
      ],
      "authorized_instances": {
        "instance-123456789": "Owner",
        "instance-987654321": "Owner",
        "instance-456789123": "Manager"
      },
      "created_at": 1677721600,
      "updated_at": 1677721600
    }
  }
}
```

### List Accounts

```
GET /account/list
```

Retrieves a list of all accounts.

**Response**:
```json
{
  "Success": {
    "List": [
      {
        "address": "0x1234567890abcdef1234567890abcdef12345678",
        "name": "Alice",
        "owned_instances": [
          "instance-123456789",
          "instance-987654321"
        ],
        "authorized_instances": {
          "instance-123456789": "Owner",
          "instance-987654321": "Owner",
          "instance-456789123": "Manager"
        },
        "created_at": 1677721600,
        "updated_at": 1677721600
      },
      {
        "address": "0x9876543210fedcba9876543210fedcba98765432",
        "name": "Bob",
        "owned_instances": [
          "instance-456789123"
        ],
        "authorized_instances": {
          "instance-456789123": "Owner",
          "instance-123456789": "ReadOnly"
        },
        "created_at": 1677722600,
        "updated_at": 1677722600
      }
    ]
  }
}
```

## Error Handling

The State Service API returns standard HTTP status codes:

- 200: Success
- 400: Bad Request (invalid parameters)
- 401: Unauthorized (authentication failure)
- 403: Forbidden (insufficient permissions)
- 404: Not Found (resource not found)
- 409: Conflict (resource already exists with the same unique identifiers)
- 500: Internal Server Error

Error responses include a JSON object with:
```json
{
  "Failure": {
    "reason": "Descriptive error message"
  }
}
```

## SDK Integration

The Formation SDK provides wrapper functions for the State Service API:

```javascript
const Formation = require('formation-sdk');

// Initialize the SDK
const formation = new Formation({
  apiKey: 'your-api-key'
});

// Get a DNS record
const dnsRecord = await formation.state.getDnsRecord('myapp.formation.cloud');
console.log(dnsRecord);

// List all instances
const instances = await formation.state.listInstances();
console.log(instances);
```

## Implementation Considerations

When working with the State Service API, keep these considerations in mind:

1. **Consistency**: The BFT-CRDT database ensures that all nodes will eventually have the same state, but there may be a slight delay in propagation.
2. **Read-only Access**: Most users should only use the GET endpoints documented here. The POST, PUT, and DELETE endpoints are primarily for internal use.
3. **Pagination**: For list endpoints that may return many items, use the `limit` and `offset` query parameters to paginate results.
4. **Filtering**: Most list endpoints support filtering parameters to narrow down results.
5. **Performance**: For critical paths, consider caching frequently accessed data locally. 