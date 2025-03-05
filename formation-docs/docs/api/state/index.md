# State Service API

The State Service maintains the globally consistent state of the Formation cloud. It is responsible for managing all network resources including peers, CIDRs, associations, DNS records, instances, nodes, and accounts.

## API Overview

The State Service operates on port 3004 by default and provides a BFT-CRDT (Byzantine Fault Tolerant Conflict-free Replicated Data Type) based globally replicated datastore, ensuring consistency across all nodes.

## Authentication

API requests to the State Service require authentication using one of the following methods:
- Ethereum wallet signatures for user-facing endpoints
- Node identity verification for node-to-node communication

## Data Types

The State Service manages several types of data:

- **Peers**: Network participants (users)
- **CIDRs**: IP address ranges for network segmentation
- **Associations**: Relationships between CIDRs
- **DNS Records**: Domain name mappings
- **Instances**: Virtual machine instances
- **Nodes**: Compute nodes in the network
- **Accounts**: User accounts and permissions

Each data type has its own set of API endpoints for creation, retrieval, updating, and deletion.

## Core Endpoints

### Health Check

```
GET /health
```

Verifies that the State Service is running and responsive.

**Response**:
```
"healthy"
```

## Peer Management

Peers represent users or services that participate in the network.

### Create Peer

```
POST /peers/create
```

Creates a new peer.

**Request Body**:
```json
{
  "name": "Alice",
  "public_key": "0x1234567890abcdef1234567890abcdef12345678",
  "peer_type": "user"
}
```

**Response**:
```json
{
  "success": true,
  "peer_id": "peer-123456789",
  "created_at": 1677721600
}
```

### Get Peer

```
GET /peers/{id}
```

Retrieves information about a specific peer.

**Response**:
```json
{
  "id": "peer-123456789",
  "name": "Alice",
  "public_key": "0x1234567890abcdef1234567890abcdef12345678",
  "peer_type": "user",
  "created_at": 1677721600,
  "updated_at": 1677721600
}
```

### Update Peer

```
POST /peers/update
```

Updates an existing peer's information.

**Request Body**:
```json
{
  "id": "peer-123456789",
  "name": "Alice (Updated)",
  "peer_type": "user"
}
```

**Response**:
```json
{
  "success": true,
  "peer_id": "peer-123456789",
  "updated_at": 1677722600
}
```

### Delete Peer

```
DELETE /peers/{id}
```

Deletes a peer.

**Response**:
```json
{
  "success": true,
  "peer_id": "peer-123456789",
  "deleted_at": 1677723600
}
```

### List Peers

```
GET /peers
```

Retrieves a list of all peers.

**Response**:
```json
{
  "success": true,
  "peers": [
    {
      "id": "peer-123456789",
      "name": "Alice",
      "public_key": "0x1234567890abcdef1234567890abcdef12345678",
      "peer_type": "user",
      "created_at": 1677721600,
      "updated_at": 1677721600
    },
    {
      "id": "peer-987654321",
      "name": "Bob",
      "public_key": "0x9876543210fedcba9876543210fedcba98765432",
      "peer_type": "user",
      "created_at": 1677722600,
      "updated_at": 1677722600
    }
  ]
}
```

## CIDR Management

CIDRs represent IP address ranges used for network segmentation.

### Create CIDR

```
POST /cidrs/create
```

Creates a new CIDR.

**Request Body**:
```json
{
  "cidr": "192.168.100.0/24",
  "name": "VM Network",
  "owner": "peer-123456789"
}
```

**Response**:
```json
{
  "success": true,
  "cidr_id": "cidr-123456789",
  "created_at": 1677721600
}
```

### Get CIDR

```
GET /cidrs/{id}
```

Retrieves information about a specific CIDR.

**Response**:
```json
{
  "id": "cidr-123456789",
  "cidr": "192.168.100.0/24",
  "name": "VM Network",
  "owner": "peer-123456789",
  "created_at": 1677721600,
  "updated_at": 1677721600
}
```

### Update CIDR

```
POST /cidrs/update
```

Updates an existing CIDR's information.

**Request Body**:
```json
{
  "id": "cidr-123456789",
  "name": "VM Network (Updated)",
  "owner": "peer-123456789"
}
```

**Response**:
```json
{
  "success": true,
  "cidr_id": "cidr-123456789",
  "updated_at": 1677722600
}
```

### Delete CIDR

```
DELETE /cidrs/{id}
```

Deletes a CIDR.

**Response**:
```json
{
  "success": true,
  "cidr_id": "cidr-123456789",
  "deleted_at": 1677723600
}
```

### List CIDRs

```
GET /cidrs
```

Retrieves a list of all CIDRs.

