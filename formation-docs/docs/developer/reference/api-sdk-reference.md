# Formation API and SDK Reference

This document provides a comprehensive reference for the Formation API and SDK, which enable programmatic interaction with the Formation platform for application deployment and management.

## Formation SDK

The Formation SDK is available for multiple programming languages and provides a convenient, idiomatic way to interact with the Formation platform programmatically.

### Supported Languages

- **JavaScript/TypeScript**: Available via npm as `@formation/sdk`
- **Python**: Available via pip as `formation-sdk`
- **Go**: Available via go modules as `github.com/formation/sdk-go`
- **Rust**: Available via cargo as `formation-sdk`

### Installation

<tabs>
<tab label="JavaScript">

```bash
npm install @formation/sdk
# or
yarn add @formation/sdk
```

</tab>
<tab label="Python">

```bash
pip install formation-sdk
```

</tab>
<tab label="Go">

```bash
go get github.com/formation/sdk-go
```

</tab>
<tab label="Rust">

```bash
cargo add formation-sdk
```

</tab>
</tabs>

## Authentication

The SDK supports multiple authentication methods:

### API Key Authentication

<tabs>
<tab label="JavaScript">

```javascript
import { FormationClient } from '@formation/sdk';

const client = new FormationClient({
  apiKey: 'your-api-key',
});
```

</tab>
<tab label="Python">

```python
from formation import FormationClient

client = FormationClient(api_key="your-api-key")
```

</tab>
<tab label="Go">

```go
import "github.com/formation/sdk-go"

client, err := formation.NewClient(formation.WithAPIKey("your-api-key"))
if err != nil {
  // Handle error
}
```

</tab>
<tab label="Rust">

```rust
use formation_sdk::Client;

let client = Client::builder()
    .with_api_key("your-api-key")
    .build()
    .expect("Failed to create client");
```

</tab>
</tabs>

### Wallet Authentication

<tabs>
<tab label="JavaScript">

```javascript
import { FormationClient } from '@formation/sdk';

// Using private key
const client = new FormationClient({
  privateKey: 'your-ethereum-private-key',
});

// Or using a mnemonic phrase
const client = new FormationClient({
  mnemonic: 'word1 word2 ... word12',
});

// Or using a keystore
const client = new FormationClient({
  keystore: keystoreJsonObject,
  keystorePassword: 'your-keystore-password',
});
```

</tab>
<tab label="Python">

```python
from formation import FormationClient

# Using private key
client = FormationClient(private_key="your-ethereum-private-key")

# Or using a mnemonic phrase
client = FormationClient(mnemonic="word1 word2 ... word12")

# Or using a keystore
client = FormationClient(
    keystore="/path/to/keystore.json",
    keystore_password="your-keystore-password"
)
```

</tab>
<tab label="Go">

```go
import "github.com/formation/sdk-go"

// Using private key
client, err := formation.NewClient(
    formation.WithPrivateKey("your-ethereum-private-key"),
)

// Or using a mnemonic phrase
client, err := formation.NewClient(
    formation.WithMnemonic("word1 word2 ... word12"),
)

// Or using a keystore
client, err := formation.NewClient(
    formation.WithKeystore("/path/to/keystore.json", "your-keystore-password"),
)
```

</tab>
<tab label="Rust">

```rust
use formation_sdk::Client;

// Using private key
let client = Client::builder()
    .with_private_key("your-ethereum-private-key")
    .build()
    .expect("Failed to create client");

// Or using a mnemonic phrase
let client = Client::builder()
    .with_mnemonic("word1 word2 ... word12")
    .build()
    .expect("Failed to create client");

// Or using a keystore
let client = Client::builder()
    .with_keystore("/path/to/keystore.json", "your-keystore-password")
    .build()
    .expect("Failed to create client");
```

</tab>
</tabs>

## Core SDK Modules

The Formation SDK is organized into modules that correspond to different aspects of the Formation platform.

### Instances

The Instances module provides functions for managing virtual machine instances.

#### List Instances

<tabs>
<tab label="JavaScript">

```javascript
// List all instances
const instances = await client.instances.list();

// With filtering
const runningInstances = await client.instances.list({
  status: 'running',
  limit: 10,
});
```

</tab>
<tab label="Python">

```python
# List all instances
instances = client.instances.list()

# With filtering
running_instances = client.instances.list(
    status="running",
    limit=10
)
```

</tab>
<tab label="Go">

