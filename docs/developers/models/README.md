# Deploying AI Models on Formation

Welcome to Formation's AI Model deployment guide! This section provides everything you need to deploy, host, and monetize custom AI models on the Formation decentralized network.

## What are Formation AI Models?

Formation AI models are containerized AI inference services that:

- **Serve Inference Requests**: Provide AI model inference via OpenAI-compatible APIs
- **Scale Automatically**: Distribute across Formation's network for high availability and performance
- **Earn Revenue**: Generate income through usage-based billing for inference requests
- **Integrate Seamlessly**: Work with existing applications through standard API interfaces

## Model Deployment Overview

Formation supports any AI model that can be containerized and serve HTTP requests:

```
┌─────────────────────────────────────────┐
│            Formation Model              │
├─────────────────────────────────────────┤
│  OpenAI-Compatible API Layer            │
│  ├── POST /v1/chat/completions          │
│  ├── POST /v1/completions               │
│  ├── GET /v1/models                     │
│  └── GET /health                        │
├─────────────────────────────────────────┤
│  Model Inference Layer                  │
│  ├── Model Loading & Initialization     │
│  ├── Request Processing                 │
│  └── Response Generation                │
├─────────────────────────────────────────┤
│  Formation Integration Layer            │
│  ├── Usage Metrics Tracking             │
│  ├── Resource Management                │
│  └── Error Handling & Logging           │
└─────────────────────────────────────────┘
```

## Quick Start

### 1. Choose Your Model Type

Formation supports various model types and frameworks:

- **🤖 Language Models**: GPT-style models, instruction-tuned models, chat models
- **🎨 Image Generation**: Stable Diffusion, DALL-E style models, image-to-image
- **🔊 Audio Models**: Speech-to-text, text-to-speech, audio generation
- **👁️ Vision Models**: Image classification, object detection, OCR
- **🧬 Specialized Models**: Code generation, scientific models, domain-specific AI

### 2. Deployment Path

```mermaid
graph LR
    A[Prepare Model] --> B[Create API Wrapper]
    B --> C[Containerize]
    C --> D[Create Formfile]
    D --> E[Test Locally]
    E --> F[Deploy to Formation]
    F --> G[Monitor & Scale]
```

### 3. Time Investment

- **Simple Model Wrapper**: 1-2 hours
- **Custom Model Integration**: 2-4 hours  
- **Complex Multi-Modal Model**: 4-8 hours

## Documentation Structure

### 📖 [Model Requirements](model-requirements.md)
**Essential Reading** - Technical specifications and API requirements
- OpenAI-compatible API requirements
- Required endpoints (`/v1/chat/completions`, `/v1/models`)
- Authentication handling
- Usage metrics reporting

### 🚀 [Deployment Guide](deployment.md)
**Step-by-Step Process** - From model to production service
- Containerization best practices
- Resource requirements specification
- Registration with `form-state` (`/models/create`)
- Testing inference endpoints

### 💡 [Examples](examples.md)
**Working Code** - Complete, runnable model deployments
- Language model deployment examples
- Image generation model examples
- Custom model integration patterns

## Core Model Concepts

### OpenAI API Compatibility

Formation models must implement OpenAI-compatible endpoints to ensure seamless integration with existing applications:

#### Required Endpoints
```http
POST /v1/chat/completions    # Chat-based completions
POST /v1/completions         # Text completions
GET  /v1/models             # List available models
GET  /health                # Health check
```

#### Optional Endpoints
```http
POST /v1/embeddings         # Text embeddings
POST /v1/images/generations # Image generation
POST /v1/audio/transcriptions # Speech-to-text
```

### Model Lifecycle

#### 1. Development Phase
- Prepare your trained model
- Create OpenAI-compatible API wrapper
- Implement required endpoints
- Add usage tracking and metrics

#### 2. Containerization Phase
- Package model and dependencies
- Optimize container size and startup time
- Configure resource requirements
- Set up health checks

#### 3. Deployment Phase
- Register model with Formation
- Deploy across network nodes
- Configure pricing and billing
- Set up monitoring and alerts

#### 4. Operation Phase
- Handle inference requests
- Monitor performance metrics
- Track usage and revenue
- Scale based on demand

## Popular Model Frameworks