**Response**:
```json
{
  "success": true,
  "cidrs": [
    {
      "id": "cidr-123456789",
      "cidr": "192.168.100.0/24",
      "name": "VM Network",
      "owner": "peer-123456789",
      "created_at": 1677721600,
      "updated_at": 1677721600
    },
    {
      "id": "cidr-987654321",
      "cidr": "192.168.101.0/24",
      "name": "Control Network",
      "owner": "peer-987654321",
      "created_at": 1677722600,
      "updated_at": 1677722600
    }
  ]
}
```

## Association Management

Associations represent relationships between CIDRs.

### Create Association

```
POST /associations/create
```

Creates a new association between CIDRs.

**Request Body**:
```json
{
  "name": "VM to Control",
  "source_cidr": "cidr-123456789",
  "target_cidr": "cidr-987654321",
  "allow_bidirectional": true
}
```

**Response**:
```json
{
  "success": true,
  "association_id": "assoc-123456789",
  "created_at": 1677721600
}
```

### Get Association

```
GET /associations/{id}
```

Retrieves information about a specific association.

**Response**:
```json
{
  "id": "assoc-123456789",
  "name": "VM to Control",
  "source_cidr": "cidr-123456789",
  "target_cidr": "cidr-987654321",
  "allow_bidirectional": true,
  "created_at": 1677721600,
  "updated_at": 1677721600
}
```

### Update Association

```
POST /associations/update
```

Updates an existing association's information.

**Request Body**:
```json
{
  "id": "assoc-123456789",
  "name": "VM to Control (Updated)",
  "allow_bidirectional": false
}
```

**Response**:
```json
{
  "success": true,
  "association_id": "assoc-123456789",
  "updated_at": 1677722600
}
```

### Delete Association

```
DELETE /associations/{id}
```

Deletes an association.

**Response**:
```json
{
  "success": true,
  "association_id": "assoc-123456789",
  "deleted_at": 1677723600
}
```

### List Associations

```
GET /associations
```

Retrieves a list of all associations.

**Response**:
```json
{
  "success": true,
  "associations": [
    {
      "id": "assoc-123456789",
      "name": "VM to Control",
      "source_cidr": "cidr-123456789",
      "target_cidr": "cidr-987654321",
      "allow_bidirectional": true,
      "created_at": 1677721600,
      "updated_at": 1677721600
    },
    {
      "id": "assoc-987654321",
      "name": "VM to Public",
      "source_cidr": "cidr-123456789",
      "target_cidr": "cidr-456789123",
      "allow_bidirectional": false,
      "created_at": 1677722600,
      "updated_at": 1677722600
    }
  ]
}
```

## DNS Record Management

DNS records map domain names to IP addresses or other resources.

### Create DNS Record

```
POST /dns/create
```

Creates a new DNS record.

**Request Body**:
```json
{
  "domain": "myapp.formation.cloud",
  "record_type": "A",
  "value": "192.168.100.10",
  "ttl": 300
}
```

**Response**:
```json
{
  "success": true,
  "dns_id": "dns-123456789",
  "created_at": 1677721600
}
```

### Get DNS Record

```
GET /dns/{domain}
```

Retrieves information about a specific DNS record.

**Response**:
```json
{
  "id": "dns-123456789",
  "domain": "myapp.formation.cloud",
  "record_type": "A",
  "value": "192.168.100.10",
  "ttl": 300,
  "created_at": 1677721600,
  "updated_at": 1677721600
}
```

### Update DNS Record

```
POST /dns/update
```

Updates an existing DNS record.

**Request Body**:
```json
{
  "domain": "myapp.formation.cloud",
  "record_type": "A",
  "value": "192.168.100.11",
  "ttl": 600
}
```

**Response**:
```json
{
  "success": true,
  "dns_id": "dns-123456789",
  "updated_at": 1677722600
}
```

### Delete DNS Record

```
DELETE /dns/{domain}
```

Deletes a DNS record.

**Response**:
```json
{
  "success": true,
  "dns_id": "dns-123456789",
  "deleted_at": 1677723600
}
```

### List DNS Records

```
GET /dns
```

Retrieves a list of all DNS records.

**Response**:
```json
{
  "success": true,
  "dns_records": [
    {
      "id": "dns-123456789",
      "domain": "myapp.formation.cloud",
      "record_type": "A",
      "value": "192.168.100.10",
      "ttl": 300,
      "created_at": 1677721600,
      "updated_at": 1677721600
    },
    {
      "id": "dns-987654321",
      "domain": "api.formation.cloud",
      "record_type": "A",
      "value": "192.168.100.11",
      "ttl": 300,
      "created_at": 1677722600,
      "updated_at": 1677722600
    }
  ]
}
```

## Instance Management

Instances represent virtual machines running on the network.

### Create Instance

```
POST /instances/create
```

Creates a new instance record.

