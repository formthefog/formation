# DNS Service API

The DNS Service provides domain name resolution for the Formation cloud. It allows instances to be accessed using friendly domain names rather than IP addresses and enables vanity domains for applications.

## API Overview

The DNS Service interfaces with the State Service to store DNS records and provides a DNS server that responds to lookups from Formation nodes and instances.

## Authentication

API requests to the DNS Service require authentication using one of the following methods:
- Ethereum wallet signatures for user-facing endpoints
- Node identity verification for node-to-node communication

## Endpoints

> **Note**: The DNS Service API is primarily accessed via the State Service API endpoints. This section documents the DNS-specific endpoints and functionality.

### Request Vanity Domain

```
GET /dns/vanity/{domain}/{build_id}
```

Associates a vanity domain with an instance.

**Parameters**:
- `domain`: The desired domain name
- `build_id`: The build ID of the instance

**Response**:
```json
{
  "success": true,
  "data": {
    "domain": "myapp.formation.cloud",
    "ip": "192.168.100.52",
    "instance_id": "build-123456789"
  }
}
```

### Request Public Domain

```
GET /dns/public/{domain}/{build_id}
```

Associates a public domain with an instance, making it accessible from outside the Formation cloud.

**Parameters**:
- `domain`: The desired domain name
- `build_id`: The build ID of the instance

**Response**:
```json
{
  "success": true,
  "data": {
    "domain": "myapp.formation.cloud",
    "ip": "203.0.113.10",
    "instance_id": "build-123456789"
  }
}
```

### Create DNS Record

```
POST /dns/create
```

Creates a new DNS record.

**Request Body**:
```json
{
  "domain": "instance3.formation.local",
  "record_type": "A",
  "value": "192.168.100.52",
  "ttl": 300
}
```

**Response**:
```json
{
  "success": true,
  "data": {
    "domain": "instance3.formation.local",
    "record_type": "A",
    "value": "192.168.100.52",
    "ttl": 300,
    "created_at": 1677725200
  }
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
  "domain": "instance3.formation.local",
  "record_type": "A",
  "value": "192.168.100.53",
  "ttl": 300
}
```

**Response**:
```json
{
  "success": true,
  "data": {
    "domain": "instance3.formation.local",
    "record_type": "A",
    "value": "192.168.100.53",
    "ttl": 300,
    "created_at": 1677725200,
    "updated_at": 1677726200
  }
}
```

### Delete DNS Record

```
DELETE /dns/{domain}
```

Deletes a DNS record.

**Request Body**:
```json
{
  "domain": "instance3.formation.local"
}
```

**Response**:
```json
{
  "success": true,
  "data": {
    "domain": "instance3.formation.local",
    "record_type": "A",
    "value": "192.168.100.53",
    "ttl": 300,
    "created_at": 1677725200,
    "updated_at": 1677726200,
    "deleted_at": 1677727200
  }
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
  "success": true,
  "data": {
    "domain": "instance3.formation.local",
    "record_type": "A",
    "value": "192.168.100.53",
    "ttl": 300,
    "created_at": 1677725200,
    "updated_at": 1677726200
  }
}
```

### Get DNS Records by Node IP

```
GET /dns/node/{ip}
```

Retrieves all DNS records associated with a specific node IP address.

**Response**:
```json
[
  {
    "domain": "instance1.formation.local",
    "record_type": "A",
    "value": "192.168.100.10",
    "ttl": 300,
    "created_at": 1677725200
  },
  {
    "domain": "instance2.formation.local",
    "record_type": "A",
    "value": "192.168.100.10",
    "ttl": 300,
    "created_at": 1677726200
  }
]
```

### List All DNS Records

```
GET /dns
```

Retrieves all DNS records in the network.

**Response**:
```json
{
  "success": true,
  "data": [
    {
      "domain": "instance1.formation.local",
      "record_type": "A",
      "value": "192.168.100.10",
      "ttl": 300,
      "created_at": 1677725200
    },
    {
      "domain": "instance2.formation.local",
      "record_type": "A",
      "value": "192.168.100.11",
      "ttl": 300,
      "created_at": 1677726200
    },
    {
      "domain": "instance3.formation.local",
      "record_type": "A",
      "value": "192.168.100.53",
      "ttl": 300,
      "created_at": 1677727200
    }
  ]
}
```

## DNS Record Types

The Formation DNS Service supports the following record types:

| Record Type | Description | Example Value |
|-------------|-------------|--------------|
| A | IPv4 address | 192.168.100.10 |
| AAAA | IPv6 address | 2001:db8::1 |
| CNAME | Canonical name | instance1.formation.local |
| TXT | Text record | v=spf1 include:_spf.formation.cloud ~all |

## Domain Naming Conventions

Formation uses the following domain name conventions:

- **Internal domains**: `*.formation.local`
- **Vanity domains**: `*.formation.cloud`
- **Custom domains**: Any valid domain name that has been properly delegated

## DNS Resolution Process

When a Formation instance or client needs to resolve a domain name:

1. The query is first sent to the local DNS resolver on the node
2. If the domain is in the Formation namespace, the resolver checks the local cache
3. If not found in cache, the resolver queries the Formation DNS Service
4. If the domain is outside the Formation namespace, the query is forwarded to the configured upstream DNS servers

## Error Handling

The DNS Service API returns standard HTTP status codes:

- 200: Success
- 400: Bad Request (invalid parameters)
- 401: Unauthorized (authentication failure)
- 403: Forbidden (insufficient permissions)
- 404: Not Found (domain not found)
- 409: Conflict (domain already exists)
- 500: Internal Server Error

Error responses include a JSON object with:
```json
{
  "success": false,
  "reason": "Descriptive error message"
}
```

## Example Usage

### Creating a DNS Record

```bash
curl -X POST https://node.formation.cloud:3004/dns/create \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer <JWT_TOKEN>" \
  -d '{
    "domain": "my-api.formation.local",
    "record_type": "A",
    "value": "192.168.100.52",
    "ttl": 300
  }'
```

### Requesting a Vanity Domain

```bash
curl -X GET https://node.formation.cloud:3004/dns/vanity/my-app/build-123456789 \
  -H "Authorization: Bearer <JWT_TOKEN>"
```

## SDK Integration

The Formation SDK provides wrapper functions for the DNS Service API:

```javascript
const Formation = require('formation-sdk');

// Initialize the SDK
const formation = new Formation({
  apiKey: 'your-api-key'
});

// Create a DNS record
const record = await formation.dns.createRecord({
  domain: 'my-api.formation.local',
  recordType: 'A',
  value: '192.168.100.52',
  ttl: 300
});
console.log(record);

// Request a vanity domain
const vanityDomain = await formation.dns.requestVanityDomain('my-app', 'build-123456789');
console.log(vanityDomain);
```

## Configuring External DNS

To make Formation vanity domains accessible from the public internet, you need to configure your external DNS provider to delegate the subdomain to Formation's DNS servers.

### Example: Delegating a Subdomain to Formation

If you own `example.com` and want to delegate `formation.example.com` to Formation:

1. Add the following NS records to your DNS provider:
```
formation.example.com. IN NS ns1.formation.cloud.
formation.example.com. IN NS ns2.formation.cloud.
```

2. Once delegation is complete, you can create DNS records for `*.formation.example.com` through the Formation DNS Service API.

## DNS Security

Formation's DNS Service implements several security measures:

- **DNSSEC**: Digital signing of DNS records to prevent spoofing
- **Access control**: Only authorized users can create or modify DNS records
- **Audit logs**: All DNS changes are logged for accountability
- **Rate limiting**: Protection against DNS amplification attacks 