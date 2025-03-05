# P2P Service API

The P2P Service provides the messaging infrastructure for Formation nodes to communicate with each other. It implements a reliable, ordered message queue system that supports topics and subscriptions.

## API Overview

The P2P Service operates on port 53333 by default and exposes endpoints for queue management and message passing between nodes. This service is the foundation of Formation's distributed communication system.

## Authentication

API requests to the P2P Service require node identity verification through the `Authorization` header, which should contain a node-specific token derived from its private key.

## Endpoints

### Health Check

```
GET /health
```

Verifies that the P2P service is running and responsive.

**Response**:
```
"healthy"
```

### Bootstrap Completion

```
POST /bootstrap/complete
```

Marks the bootstrap process as complete for this node, enabling it to fully participate in the P2P network.

**Response**:
- 200 OK: Empty response with success status

### Write Operation

```
POST /queue/write
```

Writes a message to the distributed queue. This operation is replicated to all nodes.

**Request Body**:
```json
{
  "topic": "instance-updates",
  "sub_topic": 1,
  "message": "base64_encoded_message_data",
  "timestamp": 1677721600
}
```

**Response**:
```json
{
  "success": true,
  "message_id": 123456,
  "topic": "instance-updates",
  "timestamp": 1677721600
}
```

### Write Local

```
POST /queue/write-local
```

Writes a message to the local queue only, without replicating it to other nodes.

**Request Body**:
```json
{
  "topic": "local-events",
  "sub_topic": 2,
  "message": "base64_encoded_message_data",
  "timestamp": 1677721600
}
```

**Response**:
```json
{
  "success": true,
  "message_id": 123457,
  "topic": "local-events",
  "timestamp": 1677721600
}
```

### Get All Messages from Topic

```
GET /queue/topic/{topic}
```

Retrieves all messages from a specific topic.

**Response**:
```json
{
  "success": true,
  "messages": [
    {
      "id": 123456,
      "topic": "instance-updates",
      "sub_topic": 1,
      "data": "base64_encoded_message_data",
      "timestamp": 1677721600
    },
    {
      "id": 123458,
      "topic": "instance-updates",
      "sub_topic": 1,
      "data": "base64_encoded_message_data_2",
      "timestamp": 1677721700
    }
  ]
}
```

### Get N Messages from Topic

```
GET /queue/topic/{topic}/n/{n}
```

Retrieves the latest N messages from a specific topic.

**Response**:
```json
{
  "success": true,
  "messages": [
    {
      "id": 123458,
      "topic": "instance-updates",
      "sub_topic": 1,
      "data": "base64_encoded_message_data_2",
      "timestamp": 1677721700
    }
  ]
}
```

### Get Messages After Index

```
GET /queue/topic/{topic}/after/{index}
```

Retrieves all messages from a specific topic that were published after the specified index.

**Response**:
```json
{
  "success": true,
  "messages": [
    {
      "id": 123458,
      "topic": "instance-updates",
      "sub_topic": 1,
      "data": "base64_encoded_message_data_2",
      "timestamp": 1677721700
    },
    {
      "id": 123459,
      "topic": "instance-updates",
      "sub_topic": 1,
      "data": "base64_encoded_message_data_3",
      "timestamp": 1677721800
    }
  ]
}
```

### Get N Messages After Index

```
GET /queue/topic/{topic}/after/{index}/n/{n}
```

Retrieves N messages from a specific topic that were published after the specified index.

**Response**:
```json
{
  "success": true,
  "messages": [
    {
      "id": 123458,
      "topic": "instance-updates",
      "sub_topic": 1,
      "data": "base64_encoded_message_data_2",
      "timestamp": 1677721700
    }
  ]
}
```

### Get All Messages

```
GET /queue/all
```

Retrieves all messages from all topics (admin only).

**Response**:
```json
{
  "success": true,
  "topics": {
    "instance-updates": [
      {
        "id": 123456,
        "topic": "instance-updates",
        "sub_topic": 1,
        "data": "base64_encoded_message_data",
        "timestamp": 1677721600
      },
      {
        "id": 123458,
        "topic": "instance-updates",
        "sub_topic": 1,
        "data": "base64_encoded_message_data_2",
        "timestamp": 1677721700
      }
    ],
    "local-events": [
      {
        "id": 123457,
        "topic": "local-events",
        "sub_topic": 2,
        "data": "base64_encoded_message_data",
        "timestamp": 1677721600
      }
    ]
  }
}
```

## Message Format

Messages in the P2P Service are encoded as binary data (represented as base64 in JSON). The structure of a message includes:

- **ID**: Unique message identifier
- **Topic**: Main topic category
- **Sub-topic**: Sub-category within the topic
- **Data**: The message payload
- **Timestamp**: When the message was created

## Standard Topics

The Formation P2P Service uses several standard topics:

| Topic | Purpose |
|-------|---------|
| `peer-updates` | Peer join/leave events and updates |
| `instance-updates` | VM instance state changes |
| `node-updates` | Node availability and resource updates |
| `dns-updates` | DNS record changes |
| `cidr-updates` | Network CIDR changes |
| `association-updates` | Network association changes |
| `account-updates` | Account changes |

## Error Handling

The P2P Service API returns standard HTTP status codes:

- 200: Success
- 400: Bad Request (invalid parameters)
- 401: Unauthorized (authentication failure)
- 403: Forbidden (insufficient permissions)
- 404: Not Found (topic not found)
- 500: Internal Server Error

Error responses include a JSON object with:
```json
{
  "success": false,
  "error": "Descriptive error message"
}
```

## Example Usage

### Writing a Message to the Queue

```bash
curl -X POST https://node.formation.cloud:53333/queue/write \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer <NODE_TOKEN>" \
  -d '{
    "topic": "instance-updates",
    "sub_topic": 1,
    "message": "eyJidWlsZF9pZCI6ImJ1aWxkLTEyMzQ1Njc4OSIsInN0YXRlIjoicnVubmluZyJ9",
    "timestamp": 1677721600
  }'
```

### Reading Messages from a Topic

```bash
curl -X GET https://node.formation.cloud:53333/queue/topic/instance-updates/n/10 \
  -H "Authorization: Bearer <NODE_TOKEN>"
```

## SDK Integration

The Formation SDK provides wrapper functions for the P2P Service API:

```javascript
const Formation = require('formation-sdk');

// Initialize the SDK with node credentials
const formation = new Formation({
  nodeId: 'node-123456789',
  nodeKey: 'your-node-private-key'
});

// Write a message to the queue
await formation.p2p.writeMessage({
  topic: 'instance-updates',
  subTopic: 1,
  message: {
    buildId: 'build-123456789',
    state: 'running'
  }
});

// Read messages from a topic
const messages = await formation.p2p.getMessages({
  topic: 'instance-updates',
  limit: 10
});
console.log(messages);

// Subscribe to a topic
formation.p2p.subscribe('instance-updates', (message) => {
  console.log(`Received message: ${JSON.stringify(message)}`);
});
```

## Implementation Considerations

When working with the P2P Service API, keep these considerations in mind:

1. **Message Ordering**: Messages within a topic are guaranteed to be processed in the order they were published.
2. **Idempotency**: The P2P system is designed to handle duplicate messages gracefully.
3. **Persistence**: Messages are persisted and can be retrieved even after node restarts.
4. **BFT Properties**: The queue system maintains Byzantine Fault Tolerance, ensuring consistency even in the presence of faulty nodes.
5. **Performance**: For high-throughput applications, consider batching messages or using sub-topics to partition message flows. 