**Request Body**:
```json
{
  "build_id": "build-123456789",
  "name": "web-server",
  "owner": "peer-123456789",
  "vcpu_count": 2,
  "mem_size_mib": 2048,
  "disk_size_gb": 10
}
```

**Response**:
```json
{
  "success": true,
  "instance_id": "instance-123456789",
  "created_at": 1677721600
}
```

### Get Instance

```
GET /instances/{id}
```

Retrieves information about a specific instance.

**Response**:
```json
{
  "id": "instance-123456789",
  "build_id": "build-123456789",
  "name": "web-server",
  "owner": "peer-123456789",
  "vcpu_count": 2,
  "mem_size_mib": 2048,
  "disk_size_gb": 10,
  "state": "running",
  "created_at": 1677721600,
  "updated_at": 1677721600
}
```

### Update Instance

```
POST /instances/update
```

Updates an existing instance's information.

**Request Body**:
```json
{
  "id": "instance-123456789",
  "name": "web-server-updated",
  "state": "stopped"
}
```

**Response**:
```json
{
  "success": true,
  "instance_id": "instance-123456789",
  "updated_at": 1677722600
}
```

### Delete Instance

```
DELETE /instances/{id}
```

Deletes an instance.

**Response**:
```json
{
  "success": true,
  "instance_id": "instance-123456789",
  "deleted_at": 1677723600
}
```

### List Instances

```
GET /instances
```

Retrieves a list of all instances.

**Response**:
```json
{
  "success": true,
  "instances": [
    {
      "id": "instance-123456789",
      "build_id": "build-123456789",
      "name": "web-server",
      "owner": "peer-123456789",
      "vcpu_count": 2,
      "mem_size_mib": 2048,
      "disk_size_gb": 10,
      "state": "running",
      "created_at": 1677721600,
      "updated_at": 1677721600
    },
    {
      "id": "instance-987654321",
      "build_id": "build-987654321",
      "name": "database",
      "owner": "peer-123456789",
      "vcpu_count": 4,
      "mem_size_mib": 4096,
      "disk_size_gb": 20,
      "state": "stopped",
      "created_at": 1677722600,
      "updated_at": 1677722600
    }
  ]
}
```

## Node Management

Nodes represent compute resources in the network.

### Create Node

```
POST /nodes/create
```

Creates a new node record.

**Request Body**:
```json
{
  "name": "Worker-1",
  "public_key": "WFmc3ixj8Ue4qZEQRTH+GYKJmUFQd2H4UBW5BJdXpXE=",
  "endpoint": "203.0.113.10:51820",
  "total_vcpus": 16,
  "total_memory_mib": 32768,
  "total_disk_gb": 1000,
  "operator_id": "peer-123456789"
}
```

**Response**:
```json
{
  "success": true,
  "node_id": "node-123456789",
  "created_at": 1677721600
}
```

### Get Node

```
GET /nodes/{id}
```

Retrieves information about a specific node.

**Response**:
```json
{
  "id": "node-123456789",
  "name": "Worker-1",
  "public_key": "WFmc3ixj8Ue4qZEQRTH+GYKJmUFQd2H4UBW5BJdXpXE=",
  "endpoint": "203.0.113.10:51820",
  "total_vcpus": 16,
  "total_memory_mib": 32768,
  "total_disk_gb": 1000,
  "available_vcpus": 12,
  "available_memory_mib": 28672,
  "available_disk_gb": 900,
  "operator_id": "peer-123456789",
  "state": "online",
  "created_at": 1677721600,
  "updated_at": 1677721600
}
```

### Update Node

```
POST /nodes/update
```

Updates an existing node's information.

**Request Body**:
```json
{
  "id": "node-123456789",
  "name": "Worker-1-Updated",
  "endpoint": "203.0.113.20:51820",
  "state": "maintenance"
}
```

**Response**:
```json
{
  "success": true,
  "node_id": "node-123456789",
  "updated_at": 1677722600
}
```

### Delete Node

```
DELETE /nodes/{id}
```

Deletes a node.

**Response**:
```json
{
  "success": true,
  "node_id": "node-123456789",
  "deleted_at": 1677723600
}
```

### List Nodes

```
GET /nodes
```

Retrieves a list of all nodes.