```go
// List all instances
instances, err := client.Instances.List(ctx, nil)

// With filtering
instances, err := client.Instances.List(ctx, &formation.InstanceListOptions{
    Status: formation.StatusRunning,
    Limit:  10,
})
```

</tab>
<tab label="Rust">

```rust
// List all instances
let instances = client.instances().list(None).await?;

// With filtering
let running_instances = client.instances()
    .list(Some(InstanceListOptions {
        status: Some(InstanceStatus::Running),
        limit: Some(10),
        ..Default::default()
    }))
    .await?;
```

</tab>
</tabs>

#### Get Instance

<tabs>
<tab label="JavaScript">

```javascript
const instance = await client.instances.get('i-1234567890abcdef');
```

</tab>
<tab label="Python">

```python
instance = client.instances.get('i-1234567890abcdef')
```

</tab>
<tab label="Go">

```go
instance, err := client.Instances.Get(ctx, "i-1234567890abcdef")
```

</tab>
<tab label="Rust">

```rust
let instance = client.instances().get("i-1234567890abcdef").await?;
```

</tab>
</tabs>

#### Start Instance

<tabs>
<tab label="JavaScript">

```javascript
// Start without waiting
await client.instances.start('i-1234567890abcdef');

// Start and wait for completion
await client.instances.start('i-1234567890abcdef', { wait: true });
```

</tab>
<tab label="Python">

```python
# Start without waiting
client.instances.start('i-1234567890abcdef')

# Start and wait for completion
client.instances.start('i-1234567890abcdef', wait=True)
```

</tab>
<tab label="Go">

```go
// Start without waiting
err := client.Instances.Start(ctx, "i-1234567890abcdef", nil)

// Start and wait for completion
err := client.Instances.Start(ctx, "i-1234567890abcdef", &formation.InstanceActionOptions{
    Wait: true,
})
```

</tab>
<tab label="Rust">

```rust
// Start without waiting
client.instances().start("i-1234567890abcdef", None).await?;

// Start and wait for completion
client.instances()
    .start(
        "i-1234567890abcdef",
        Some(InstanceActionOptions {
            wait: Some(true),
            ..Default::default()
        }),
    )
    .await?;
```

</tab>
</tabs>

#### Stop Instance

<tabs>
<tab label="JavaScript">

```javascript
// Stop without waiting
await client.instances.stop('i-1234567890abcdef');

// Stop and wait for completion
await client.instances.stop('i-1234567890abcdef', { 
  wait: true,
  force: false, // graceful shutdown
});
```

</tab>
<tab label="Python">

```python
# Stop without waiting
client.instances.stop('i-1234567890abcdef')

# Stop and wait for completion
client.instances.stop(
    'i-1234567890abcdef',
    wait=True,
    force=False  # graceful shutdown
)
```

</tab>
<tab label="Go">

```go
// Stop without waiting
err := client.Instances.Stop(ctx, "i-1234567890abcdef", nil)

// Stop and wait for completion
err := client.Instances.Stop(ctx, "i-1234567890abcdef", &formation.InstanceActionOptions{
    Wait:  true,
    Force: false, // graceful shutdown
})
```

</tab>
<tab label="Rust">

```rust
// Stop without waiting
client.instances().stop("i-1234567890abcdef", None).await?;

// Stop and wait for completion
client.instances()
    .stop(
        "i-1234567890abcdef",
        Some(InstanceActionOptions {
            wait: Some(true),
            force: Some(false), // graceful shutdown
            ..Default::default()
        }),
    )
    .await?;
```

</tab>
</tabs>

#### Delete Instance

<tabs>
<tab label="JavaScript">

```javascript
// Delete without waiting
await client.instances.delete('i-1234567890abcdef');

// Delete and wait for completion
await client.instances.delete('i-1234567890abcdef', { 
  wait: true,
  force: true, // skip confirmation
});
```

</tab>
<tab label="Python">

```python
# Delete without waiting
client.instances.delete('i-1234567890abcdef')

# Delete and wait for completion
client.instances.delete(
    'i-1234567890abcdef',
    wait=True,
    force=True  # skip confirmation
)
```

</tab>
<tab label="Go">

```go
// Delete without waiting
err := client.Instances.Delete(ctx, "i-1234567890abcdef", nil)

// Delete and wait for completion
err := client.Instances.Delete(ctx, "i-1234567890abcdef", &formation.InstanceActionOptions{
    Wait:  true,
    Force: true, // skip confirmation
})
```

</tab>
<tab label="Rust">

