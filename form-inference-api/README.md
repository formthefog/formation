# Form Inference API

A REST API service that provides inference endpoints for AI models, integrated with the Formation ecosystem.

## Overview

The Form Inference API serves as a gateway for model inference requests, integrating with `form-state` to retrieve model information and manage model deployments. It provides a standardized interface for clients to discover and interact with AI models in the Formation network.

## Features

- **Model Discovery**: List all available models from the Formation network
- **Model Details**: Retrieve detailed information about specific models
- **Chat Completions**: Generate text completions using models via chat interface
- **Form-State Integration**: Seamlessly integrates with `form-state` for model metadata and node discovery
- **Model Inference**: Makes direct inference requests to model services
- **Error Handling**: Comprehensive error handling for network and service failures
- **RESTful API**: Standard REST endpoints following OpenAPI specifications

## API Endpoints

### Models

#### GET /models
Retrieves a list of all available models from the Formation network.

**Response:**
```json
{
  "success": true,
  "models": [
    {
      "model_id": "string",
      "name": "string",
      "owner_id": "string",
      "version": "string",
      "description": "string",
      "model_type": "string",
      "framework": "string",
      "io_modes": ["string"],
      "parameters": 0,
      "resource_requirements": {
        "min_vcpus": 0,
        "recommended_vcpus": 0,
        "min_memory_mb": 0,
        "recommended_memory_mb": 0,
        "requires_gpu": false
      },
      "capabilities": ["string"],
      "is_featured": false,
      "is_private": false
    }
  ],
  "total": 0
}
```

#### GET /models/{model_id}
Retrieves detailed information about a specific model.

**Response:**
```json
{
  "model_id": "string",
  "name": "string",
  "owner_id": "string",
  "version": "string",
  "description": "string",
  "documentation": "string",
  "license": "string",
  "model_type": "string",
  "framework": "string",
  "io_modes": ["string"],
  "parameters": 0,
  "quantization": "string",
  "context_length": 0,
  "input_tokens_per_second": 0,
  "output_tokens_per_second": 0,
  "tags": ["string"],
  "created_at": 0,
  "updated_at": 0,
  "formfile_template": "string",
  "weights_url": "string",
  "weights_checksum": "string",
  "weights_size_bytes": 0,
  "resource_requirements": {
    "min_vcpus": 0,
    "recommended_vcpus": 0,
    "min_memory_mb": 0,
    "recommended_memory_mb": 0,
    "min_disk_gb": 0,
    "recommended_disk_gb": 0,
    "requires_gpu": false,
    "min_vram_gb": 0,
    "recommended_vram_gb": 0,
    "cuda_cores": 0,
    "tensor_cores": 0,
    "required_cpu_extensions": ["string"],
    "required_cuda_version": "string"
  },
  "capabilities": ["string"],
  "average_rating": 0,
  "deployment_count": 0,
  "usage_count": 0,
  "is_featured": false,
  "is_private": false,
  "metadata": {},
  "repository_url": "string",
  "demo_url": "string",
  "paper_url": "string",
  "price_per_1m_tokens": 0,
  "usage_tracking": {
    "track_tokens": false,
    "track_requests": false,
    "enable_royalties": false,
    "royalty_percentage": 0,
    "custom_metrics": ["string"]
  },
  "nodes": []
}
```

### Chat Completions

#### POST /chat/completions
Generates text completions using a specified model.

**Request:**
```json
{
  "model": "string",
  "messages": [
    {
      "role": "user",
      "content": "Write me a 10 word story"
    }
  ],
  "max_tokens": 50,
  "temperature": 0.7,
  "stream": false
}
```

**Response:**
```json
{
  "id": "chatcmpl_uuid",
  "model": "string",
  "choices": [
    {
      "index": 0,
      "message": {
        "role": "assistant",
        "content": "Generated text response from the model"
      }
    }
  ]
}
```

**Error Response:**
```json
{
  "success": false,
  "error": "Error description"
}
```

## Dependencies

### Required Services

- **form-state**: Must be running on `localhost:3004` (configurable)
  - Provides model metadata and registration
  - Handles model discovery and filtering
  - Manages model access permissions
  - Provides node discovery for model inference

- **Model Services**: Must be running on localhost with `/generate` endpoint
  - Accepts inference requests in the specified format
  - Returns generated text responses
  - Should be accessible on the port specified by form-state

### External Dependencies

- **Rust 1.70+**
- **Cargo**
- **reqwest**: HTTP client for form-state and model service communication
- **axum**: Web framework
- **serde**: Serialization/deserialization
- **tokio**: Async runtime
- **uuid**: UUID generation for request IDs

## Configuration

### Environment Variables

- `FORM_STATE_URL`: Base URL for form-state service (default: `http://localhost:3004`)

### Service Ports