**Response**:
```json
{
  "success": true,
  "nodes": [
    {
      "id": "node-123456789",
      "name": "Worker-1",
      "public_key": "WFmc3ixj8Ue4qZEQRTH+GYKJmUFQd2H4UBW5BJdXpXE=",
      "endpoint": "203.0.113.10:51820",
      "total_vcpus": 16,
      "total_memory_mib": 32768,
      "total_disk_gb": 1000,
      "available_vcpus": 12,
      "available_memory_mib": 28672,
      "available_disk_gb": 900,
      "operator_id": "peer-123456789",
      "state": "online",
      "created_at": 1677721600,
      "updated_at": 1677721600
    },
    {
      "id": "node-987654321",
      "name": "Worker-2",
      "public_key": "xTIBA5rboUvnH4htodjb6e+4dOEcNqkq/JZFJpBYCnM=",
      "endpoint": "203.0.113.11:51820",
      "total_vcpus": 32,
      "total_memory_mib": 65536,
      "total_disk_gb": 2000,
      "available_vcpus": 24,
      "available_memory_mib": 49152,
      "available_disk_gb": 1800,
      "operator_id": "peer-987654321",
      "state": "online",
      "created_at": 1677722600,
      "updated_at": 1677722600
    }
  ]
}
```

## Account Management

Accounts represent user accounts with authentication and permission information.

### Create Account

```
POST /accounts/create
```

Creates a new account.

**Request Body**:
```json
{
  "name": "Alice",
  "ethereum_address": "0x1234567890abcdef1234567890abcdef12345678",
  "email": "alice@example.com",
  "role": "user"
}
```

**Response**:
```json
{
  "success": true,
  "account_id": "account-123456789",
  "created_at": 1677721600
}
```

### Get Account

```
GET /accounts/{id}
```

Retrieves information about a specific account.

**Response**:
```json
{
  "id": "account-123456789",
  "name": "Alice",
  "ethereum_address": "0x1234567890abcdef1234567890abcdef12345678",
  "email": "alice@example.com",
  "role": "user",
  "created_at": 1677721600,
  "updated_at": 1677721600
}
```

### Update Account

```
POST /accounts/update
```

Updates an existing account's information.

**Request Body**:
```json
{
  "id": "account-123456789",
  "name": "Alice Smith",
  "email": "alice.smith@example.com",
  "role": "admin"
}
```

**Response**:
```json
{
  "success": true,
  "account_id": "account-123456789",
  "updated_at": 1677722600
}
```

### Delete Account

```
DELETE /accounts/{id}
```

Deletes an account.

**Response**:
```json
{
  "success": true,
  "account_id": "account-123456789",
  "deleted_at": 1677723600
}
```

### List Accounts

```
GET /accounts
```

Retrieves a list of all accounts.

**Response**:
```json
{
  "success": true,
  "accounts": [
    {
      "id": "account-123456789",
      "name": "Alice",
      "ethereum_address": "0x1234567890abcdef1234567890abcdef12345678",
      "email": "alice@example.com",
      "role": "user",
      "created_at": 1677721600,
      "updated_at": 1677721600
    },
    {
      "id": "account-987654321",
      "name": "Bob",
      "ethereum_address": "0x9876543210fedcba9876543210fedcba98765432",
      "email": "bob@example.com",
      "role": "user",
      "created_at": 1677722600,
      "updated_at": 1677722600
    }
  ]
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
  "success": false,
  "error": "Descriptive error message",
  "code": "ERROR_CODE"
}
```

## Example Usage

### Creating a Node

```bash
curl -X POST https://node.formation.cloud:3004/nodes/create \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer <JWT_TOKEN>" \
  -d '{
    "name": "Worker-1",
    "public_key": "WFmc3ixj8Ue4qZEQRTH+GYKJmUFQd2H4UBW5BJdXpXE=",
    "endpoint": "203.0.113.10:51820",
    "total_vcpus": 16,
    "total_memory_mib": 32768,
    "total_disk_gb": 1000,
    "operator_id": "peer-123456789"
  }'
```

### Getting an Instance

```bash
curl -X GET https://node.formation.cloud:3004/instances/build-123456789 \
  -H "Authorization: Bearer <JWT_TOKEN>"
```

## SDK Integration

The Formation SDK provides wrapper functions for the State Service API:

```javascript
const Formation = require('formation-sdk');

// Initialize the SDK
const formation = new Formation({
  apiKey: 'your-api-key'
});

// Create a DNS record
const dnsRecord = await formation.state.createDnsRecord({
  domain: 'myapp.formation.cloud',
  recordType: 'A',
  value: '192.168.100.10',
  ttl: 300
});
console.log(dnsRecord);

// List all instances
const instances = await formation.state.listInstances();
console.log(instances);
```

## Implementation Considerations

When working with the State Service API, keep these considerations in mind:

1. **Consistency**: The BFT-CRDT database ensures that all nodes will eventually have the same state, but there may be a slight delay in propagation.
2. **Idempotency**: API operations are designed to be idempotent, allowing for safe retries.
3. **Pagination**: For list endpoints that may return many items, use the `limit` and `offset` query parameters to paginate results.
4. **Filtering**: Most list endpoints support filtering parameters to narrow down results.
5. **Performance**: For critical paths, consider caching frequently accessed data locally. 