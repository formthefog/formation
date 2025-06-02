# Making Inference Requests to AI Models

This guide shows you how to make inference requests to AI models deployed on the Formation network. Formation provides OpenAI-compatible APIs, making it easy to integrate with existing applications and tools.

## Overview

Formation AI models serve inference requests through OpenAI-compatible endpoints, allowing you to:

- **Use Existing Tools**: Works with OpenAI client libraries and applications
- **Switch Seamlessly**: Drop-in replacement for OpenAI API calls
- **Access Specialized Models**: Use custom models not available elsewhere
- **Pay Per Use**: Only pay for the inference requests you make

### Formation vs OpenAI API

| Feature | Formation | OpenAI |
|---------|-----------|---------|
| **API Format** | OpenAI-compatible | Native OpenAI |
| **Authentication** | ECDSA signatures | API keys |
| **Model Selection** | Custom + standard models | OpenAI models only |
| **Billing** | Credit-based, transparent | Usage-based |
| **Availability** | Decentralized network | Centralized service |

---

## OpenAI API Compatibility Overview

Formation models implement the OpenAI API v1 specification, ensuring compatibility with existing tools and libraries.

### Supported Endpoints

#### Core Inference Endpoints
```http
POST /v1/models/{model_id}/inference    # Formation-specific endpoint
POST /v1/chat/completions               # OpenAI-compatible chat
POST /v1/completions                    # OpenAI-compatible text completion
GET  /v1/models                         # List available models
```

#### Optional Endpoints (model-dependent)
```http
POST /v1/embeddings                     # Text embeddings
POST /v1/images/generations             # Image generation
POST /v1/audio/transcriptions           # Speech-to-text
```

### Request/Response Format

Formation models accept and return data in the exact same format as OpenAI's API:

```json
{
  "model": "formation-model-id",
  "messages": [
    {"role": "user", "content": "Hello, how are you?"}
  ],
  "max_tokens": 1000,
  "temperature": 0.7
}
```

---

## Formation Endpoint Configuration

### 1. Base URL Structure

Formation uses a different base URL structure than OpenAI:

```
OpenAI:    https://api.openai.com/v1/chat/completions
Formation: https://formation.ai/v1/models/{model_id}/inference
```

### 2. Model-Specific Endpoints

Each model on Formation has its own inference endpoint:

```bash
# General format
https://formation.ai/v1/models/{model_id}/inference

# Examples
https://formation.ai/v1/models/llama2-7b-chat/inference
https://formation.ai/v1/models/stable-diffusion-xl/inference
https://formation.ai/v1/models/whisper-large-v3/inference
```

### 3. Discovering Available Models

List all available models:

```bash
curl -X GET "https://formation.ai/v1/models" \
  -H "X-Formation-Address: 0x1234567890abcdef..." \
  -H "X-Formation-Signature: 0xabcdef..." \
  -H "X-Formation-Message: Formation authentication request"
```

**Response:**
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
      "context_length": 4096
    }
  ],
  "total": 1
}
```

---

## Authentication with ECDSA Signatures

Formation uses ECDSA signature authentication instead of API keys for enhanced security.

### 1. Understanding ECDSA Authentication

- **More Secure**: No API keys to leak or manage
- **Wallet Compatible**: Works with Ethereum wallets
- **Cryptographically Secure**: Uses the same security as blockchain transactions
- **Decentralized**: No central authority manages keys

### 2. Required Headers

Every request must include these headers:

```http
X-Formation-Address: 0x1234567890abcdef1234567890abcdef12345678
X-Formation-Signature: 0xabcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890ab
X-Formation-Message: Formation authentication request
Content-Type: application/json
```

### 3. Generating Signatures

#### Python Example

```python
import hashlib
from eth_account import Account

# Your private key (keep this secure!)
private_key = "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef12"

# Create account from private key
account = Account.from_key(private_key)

# Create message to sign
message = "Formation authentication request"
message_hash = hashlib.sha256(message.encode()).hexdigest()

# Sign the message
signature = account.signHash(message_hash)

# Headers for API requests
headers = {
    "X-Formation-Address": account.address,
    "X-Formation-Signature": signature.signature.hex(),
    "X-Formation-Message": message,
    "Content-Type": "application/json"
}
```

#### JavaScript Example

```javascript
const { ethers } = require('ethers');
const crypto = require('crypto');