```rust
// Delete without waiting
client.instances().delete("i-1234567890abcdef", None).await?;

// Delete and wait for completion
client.instances()
    .delete(
        "i-1234567890abcdef",
        Some(InstanceActionOptions {
            wait: Some(true),
            force: Some(true), // skip confirmation
            ..Default::default()
        }),
    )
    .await?;
```

</tab>
</tabs>

#### Get Instance Logs

<tabs>
<tab label="JavaScript">

```javascript
// Get all logs
const logs = await client.instances.logs('i-1234567890abcdef');

// With filtering
const recentLogs = await client.instances.logs('i-1234567890abcdef', {
  tail: 100,
  since: '10m', // logs from the last 10 minutes
});
```

</tab>
<tab label="Python">

```python
# Get all logs
logs = client.instances.logs('i-1234567890abcdef')

# With filtering
recent_logs = client.instances.logs(
    'i-1234567890abcdef',
    tail=100,
    since='10m'  # logs from the last 10 minutes
)
```

</tab>
<tab label="Go">

```go
// Get all logs
logs, err := client.Instances.Logs(ctx, "i-1234567890abcdef", nil)

// With filtering
logs, err := client.Instances.Logs(ctx, "i-1234567890abcdef", &formation.LogsOptions{
    Tail:  100,
    Since: "10m", // logs from the last 10 minutes
})
```

</tab>
<tab label="Rust">

```rust
// Get all logs
let logs = client.instances().logs("i-1234567890abcdef", None).await?;

// With filtering
let recent_logs = client.instances()
    .logs(
        "i-1234567890abcdef",
        Some(LogsOptions {
            tail: Some(100),
            since: Some("10m".to_string()), // logs from the last 10 minutes
            ..Default::default()
        }),
    )
    .await?;
```

</tab>
</tabs>

### Builds

The Builds module provides functions for building and deploying applications.

#### Create Build

<tabs>
<tab label="JavaScript">

```javascript
// Build from a directory
const build = await client.builds.create({
  context: './my-app',
  formfilePath: './my-app/Formfile',
  noCache: false,
});
```

</tab>
<tab label="Python">

```python
# Build from a directory
build = client.builds.create(
    context="./my-app",
    formfile_path="./my-app/Formfile",
    no_cache=False
)
```

</tab>
<tab label="Go">

```go
// Build from a directory
build, err := client.Builds.Create(ctx, &formation.BuildCreateOptions{
    Context:      "./my-app",
    FormfilePath: "./my-app/Formfile",
    NoCache:      false,
})
```

</tab>
<tab label="Rust">

```rust
// Build from a directory
let build = client.builds()
    .create(BuildCreateOptions {
        context: "./my-app".to_string(),
        formfile_path: "./my-app/Formfile".to_string(),
        no_cache: Some(false),
        ..Default::default()
    })
    .await?;
```

</tab>
</tabs>

#### Get Build Status

<tabs>
<tab label="JavaScript">

```javascript
const buildStatus = await client.builds.status('b-1234567890abcdef');
```

</tab>
<tab label="Python">

```python
build_status = client.builds.status('b-1234567890abcdef')
```

</tab>
<tab label="Go">

```go
buildStatus, err := client.Builds.Status(ctx, "b-1234567890abcdef")
```

</tab>
<tab label="Rust">

```rust
let build_status = client.builds().status("b-1234567890abcdef").await?;
```

</tab>
</tabs>

#### Deploy Build

<tabs>
<tab label="JavaScript">

```javascript
// Deploy build without waiting
const instance = await client.builds.deploy('b-1234567890abcdef', {
  name: 'my-app-instance',
});

// Deploy and wait for completion
const instance = await client.builds.deploy('b-1234567890abcdef', {
  name: 'my-app-instance',
  wait: true,
});
```

</tab>
<tab label="Python">

```python
# Deploy build without waiting
instance = client.builds.deploy(
    'b-1234567890abcdef',
    name='my-app-instance'
)

# Deploy and wait for completion
instance = client.builds.deploy(
    'b-1234567890abcdef',
    name='my-app-instance',
    wait=True
)
```

</tab>
<tab label="Go">

```go
// Deploy build without waiting
instance, err := client.Builds.Deploy(ctx, "b-1234567890abcdef", &formation.DeployOptions{
    Name: "my-app-instance",
})

// Deploy and wait for completion
instance, err := client.Builds.Deploy(ctx, "b-1234567890abcdef", &formation.DeployOptions{
    Name: "my-app-instance",
    Wait: true,
})
```

</tab>
<tab label="Rust">

