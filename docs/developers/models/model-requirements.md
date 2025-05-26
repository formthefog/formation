# AI Model Requirements for Formation

This document outlines the technical requirements for deploying custom AI models on the Formation network. All models must implement these specifications to ensure compatibility with Formation's infrastructure and billing systems.

## Overview

Formation models are containerized AI inference services that serve OpenAI-compatible APIs. They integrate with Formation's decentralized network to provide scalable, monetizable AI inference capabilities.

### Key Requirements

- ✅ **OpenAI-Compatible API**: Implement standard OpenAI endpoints
- ✅ **HTTP Server**: Serve requests on port 8080 within container
- ✅ **Usage Metrics**: Report accurate usage data for billing
- ✅ **Health Monitoring**: Provide health check endpoints
- ✅ **Containerization**: Package as Docker container
- ✅ **Resource Efficiency**: Optimize for memory and compute usage

---

## 1. OpenAI-Compatible API Requirements

### 1.1 API Compatibility Standard

Formation models **MUST** implement OpenAI-compatible endpoints to ensure seamless integration with existing applications and Formation's routing infrastructure.

#### Supported OpenAI API Version
- **Target Version**: OpenAI API v1
- **Specification**: Follow [OpenAI API Reference](https://platform.openai.com/docs/api-reference)
- **Content-Type**: `application/json`
- **Character Encoding**: UTF-8

#### Request/Response Format
All endpoints must accept and return JSON in the exact format specified by OpenAI's API documentation.

### 1.2 Model Types and Capabilities

Formation supports various model types with specific API requirements:

#### Text Generation Models
```json
{
  "model_type": "text_generation",
  "required_endpoints": ["/v1/chat/completions", "/v1/completions"],
  "optional_endpoints": ["/v1/embeddings"],
  "io_modes": ["text_to_text"]
}
```

#### Image Generation Models
```json
{
  "model_type": "image_generation", 
  "required_endpoints": ["/v1/images/generations"],
  "optional_endpoints": ["/v1/images/edits", "/v1/images/variations"],
  "io_modes": ["text_to_image", "image_to_image"]
}
```

#### Multimodal Models
```json
{
  "model_type": "multimodal",
  "required_endpoints": ["/v1/chat/completions"],
  "optional_endpoints": ["/v1/completions", "/v1/embeddings"],
  "io_modes": ["text_to_text", "image_to_text", "text_to_image"]
}
```

---

## 2. Required Endpoints

### 2.1 POST /v1/chat/completions (Required for Chat Models)

Primary endpoint for chat-based completions.

#### Request Format

```json
{
  "model": "your-model-id",
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

#### Response Format

**Non-Streaming Response:**
```json
{
  "id": "chatcmpl-123",
  "object": "chat.completion",
  "created": 1677652288,
  "model": "your-model-id",
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

**Streaming Response (if stream=true):**
```
data: {"id":"chatcmpl-123","object":"chat.completion.chunk","created":1677652288,"model":"your-model-id","choices":[{"index":0,"delta":{"role":"assistant","content":""},"finish_reason":null}]}

data: {"id":"chatcmpl-123","object":"chat.completion.chunk","created":1677652288,"model":"your-model-id","choices":[{"index":0,"delta":{"content":"Hello"},"finish_reason":null}]}

data: {"id":"chatcmpl-123","object":"chat.completion.chunk","created":1677652288,"model":"your-model-id","choices":[{"index":0,"delta":{"content":"!"},"finish_reason":null}]}

data: {"id":"chatcmpl-123","object":"chat.completion.chunk","created":1677652288,"model":"your-model-id","choices":[{"index":0,"delta":{},"finish_reason":"stop"}]}

data: [DONE]
```

#### Implementation Example

```python
from flask import Flask, request, jsonify
import time
import uuid

app = Flask(__name__)

@app.route('/v1/chat/completions', methods=['POST'])
def chat_completions():
    data = request.json
    
    # Extract parameters
    messages = data.get('messages', [])
    max_tokens = data.get('max_tokens', 1000)
    temperature = data.get('temperature', 0.7)
    stream = data.get('stream', False)
    model = data.get('model', 'your-model-id')
    
    # Validate required fields
    if not messages:
        return jsonify({"error": {"message": "messages is required"}}), 400
    
    # Generate response using your model
    response_content = generate_response(messages, max_tokens, temperature)
    
    # Calculate token usage
    prompt_tokens = sum(len(msg['content'].split()) for msg in messages)
    completion_tokens = len(response_content.split())
    total_tokens = prompt_tokens + completion_tokens
    
    if stream:
        return stream_response(response_content, model, prompt_tokens, completion_tokens)
    else:
        return jsonify({
            "id": f"chatcmpl-{uuid.uuid4().hex[:8]}",
            "object": "chat.completion",
            "created": int(time.time()),
            "model": model,
            "choices": [{
                "index": 0,
                "message": {
                    "role": "assistant",
                    "content": response_content
                },
                "finish_reason": "stop"
            }],
            "usage": {
                "prompt_tokens": prompt_tokens,
                "completion_tokens": completion_tokens,
                "total_tokens": total_tokens
            }
        })

def generate_response(messages, max_tokens, temperature):
    # Your model inference logic here
    # This is where you call your actual model
    return "Generated response based on the conversation"
```

### 2.2 POST /v1/completions (Required for Text Models)

Text completion endpoint for non-chat models.

#### Request Format

```json
{
  "model": "your-model-id",
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

#### Response Format

```json
{
  "id": "cmpl-123",
  "object": "text_completion",
  "created": 1677652288,
  "model": "your-model-id",
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

### 2.3 GET /v1/models (Required)

List available models endpoint.

#### Response Format

```json
{
  "object": "list",
  "data": [
    {
      "id": "your-model-id",
      "object": "model",
      "created": 1677652288,
      "owned_by": "formation",
      "permission": [],
      "root": "your-model-id",
      "parent": null
    }
  ]
}
```

#### Implementation Example

```python
@app.route('/v1/models', methods=['GET'])
def list_models():
    return jsonify({
        "object": "list",
        "data": [
            {
                "id": "your-model-id",
                "object": "model", 
                "created": int(time.time()),
                "owned_by": "formation",
                "permission": [],
                "root": "your-model-id",
                "parent": None
            }
        ]
    })
```

### 2.4 GET /health (Required)

Health check endpoint for monitoring and load balancing.

#### Response Format

```json
{
  "status": "healthy",
  "model_loaded": true,
  "version": "1.0.0",
  "framework": "pytorch",
  "gpu_available": true,
  "memory_usage": {
    "used_mb": 2048,
    "total_mb": 8192,
    "percentage": 25.0
  },
  "model_info": {
    "model_id": "your-model-id",
    "parameters": "7B",
    "quantization": "int8"
  }
}
```

#### Status Values
- `healthy`: Model is fully operational
- `degraded`: Model is operational but with reduced performance  
- `unhealthy`: Model is not operational

#### Implementation Example

```python
@app.route('/health', methods=['GET'])
def health():
    try:
        # Check if model is loaded and responsive
        model_status = check_model_health()
        
        return jsonify({
            "status": "healthy" if model_status else "unhealthy",
            "model_loaded": model_status,
            "version": "1.0.0",
            "framework": "pytorch",
            "gpu_available": torch.cuda.is_available(),
            "memory_usage": get_memory_usage(),
            "model_info": {
                "model_id": "your-model-id",
                "parameters": "7B",
                "quantization": "int8"
            }
        })
    except Exception as e:
        return jsonify({
            "status": "unhealthy",
            "error": str(e)
        }), 500
```

### 2.5 Optional Endpoints

#### POST /v1/embeddings
For models that support text embeddings:

```json
{
  "input": ["The food was delicious and the waiter was friendly."],
  "model": "your-embedding-model",
  "encoding_format": "float"
}
```

#### POST /v1/images/generations
For image generation models:

```json
{
  "prompt": "A cute baby sea otter",
  "n": 1,
  "size": "1024x1024",
  "response_format": "url"
}
```

---

## 3. Authentication Handling

### 3.1 Formation-Managed Authentication

**IMPORTANT**: Formation handles ALL authentication for requests coming through the Formation network. Your model should NOT implement its own authentication for Formation requests.

#### Request Flow
```
User Request → Formation Gateway → Authentication → Your Model
```

#### What Your Model Receives
- **Pre-authenticated requests**: Formation has already verified the user
- **Clean HTTP requests**: No authentication headers to process
- **User context**: Available in request headers if needed

#### Implementation Guidelines

```python
@app.route('/v1/chat/completions', methods=['POST'])
def chat_completions():
    # DO NOT check authentication - Formation handles this
    # Just process the request directly
    
    data = request.json
    
    # Optional: Extract user context from Formation headers
    user_id = request.headers.get('X-Formation-User-Id')
    request_id = request.headers.get('X-Formation-Request-Id')
    
    # Process the request
    response = process_chat_completion(data)
    
    return jsonify(response)
```

### 3.2 Direct Access Protection

Your model should reject requests that bypass Formation:

```python
def check_formation_request():
    """Verify request comes from Formation"""
    formation_signature = request.headers.get('X-Formation-Signature')
    if not formation_signature:
        return False
    
    # Verify Formation's signature (optional additional security)
    return verify_formation_signature(formation_signature)

@app.before_request
def before_request():
    # Allow health checks from anywhere
    if request.path == '/health':
        return
    
    # Require Formation signature for API endpoints
    if not check_formation_request():
        return jsonify({"error": "Direct access not allowed"}), 403
```

### 3.3 Security Best Practices

- **No API Keys**: Don't require API keys from users
- **No User Authentication**: Formation handles user verification
- **Input Validation**: Always validate and sanitize inputs
- **Rate Limiting**: Implement basic rate limiting if needed
- **Error Handling**: Don't expose internal errors to users

---

## 4. Usage Metrics Reporting

### 4.1 Required Metrics

Your model **MUST** report accurate usage metrics for Formation's billing system:

#### Token-Based Metrics
```json
{
  "prompt_tokens": 150,
  "completion_tokens": 75,
  "total_tokens": 225
}
```

#### Resource-Based Metrics
```json
{
  "compute_units": 1.5,
  "memory_mb": 2048,
  "duration_seconds": 3.2,
  "gpu_seconds": 2.1
}
```

#### Request-Based Metrics
```json
{
  "requests_processed": 1,
  "cache_hits": 0,
  "cache_misses": 1
}
```

### 4.2 Token Counting

Accurate token counting is critical for billing:

#### For Text Models
```python
import tiktoken

def count_tokens(text, model="gpt-4"):
    """Count tokens using tiktoken (OpenAI's tokenizer)"""
    try:
        encoding = tiktoken.encoding_for_model(model)
        return len(encoding.encode(text))
    except KeyError:
        # Fallback to approximate counting
        return len(text.split()) * 1.3  # Rough approximation

def count_message_tokens(messages, model="gpt-4"):
    """Count tokens in a list of messages"""
    total_tokens = 0
    for message in messages:
        total_tokens += count_tokens(message.get('content', ''), model)
        total_tokens += 4  # Every message has overhead tokens
    total_tokens += 2  # Conversation overhead
    return total_tokens
```

#### For Image Models
```python
def count_image_tokens(width, height, detail="auto"):
    """Count tokens for image processing"""
    if detail == "low":
        return 85
    elif detail == "high":
        # Calculate based on image tiles
        tiles = math.ceil(width / 512) * math.ceil(height / 512)
        return 170 * tiles + 85
    else:
        # Auto mode
        if width <= 512 and height <= 512:
            return 85
        else:
            return count_image_tokens(width, height, "high")
```

### 4.3 Usage Tracking Implementation

```python
import time
import psutil
import threading

class UsageTracker:
    def __init__(self):
        self.start_time = None
        self.start_memory = 0
        self.peak_memory = 0
        self.gpu_start_time = 0
        self.monitoring = False
        
    def start_tracking(self):
        """Start tracking resource usage"""
        self.start_time = time.time()
        self.start_memory = psutil.virtual_memory().used / 1024 / 1024  # MB
        self.peak_memory = self.start_memory
        self.monitoring = True
        
        # Start background monitoring
        threading.Thread(target=self._monitor_resources, daemon=True).start()
        
    def stop_tracking(self):
        """Stop tracking and return metrics"""
        self.monitoring = False
        end_time = time.time()
        
        duration = end_time - self.start_time
        memory_used = self.peak_memory - self.start_memory
        
        return {
            "duration_seconds": round(duration, 3),
            "memory_mb": max(0, round(memory_used, 1)),
            "peak_memory_mb": round(self.peak_memory, 1),
            "compute_units": round(duration * 1.0, 3)  # Adjust multiplier based on model
        }
    
    def _monitor_resources(self):
        """Monitor resource usage in background"""
        while self.monitoring:
            try:
                current_memory = psutil.virtual_memory().used / 1024 / 1024
                self.peak_memory = max(self.peak_memory, current_memory)
                time.sleep(0.1)
            except:
                break

# Usage in endpoint
@app.route('/v1/chat/completions', methods=['POST'])
def chat_completions():
    tracker = UsageTracker()
    tracker.start_tracking()
    
    try:
        data = request.json
        
        # Count input tokens
        prompt_tokens = count_message_tokens(data['messages'])
        
        # Generate response
        response_content = generate_response(data['messages'])
        
        # Count output tokens
        completion_tokens = count_tokens(response_content)
        total_tokens = prompt_tokens + completion_tokens
        
        # Get resource metrics
        resource_metrics = tracker.stop_tracking()
        
        return jsonify({
            "id": f"chatcmpl-{uuid.uuid4().hex[:8]}",
            "object": "chat.completion",
            "created": int(time.time()),
            "model": data.get('model'),
            "choices": [{
                "index": 0,
                "message": {
                    "role": "assistant",
                    "content": response_content
                },
                "finish_reason": "stop"
            }],
            "usage": {
                "prompt_tokens": prompt_tokens,
                "completion_tokens": completion_tokens,
                "total_tokens": total_tokens,
                # Additional Formation metrics
                "duration_seconds": resource_metrics["duration_seconds"],
                "memory_mb": resource_metrics["memory_mb"],
                "compute_units": resource_metrics["compute_units"]
            }
        })
        
    except Exception as e:
        tracker.stop_tracking()
        return jsonify({"error": {"message": str(e)}}), 500
```

### 4.4 Billing Integration

Formation automatically processes usage metrics from your responses:

```python
# Formation extracts these fields for billing
billing_fields = {
    "prompt_tokens": response["usage"]["prompt_tokens"],
    "completion_tokens": response["usage"]["completion_tokens"], 
    "total_tokens": response["usage"]["total_tokens"],
    "duration_seconds": response["usage"].get("duration_seconds", 0),
    "memory_mb": response["usage"].get("memory_mb", 0),
    "compute_units": response["usage"].get("compute_units", 0)
}
```

---

## 5. Error Handling

### 5.1 HTTP Status Codes

Use appropriate HTTP status codes:

- `200 OK`: Successful completion
- `400 Bad Request`: Invalid request parameters
- `401 Unauthorized`: Authentication failed (should not occur with Formation)
- `403 Forbidden`: Request not allowed
- `404 Not Found`: Model or endpoint not found
- `429 Too Many Requests`: Rate limit exceeded
- `500 Internal Server Error`: Model processing error
- `503 Service Unavailable`: Model temporarily unavailable

### 5.2 Error Response Format

Follow OpenAI's error format:

```json
{
  "error": {
    "message": "The model is currently overloaded. Please try again later.",
    "type": "server_error",
    "param": null,
    "code": "model_overloaded"
  }
}
```

### 5.3 Error Handling Implementation

```python
@app.errorhandler(400)
def bad_request(error):
    return jsonify({
        "error": {
            "message": "Invalid request parameters",
            "type": "invalid_request_error",
            "param": None,
            "code": "invalid_parameters"
        }
    }), 400

@app.errorhandler(500)
def internal_error(error):
    return jsonify({
        "error": {
            "message": "Internal server error occurred",
            "type": "server_error", 
            "param": None,
            "code": "internal_error"
        }
    }), 500

def validate_chat_request(data):
    """Validate chat completion request"""
    if not data:
        raise ValueError("Request body is required")
    
    if "messages" not in data:
        raise ValueError("messages field is required")
    
    if not isinstance(data["messages"], list):
        raise ValueError("messages must be an array")
    
    if len(data["messages"]) == 0:
        raise ValueError("messages array cannot be empty")
    
    for i, message in enumerate(data["messages"]):
        if "role" not in message:
            raise ValueError(f"messages[{i}].role is required")
        
        if "content" not in message:
            raise ValueError(f"messages[{i}].content is required")
        
        if message["role"] not in ["system", "user", "assistant"]:
            raise ValueError(f"messages[{i}].role must be 'system', 'user', or 'assistant'")
```

---

## 6. Performance Requirements

### 6.1 Response Time Targets

- **Health Check**: < 100ms
- **Model List**: < 200ms  
- **Text Generation**: < 30 seconds
- **Image Generation**: < 60 seconds
- **Embeddings**: < 5 seconds

### 6.2 Throughput Requirements

- **Concurrent Requests**: Support at least 10 concurrent requests
- **Memory Efficiency**: Use memory efficiently to maximize throughput
- **GPU Utilization**: Optimize GPU usage if applicable

### 6.3 Resource Optimization

```python
# Example optimization techniques
import torch
import gc

def optimize_model_memory():
    """Optimize model memory usage"""
    if torch.cuda.is_available():
        torch.cuda.empty_cache()
    gc.collect()

def batch_requests(requests, max_batch_size=4):
    """Batch multiple requests for efficiency"""
    batches = []
    for i in range(0, len(requests), max_batch_size):
        batches.append(requests[i:i + max_batch_size])
    return batches

# Use model quantization
model = torch.quantization.quantize_dynamic(
    model, {torch.nn.Linear}, dtype=torch.qint8
)
```

---

## 7. Testing and Validation

### 7.1 API Compatibility Testing

Test your model against OpenAI's API specification:

```python
import requests
import json

def test_chat_completions():
    """Test chat completions endpoint"""
    url = "http://localhost:8080/v1/chat/completions"
    
    payload = {
        "model": "your-model-id",
        "messages": [
            {"role": "user", "content": "Hello, how are you?"}
        ],
        "max_tokens": 100
    }
    
    response = requests.post(url, json=payload)
    
    assert response.status_code == 200
    data = response.json()
    
    # Validate response structure
    assert "choices" in data
    assert "usage" in data
    assert len(data["choices"]) > 0
    assert "message" in data["choices"][0]
    assert "content" in data["choices"][0]["message"]
    
    print("✅ Chat completions test passed")

def test_models_endpoint():
    """Test models list endpoint"""
    url = "http://localhost:8080/v1/models"
    
    response = requests.get(url)
    
    assert response.status_code == 200
    data = response.json()
    
    assert "object" in data
    assert "data" in data
    assert data["object"] == "list"
    assert isinstance(data["data"], list)
    
    print("✅ Models endpoint test passed")

def test_health_endpoint():
    """Test health check endpoint"""
    url = "http://localhost:8080/health"
    
    response = requests.get(url)
    
    assert response.status_code == 200
    data = response.json()
    
    assert "status" in data
    assert data["status"] in ["healthy", "degraded", "unhealthy"]
    
    print("✅ Health endpoint test passed")

if __name__ == "__main__":
    test_health_endpoint()
    test_models_endpoint() 
    test_chat_completions()
    print("🎉 All tests passed!")
```

### 7.2 Load Testing

```bash
# Install Apache Bench for load testing
sudo apt-get install apache2-utils

# Test concurrent requests
ab -n 100 -c 10 -T application/json -p chat_request.json http://localhost:8080/v1/chat/completions

# Test health endpoint
ab -n 1000 -c 50 http://localhost:8080/health
```

### 7.3 Memory and Performance Profiling

```python
import cProfile
import pstats
import tracemalloc

def profile_inference():
    """Profile model inference performance"""
    tracemalloc.start()
    
    # Your inference code here
    result = model.generate("Test prompt")
    
    current, peak = tracemalloc.get_traced_memory()
    tracemalloc.stop()
    
    print(f"Current memory usage: {current / 1024 / 1024:.1f} MB")
    print(f"Peak memory usage: {peak / 1024 / 1024:.1f} MB")
    
    return result

# Profile with cProfile
cProfile.run('profile_inference()', 'profile_stats')
stats = pstats.Stats('profile_stats')
stats.sort_stats('cumulative').print_stats(10)
```

---

## 8. Deployment Checklist

### 8.1 Pre-Deployment Validation

- [ ] **API Compatibility**: All required endpoints implemented
- [ ] **OpenAI Format**: Request/response formats match OpenAI spec
- [ ] **Token Counting**: Accurate token counting implemented
- [ ] **Usage Metrics**: Resource tracking working correctly
- [ ] **Error Handling**: Proper error responses for all failure cases
- [ ] **Health Checks**: Health endpoint returns correct status
- [ ] **Performance**: Meets response time requirements
- [ ] **Memory Usage**: Optimized memory consumption
- [ ] **Containerization**: Builds and runs in Docker container
- [ ] **Port Configuration**: Listens on port 8080

### 8.2 Testing Checklist

- [ ] **Unit Tests**: Core functionality tested
- [ ] **Integration Tests**: API endpoints tested end-to-end
- [ ] **Load Tests**: Performance under concurrent load
- [ ] **Error Tests**: Error handling scenarios covered
- [ ] **Memory Tests**: No memory leaks detected
- [ ] **Compatibility Tests**: Works with OpenAI client libraries

### 8.3 Documentation Checklist

- [ ] **Model Description**: Clear description of capabilities
- [ ] **API Documentation**: Endpoint documentation complete
- [ ] **Usage Examples**: Working code examples provided
- [ ] **Resource Requirements**: CPU, memory, GPU requirements specified
- [ ] **Pricing Information**: Token costs and pricing model defined

---

## 9. Common Implementation Patterns

### 9.1 Framework-Specific Examples

#### Hugging Face Transformers
```python
from transformers import AutoTokenizer, AutoModelForCausalLM
import torch

class HuggingFaceModel:
    def __init__(self, model_name):
        self.tokenizer = AutoTokenizer.from_pretrained(model_name)
        self.model = AutoModelForCausalLM.from_pretrained(
            model_name,
            torch_dtype=torch.float16,
            device_map="auto"
        )
    
    def generate(self, messages, max_tokens=1000, temperature=0.7):
        # Convert messages to prompt
        prompt = self.format_messages(messages)
        
        # Tokenize
        inputs = self.tokenizer(prompt, return_tensors="pt")
        
        # Generate
        with torch.no_grad():
            outputs = self.model.generate(
                inputs.input_ids,
                max_new_tokens=max_tokens,
                temperature=temperature,
                do_sample=True,
                pad_token_id=self.tokenizer.eos_token_id
            )
        
        # Decode response
        response = self.tokenizer.decode(
            outputs[0][inputs.input_ids.shape[1]:], 
            skip_special_tokens=True
        )
        
        return response
```

#### vLLM Integration
```python
from vllm import LLM, SamplingParams

class VLLMModel:
    def __init__(self, model_name):
        self.llm = LLM(model=model_name)
    
    def generate(self, messages, max_tokens=1000, temperature=0.7):
        prompt = self.format_messages(messages)
        
        sampling_params = SamplingParams(
            temperature=temperature,
            max_tokens=max_tokens
        )
        
        outputs = self.llm.generate([prompt], sampling_params)
        return outputs[0].outputs[0].text
```

#### Ollama Integration
```python
import requests

class OllamaModel:
    def __init__(self, model_name, ollama_url="http://localhost:11434"):
        self.model_name = model_name
        self.ollama_url = ollama_url
    
    def generate(self, messages, max_tokens=1000, temperature=0.7):
        prompt = self.format_messages(messages)
        
        response = requests.post(f"{self.ollama_url}/api/generate", json={
            "model": self.model_name,
            "prompt": prompt,
            "stream": False,
            "options": {
                "temperature": temperature,
                "num_predict": max_tokens
            }
        })
        
        return response.json()["response"]
```

### 9.2 Complete Model Server Template

```python
#!/usr/bin/env python3
"""
Formation Model Server Template
Implements OpenAI-compatible API for custom models
"""

import time
import uuid
import logging
from flask import Flask, request, jsonify
from your_model import YourModel  # Replace with your model

# Configure logging
logging.basicConfig(level=logging.INFO)
logger = logging.getLogger(__name__)

app = Flask(__name__)

# Initialize your model
model = YourModel()

@app.route('/v1/chat/completions', methods=['POST'])
def chat_completions():
    try:
        data = request.json
        
        # Validate request
        if not data or 'messages' not in data:
            return jsonify({
                "error": {
                    "message": "messages field is required",
                    "type": "invalid_request_error"
                }
            }), 400
        
        # Extract parameters
        messages = data['messages']
        max_tokens = data.get('max_tokens', 1000)
        temperature = data.get('temperature', 0.7)
        model_id = data.get('model', 'your-model-id')
        
        # Count input tokens
        prompt_tokens = count_message_tokens(messages)
        
        # Generate response
        start_time = time.time()
        response_content = model.generate(messages, max_tokens, temperature)
        duration = time.time() - start_time
        
        # Count output tokens
        completion_tokens = count_tokens(response_content)
        total_tokens = prompt_tokens + completion_tokens
        
        return jsonify({
            "id": f"chatcmpl-{uuid.uuid4().hex[:8]}",
            "object": "chat.completion",
            "created": int(time.time()),
            "model": model_id,
            "choices": [{
                "index": 0,
                "message": {
                    "role": "assistant",
                    "content": response_content
                },
                "finish_reason": "stop"
            }],
            "usage": {
                "prompt_tokens": prompt_tokens,
                "completion_tokens": completion_tokens,
                "total_tokens": total_tokens,
                "duration_seconds": round(duration, 3)
            }
        })
        
    except Exception as e:
        logger.error(f"Error in chat_completions: {e}")
        return jsonify({
            "error": {
                "message": "Internal server error",
                "type": "server_error"
            }
        }), 500

@app.route('/v1/models', methods=['GET'])
def list_models():
    return jsonify({
        "object": "list",
        "data": [{
            "id": "your-model-id",
            "object": "model",
            "created": int(time.time()),
            "owned_by": "formation"
        }]
    })

@app.route('/health', methods=['GET'])
def health():
    return jsonify({
        "status": "healthy",
        "model_loaded": True,
        "version": "1.0.0"
    })

def count_tokens(text):
    """Simple token counting - replace with proper tokenizer"""
    return len(text.split())

def count_message_tokens(messages):
    """Count tokens in messages"""
    total = 0
    for message in messages:
        total += count_tokens(message.get('content', ''))
        total += 4  # Message overhead
    return total + 2  # Conversation overhead

if __name__ == '__main__':
    logger.info("Starting Formation model server...")
    app.run(host='0.0.0.0', port=8080)
```

---

## 10. Next Steps

Once your model meets these requirements:

1. **[Model Deployment Guide](deployment.md)**: Learn how to package and deploy your model
2. **[Model Examples](examples.md)**: See complete working examples
3. **[Monetization Guide](../monetization/)**: Configure pricing and billing
4. **[Testing Guide](../testing/)**: Comprehensive testing strategies

---

## Support and Resources

### Documentation
- **[OpenAI API Reference](https://platform.openai.com/docs/api-reference)**: Official API specification
- **[Formation Deployment Guide](deployment.md)**: Step-by-step deployment process
- **[Troubleshooting Guide](../../operators/troubleshooting.md)**: Common issues and solutions

### Community
- **Formation Discord**: Get help from the community
- **GitHub Issues**: Report bugs and request features
- **Developer Forum**: Share experiences and best practices

### Tools and Libraries
- **tiktoken**: OpenAI's tokenizer for accurate token counting
- **transformers**: Hugging Face model library
- **vLLM**: High-performance inference server
- **Ollama**: Local model deployment platform

---

**Ready to deploy?** Continue to the [Model Deployment Guide](deployment.md) to package and deploy your model to Formation! 🚀 