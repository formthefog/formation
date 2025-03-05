# Formation Inference Engine

> **Note**: The Inference Engine is currently under development and not yet fully integrated into the Formation network. This documentation outlines the planned functionality and will be updated as features become available.

Formation's Inference Engine is a distributed AI inference platform built on the Formation network. It provides a scalable, efficient way to run AI models with the privacy and security guarantees inherent to Formation's architecture.

## Overview

The Inference Engine enables:

- Running large AI models across distributed nodes
- API-compatible interfaces for popular AI services
- Confidential inference with data privacy
- Dynamic resource allocation based on workload

## Key Features

### 1. Distributed Model Serving

Unlike traditional centralized inference systems, Formation's Inference Engine distributes model weights and computation across multiple nodes, enabling:

- Serving models larger than any single node's memory capacity
- Parallel processing for improved inference speed
- Fault tolerance through redundancy
- Geographic distribution for reduced latency

### 2. Standard API Compatibility

The Inference Engine implements standard API formats used by leading AI providers:

- **OpenAI-compatible API**: Drop-in replacement for OpenAI's chat completions and embeddings APIs
- **Anthropic-compatible API**: Support for Claude models API format
- **HuggingFace-compatible endpoints**: For a wide range of models
- **Custom Formation extensions**: Enhanced capabilities unique to Formation

Example OpenAI-compatible request:

```json
POST /v1/chat/completions
{
  "model": "formation-llm",
  "messages": [
    {"role": "system", "content": "You are a helpful assistant."},
    {"role": "user", "content": "Tell me about Formation's Inference Engine."}
  ],
  "temperature": 0.7
}
```

### 3. Model Formats and Support

The Inference Engine supports multiple types of models:

- **Language Models**: GPT-compatible, Claude-compatible, and other LLMs
- **Embeddings Models**: For vector representations of text
- **Vision Models**: For image understanding
- **Multimodal Models**: Combining text, image, and other data types

### 4. Privacy and Security

Formation's focus on confidentiality extends to the Inference Engine:

- **Confidential Inference**: Data never leaves the secure environment
- **End-to-End Encryption**: All communication is encrypted
- **Isolated Execution**: Models run in isolated environments
- **Access Controls**: Granular permissions for model access

## Pricing and Resource Usage

The Inference Engine uses a token-based pricing model based on model size:

| Model Size | Price (per 1 Million tokens) |
|------------|------------------------------|
| < 32B Parameters | $0.1 |
| 32B - 110B Parameters | $0.2 |
| 110B - 500B Parameters | $0.3 |
| > 500B Parameters | $0.4 |

> **Note**: Pricing is subject to change as the Inference Engine moves from development to production.

## Hardware Requirements

Formation nodes participating in the Inference Engine have additional hardware requirements:

- **GPU**: NVIDIA GPUs with CUDA support (RTX 3090 or better recommended)
- **Memory**: Minimum 64GB RAM, 128GB+ recommended
- **Storage**: High-speed NVMe SSD for model weights
- **Network**: High-bandwidth, low-latency connection

## Developer Usage

### Authentication

Authenticate to the Inference Engine using your Formation wallet or API keys:

```bash
# Using Formation CLI
form inference auth --key <your-api-key>

# Using curl
curl -X POST https://inference.formation.cloud/v1/chat/completions \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer <your-api-key>" \
  -d '{
    "model": "formation-llm",
    "messages": [
      {"role": "system", "content": "You are a helpful assistant."},
      {"role": "user", "content": "Hello, world!"}
    ]
  }'
```

### API Endpoints

The Inference Engine provides multiple endpoints:

- `/v1/chat/completions`: For chat-based interactions
- `/v1/completions`: For standard completions
- `/v1/embeddings`: For generating embeddings
- `/v1/images/generate`: For image generation
- `/v1/models`: List available models

### Streaming Responses

For real-time responses, use streaming mode:

```javascript
const response = await fetch('https://inference.formation.cloud/v1/chat/completions', {
  method: 'POST',
  headers: {
    'Content-Type': 'application/json',
    'Authorization': `Bearer ${apiKey}`
  },
  body: JSON.stringify({
    model: 'formation-llm',
    messages: [
      { role: 'user', content: 'Write a poem about formation.' }
    ],
    stream: true
  })
});

// Process the stream
const reader = response.body.getReader();
const decoder = new TextDecoder();
while (true) {
  const { done, value } = await reader.read();
  if (done) break;
  const chunk = decoder.decode(value);
  // Parse and handle the chunk
  console.log(chunk);
}
```

## Available Models

The initial Inference Engine release will include:

| Model ID | Description | Context Size | Capabilities |
|----------|-------------|--------------|--------------|
| formation-llm | General purpose language model | 8K tokens | Text generation, reasoning |
| formation-llm-32k | Extended context language model | 32K tokens | Long-form content, analysis |
| formation-vision | Vision-capable model | 8K tokens | Image analysis, text generation |
| formation-embeddings | Text embeddings model | 512 tokens | Vector embeddings, semantic search |

## Deployment Options

The Inference Engine can be deployed in several configurations:

- **Public Network**: Access the Formation public inference network
- **Private Network**: Deploy on a private Formation cloud
- **Hybrid**: Combination of public and private resources

## Future Roadmap

The Inference Engine roadmap includes:

- **Model Fine-tuning**: Customize models for specific tasks
- **Custom Model Deployment**: Deploy proprietary models
- **Enhanced Multimodal Support**: Audio, video, and more
- **Model Marketplace**: Discover and share models
- **Advanced Routing**: Automatic model selection based on query

## Limitations and Considerations

Current limitations of the Inference Engine:

- **Beta Status**: The engine is in active development
- **Model Availability**: Limited selection during initial release
- **GPU Requirements**: Specific hardware required for participation
- **Performance Variability**: Latency may vary based on network conditions

## Next Steps

- [Getting Started with the Inference Engine](./getting-started.md)
- [API Reference](./api-reference.md)
- [Model Details](./model-details.md)
- [Security and Privacy](./security.md)
- [Example Applications](./examples.md) 