```rust
// Deploy build without waiting
let instance = client.builds()
    .deploy(
        "b-1234567890abcdef",
        DeployOptions {
            name: Some("my-app-instance".to_string()),
            ..Default::default()
        },
    )
    .await?;

// Deploy and wait for completion
let instance = client.builds()
    .deploy(
        "b-1234567890abcdef",
        DeployOptions {
            name: Some("my-app-instance".to_string()),
            wait: Some(true),
            ..Default::default()
        },
    )
    .await?;
```

</tab>
</tabs>

### Wallet

The Wallet module provides functions for managing Ethereum wallets and instance ownership.

#### Get Wallet Info

<tabs>
<tab label="JavaScript">

```javascript
const walletInfo = await client.wallet.info();
```

</tab>
<tab label="Python">

```python
wallet_info = client.wallet.info()
```

</tab>
<tab label="Go">

```go
walletInfo, err := client.Wallet.Info(ctx)
```

</tab>
<tab label="Rust">

```rust
let wallet_info = client.wallet().info().await?;
```

</tab>
</tabs>

#### Transfer Instance Ownership

<tabs>
<tab label="JavaScript">

```javascript
await client.wallet.transfer('i-1234567890abcdef', '0x1234567890abcdef1234567890abcdef12345678');
```

</tab>
<tab label="Python">

```python
client.wallet.transfer(
    'i-1234567890abcdef',
    '0x1234567890abcdef1234567890abcdef12345678'
)
```

</tab>
<tab label="Go">

```go
err := client.Wallet.Transfer(ctx, "i-1234567890abcdef", "0x1234567890abcdef1234567890abcdef12345678")
```

</tab>
<tab label="Rust">

```rust
client.wallet()
    .transfer(
        "i-1234567890abcdef",
        "0x1234567890abcdef1234567890abcdef12345678"
    )
    .await?;
```

</tab>
</tabs>

### Domains

The Domains module provides functions for managing domain names associated with instances.

#### List Domains

<tabs>
<tab label="JavaScript">

```javascript
// List all domains
const domains = await client.domains.list();

// List domains for a specific instance
const instanceDomains = await client.domains.list({
  instanceId: 'i-1234567890abcdef',
});
```

</tab>
<tab label="Python">

```python
# List all domains
domains = client.domains.list()

# List domains for a specific instance
instance_domains = client.domains.list(
    instance_id='i-1234567890abcdef'
)
```

</tab>
<tab label="Go">

```go
// List all domains
domains, err := client.Domains.List(ctx, nil)

// List domains for a specific instance
domains, err := client.Domains.List(ctx, &formation.DomainListOptions{
    InstanceID: "i-1234567890abcdef",
})
```

</tab>
<tab label="Rust">

```rust
// List all domains
let domains = client.domains().list(None).await?;

// List domains for a specific instance
let instance_domains = client.domains()
    .list(Some(DomainListOptions {
        instance_id: Some("i-1234567890abcdef".to_string()),
        ..Default::default()
    }))
    .await?;
```

</tab>
</tabs>

#### Add Domain

<tabs>
<tab label="JavaScript">

```javascript
await client.domains.add('example.com', 'i-1234567890abcdef', {
  skipVerification: false,
  wait: true,
});
```

</tab>
<tab label="Python">

```python
client.domains.add(
    'example.com',
    'i-1234567890abcdef',
    skip_verification=False,
    wait=True
)
```

</tab>
<tab label="Go">

```go
err := client.Domains.Add(ctx, "example.com", "i-1234567890abcdef", &formation.DomainAddOptions{
    SkipVerification: false,
    Wait:             true,
})
```

</tab>
<tab label="Rust">

```rust
client.domains()
    .add(
        "example.com",
        "i-1234567890abcdef",
        Some(DomainAddOptions {
            skip_verification: Some(false),
            wait: Some(true),
            ..Default::default()
        }),
    )
    .await?;
```

</tab>
</tabs>

#### Remove Domain

<tabs>
<tab label="JavaScript">

```javascript
await client.domains.remove('example.com', {
  force: true,
});
```

</tab>
<tab label="Python">

```python
client.domains.remove(
    'example.com',
    force=True
)
```

</tab>
<tab label="Go">

```go
err := client.Domains.Remove(ctx, "example.com", &formation.DomainRemoveOptions{
    Force: true,
})
```

</tab>
<tab label="Rust">

```rust
client.domains()
    .remove(
        "example.com",
        Some(DomainRemoveOptions {
            force: Some(true),
            ..Default::default()
        }),
    )
    .await?;
```