- **form-inference-api**: `3000` (default)
- **form-state**: `3004` (default)
- **Model Services**: Dynamic ports (retrieved from form-state)

## Installation

### Local Development

1. **Clone the repository:**
   ```bash
   git clone <repository-url>
   cd form-inference-api
   ```

2. **Build the service:**
   ```bash
   cargo build
   ```

3. **Run the service:**
   ```bash
   cargo run -- --config /path/to/.operator-config.json
   ```

### Docker Deployment

1. **Build the Docker image:**
   ```bash
   ./build-docker.sh
   ```

2. **Run with Docker:**
   ```bash
   docker run -p 3006:3006 \
     -v /path/to/.operator-config.json:/etc/formation/config/.operator-config.json:ro \
     form-inference-api:latest
   ```

3. **Or use docker-compose:**
   ```bash
   docker-compose up
   ```

## Development

### Prerequisites

1. **Start form-state:**
   ```bash
   cd ../form-state
   cargo run
   ```

2. **Start form-inference-api:**
   ```bash
   cd ../form-inference-api
   cargo run
   ```

3. **Start a model service** (example):
   ```bash
   # Your model service should be running on localhost with a /generate endpoint
   # The service should accept requests in this format:
   curl --request POST \
       --url http://127.0.0.1:30000/generate \
       --header 'Content-Type: application/json' \
       --data '{
       "text": "Write me a 10 word story",
       "sampling_params": {
       "max_new_tokens": 10,
       "temperature": 0.7
       }
       }'
   ```

### Testing

Run the integration test scripts:

```bash
# Test models endpoints
./test_integration.sh

# Test chat completions endpoint
./test_chat_integration.sh
```

These scripts will:
- Verify both services are running
- Test the `/models` endpoint
- Test the `/chat/completions` endpoint
- Test individual model retrieval
- Test error handling

### Manual Testing

1. **Test model listing:**
   ```bash
   curl http://localhost:3000/models
   ```

2. **Test specific model retrieval:**
   ```bash
   curl http://localhost:3000/models/{model_id}
   ```

3. **Test chat completions:**
   ```bash
   curl -X POST http://localhost:3000/chat/completions \
     -H "Content-Type: application/json" \
     -d '{
       "model": "your-model-id",
       "messages": [
         {
           "role": "user",
           "content": "Write me a 10 word story"
         }
       ],
       "max_tokens": 50,
       "temperature": 0.7
     }'
   ```

4. **Test error handling:**
   ```bash
   curl http://localhost:3000/models/non-existent-model
   ```

## Architecture

### Integration with Form-State

The Form Inference API integrates with `form-state` through HTTP requests:

1. **Model Discovery**: Queries `form-state`'s `/v1/models` endpoint
2. **Model Details**: Queries `form-state`'s `/v1/models/{model_id}` endpoint
3. **Node Discovery**: Queries `form-state`'s `/v1/models/{model_id}/get_node` endpoint
4. **Error Handling**: Gracefully handles form-state unavailability
5. **Data Mapping**: Converts form-state's AIModel format to API response format

### Chat Completions Flow

```
Client Request → form-inference-api → form-state → Model Service → Response
```

1. Client sends chat completion request to form-inference-api
2. form-inference-api queries form-state for model node information
3. form-state returns node address and port
4. form-inference-api determines if node is local or remote:
   - **Local nodes**: Uses the port from form-state response (e.g., `http://127.0.0.1:30000/generate`)
   - **Remote nodes**: Forwards to port 3006 on the remote IP (e.g., `http://67.77.107.55:3006/generate`)
5. form-inference-api makes inference request to model service
6. Model service returns generated text
7. form-inference-api formats and returns chat completion response

### Node Address Support

The form-inference-api supports both local and remote node addresses:

- **Local nodes**: `"localhost"` - Uses the port from form-state response
- **Remote nodes**: `{"Ipv4": "67.77.107.55"}` - Forwards to port 3006 on the remote IP

### Model Inference Integration

The API integrates with model services using a standardized format:

**Request Format:**
```json
{
  "text": "User input text",
  "sampling_params": {
    "max_new_tokens": 100,
    "temperature": 0.7
  }
}
```

**Response Format:**
```json
{
  "text": "Generated text response"
}
```

## Error Handling

The API provides comprehensive error handling:

- **Network Errors**: When form-state or model services are unreachable
- **HTTP Errors**: When services return non-200 status codes
- **Parsing Errors**: When services return invalid JSON
- **Not Found**: When requested model doesn't exist
- **Service Unavailable**: When no nodes are available for a model
- **Model Errors**: When model inference fails

All errors return appropriate HTTP status codes and descriptive error messages.

## Contributing

1. Follow the existing code style and patterns
2. Add tests for new functionality
3. Update documentation for API changes
4. Ensure integration tests pass

## License

[License information]