Formation supports models built with any framework. Here are examples of popular frameworks and how to deploy them:

### 1. Hugging Face Transformers
```python
# Hugging Face model with OpenAI-compatible API
from transformers import AutoTokenizer, AutoModelForCausalLM
from flask import Flask, request, jsonify
import torch

app = Flask(__name__)

# Load model and tokenizer
model_name = "microsoft/DialoGPT-medium"
tokenizer = AutoTokenizer.from_pretrained(model_name)
model = AutoModelForCausalLM.from_pretrained(model_name)

@app.route('/v1/chat/completions', methods=['POST'])
def chat_completions():
    data = request.json
    messages = data.get('messages', [])
    
    # Convert messages to prompt
    prompt = "\n".join([f"{msg['role']}: {msg['content']}" for msg in messages])
    
    # Generate response
    inputs = tokenizer.encode(prompt, return_tensors='pt')
    with torch.no_grad():
        outputs = model.generate(inputs, max_length=150, pad_token_id=tokenizer.eos_token_id)
    
    response = tokenizer.decode(outputs[0], skip_special_tokens=True)
    
    return jsonify({
        "choices": [{
            "message": {
                "role": "assistant",
                "content": response
            }
        }],
        "usage": {
            "prompt_tokens": len(inputs[0]),
            "completion_tokens": len(outputs[0]) - len(inputs[0]),
            "total_tokens": len(outputs[0])
        }
    })

@app.route('/v1/models', methods=['GET'])
def list_models():
    return jsonify({
        "data": [{
            "id": model_name,
            "object": "model",
            "owned_by": "formation"
        }]
    })

@app.route('/health', methods=['GET'])
def health():
    return jsonify({"status": "healthy", "framework": "huggingface"})

if __name__ == '__main__':
    app.run(host='0.0.0.0', port=8080)
```

### 2. vLLM (High-Performance Inference)
```python
# vLLM model deployment with OpenAI API
from vllm import LLM, SamplingParams
from vllm.entrypoints.openai.api_server import app
import uvicorn

# Initialize vLLM model
llm = LLM(model="meta-llama/Llama-2-7b-chat-hf")

# vLLM automatically provides OpenAI-compatible endpoints
# Just need to add Formation-specific health endpoint

@app.get("/health")
async def health():
    return {"status": "healthy", "framework": "vllm"}

if __name__ == "__main__":
    uvicorn.run(app, host="0.0.0.0", port=8080)
```

### 3. Ollama Integration
```python
# Ollama model with Formation integration
import requests
from flask import Flask, request, jsonify

app = Flask(__name__)

OLLAMA_BASE_URL = "http://localhost:11434"

@app.route('/v1/chat/completions', methods=['POST'])
def chat_completions():
    data = request.json
    messages = data.get('messages', [])
    
    # Convert to Ollama format
    prompt = messages[-1]['content'] if messages else ""
    
    # Call Ollama API
    response = requests.post(f"{OLLAMA_BASE_URL}/api/generate", json={
        "model": "llama2",
        "prompt": prompt,
        "stream": False
    })
    
    result = response.json()
    
    return jsonify({
        "choices": [{
            "message": {
                "role": "assistant",
                "content": result.get("response", "")
            }
        }],
        "usage": {
            "total_tokens": len(result.get("response", "").split())
        }
    })

@app.route('/v1/models', methods=['GET'])
def list_models():
    # Get available Ollama models
    response = requests.get(f"{OLLAMA_BASE_URL}/api/tags")
    models = response.json().get("models", [])
    
    return jsonify({
        "data": [{"id": model["name"], "object": "model"} for model in models]
    })

@app.route('/health', methods=['GET'])
def health():
    return jsonify({"status": "healthy", "framework": "ollama"})

if __name__ == '__main__':
    app.run(host='0.0.0.0', port=8080)
```

