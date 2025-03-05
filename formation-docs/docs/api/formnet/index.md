# Formnet API

Formnet is Formation's secure networking layer that provides encrypted connectivity between nodes and instances. It is built on WireGuard and enables the creation of virtual private networks with automatic configuration and peer discovery.

## API Overview

The Formnet API operates on port 3003 by default and provides endpoints for managing network connections, interfaces, and routing within the Formation cloud.

## Authentication

API requests to the Formnet API require node identity verification through the `Authorization` header, which should contain a node-specific token derived from its private key.

## Endpoints

### Health Check

```
GET /health
```

Verifies that the Formnet service is running and responsive.

**Response**:
```
"healthy"
```

### Get Interface Status

```
GET /interface
```

Retrieves the status of the Formnet interface.

**Response**:
```json
{
  "name": "formnet0",
  "public_key": "WFmc3ixj8Ue4qZEQRTH+GYKJmUFQd2H4UBW5BJdXpXE=",
  "listen_port": 51820,
  "ip_address": "192.168.100.1/24",
  "status": "up",
  "peers": 12,
  "tx_bytes": 15678901,
  "rx_bytes": 12345678
}
```

### List Peers

```
GET /peers
```

Retrieves a list of all peers connected to the node.

**Response**:
```json
{
  "peers": [
    {
      "id": "peer-123456789",
      "name": "Node1",
      "public_key": "gN65BkIKy1eCE9pP1wdc8ROUtkHLF2PfAqYdyYBuZQA=",
      "endpoint": "203.0.113.10:51820",
      "allowed_ips": ["192.168.100.2/32"],
      "latest_handshake": 1677721600,
      "transfer_rx": 1234567,
      "transfer_tx": 7654321,
      "persistent_keepalive": 25
    },
    {
      "id": "peer-987654321",
      "name": "Node2",
      "public_key": "xTIBA5rboUvnH4htodjb6e+4dOEcNqkq/JZFJpBYCnM=",
      "endpoint": "203.0.113.11:51820",
      "allowed_ips": ["192.168.100.3/32"],
      "latest_handshake": 1677721700,
      "transfer_rx": 2345678,
      "transfer_tx": 8765432,
      "persistent_keepalive": 25
    }
  ]
}
```

### Add Peer

```
POST /peers/add
```

Adds a new peer to the Formnet interface.

**Request Body**:
```json
{
  "public_key": "gN65BkIKy1eCE9pP1wdc8ROUtkHLF2PfAqYdyYBuZQA=",
  "allowed_ips": ["192.168.100.2/32"],
  "endpoint": "203.0.113.10:51820",
  "persistent_keepalive": 25
}
```

**Response**:
```json
{
  "success": true,
  "peer": {
    "public_key": "gN65BkIKy1eCE9pP1wdc8ROUtkHLF2PfAqYdyYBuZQA=",
    "allowed_ips": ["192.168.100.2/32"],
    "endpoint": "203.0.113.10:51820",
    "persistent_keepalive": 25
  }
}
```

### Remove Peer

```
POST /peers/remove
```

Removes a peer from the Formnet interface.

**Request Body**:
```json
{
  "public_key": "gN65BkIKy1eCE9pP1wdc8ROUtkHLF2PfAqYdyYBuZQA="
}
```

**Response**:
```json
{
  "success": true,
  "message": "Peer removed successfully"
}
```

### Update Peer

```
POST /peers/update
```

Updates an existing peer's configuration.

**Request Body**:
```json
{
  "public_key": "gN65BkIKy1eCE9pP1wdc8ROUtkHLF2PfAqYdyYBuZQA=",
  "allowed_ips": ["192.168.100.2/32", "192.168.100.100/32"],
  "endpoint": "203.0.113.10:51820",
  "persistent_keepalive": 25
}
```

**Response**:
```json
{
  "success": true,
  "peer": {
    "public_key": "gN65BkIKy1eCE9pP1wdc8ROUtkHLF2PfAqYdyYBuZQA=",
    "allowed_ips": ["192.168.100.2/32", "192.168.100.100/32"],
    "endpoint": "203.0.113.10:51820",
    "persistent_keepalive": 25
  }
}
```

### Get Peer

```
GET /peers/{public_key}
```

Retrieves information about a specific peer.

**Response**:
```json
{
  "id": "peer-123456789",
  "name": "Node1",
  "public_key": "gN65BkIKy1eCE9pP1wdc8ROUtkHLF2PfAqYdyYBuZQA=",
  "endpoint": "203.0.113.10:51820",
  "allowed_ips": ["192.168.100.2/32"],
  "latest_handshake": 1677721600,
  "transfer_rx": 1234567,
  "transfer_tx": 7654321,
  "persistent_keepalive": 25
}
```

### Configure Interface

```
POST /interface/configure
```

Configures the Formnet interface.

