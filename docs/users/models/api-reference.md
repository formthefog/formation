# Formation Model API Reference

Complete API reference for making inference requests to AI models on the Formation network. All endpoints follow OpenAI API v1 specifications for maximum compatibility.

## Base URL

```
https://formation.ai
```

## Authentication

All requests require ECDSA signature authentication. See the [Inference Guide](inference-guide.md#authentication-with-ecdsa-signatures) for details.

### Required Headers

```http
X-Formation-Address: 0x1234567890abcdef1234567890abcdef12345678
X-Formation-Signature: 0xabcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890ab
X-Formation-Message: Formation authentication request
Content-Type: application/json
```

---

## Core Endpoints

### List Models

Get a list of all available models.

```http
GET /v1/models
```

#### Response

```json
{
  "success": true,
  "models": [
    {
      "id": "llama2-7b-chat",
      "name": "Llama 2 7B Chat",
      "description": "Meta's Llama 2 7B parameter chat model",
      "type": "text_generation",
      "owner_id": "0x9876543210fedcba...",
      "is_private": false,
      "pricing": {
        "model": "per_token",
        "input_rate": 0.5,
        "output_rate": 1.0,
        "currency": "credits_per_1k_tokens"
      },
      "capabilities": ["chat", "text_generation"],
      "max_tokens": 4096,
      "context_length": 4096,
      "created_at": 1640995200,
      "updated_at": 1640995800
    }
  ],
  "total": 1
}
```

#### Response Fields

| Field | Type | Description |
|-------|------|-------------|
| `success` | boolean | Whether the request was successful |
| `models` | array | Array of model objects |
| `total` | integer | Total number of models |

#### Model Object Fields

| Field | Type | Description |
|-------|------|-------------|
| `id` | string | Unique model identifier |
| `name` | string | Human-readable model name |
| `description` | string | Model description |
| `type` | string | Model type (`text_generation`, `image_generation`, etc.) |
| `owner_id` | string | Ethereum address of model owner |
| `is_private` | boolean | Whether model is private |
| `pricing` | object | Pricing information |
| `capabilities` | array | List of model capabilities |
| `max_tokens` | integer | Maximum tokens per request |
| `context_length` | integer | Maximum context length |
| `created_at` | integer | Unix timestamp of creation |
| `updated_at` | integer | Unix timestamp of last update |

---

### Get Model Details

Get detailed information about a specific model.

```http
GET /v1/models/{model_id}
```

#### Parameters

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `model_id` | string | Yes | The ID of the model to retrieve |

#### Response

```json
{
  "success": true,
  "model": {
    "id": "llama2-7b-chat",
    "name": "Llama 2 7B Chat",
    "description": "Meta's Llama 2 7B parameter chat model optimized for conversational AI",
    "type": "text_generation",
    "owner_id": "0x9876543210fedcba...",
    "is_private": false,
    "pricing": {
      "model": "per_token",
      "input_rate": 0.5,
      "output_rate": 1.0,
      "currency": "credits_per_1k_tokens"
    },
    "capabilities": ["chat", "text_generation", "instruction_following"],
    "max_tokens": 4096,
    "context_length": 4096,
    "parameters": {
      "size": "7B",
      "architecture": "Llama",
      "precision": "fp16",
      "quantization": "none"
    },
    "supported_languages": ["en", "es", "fr", "de", "it", "pt", "ru", "ja", "ko", "zh"],
    "tags": ["conversational", "instruction", "chat"],
    "version": "2.0",
    "license": "custom",
    "created_at": 1640995200,
    "updated_at": 1640995800
  }
}
```

---

### Model Inference

Make an inference request to a specific model.

```http
POST /v1/models/{model_id}/inference
```

#### Parameters

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `model_id` | string | Yes | The ID of the model to use for inference |

#### Request Body

The request body format depends on the model type. See [Model-Specific Schemas](#model-specific-schemas) for details.

#### Common Parameters

| Parameter | Type | Required | Default | Description |
|-----------|------|----------|---------|-------------|
| `max_tokens` | integer | No | 1000 | Maximum tokens to generate |
| `temperature` | number | No | 0.7 | Sampling temperature (0.0 to 2.0) |
| `top_p` | number | No | 1.0 | Nucleus sampling parameter |
| `stream` | boolean | No | false | Whether to stream the response |
| `stop` | string/array | No | null | Stop sequences |
| `presence_penalty` | number | No | 0.0 | Presence penalty (-2.0 to 2.0) |
| `frequency_penalty` | number | No | 0.0 | Frequency penalty (-2.0 to 2.0) |

---

## Model-Specific Schemas

### Chat Completion Models

For models that support chat-based interactions.

#### Request Schema

```json
{
  "messages": [
    {
      "role": "system",
      "content": "You are a helpful assistant."
    },
    {
      "role": "user",
      "content": "Hello, how are you?"
    }
  ],
  "max_tokens": 1000,
  "temperature": 0.7,
  "top_p": 1.0,
  "n": 1,
  "stream": false,
  "stop": null,
  "presence_penalty": 0,
  "frequency_penalty": 0,
  "logit_bias": {},
  "user": "user-123"
}
```

#### Message Object

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `role` | string | Yes | Message role (`system`, `user`, `assistant`) |
| `content` | string | Yes | Message content |
| `name` | string | No | Name of the message author |

#### Response Schema (Non-Streaming)

```json
{
  "id": "chatcmpl-abc123",
  "object": "chat.completion",
  "created": 1640995200,
  "model": "llama2-7b-chat",
  "choices": [
    {
      "index": 0,
      "message": {
        "role": "assistant",
        "content": "Hello! I'm doing well, thank you for asking. How can I help you today?"
      },
      "finish_reason": "stop"
    }
  ],
  "usage": {
    "prompt_tokens": 13,
    "completion_tokens": 17,
    "total_tokens": 30
  }
}
```

#### Response Schema (Streaming)

```
data: {"id":"chatcmpl-abc123","object":"chat.completion.chunk","created":1640995200,"model":"llama2-7b-chat","choices":[{"index":0,"delta":{"role":"assistant","content":""},"finish_reason":null}]}

data: {"id":"chatcmpl-abc123","object":"chat.completion.chunk","created":1640995200,"model":"llama2-7b-chat","choices":[{"index":0,"delta":{"content":"Hello"},"finish_reason":null}]}

data: {"id":"chatcmpl-abc123","object":"chat.completion.chunk","created":1640995200,"model":"llama2-7b-chat","choices":[{"index":0,"delta":{"content":"!"},"finish_reason":null}]}

data: {"id":"chatcmpl-abc123","object":"chat.completion.chunk","created":1640995200,"model":"llama2-7b-chat","choices":[{"index":0,"delta":{},"finish_reason":"stop"}]}

data: [DONE]
```

### Text Completion Models

For models that complete text prompts.

#### Request Schema

```json
{
  "prompt": "Once upon a time",
  "max_tokens": 100,
  "temperature": 0.7,
  "top_p": 1.0,
  "n": 1,
  "stream": false,
  "logprobs": null,
  "echo": false,
  "stop": null,
  "presence_penalty": 0,
  "frequency_penalty": 0,
  "best_of": 1,
  "logit_bias": {},
  "user": "user-123"
}
```

#### Response Schema

```json
{
  "id": "cmpl-abc123",
  "object": "text_completion",
  "created": 1640995200,
  "model": "gpt-3.5-turbo-instruct",
  "choices": [
    {
      "text": ", there was a brave knight who embarked on a quest to save the kingdom.",
      "index": 0,
      "logprobs": null,
      "finish_reason": "stop"
    }
  ],
  "usage": {
    "prompt_tokens": 4,
    "completion_tokens": 16,
    "total_tokens": 20
  }
}
```

### Image Generation Models

For models that generate images from text prompts.

#### Request Schema

```json
{
  "prompt": "A cute baby sea otter",
  "n": 1,
  "size": "1024x1024",
  "response_format": "url",
  "user": "user-123"
}
```

#### Parameters

| Parameter | Type | Required | Default | Description |
|-----------|------|----------|---------|-------------|
| `prompt` | string | Yes | - | Text description of the desired image |
| `n` | integer | No | 1 | Number of images to generate (1-10) |
| `size` | string | No | "1024x1024" | Image size (`256x256`, `512x512`, `1024x1024`) |
| `response_format` | string | No | "url" | Response format (`url` or `b64_json`) |
| `user` | string | No | - | Unique identifier for the user |

#### Response Schema

```json
{
  "created": 1640995200,
  "data": [
    {
      "url": "https://formation.ai/generated/images/abc123.png"
    }
  ]
}
```

### Embedding Models

For models that generate text embeddings.

#### Request Schema

```json
{
  "input": ["The food was delicious and the waiter was friendly."],
  "model": "text-embedding-ada-002",
  "encoding_format": "float",
  "user": "user-123"
}
```

#### Parameters

| Parameter | Type | Required | Default | Description |
|-----------|------|----------|---------|-------------|
| `input` | string/array | Yes | - | Text to embed |
| `encoding_format` | string | No | "float" | Encoding format (`float` or `base64`) |
| `user` | string | No | - | Unique identifier for the user |

#### Response Schema

```json
{
  "object": "list",
  "data": [
    {
      "object": "embedding",
      "embedding": [0.0023064255, -0.009327292, ...],
      "index": 0
    }
  ],
  "model": "text-embedding-ada-002",
  "usage": {
    "prompt_tokens": 8,
    "total_tokens": 8
  }
}
```

---

## Response Objects

### Usage Object

Appears in all inference responses to track token usage and costs.

```json
{
  "prompt_tokens": 13,
  "completion_tokens": 17,
  "total_tokens": 30,
  "cost_credits": 0.03
}
```

| Field | Type | Description |
|-------|------|-------------|
| `prompt_tokens` | integer | Number of tokens in the prompt |
| `completion_tokens` | integer | Number of tokens in the completion |
| `total_tokens` | integer | Total tokens used |
| `cost_credits` | number | Cost in Formation credits |

### Choice Object

Represents a single completion choice.

```json
{
  "index": 0,
  "message": {
    "role": "assistant",
    "content": "Hello! How can I help you?"
  },
  "finish_reason": "stop"
}
```

| Field | Type | Description |
|-------|------|-------------|
| `index` | integer | Choice index |
| `message` | object | Message object (for chat completions) |
| `text` | string | Generated text (for text completions) |
| `finish_reason` | string | Reason completion finished |

### Finish Reasons

| Reason | Description |
|--------|-------------|
| `stop` | Natural stopping point or stop sequence reached |
| `length` | Maximum token limit reached |
| `content_filter` | Content filtered due to policy violations |
| `function_call` | Model called a function (for function-calling models) |

---

## Error Codes and Handling

### HTTP Status Codes

| Status | Description |
|--------|-------------|
| `200` | Success |
| `400` | Bad Request - Invalid parameters |
| `401` | Unauthorized - Authentication failed |
| `403` | Forbidden - Access denied |
| `404` | Not Found - Model or resource not found |
| `429` | Too Many Requests - Rate limit exceeded |
| `500` | Internal Server Error |
| `502` | Bad Gateway - Model service unavailable |
| `503` | Service Unavailable - Temporary overload |

### Error Response Format

```json
{
  "error": {
    "code": "ERROR_CODE",
    "message": "Human-readable error message",
    "type": "error_type",
    "param": "parameter_name",
    "details": {
      "additional": "context"
    }
  }
}
```

### Common Error Codes

#### Authentication Errors

```json
{
  "error": {
    "code": "AUTHENTICATION_FAILED",
    "message": "Invalid signature or address",
    "type": "authentication_error"
  }
}
```

```json
{
  "error": {
    "code": "INVALID_SIGNATURE",
    "message": "ECDSA signature verification failed",
    "type": "authentication_error",
    "details": {
      "provided_address": "0x1234567890abcdef...",
      "recovered_address": "0xabcdef1234567890..."
    }
  }
}
```

#### Request Validation Errors

```json
{
  "error": {
    "code": "INVALID_REQUEST",
    "message": "Missing required parameter: messages",
    "type": "invalid_request_error",
    "param": "messages"
  }
}
```

```json
{
  "error": {
    "code": "INVALID_PARAMETER",
    "message": "temperature must be between 0.0 and 2.0",
    "type": "invalid_request_error",
    "param": "temperature",
    "details": {
      "provided_value": 3.5,
      "valid_range": "0.0 - 2.0"
    }
  }
}
```

#### Model Errors

```json
{
  "error": {
    "code": "MODEL_NOT_FOUND",
    "message": "Model 'invalid-model-id' not found",
    "type": "invalid_request_error",
    "param": "model"
  }
}
```

```json
{
  "error": {
    "code": "MODEL_UNAVAILABLE",
    "message": "Model is temporarily unavailable",
    "type": "service_unavailable_error",
    "details": {
      "retry_after_seconds": 30,
      "estimated_wait_time": "1-2 minutes"
    }
  }
}
```

#### Rate Limiting Errors

```json
{
  "error": {
    "code": "RATE_LIMIT_EXCEEDED",
    "message": "Rate limit exceeded. Try again in 3600 seconds.",
    "type": "rate_limit_error",
    "details": {
      "retry_after": 3600,
      "limit_type": "requests_per_hour",
      "current_limit": 1000,
      "reset_time": 1640999200
    }
  }
}
```

#### Billing Errors

```json
{
  "error": {
    "code": "INSUFFICIENT_CREDITS",
    "message": "Insufficient credits for this request",
    "type": "billing_error",
    "details": {
      "required_credits": 50,
      "available_credits": 25,
      "shortfall": 25
    }
  }
}
```

```json
{
  "error": {
    "code": "BUDGET_EXCEEDED",
    "message": "Monthly budget limit exceeded",
    "type": "billing_error",
    "details": {
      "monthly_limit": 10000,
      "current_usage": 10050,
      "reset_date": "2024-02-01"
    }
  }
}
```

#### Content Policy Errors

```json
{
  "error": {
    "code": "CONTENT_POLICY_VIOLATION",
    "message": "Content violates usage policies",
    "type": "content_policy_error",
    "details": {
      "violation_type": "harmful_content",
      "flagged_content": "portion of content that triggered the filter"
    }
  }
}
```

#### Server Errors

```json
{
  "error": {
    "code": "INTERNAL_ERROR",
    "message": "An internal error occurred",
    "type": "server_error",
    "details": {
      "request_id": "req_abc123",
      "timestamp": 1640995200
    }
  }
}
```

```json
{
  "error": {
    "code": "MODEL_TIMEOUT",
    "message": "Model inference timed out",
    "type": "server_error",
    "details": {
      "timeout_seconds": 30,
      "partial_response": false
    }
  }
}
```

---

## Rate Limiting

Formation implements rate limiting to ensure fair usage across all users.

### Rate Limit Headers

All responses include rate limiting information:

```http
X-RateLimit-Limit: 1000
X-RateLimit-Remaining: 999
X-RateLimit-Reset: 1640999200
X-RateLimit-Type: requests_per_hour
X-RateLimit-Retry-After: 3600
```

| Header | Description |
|--------|-------------|
| `X-RateLimit-Limit` | Maximum requests allowed in the time window |
| `X-RateLimit-Remaining` | Requests remaining in current window |
| `X-RateLimit-Reset` | Unix timestamp when the rate limit resets |
| `X-RateLimit-Type` | Type of rate limit (`requests_per_hour`, `tokens_per_hour`) |
| `X-RateLimit-Retry-After` | Seconds to wait before retrying (when rate limited) |

### Rate Limit Tiers

| Tier | Requests/Hour | Tokens/Hour | Concurrent Requests |
|------|---------------|-------------|-------------------|
| **Free** | 100 | 10,000 | 2 |
| **Pro** | 1,000 | 100,000 | 5 |
| **Pro Plus** | 5,000 | 500,000 | 10 |
| **Power** | 10,000 | 1,000,000 | 20 |
| **Power Plus** | 50,000 | 5,000,000 | 50 |

### Handling Rate Limits

When you exceed rate limits, implement exponential backoff:

```python
import time
import random

def handle_rate_limit(response):
    """Handle rate limit response with exponential backoff"""
    if response.status_code == 429:
        retry_after = int(response.headers.get('X-RateLimit-Retry-After', 60))
        jitter = random.uniform(0.1, 0.3) * retry_after
        wait_time = retry_after + jitter
        
        print(f"Rate limited. Waiting {wait_time:.2f} seconds...")
        time.sleep(wait_time)
        return True
    return False
```

---

## Webhooks (Optional)

Some models support webhook notifications for long-running inference requests.

### Webhook Request

```json
{
  "messages": [{"role": "user", "content": "Generate a long story"}],
  "max_tokens": 4000,
  "webhook_url": "https://your-app.com/webhooks/inference",
  "webhook_secret": "your-webhook-secret"
}
```

### Webhook Payload

```json
{
  "request_id": "req_abc123",
  "model_id": "llama2-7b-chat",
  "status": "completed",
  "result": {
    "id": "chatcmpl-abc123",
    "object": "chat.completion",
    "created": 1640995200,
    "model": "llama2-7b-chat",
    "choices": [
      {
        "index": 0,
        "message": {
          "role": "assistant",
          "content": "Once upon a time..."
        },
        "finish_reason": "stop"
      }
    ],
    "usage": {
      "prompt_tokens": 13,
      "completion_tokens": 1500,
      "total_tokens": 1513
    }
  },
  "timestamp": 1640995800
}
```

---

## SDK Examples

### Python

```python
import requests
from typing import Dict, Any, Optional

class FormationClient:
    def __init__(self, private_key: str):
        self.base_url = "https://formation.ai"
        self.headers = self._generate_auth_headers(private_key)
    
    def chat_completion(self, model_id: str, messages: list, **kwargs) -> Dict[str, Any]:
        """Create a chat completion"""
        payload = {
            "messages": messages,
            **kwargs
        }
        
        response = requests.post(
            f"{self.base_url}/v1/models/{model_id}/inference",
            headers=self.headers,
            json=payload
        )
        
        response.raise_for_status()
        return response.json()
    
    def list_models(self) -> Dict[str, Any]:
        """List available models"""
        response = requests.get(
            f"{self.base_url}/v1/models",
            headers=self.headers
        )
        
        response.raise_for_status()
        return response.json()

# Usage
client = FormationClient("0x1234567890abcdef...")
models = client.list_models()
result = client.chat_completion(
    "llama2-7b-chat",
    [{"role": "user", "content": "Hello!"}]
)
```

### JavaScript

```javascript
class FormationClient {
    constructor(privateKey) {
        this.baseUrl = 'https://formation.ai';
        this.headers = this._generateAuthHeaders(privateKey);
    }
    
    async chatCompletion(modelId, messages, options = {}) {
        const payload = {
            messages,
            ...options
        };
        
        const response = await fetch(`${this.baseUrl}/v1/models/${modelId}/inference`, {
            method: 'POST',
            headers: this.headers,
            body: JSON.stringify(payload)
        });
        
        if (!response.ok) {
            throw new Error(`HTTP ${response.status}: ${response.statusText}`);
        }
        
        return response.json();
    }
    
    async listModels() {
        const response = await fetch(`${this.baseUrl}/v1/models`, {
            headers: this.headers
        });
        
        if (!response.ok) {
            throw new Error(`HTTP ${response.status}: ${response.statusText}`);
        }
        
        return response.json();
    }
}

// Usage
const client = new FormationClient('0x1234567890abcdef...');
const models = await client.listModels();
const result = await client.chatCompletion(
    'llama2-7b-chat',
    [{ role: 'user', content: 'Hello!' }]
);
```

---

## OpenAI Compatibility

Formation's API is designed to be a drop-in replacement for OpenAI's API. Here's how to migrate:

### URL Changes

```python
# OpenAI
openai.api_base = "https://api.openai.com/v1"

# Formation
formation_base = "https://formation.ai/v1/models/{model_id}/inference"
```

### Authentication Changes

```python
# OpenAI
openai.api_key = "sk-..."

# Formation
headers = {
    "X-Formation-Address": "0x...",
    "X-Formation-Signature": "0x...",
    "X-Formation-Message": "Formation authentication request"
}
```

### Request Changes

```python
# OpenAI
response = openai.ChatCompletion.create(
    model="gpt-3.5-turbo",
    messages=[{"role": "user", "content": "Hello"}]
)

# Formation
response = requests.post(
    "https://formation.ai/v1/models/llama2-7b-chat/inference",
    headers=headers,
    json={
        "messages": [{"role": "user", "content": "Hello"}]
    }
)
```

---

## Next Steps

- **[Inference Guide](inference-guide.md)** - Getting started with model inference
- **[Code Examples](examples.md)** - Working examples and integration patterns
- **[Agent API Reference](../agents/hiring-agents.md)** - Learn about AI agents

---

**Need help with the API?** Check our [troubleshooting guide](inference-guide.md#error-handling-and-troubleshooting) or contact support! 🚀 