</tab>
</tabs>

### Network

The Network module provides functions for managing the Formation cloud connection.

#### Join Network

<tabs>
<tab label="JavaScript">

```javascript
await client.network.join({
  network: 'mainnet',
  force: false,
});
```

</tab>
<tab label="Python">

```python
client.network.join(
    network='mainnet',
    force=False
)
```

</tab>
<tab label="Go">

```go
// Join the Formation cloud
err := client.Network.Join(ctx, &formation.NetworkJoinOptions{
    Network: "mainnet",
    Force:   false,
})
```

</tab>
<tab label="Rust">

```rust
client.network()
    .join(NetworkJoinOptions {
        network: Some("mainnet".to_string()),
        force: Some(false),
        ..Default::default()
    })
    .await?;
```

</tab>
</tabs>

#### Check Network Status

<tabs>
<tab label="JavaScript">

```javascript
const status = await client.network.status();
```

</tab>
<tab label="Python">

```python
status = client.network.status()
```

</tab>
<tab label="Go">

```go
status, err := client.Network.Status(ctx)
```

</tab>
<tab label="Rust">

```rust
let status = client.network().status().await?;
```

</tab>
</tabs>

#### Restart Network Connection

<tabs>
<tab label="JavaScript">

```javascript
await client.network.restart({
  force: true,
});
```

</tab>
<tab label="Python">

```python
client.network.restart(
    force=True
)
```

</tab>
<tab label="Go">

```go
err := client.Network.Restart(ctx, &formation.NetworkRestartOptions{
    Force: true,
})
```

</tab>
<tab label="Rust">

```rust
client.network()
    .restart(NetworkRestartOptions {
        force: Some(true),
        ..Default::default()
    })
    .await?;
```

</tab>
</tabs>

## RESTful API

The Formation REST API can be used directly for advanced integrations or in environments where the SDK is not available.

### Base URL

```
https://api.formation.cloud/v1
```

### Authentication

All API requests require authentication using one of the following methods:

#### API Key

Include your API key in the `Authorization` header:

```
Authorization: Bearer your-api-key
```

#### Wallet Authentication

For wallet-based authentication, create a signature of the request and include it in the headers:

```
X-Formation-Address: your-ethereum-address
X-Formation-Timestamp: current-timestamp
X-Formation-Signature: signature-of-request
```

### API Endpoints

#### Instances

- `GET /instances` - List instances
- `GET /instances/{id}` - Get instance details
- `POST /instances/{id}/start` - Start an instance
- `POST /instances/{id}/stop` - Stop an instance
- `DELETE /instances/{id}` - Delete an instance
- `GET /instances/{id}/logs` - Get instance logs

#### Builds

- `POST /builds` - Create a new build
- `GET /builds/{id}` - Get build status
- `POST /builds/{id}/deploy` - Deploy a build

#### Wallet

- `GET /wallet` - Get wallet information
- `POST /wallet/transfer` - Transfer instance ownership

#### Domains

- `GET /domains` - List domains
- `POST /domains` - Add a domain
- `DELETE /domains/{domain}` - Remove a domain

#### Network

- `POST /network/join` - Join the Formation cloud
- `GET /network/status` - Check network status
- `POST /network/restart` - Restart network connection

### Example Requests

#### Create a Build

```http
POST /builds HTTP/1.1
Host: api.formation.cloud
Authorization: Bearer your-api-key
Content-Type: application/json

{
  "formfile": "NAME my-app\nFROM ubuntu:22.04\nRUN echo Hello, World!",
  "noCache": false
}
```

#### Deploy a Build

```http
POST /builds/b-1234567890abcdef/deploy HTTP/1.1
Host: api.formation.cloud
Authorization: Bearer your-api-key
Content-Type: application/json

{
  "name": "my-app-instance",
  "wait": true
}
```

#### List Instances

```http
GET /instances?status=running&limit=10 HTTP/1.1
Host: api.formation.cloud
Authorization: Bearer your-api-key
```

#### Start an Instance

```http
POST /instances/i-1234567890abcdef/start HTTP/1.1
Host: api.formation.cloud
Authorization: Bearer your-api-key
Content-Type: application/json

{
  "wait": true
}
```

## Webhook API

Formation provides a webhook API that allows you to receive notifications about events related to your instances, builds, and other resources.

### Configuring Webhooks

To configure a webhook, you need to:

1. Create a publicly accessible endpoint that can receive HTTP POST requests
2. Register the endpoint with Formation
3. Implement logic to handle and verify the webhook payloads