// Your private key
const privateKey = '0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef12';

// Create wallet
const wallet = new ethers.Wallet(privateKey);

// Create message to sign
const message = 'Formation authentication request';
const messageHash = crypto.createHash('sha256').update(message).digest('hex');

// Sign the message
const signature = wallet.signMessage(messageHash);

// Headers for API requests
const headers = {
    'X-Formation-Address': wallet.address,
    'X-Formation-Signature': signature,
    'X-Formation-Message': message,
    'Content-Type': 'application/json'
};
```

### 4. Security Best Practices

- **Protect Private Keys**: Never share or expose your private keys
- **Use Environment Variables**: Store keys securely in environment variables
- **Rotate Keys**: Periodically generate new key pairs
- **Monitor Usage**: Track your API usage and costs

---

## Making Your First Inference Request

### 1. Simple Text Generation

```bash
curl -X POST "https://formation.ai/v1/models/llama2-7b-chat/inference" \
  -H "X-Formation-Address: 0x1234567890abcdef..." \
  -H "X-Formation-Signature: 0xabcdef..." \
  -H "X-Formation-Message: Formation authentication request" \
  -H "Content-Type: application/json" \
  -d '{
    "messages": [
      {
        "role": "user",
        "content": "Explain quantum computing in simple terms"
      }
    ],
    "max_tokens": 500,
    "temperature": 0.7
  }'
```

**Response:**
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
        "content": "Quantum computing is like having a super-powered computer that can solve certain problems much faster than regular computers. Instead of using regular bits that are either 0 or 1, quantum computers use quantum bits (qubits) that can be both 0 and 1 at the same time..."
      },
      "finish_reason": "stop"
    }
  ],
  "usage": {
    "prompt_tokens": 12,
    "completion_tokens": 156,
    "total_tokens": 168
  }
}
```

### 2. Chat Conversation

```python
import requests
import json

# Authentication headers (generated as shown above)
headers = {
    "X-Formation-Address": "0x1234567890abcdef...",
    "X-Formation-Signature": "0xabcdef...",
    "X-Formation-Message": "Formation authentication request",
    "Content-Type": "application/json"
}

# Start a conversation
conversation = [
    {"role": "system", "content": "You are a helpful AI assistant."},
    {"role": "user", "content": "What's the weather like today?"}
]

response = requests.post(
    "https://formation.ai/v1/models/llama2-7b-chat/inference",
    headers=headers,
    json={
        "messages": conversation,
        "max_tokens": 200,
        "temperature": 0.7
    }
)

result = response.json()
assistant_message = result["choices"][0]["message"]["content"]

# Continue the conversation
conversation.append({"role": "assistant", "content": assistant_message})
conversation.append({"role": "user", "content": "What about tomorrow?"})

# Make another request
response = requests.post(
    "https://formation.ai/v1/models/llama2-7b-chat/inference",
    headers=headers,
    json={
        "messages": conversation,
        "max_tokens": 200,
        "temperature": 0.7
    }
)
```

### 3. Streaming Responses

For real-time response streaming:

```python
import requests
import json

def stream_inference(model_id, messages, headers):
    """Stream inference responses in real-time"""
    
    response = requests.post(
        f"https://formation.ai/v1/models/{model_id}/inference",
        headers={**headers, "Accept": "text/event-stream"},
        json={
            "messages": messages,
            "max_tokens": 500,
            "temperature": 0.7,
            "stream": True
        },
        stream=True
    )
    
    for line in response.iter_lines():
        if line:
            line = line.decode('utf-8')
            if line.startswith('data: '):
                data = line[6:]  # Remove 'data: ' prefix
                if data == '[DONE]':
                    break
                try:
                    chunk = json.loads(data)
                    if 'choices' in chunk and len(chunk['choices']) > 0:
                        delta = chunk['choices'][0].get('delta', {})
                        content = delta.get('content', '')
                        if content:
                            print(content, end='', flush=True)
                except json.JSONDecodeError:
                    continue

# Usage
messages = [{"role": "user", "content": "Write a short story about a robot"}]
stream_inference("llama2-7b-chat", messages, headers)
```

---

## Rate Limiting and Billing Considerations

### 1. Understanding Formation's Billing

Formation uses a credit-based billing system:

- **Credits**: Internal currency for paying for inference
- **Token-Based**: Most models charge per input/output token
- **Transparent Pricing**: See exact costs before making requests
- **No Surprise Bills**: Credits are pre-purchased

### 2. Rate Limits

Formation implements rate limiting to ensure fair usage:

#### Default Rate Limits
- **Free Tier**: 100 requests/hour, 10,000 tokens/hour
- **Pro Tier**: 1,000 requests/hour, 100,000 tokens/hour
- **Pro Plus Tier**: 5,000 requests/hour, 500,000 tokens/hour
- **Power Tier**: 10,000 requests/hour, 1,000,000 tokens/hour

#### Rate Limit Headers

Responses include rate limit information:

```http
X-RateLimit-Limit: 1000
X-RateLimit-Remaining: 999
X-RateLimit-Reset: 1640995800
X-RateLimit-Type: requests_per_hour
```

### 3. Cost Estimation

Before making requests, estimate costs:

```python
def estimate_inference_cost(model_id, input_text, max_tokens=1000):
    """Estimate the cost of an inference request"""
    
    # Get model pricing information
    response = requests.get(
        f"https://formation.ai/v1/models/{model_id}",
        headers=headers
    )
    
    model_info = response.json()["model"]
    pricing = model_info["pricing"]
    
    if pricing["model"] == "per_token":
        # Estimate input tokens (rough approximation: 1 token ≈ 4 characters)
        estimated_input_tokens = len(input_text) // 4
        estimated_output_tokens = max_tokens
        
        input_cost = (estimated_input_tokens / 1000) * pricing["input_rate"]
        output_cost = (estimated_output_tokens / 1000) * pricing["output_rate"]
        
        total_cost = input_cost + output_cost
        
        return {
            "estimated_input_tokens": estimated_input_tokens,
            "estimated_output_tokens": estimated_output_tokens,
            "estimated_cost_credits": total_cost,
            "pricing_model": "per_token"
        }
    
    elif pricing["model"] == "per_request":
        return {
            "estimated_cost_credits": pricing["base_cost"],
            "pricing_model": "per_request"
        }

# Example usage
cost_estimate = estimate_inference_cost(
    "llama2-7b-chat",
    "Explain the theory of relativity",
    max_tokens=500
)

print(f"Estimated cost: {cost_estimate['estimated_cost_credits']} credits")
```

### 4. Monitoring Usage and Costs

Track your usage to avoid unexpected charges:

```python
def get_usage_summary(time_period="today"):
    """Get usage summary for a time period"""
    
    response = requests.get(
        f"https://formation.ai/v1/account/usage?period={time_period}",
        headers=headers
    )
    
    usage_data = response.json()
    
    return {
        "total_requests": usage_data["total_requests"],
        "total_tokens": usage_data["total_tokens"],
        "total_cost_credits": usage_data["total_cost_credits"],
        "models_used": usage_data["models_used"],
        "period": time_period
    }

# Check today's usage
today_usage = get_usage_summary("today")
print(f"Today's usage: {today_usage['total_requests']} requests, {today_usage['total_cost_credits']} credits")

# Check this month's usage
month_usage = get_usage_summary("this_month")
print(f"This month: {month_usage['total_requests']} requests, {month_usage['total_cost_credits']} credits")
```

### 5. Budget Management

Set up budget alerts and limits:

```python
def set_budget_limit(monthly_limit_credits):
    """Set a monthly budget limit"""
    
    response = requests.post(
        "https://formation.ai/v1/account/budget",
        headers=headers,
        json={
            "monthly_limit_credits": monthly_limit_credits,
            "alert_thresholds": [50, 75, 90],  # Alert at 50%, 75%, 90% of budget
            "hard_limit": True  # Stop requests when budget is exceeded
        }
    )
    
    return response.json()

# Set a $100 equivalent budget (assuming 1 credit = $0.01)
budget_result = set_budget_limit(10000)
print(f"Budget set: {budget_result}")
```

---

## Working with Different Model Types

### 1. Text Generation Models

#### Chat Models
Best for conversational AI and instruction-following:

```python
def chat_with_model(model_id, system_prompt, user_message):
    """Chat with a conversational model"""
    
    messages = [
        {"role": "system", "content": system_prompt},
        {"role": "user", "content": user_message}
    ]
    
    response = requests.post(
        f"https://formation.ai/v1/models/{model_id}/inference",
        headers=headers,
        json={
            "messages": messages,
            "max_tokens": 1000,
            "temperature": 0.7
        }
    )
    
    return response.json()["choices"][0]["message"]["content"]

# Example usage
response = chat_with_model(
    "llama2-7b-chat",
    "You are a helpful coding assistant.",
    "Write a Python function to calculate fibonacci numbers"
)
```

#### Completion Models
Best for text completion and generation:

```python
def complete_text(model_id, prompt, max_tokens=500):
    """Complete text using a completion model"""
    
    response = requests.post(
        f"https://formation.ai/v1/models/{model_id}/inference",
        headers=headers,
        json={
            "prompt": prompt,
            "max_tokens": max_tokens,
            "temperature": 0.8,
            "top_p": 0.9
        }
    )
    
    return response.json()["choices"][0]["text"]

# Example usage
completed_text = complete_text(
    "gpt-3.5-turbo-instruct",
    "The future of artificial intelligence is"
)
```

### 2. Image Generation Models

```python
def generate_image(model_id, prompt, size="1024x1024"):
    """Generate an image from a text prompt"""
    
    response = requests.post(
        f"https://formation.ai/v1/models/{model_id}/inference",
        headers=headers,
        json={
            "prompt": prompt,
            "size": size,
            "n": 1,
            "response_format": "url"
        }
    )
    
    return response.json()["data"][0]["url"]

# Example usage
image_url = generate_image(
    "stable-diffusion-xl",
    "A beautiful sunset over a mountain lake, photorealistic"
)
print(f"Generated image: {image_url}")
```

### 3. Embedding Models

```python
def get_embeddings(model_id, texts):
    """Get embeddings for text inputs"""
    
    response = requests.post(
        f"https://formation.ai/v1/models/{model_id}/inference",
        headers=headers,
        json={
            "input": texts,
            "encoding_format": "float"
        }
    )
    
    return [item["embedding"] for item in response.json()["data"]]

# Example usage
embeddings = get_embeddings(
    "text-embedding-ada-002",
    ["Hello world", "How are you?", "Goodbye"]
)
```

---

## Error Handling and Troubleshooting

### 1. Common Error Codes

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

**Solution**: Verify your signature generation and ensure the message matches.

#### Model Not Found

```json
{
  "error": {
    "code": "MODEL_NOT_FOUND",
    "message": "Model 'invalid-model-id' not found",
    "type": "invalid_request_error"
  }
}
```

**Solution**: Check the model ID and ensure it exists and is accessible.

#### Rate Limit Exceeded

```json
{
  "error": {
    "code": "RATE_LIMIT_EXCEEDED",
    "message": "Rate limit exceeded. Try again in 3600 seconds.",
    "type": "rate_limit_error",
    "retry_after": 3600
  }
}
```

**Solution**: Wait for the specified time or upgrade your subscription tier.

#### Insufficient Credits

```json
{
  "error": {
    "code": "INSUFFICIENT_CREDITS",
    "message": "Insufficient credits for this request. Required: 50, Available: 25",
    "type": "billing_error",
    "required_credits": 50,
    "available_credits": 25
  }
}
```

**Solution**: Add credits to your account or upgrade your subscription.

### 2. Implementing Retry Logic

```python
import time
import random
from typing import Dict, Any

def make_inference_request_with_retry(
    model_id: str,
    payload: Dict[str, Any],
    max_retries: int = 3,
    base_delay: float = 1.0
) -> Dict[str, Any]:
    """Make inference request with exponential backoff retry logic"""
    
    for attempt in range(max_retries + 1):
        try:
            response = requests.post(
                f"https://formation.ai/v1/models/{model_id}/inference",
                headers=headers,
                json=payload,
                timeout=30
            )
            
            if response.status_code == 200:
                return response.json()
            
            elif response.status_code == 429:  # Rate limited
                error_data = response.json()
                retry_after = error_data.get("retry_after", base_delay * (2 ** attempt))
                
                if attempt < max_retries:
                    print(f"Rate limited. Retrying in {retry_after} seconds...")
                    time.sleep(retry_after)
                    continue
                else:
                    raise Exception(f"Rate limit exceeded after {max_retries} retries")
            
            elif response.status_code in [500, 502, 503, 504]:  # Server errors
                if attempt < max_retries:
                    delay = base_delay * (2 ** attempt) + random.uniform(0, 1)
                    print(f"Server error. Retrying in {delay:.2f} seconds...")
                    time.sleep(delay)
                    continue
                else:
                    raise Exception(f"Server error after {max_retries} retries")
            
            else:
                # Non-retryable error
                response.raise_for_status()
                
        except requests.exceptions.RequestException as e:
            if attempt < max_retries:
                delay = base_delay * (2 ** attempt) + random.uniform(0, 1)
                print(f"Request failed: {e}. Retrying in {delay:.2f} seconds...")
                time.sleep(delay)
                continue
            else:
                raise e
    
    raise Exception("All retry attempts failed")

# Example usage
try:
    result = make_inference_request_with_retry(
        "llama2-7b-chat",
        {
            "messages": [{"role": "user", "content": "Hello!"}],
            "max_tokens": 100
        }
    )
    print(result["choices"][0]["message"]["content"])
except Exception as e:
    print(f"Request failed: {e}")
```