### 4. Custom PyTorch/TensorFlow Models
```python
# Custom model with OpenAI-compatible wrapper
import torch
import torch.nn as nn
from flask import Flask, request, jsonify

app = Flask(__name__)

class CustomModel(nn.Module):
    def __init__(self):
        super().__init__()
        # Your custom model architecture
        self.layers = nn.Sequential(
            nn.Linear(768, 512),
            nn.ReLU(),
            nn.Linear(512, 256),
            nn.ReLU(),
            nn.Linear(256, 50257)  # Vocab size
        )
    
    def forward(self, x):
        return self.layers(x)

# Load your trained model
model = CustomModel()
model.load_state_dict(torch.load('model.pth'))
model.eval()

@app.route('/v1/completions', methods=['POST'])
def completions():
    data = request.json
    prompt = data.get('prompt', '')
    
    # Your custom inference logic
    with torch.no_grad():
        # Process prompt and generate response
        response = f"Generated response for: {prompt}"
    
    return jsonify({
        "choices": [{
            "text": response
        }],
        "usage": {
            "prompt_tokens": len(prompt.split()),
            "completion_tokens": len(response.split()),
            "total_tokens": len(prompt.split()) + len(response.split())
        }
    })

@app.route('/health', methods=['GET'])
def health():
    return jsonify({"status": "healthy", "framework": "pytorch"})

if __name__ == '__main__':
    app.run(host='0.0.0.0', port=8080)
```

## Development Best Practices

### Performance Optimization
- **Model Quantization**: Use INT8/FP16 to reduce memory usage
- **Batch Processing**: Handle multiple requests efficiently
- **Caching**: Cache frequently used model outputs
- **GPU Utilization**: Optimize GPU memory and compute usage

### Resource Management
- **Memory Efficiency**: Monitor and optimize memory usage
- **Startup Time**: Minimize container startup time
- **Health Checks**: Implement robust health monitoring
- **Graceful Shutdown**: Handle shutdown signals properly

### API Compatibility
- **OpenAI Standards**: Follow OpenAI API specifications exactly
- **Error Handling**: Return proper HTTP status codes and error messages
- **Rate Limiting**: Implement request rate limiting
- **Input Validation**: Validate all incoming requests

## Monetization Strategies

### Pricing Models
- **Per-Token**: Charge based on input/output tokens
- **Per-Request**: Fixed price per inference request
- **Per-Minute**: Time-based pricing for long-running inference
- **Tiered Pricing**: Different rates for different model sizes/capabilities

### Revenue Optimization
- **Efficient Inference**: Faster inference = more requests = more revenue
- **Model Specialization**: Specialized models command premium pricing
- **Quality Service**: High-quality outputs improve customer retention
- **Resource Optimization**: Lower hosting costs increase profit margins

## Getting Started Checklist

Ready to deploy your first Formation model? Follow this checklist:

- [ ] **Read [Model Requirements](model-requirements.md)** - Understand API specifications
- [ ] **Prepare Your Model** - Ensure model is trained and ready
- [ ] **Create API Wrapper** - Implement OpenAI-compatible endpoints
- [ ] **Add Usage Tracking** - Implement metrics for billing
- [ ] **Containerize Model** - Create Docker container
- [ ] **Test Locally** - Verify all endpoints work correctly
- [ ] **Follow [Deployment Guide](deployment.md)** - Deploy to Formation network
- [ ] **Monitor Performance** - Track metrics and optimize
- [ ] **Scale and Iterate** - Improve based on usage patterns

## Support and Resources

### Documentation
- **[Model Requirements](model-requirements.md)**: Technical specifications and API requirements
- **[Deployment Guide](deployment.md)**: Step-by-step deployment process
- **[Examples](examples.md)**: Working model deployment examples

### Tools and APIs
- **form-pack**: Container building and packaging
- **form-state API**: Model registration and management
- **form-vmm API**: Instance deployment and monitoring

### Community
- **Model Marketplace**: Discover and share model implementations
- **Performance Benchmarks**: Compare model performance and efficiency
- **Best Practices**: Learn from experienced model deployers

## Next Steps

Choose your next action based on your experience level:

### 🆕 **New to Model Deployment**
Start with **[Model Requirements](model-requirements.md)** to understand the fundamentals

### 🔧 **Ready to Deploy**
Jump to **[Examples](examples.md)** for working deployment templates

### 🚀 **Ready for Production**
Follow the **[Deployment Guide](deployment.md)** for step-by-step instructions

### 💰 **Focus on Revenue**
Check out **[Monetization Strategies](../monetization/)** for pricing optimization

---

**Ready to make your AI models globally accessible?** Let's deploy your first Formation model! 