### Webhook Payload Structure

All webhook payloads have the following structure:

```json
{
  "event": "instance.started",
  "timestamp": "2023-06-15T12:34:56Z",
  "id": "evt_1234567890abcdef",
  "data": {
    "instanceId": "i-1234567890abcdef",
    "status": "running",
    "startedAt": "2023-06-15T12:34:50Z"
  }
}
```

### Webhook Verification

For security, you should verify that webhooks are coming from Formation. The request includes the following headers:

- `X-Formation-Signature`: HMAC signature of the request body
- `X-Formation-Timestamp`: Timestamp when the webhook was sent

To verify the signature:

1. Concatenate the timestamp and request body
2. Compute an HMAC-SHA256 using your webhook secret
3. Compare with the signature in the header

<tabs>
<tab label="JavaScript">

```javascript
const crypto = require('crypto');

function verifyWebhook(body, signature, timestamp, secret) {
  const payload = timestamp + body;
  const expectedSignature = crypto
    .createHmac('sha256', secret)
    .update(payload)
    .digest('hex');
  
  return crypto.timingSafeEqual(
    Buffer.from(signature),
    Buffer.from(expectedSignature)
  );
}
```

</tab>
<tab label="Python">

```python
import hmac
import hashlib

def verify_webhook(body, signature, timestamp, secret):
    payload = timestamp + body
    expected_signature = hmac.new(
        secret.encode(),
        payload.encode(),
        hashlib.sha256
    ).hexdigest()
    
    return hmac.compare_digest(signature, expected_signature)
```

</tab>
</tabs>

### Available Webhook Events

- `instance.created` - An instance was created
- `instance.started` - An instance was started
- `instance.stopped` - An instance was stopped
- `instance.deleted` - An instance was deleted
- `instance.failed` - An instance failed to start
- `build.started` - A build was started
- `build.completed` - A build was completed
- `build.failed` - A build failed
- `domain.added` - A domain was added
- `domain.removed` - A domain was removed
- `wallet.transferred` - Instance ownership was transferred

## Rate Limits

The Formation API implements rate limiting to ensure fair usage. Rate limits are applied on a per-API key basis:

- Standard tier: 60 requests per minute
- Professional tier: 300 requests per minute
- Enterprise tier: Custom limits

When you exceed the rate limit, the API will return a 429 Too Many Requests status code.

The response headers include information about your rate limit status:

- `X-RateLimit-Limit`: The maximum number of requests per minute
- `X-RateLimit-Remaining`: The number of requests remaining in the current window
- `X-RateLimit-Reset`: The time when the rate limit window resets, in Unix epoch seconds

## Error Handling

The API returns standard HTTP status codes to indicate the success or failure of a request:

- `200 OK` - Success
- `201 Created` - Resource created
- `400 Bad Request` - Invalid request
- `401 Unauthorized` - Authentication error
- `403 Forbidden` - Permission error
- `404 Not Found` - Resource not found
- `409 Conflict` - Resource conflict
- `429 Too Many Requests` - Rate limit exceeded
- `500 Internal Server Error` - Server error

Error responses include a JSON body with details:

```json
{
  "error": {
    "code": "instance_not_found",
    "message": "Instance i-1234567890abcdef not found",
    "details": {
      "instanceId": "i-1234567890abcdef"
    }
  }
}
```

## Pagination

API endpoints that return lists of resources support pagination using the `limit` and `offset` parameters:

```
GET /instances?limit=10&offset=20
```

The response includes pagination metadata:

```json
{
  "data": [...],
  "pagination": {
    "total": 45,
    "limit": 10,
    "offset": 20,
    "next": "/v1/instances?limit=10&offset=30",
    "previous": "/v1/instances?limit=10&offset=10"
  }
}
```

## Best Practices

1. **Use SDK for convenience**: The SDK handles authentication, serialization, and error handling.
2. **Handle rate limits**: Implement exponential backoff retry logic for rate limit errors.
3. **Validate inputs**: Validate inputs before sending them to the API to avoid validation errors.
4. **Use webhooks**: For real-time updates, use webhooks instead of polling the API.
5. **Secure credentials**: Store API keys and private keys securely, never expose them in client-side code.
6. **Monitor usage**: Set up monitoring for API usage to avoid unexpected rate limiting.
7. **Check errors**: Always check for and handle errors from the API properly.
8. **Use pagination**: For endpoints that return lists, always use pagination parameters to avoid large responses. 