### 3. Debugging Connection Issues

```python
def debug_connection(model_id):
    """Debug connection issues with Formation API"""
    
    print("🔍 Debugging Formation API connection...")
    
    # Test 1: Check if Formation API is reachable
    try:
        response = requests.get("https://formation.ai/health", timeout=10)
        print(f"✅ Formation API reachable: {response.status_code}")
    except Exception as e:
        print(f"❌ Formation API unreachable: {e}")
        return
    
    # Test 2: Check authentication
    try:
        response = requests.get(
            "https://formation.ai/v1/models",
            headers=headers,
            timeout=10
        )
        if response.status_code == 200:
            print("✅ Authentication successful")
        else:
            print(f"❌ Authentication failed: {response.status_code}")
            print(f"Response: {response.text}")
    except Exception as e:
        print(f"❌ Authentication test failed: {e}")
    
    # Test 3: Check if model exists
    try:
        response = requests.get(
            f"https://formation.ai/v1/models/{model_id}",
            headers=headers,
            timeout=10
        )
        if response.status_code == 200:
            print(f"✅ Model {model_id} exists and accessible")
        else:
            print(f"❌ Model {model_id} not accessible: {response.status_code}")
    except Exception as e:
        print(f"❌ Model check failed: {e}")
    
    # Test 4: Simple inference test
    try:
        response = requests.post(
            f"https://formation.ai/v1/models/{model_id}/inference",
            headers=headers,
            json={
                "messages": [{"role": "user", "content": "test"}],
                "max_tokens": 10
            },
            timeout=30
        )
        if response.status_code == 200:
            print("✅ Inference test successful")
        else:
            print(f"❌ Inference test failed: {response.status_code}")
            print(f"Response: {response.text}")
    except Exception as e:
        print(f"❌ Inference test failed: {e}")

# Run diagnostics
debug_connection("llama2-7b-chat")
```

---

## Best Practices

### 1. Efficient API Usage

- **Batch Requests**: Group multiple requests when possible
- **Optimize Prompts**: Use clear, concise prompts to reduce token usage
- **Set Appropriate Limits**: Use `max_tokens` to control response length
- **Cache Results**: Store frequently used responses to avoid repeated requests

### 2. Cost Optimization

- **Monitor Usage**: Regularly check your usage and costs
- **Choose Right Models**: Use smaller models for simpler tasks
- **Optimize Parameters**: Adjust temperature and other parameters for efficiency
- **Set Budgets**: Use budget limits to prevent unexpected charges

### 3. Security

- **Protect Private Keys**: Never expose private keys in code or logs
- **Use Environment Variables**: Store credentials securely
- **Rotate Keys**: Periodically generate new key pairs
- **Monitor Access**: Track API usage for unusual patterns

### 4. Performance

- **Handle Timeouts**: Set appropriate timeout values
- **Implement Retry Logic**: Handle transient failures gracefully
- **Use Streaming**: For long responses, use streaming to improve user experience
- **Parallel Requests**: Make concurrent requests when appropriate

---

## Next Steps

- **[API Reference](api-reference.md)** - Complete API documentation and schemas
- **[Code Examples](examples.md)** - Working examples in multiple programming languages
- **[Agent Usage Guide](../agents/getting-started.md)** - Learn about using AI agents

---

**Ready to start making inference requests?** Begin with simple text generation and gradually explore more advanced features! 🚀 