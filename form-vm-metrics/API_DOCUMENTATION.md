# Form VM Metrics API Documentation

## Overview

The Form VM Metrics API provides access to resource usage metrics for virtual machines in the Formation Network. This API allows you to retrieve real-time metrics for CPU, memory, storage, network, and GPU usage, as well as monitor the health of the metrics collection service.

## Base URL

By default, the API is accessible at:

```
http://localhost:8080
```

The port can be configured using the `--port` command-line argument when starting the service.

## Authentication

Currently, the API does not require authentication. This feature will be added in future releases.

## Endpoints

### Get Current Metrics

Retrieves the current system metrics including CPU, memory, disk, network, and GPU usage.

**Endpoint:** `GET /get`

**Response Format:**

```json
{
  "timestamp": 1626350430,
  "instance_id": "instance-abc123",
  "account_id": "account-xyz789",
  "cpu": {
    "usage_pct": 25,
    "process_count": 42
  },
  "memory": {
    "total": 16777216000,
    "free": 8388608000,
    "available": 12582912000,
    "used": 8388608000,
    "total_swap": 4294967296,
    "used_swap": 1073741824
  },
  "disks": [
    {
      "device_name": "sda1",
      "reads_completed": 1000,
      "reads_merged": 500,
      "sectors_read": 2000,
      "time_reading": 100,
      "writes_completed": 1000,
      "writes_merged": 500,
      "sectors_written": 2097152,
      "time_writing": 100,
      "io_in_progress": 10,
      "time_doing_io": 200,
      "weighted_time_doing_io": 300
    }
  ],
  "network": {
    "interfaces": [
      {
        "name": "eth0",
        "bytes_sent": 104857600,
        "bytes_received": 209715200,
        "packets_sent": 1000,
        "packets_received": 2000,
        "errors_in": 0,
        "errors_out": 0,
        "drops_in": 0,
        "drops_out": 0,
        "speed": 1000000000
      }
    ]
  },
  "gpus": [
    {
      "index": 0,
      "model": "NVIDIA RTX 3080",
      "utilization_bps": 5000,
      "memory_usage_bps": 5000,
      "temperature_deci_c": 700,
      "power_draw_deci_w": 1500
    }
  ],
  "load": {
    "load1": 100,
    "load5": 120,
    "load15": 110
  }
}
```

### Basic Health Check

A simple health check endpoint that returns "healthy" if the service is running. This is suitable for basic liveness probes in container orchestration systems.

**Endpoint:** `GET /health`

**Response Format:**

Plain text: `healthy`

### Detailed Health Status

Returns comprehensive information about the service health, including uptime, component status, and version information. This endpoint is intended for monitoring systems and dashboards.

**Endpoint:** `GET /api/v1/health/status`

**Response Format:**

```json
{
  "status": "ok",
  "uptime_seconds": 3600,
  "components": {
    "metrics_collection": {
      "status": "ok",
      "last_success": 1626350430,
      "details": "Last metrics collection at timestamp 1626350430"
    },
    "event_publishing": {
      "status": "ok",
      "last_success": 1626350430,
      "details": "Event publishing appears operational"
    },
    "api": {
      "status": "ok",
      "last_success": null,
      "details": "API is responding to requests"
    }
  },
  "version": "0.1.0"
}
```

### Register Webhook

Registers a new webhook endpoint for receiving real-time event notifications.

**Endpoint:** `POST /api/v1/webhooks`

**Request Format:**

```json
{
  "url": "https://example.com/webhook",
  "event_types": ["metrics", "threshold_violation"],
  "secret": "optional_shared_secret"
}
```

**Response Format:**

```json
{
  "id": "webhook_abc123",
  "status": "registered",
  "url": "https://example.com/webhook",
  "event_types": ["metrics", "threshold_violation"],
  "registered_at": 1626350430
}
```

**Notes:**
- The `url` must be a valid HTTP or HTTPS URL
- Valid event types are `metrics` and `threshold_violation`
- The `secret` is optional and will be used to sign webhook payloads

### List Registered Webhooks

Returns a list of all registered webhooks.

**Endpoint:** `GET /api/v1/webhooks`

**Response Format:**

```json
[
  {
    "id": "webhook_abc123",
    "url": "https://example.com/webhook",
    "event_types": ["metrics", "threshold_violation"],
    "registered_at": 1626350430
  }
]
```

**Notes:**
- The `secret` field is not included in the response for security reasons

### Delete Webhook

Unregisters a webhook with the specified ID.

**Endpoint:** `DELETE /api/v1/webhooks/:id`

**Path Parameters:**
- `id` - The unique ID of the webhook to delete

**Response:**
- `204 No Content` - Webhook was successfully deleted
- `404 Not Found` - Webhook with the specified ID was not found

### Webhook Payloads

When an event is triggered, the service will make an HTTP POST request to the registered webhook URL with the following payload structure:

```json
{
  "event_type": "metrics",
  "timestamp": 1626350430,
  "data": {
    // The full metrics object, same format as the /get endpoint
  }
}
```

**Headers:**
- `Content-Type: application/json`
- `User-Agent: Form-VM-Metrics-Webhook`
- `X-Webhook-Event: metrics` (or other event type)
- `X-Webhook-Signature: abc123...` (only if a secret was provided during registration)

**Signature Verification:**
If a secret was provided during webhook registration, the service will include an HMAC-SHA256 signature in the `X-Webhook-Signature` header. You can verify this signature by:

```javascript
// Example in JavaScript
const crypto = require('crypto');

function verifySignature(payload, signature, secret) {
  const hmac = crypto.createHmac('sha256', secret);
  const expectedSignature = hmac.update(JSON.stringify(payload)).digest('hex');
  return crypto.timingSafeEqual(
    Buffer.from(signature, 'hex'),
    Buffer.from(expectedSignature, 'hex')
  );
}
```

## Status Codes

- `200 OK`: The request was successful
- `201 Created`: The resource was successfully created
- `204 No Content`: The request was successful but there is no content to return
- `400 Bad Request`: The request was invalid
- `404 Not Found`: The requested resource was not found
- `500 Internal Server Error`: An unexpected error occurred

## Usage Examples

### Curl Examples

**Getting current metrics:**

```shell
curl -X GET http://localhost:8080/get
```

**Checking service health:**

```shell
curl -X GET http://localhost:8080/health
```

**Getting detailed health status:**

```shell
curl -X GET http://localhost:8080/api/v1/health/status
```

**Registering a webhook:**

```shell
curl -X POST http://localhost:8080/api/v1/webhooks \
  -H "Content-Type: application/json" \
  -d '{
    "url": "https://example.com/webhook",
    "event_types": ["metrics"],
    "secret": "my_secret"
  }'
```

**Listing webhooks:**

```shell
curl -X GET http://localhost:8080/api/v1/webhooks
```

**Deleting a webhook:**

```shell
curl -X DELETE http://localhost:8080/api/v1/webhooks/webhook_abc123
```

### Monitoring Example

Here's an example of how to periodically check the service health using a shell script:

```bash
#!/bin/bash
while true; do
  response=$(curl -s -w "%{http_code}" http://localhost:8080/api/v1/health/status)
  status=$(echo $response | grep -o '"status":"[^"]*"' | cut -d '"' -f 4)
  
  if [ "$status" != "ok" ]; then
    echo "Service health check failed: $status"
    # Add notification logic here
  fi
  
  sleep 60
done
```

## Future Enhancements

The following features are planned for future releases:

- Authentication and authorization
- Filtering parameters for VM-specific metrics
- Historical metrics querying
- Advanced filtering and aggregation capabilities 