**Request Body**:
```json
{
  "private_key": "yAnz5TF+lXXJte14tji3zlMNq+hd2rYUIgJBgB3fBmk=",
  "address": "192.168.100.1/24",
  "listen_port": 51820,
  "dns": ["1.1.1.1", "8.8.8.8"]
}
```

**Response**:
```json
{
  "success": true,
  "interface": {
    "name": "formnet0",
    "public_key": "WFmc3ixj8Ue4qZEQRTH+GYKJmUFQd2H4UBW5BJdXpXE=",
    "address": "192.168.100.1/24",
    "listen_port": 51820,
    "dns": ["1.1.1.1", "8.8.8.8"]
  }
}
```

### Bring Interface Up

```
POST /interface/up
```

Brings the Formnet interface up.

**Response**:
```json
{
  "success": true,
  "message": "Interface formnet0 is up"
}
```

### Bring Interface Down

```
POST /interface/down
```

Brings the Formnet interface down.

**Response**:
```json
{
  "success": true,
  "message": "Interface formnet0 is down"
}
```

### Get Routes

```
GET /routes
```

Retrieves all routes configured for the Formnet interface.

**Response**:
```json
{
  "routes": [
    {
      "destination": "192.168.100.0/24",
      "gateway": "192.168.100.1",
      "interface": "formnet0"
    },
    {
      "destination": "192.168.101.0/24",
      "gateway": "192.168.100.2",
      "interface": "formnet0"
    }
  ]
}
```

### Add Route

```
POST /routes/add
```

Adds a new route to the routing table.

**Request Body**:
```json
{
  "destination": "192.168.101.0/24",
  "gateway": "192.168.100.2",
  "interface": "formnet0"
}
```

**Response**:
```json
{
  "success": true,
  "route": {
    "destination": "192.168.101.0/24",
    "gateway": "192.168.100.2",
    "interface": "formnet0"
  }
}
```

### Remove Route

```
POST /routes/remove
```

Removes a route from the routing table.

**Request Body**:
```json
{
  "destination": "192.168.101.0/24"
}
```

**Response**:
```json
{
  "success": true,
  "message": "Route removed successfully"
}
```

## Network Configuration

### Formnet Interface Configuration

The Formnet interface is configured with the following default parameters:

- **Interface name**: formnet0
- **Listen port**: 51820
- **MTU**: 1420
- **IP addressing**: Automatic assignment from the network's CIDR range
- **Endpoint**: Auto-detected based on the node's public IP and port

### Peer Connection Lifecycle

1. **Discovery**: Peers are discovered through the State Service
2. **Handshake**: WireGuard handshake establishes the encrypted tunnel
3. **Monitoring**: Regular keepalives maintain the connection
4. **Teardown**: Peers are removed when they leave the network

## Error Handling

The Formnet API returns standard HTTP status codes:

- 200: Success
- 400: Bad Request (invalid parameters)
- 401: Unauthorized (authentication failure)
- 403: Forbidden (insufficient permissions)
- 404: Not Found (resource not found)
- 409: Conflict (operation cannot be performed in current state)
- 500: Internal Server Error

Error responses include a JSON object with:
```json
{
  "success": false,
  "error": "Descriptive error message"
}
```

## Example Usage

### Adding a Peer

```bash
curl -X POST https://node.formation.cloud:3003/peers/add \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer <NODE_TOKEN>" \
  -d '{
    "public_key": "gN65BkIKy1eCE9pP1wdc8ROUtkHLF2PfAqYdyYBuZQA=",
    "allowed_ips": ["192.168.100.2/32"],
    "endpoint": "203.0.113.10:51820",
    "persistent_keepalive": 25
  }'
```

### Retrieving Interface Status

```bash
curl -X GET https://node.formation.cloud:3003/interface \
  -H "Authorization: Bearer <NODE_TOKEN>"
```

## SDK Integration

The Formation SDK provides wrapper functions for the Formnet API:

```javascript
const Formation = require('formation-sdk');

// Initialize the SDK with node credentials
const formation = new Formation({
  nodeId: 'node-123456789',
  nodeKey: 'your-node-private-key'
});

// Get interface status
const interfaceStatus = await formation.formnet.getInterfaceStatus();
console.log(interfaceStatus);

// Add a peer
const peer = await formation.formnet.addPeer({
  publicKey: 'gN65BkIKy1eCE9pP1wdc8ROUtkHLF2PfAqYdyYBuZQA=',
  allowedIps: ['192.168.100.2/32'],
  endpoint: '203.0.113.10:51820',
  persistentKeepalive: 25
});
console.log(peer);
```

## Security Considerations

Formnet provides robust security features:

- **Encryption**: All traffic is encrypted using WireGuard's modern cryptography
- **Perfect Forward Secrecy**: New session keys are generated for each connection
- **Authentication**: Peers are authenticated using public/private key pairs
- **Isolation**: Virtual network interfaces isolate Formation traffic from other network interfaces
- **Short Key Rotation**: Keys are rotated regularly for enhanced security 