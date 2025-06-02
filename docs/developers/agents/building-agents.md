# Building AI Agents on Formation

This guide provides the technical requirements and specifications for building AI agents that integrate with Formation's decentralized network. Follow these specifications to ensure your agent works seamlessly with Formation's task dispatch and billing systems.

## Formation Architecture Overview

Formation operates as a decentralized network where:

- **Agents**: Run in Linux VMs as containerized HTTP services
- **Task Assignment**: Formation sends HTTP requests directly to agent endpoints
- **Billing**: Formation handles all billing and cost calculation centrally
- **Authentication**: Formation manages authentication for all network requests
- **PoC System**: Proof of Claim affects node-level responsibilities, not individual agents
- **Model Access**: ALL model requests must go through Formation's infrastructure

### Critical Architecture Rule

**🚨 IMPORTANT**: Agents cannot call external APIs (OpenAI, Anthropic, etc.) directly. ALL requests to external models must be routed through Formation's model infrastructure. Formation handles:

- Authentication with external providers
- Rate limiting and quota management
- Billing and usage tracking
- Load balancing and failover
- Security and compliance

## Agent Architecture Requirements

### Core Architecture Principles

Formation agents must follow these architectural principles:

1. **Stateless Design**: Agents should not rely on persistent local state between requests
2. **HTTP API Interface**: All communication happens via HTTP endpoints
3. **Containerized Deployment**: Agents run in Docker containers within Linux VMs
4. **Resource Efficiency**: Optimize for CPU, memory, and network usage
5. **Error Resilience**: Handle failures gracefully and provide meaningful error messages

### Required System Architecture

```
┌─────────────────────────────────────────┐
│              Formation Agent            │
├─────────────────────────────────────────┤
│  HTTP Server Layer                      │
│  ├── POST /run_task (Required)          │
│  ├── GET /health (Required)             │
│  └── GET /capabilities (Optional)       │
├─────────────────────────────────────────┤
│  Business Logic Layer                   │
│  ├── Task Processing Engine             │
│  ├── Input Validation & Sanitization    │
│  ├── Output Formatting                  │
│  └── Error Handling                     │
├─────────────────────────────────────────┤
│  Integration Layer                      │
│  ├── Usage Metrics Collection           │
│  ├── Resource Monitoring                │
│  ├── Logging & Debugging                │
│  └── Configuration Management           │
├─────────────────────────────────────────┤
│  External Dependencies (Optional)       │
│  ├── Formation Model APIs               │
│  ├── Database Connections               │
│  ├── Third-party Services               │
│  └── File System Access                 │
└─────────────────────────────────────────┘
```

## Required API Endpoints

### 1. POST /run_task (Required)

This is the primary endpoint where Formation sends tasks for execution.

#### Request Format

```json
{
  "task_id": "task_12345",
  "task_type": "text_generation",
  "parameters": {
    "prompt": "Write a summary of the following text...",
    "max_tokens": 1000,
    "temperature": 0.7,
    "custom_param": "value"
  },
  "requester_address": "0x1234567890abcdef...",
  "timestamp": 1640995200,
  "timeout_seconds": 300,
  "priority": "normal"
}
```

**Field Descriptions:**
- `task_id`: Unique identifier for the task (string)
- `task_type`: Type of task being requested (string)
- `parameters`: Task-specific parameters (object)
- `requester_address`: ECDSA address of the requester (string)
- `timestamp`: Unix timestamp of the request (integer)
- `timeout_seconds`: Maximum execution time allowed (integer)
- `priority`: Task priority level: "low", "normal", "high" (string)

#### Response Format

**Success Response (200 OK):**
```json
{
  "task_id": "task_12345",
  "status": "completed",
  "result": {
    "output": "Generated text response...",
    "metadata": {
      "model_used": "gpt-4",
      "processing_steps": ["tokenization", "generation", "post_processing"]
    }
  },
  "usage_metrics": {
    "compute_units": 1.5,
    "memory_mb": 512,
    "duration_seconds": 2.5,
    "tokens_processed": 1200,
    "api_calls": 1
  }
}
```

**Error Response (400/500):**
```json
{
  "task_id": "task_12345",
  "status": "failed",
  "error": {
    "code": "INVALID_INPUT",
    "message": "The prompt parameter is required",
    "details": {
      "field": "parameters.prompt",
      "expected": "string",
      "received": "null"
    }
  },
  "partial_usage": {
    "compute_units": 0.1,
    "duration_seconds": 0.5
  }
}
```

**Status Values:**
- `completed`: Task finished successfully
- `failed`: Task failed with error
- `timeout`: Task exceeded timeout limit
- `cancelled`: Task was cancelled

### 2. GET /health (Required)

Health check endpoint for monitoring and load balancing.

#### Response Format

```json
{
  "status": "healthy",
  "version": "1.0.0",
  "uptime_seconds": 3600,
  "capabilities": ["text_generation", "summarization"],
  "resource_usage": {
    "cpu_percent": 25.5,
    "memory_mb": 1024,
    "disk_mb": 2048
  },
  "dependencies": {
    "openai_api": "connected",
    "database": "connected",
    "external_service": "degraded"
  }
}
```

**Status Values:**
- `healthy`: Agent is fully operational
- `degraded`: Agent is operational but with reduced performance
- `unhealthy`: Agent is not operational

### 3. GET /capabilities (Optional)

Describes the agent's capabilities and supported task types.

#### Response Format

```json
{
  "agent_id": "text-processor-v1",
  "name": "Advanced Text Processing Agent",
  "description": "Handles text generation, summarization, and translation tasks",
  "version": "1.2.0",
  "supported_task_types": [
    {
      "type": "text_generation",
      "description": "Generate text based on prompts",
      "parameters": {
        "prompt": {"type": "string", "required": true},
        "max_tokens": {"type": "integer", "default": 1000},
        "temperature": {"type": "number", "default": 0.7}
      }
    },
    {
      "type": "summarization",
      "description": "Summarize long text content",
      "parameters": {
        "text": {"type": "string", "required": true},
        "max_length": {"type": "integer", "default": 500}
      }
    }
  ],
  "resource_requirements": {
    "min_memory_mb": 512,
    "min_cpu_cores": 1,
    "gpu_required": false
  },
  "pricing": {
    "model": "per_token",
    "input_rate": 0.001,
    "output_rate": 0.002,
    "currency": "USD"
  }
}
```

## Request/Response Format Specifications

### Content Type Requirements

- **Request Content-Type**: `application/json`
- **Response Content-Type**: `application/json`
- **Character Encoding**: UTF-8

### HTTP Status Codes

Your agent must return appropriate HTTP status codes:

- `200 OK`: Successful task completion
- `400 Bad Request`: Invalid request format or parameters
- `408 Request Timeout`: Task exceeded timeout limit
- `429 Too Many Requests`: Rate limiting applied
- `500 Internal Server Error`: Agent internal error
- `503 Service Unavailable`: Agent temporarily unavailable

### Error Handling Standards

#### Error Response Structure

```json
{
  "task_id": "task_12345",
  "status": "failed",
  "error": {
    "code": "ERROR_CODE",
    "message": "Human-readable error message",
    "details": {
      "additional": "context",
      "field": "specific_field_with_error"
    }
  },
  "partial_usage": {
    "compute_units": 0.1,
    "duration_seconds": 1.0
  }
}
```

#### Standard Error Codes

- `INVALID_INPUT`: Request parameters are invalid
- `MISSING_PARAMETER`: Required parameter is missing
- `TIMEOUT`: Task execution exceeded time limit
- `RESOURCE_EXHAUSTED`: Insufficient resources to complete task
- `EXTERNAL_SERVICE_ERROR`: External dependency failed
- `RATE_LIMITED`: Too many requests from requester
- `UNSUPPORTED_TASK_TYPE`: Task type not supported by agent

### Input Validation Requirements

Your agent must validate all inputs:

```python
def validate_task_request(request_data):
    """Validate incoming task request"""
    required_fields = ['task_id', 'task_type', 'parameters']
    
    for field in required_fields:
        if field not in request_data:
            raise ValidationError(f"Missing required field: {field}")
    
    # Validate task_id format
    if not isinstance(request_data['task_id'], str) or len(request_data['task_id']) == 0:
        raise ValidationError("task_id must be a non-empty string")
    
    # Validate parameters based on task type
    task_type = request_data['task_type']
    parameters = request_data['parameters']
    
    if task_type == 'text_generation':
        if 'prompt' not in parameters:
            raise ValidationError("prompt parameter required for text_generation")
        if not isinstance(parameters['prompt'], str):
            raise ValidationError("prompt must be a string")
    
    return True
```

## Usage Reporting Requirements

### Metrics Collection

Your agent must collect and report accurate usage metrics for billing:

#### Required Metrics

```python
class UsageMetrics:
    def __init__(self):
        self.compute_units = 0.0      # CPU time * memory usage
        self.memory_mb = 0            # Peak memory usage in MB
        self.duration_seconds = 0.0   # Total execution time
        self.tokens_processed = 0     # Input + output tokens
        self.api_calls = 0           # External API calls made
        
    def calculate_compute_units(self, cpu_time_seconds, memory_mb):
        """Calculate compute units based on CPU time and memory"""
        return cpu_time_seconds * (memory_mb / 1024.0)  # GB-seconds
    
    def to_dict(self):
        return {
            "compute_units": self.compute_units,
            "memory_mb": self.memory_mb,
            "duration_seconds": self.duration_seconds,
            "tokens_processed": self.tokens_processed,
            "api_calls": self.api_calls
        }
```

#### Usage Tracking Implementation

```python
import time
import psutil
import threading

class UsageTracker:
    def __init__(self):
        self.start_time = None
        self.peak_memory = 0
        self.cpu_start = None
        self.monitoring = False
        
    def start_tracking(self):
        """Start tracking resource usage"""
        self.start_time = time.time()
        self.cpu_start = psutil.cpu_times()
        self.peak_memory = 0
        self.monitoring = True
        
        # Start memory monitoring thread
        threading.Thread(target=self._monitor_memory, daemon=True).start()
        
    def stop_tracking(self):
        """Stop tracking and return metrics"""
        self.monitoring = False
        end_time = time.time()
        cpu_end = psutil.cpu_times()
        
        duration = end_time - self.start_time
        cpu_time = (cpu_end.user - self.cpu_start.user) + (cpu_end.system - self.cpu_start.system)
        
        return UsageMetrics(
            duration_seconds=duration,
            memory_mb=self.peak_memory,
            compute_units=cpu_time * (self.peak_memory / 1024.0)
        )
    
    def _monitor_memory(self):
        """Monitor memory usage in background"""
        process = psutil.Process()
        while self.monitoring:
            memory_mb = process.memory_info().rss / 1024 / 1024
            self.peak_memory = max(self.peak_memory, memory_mb)
            time.sleep(0.1)
```

### Task Assignment & Billing

#### How Formation Assigns Tasks

Formation uses a centralized HTTP-based task assignment system:

1. **Direct HTTP Requests**: Formation sends HTTP requests directly to agent `/run_task` endpoints
2. **Task Execution**: Agents process tasks and return responses via HTTP
3. **Response Collection**: Formation collects and processes agent responses
4. **Model Integration**: Agents can use Formation's model endpoints to route requests to hosted or external models

#### Model Access Through Formation

**IMPORTANT**: All model requests must go through Formation's infrastructure. Agents cannot call external APIs (OpenAI, Anthropic, etc.) directly.

Formation routes model requests to the appropriate providers:

```python
import requests

class FormationModelClient:
    def __init__(self, formation_base_url="http://formation-api"):
        self.base_url = formation_base_url
    
    def call_model(self, model_id, messages, **kwargs):
        """Call a model through Formation's infrastructure"""
        # Formation handles routing to OpenAI, Anthropic, or hosted models
        response = requests.post(
            f"{self.base_url}/v1/models/{model_id}/inference",
            json={
                "messages": messages,
                "max_tokens": kwargs.get("max_tokens", 1000),
                "temperature": kwargs.get("temperature", 0.7)
            }
        )
        return response.json()

# Usage in agent
def _generate_text(self, parameters):
    formation_client = FormationModelClient()
    result = formation_client.call_model(
        model_id="gpt-4",
        messages=[{"role": "user", "content": parameters["prompt"]}],
        max_tokens=parameters.get("max_tokens", 1000)
    )
    return result["choices"][0]["message"]["content"]
```

**Supported Models**: Formation routes to various providers:
- **OpenAI**: gpt-4, gpt-3.5-turbo, etc.
- **Anthropic**: claude-3-opus, claude-3-sonnet, etc.
- **Hosted Models**: Custom models deployed on Formation
- **External Models**: Other providers integrated with Formation

#### Authentication & Security

- **Formation-Managed Authentication**: Formation handles all authentication for requests coming through the Formation network
- **Direct Access Protection**: Agents cannot handle authentication for requests sent directly (bypassing Formation)
- **Secure Communication**: All communication between Formation and agents uses secure HTTP protocols

#### Billing Integration

Formation handles all billing centrally - agents don't need to implement billing logic:

- **Automatic Tracking**: Formation automatically tracks usage, costs, and billing
- **No Agent Billing Code**: Agents don't need to calculate costs or handle billing
- **Transparent Pricing**: Formation manages pricing models and revenue sharing
- **Usage Reporting**: Formation automatically reports usage based on API calls and responses

**Important**: While agents can include usage metrics in their responses for monitoring purposes, Formation handles all actual billing calculations and transactions.
```

## Implementation Examples

### Basic Agent Structure

```python
from flask import Flask, request, jsonify
import time
import logging
import requests

app = Flask(__name__)
logging.basicConfig(level=logging.INFO)

# Formation model client (required for model access)
class FormationModelClient:
    def __init__(self, formation_base_url="http://formation-api"):
        self.base_url = formation_base_url
    
    def call_model(self, model_id, messages, **kwargs):
        """Call a model through Formation's infrastructure"""
        response = requests.post(
            f"{self.base_url}/v1/models/{model_id}/inference",
            json={
                "messages": messages,
                "max_tokens": kwargs.get("max_tokens", 1000),
                "temperature": kwargs.get("temperature", 0.7)
            }
        )
        return response.json()["choices"][0]["message"]["content"]

class FormationAgent:
    def __init__(self):
        self.capabilities = ["text_generation", "summarization"]
        self.version = "1.0.0"
        
    def process_task(self, task_data):
        """Main task processing logic"""
        task_type = task_data["task_type"]
        parameters = task_data["parameters"]
        
        if task_type == "text_generation":
            return self._generate_text(parameters)
        elif task_type == "summarization":
            return self._summarize_text(parameters)
        else:
            raise ValueError(f"Unsupported task type: {task_type}")
    
    def _generate_text(self, parameters):
        """Generate text using Formation's model infrastructure"""
        prompt = parameters["prompt"]
        max_tokens = parameters.get("max_tokens", 1000)
        
        # Call model through Formation (required)
        formation_client = FormationModelClient()
        result = formation_client.call_model(
            model_id="gpt-4",
            messages=[{"role": "user", "content": prompt}],
            max_tokens=max_tokens
        )
        
        return {
            "output": result,
            "metadata": {
                "model_used": "gpt-4",
                "tokens_generated": len(result.split())
            }
        }
    
    def _summarize_text(self, parameters):
        """Summarize provided text"""
        text = parameters["text"]
        max_length = parameters.get("max_length", 500)
        
        # Your summarization logic here
        summary = f"Summary of {len(text)} characters..."
        
        return {
            "output": summary,
            "metadata": {
                "original_length": len(text),
                "summary_length": len(summary)
            }
        }

agent = FormationAgent()

@app.route('/run_task', methods=['POST'])
def run_task():
    try:
        task_data = request.json
        
        # Validate request
        if not task_data or 'task_id' not in task_data:
            return jsonify({
                "status": "failed",
                "error": {
                    "code": "INVALID_INPUT",
                    "message": "Missing task_id"
                }
            }), 400
        
        # Start usage tracking
        tracker = UsageTracker()
        tracker.start_tracking()
        
        # Process task
        result = agent.process_task(task_data)
        
        # Stop tracking and get metrics
        usage_metrics = tracker.stop_tracking()
        
        return jsonify({
            "task_id": task_data["task_id"],
            "status": "completed",
            "result": result,
            "usage_metrics": usage_metrics.to_dict()
        })
        
    except Exception as e:
        logging.error(f"Task processing failed: {str(e)}")
        return jsonify({
            "task_id": task_data.get("task_id", "unknown"),
            "status": "failed",
            "error": {
                "code": "INTERNAL_ERROR",
                "message": str(e)
            }
        }), 500

@app.route('/health', methods=['GET'])
def health():
    return jsonify({
        "status": "healthy",
        "version": agent.version,
        "capabilities": agent.capabilities,
        "uptime_seconds": time.time() - start_time
    })

@app.route('/capabilities', methods=['GET'])
def capabilities():
    return jsonify({
        "agent_id": "formation-agent-v1",
        "name": "Formation Example Agent",
        "version": agent.version,
        "supported_task_types": [
            {
                "type": "text_generation",
                "description": "Generate text from prompts",
                "parameters": {
                    "prompt": {"type": "string", "required": True},
                    "max_tokens": {"type": "integer", "default": 1000}
                }
            }
        ]
    })

if __name__ == '__main__':
    start_time = time.time()
    app.run(host='0.0.0.0', port=8080)
```

## Framework Examples

Formation supports agents built with any framework. Your agent runs in a Linux VM and serves HTTP endpoints. Here are examples using popular frameworks:

### 1. LangChain Agent

```python
from langchain.agents import AgentExecutor, create_openai_functions_agent
from langchain_openai import ChatOpenAI
from langchain.tools import Tool
from flask import Flask, request, jsonify

app = Flask(__name__)

# Initialize LangChain agent
# Note: All model requests go through Formation, not directly to OpenAI
llm = ChatOpenAI(
    model="gpt-4",
    base_url="http://formation-api/v1",  # Formation routes to actual models
    api_key="formation-managed"
)

tools = [
    Tool(
        name="calculator",
        description="Useful for math calculations",
        func=lambda x: str(eval(x))
    )
]

agent = create_openai_functions_agent(llm, tools, prompt)
agent_executor = AgentExecutor(agent=agent, tools=tools)

@app.route('/run_task', methods=['POST'])
def run_task():
    task_data = request.json
    
    try:
        # Execute LangChain agent
        result = agent_executor.invoke({
            "input": task_data["parameters"]["prompt"]
        })
        
        return jsonify({
            "task_id": task_data["task_id"],
            "status": "completed",
            "result": {"output": result["output"]},
            "usage_metrics": {
                "tokens_processed": len(result["output"].split()),
                "duration_seconds": 2.5
            }
        })
    except Exception as e:
        return jsonify({
            "task_id": task_data["task_id"],
            "status": "failed",
            "error": {"code": "EXECUTION_ERROR", "message": str(e)}
        }), 500

@app.route('/health', methods=['GET'])
def health():
    return jsonify({"status": "healthy", "framework": "langchain"})

if __name__ == '__main__':
    app.run(host='0.0.0.0', port=8080)
```

### 2. LangChain.js Agent

```javascript
import { ChatOpenAI } from "@langchain/openai";
import { AgentExecutor, createOpenAIFunctionsAgent } from "langchain/agents";
import { Calculator } from "langchain/tools/calculator";
import express from 'express';

const app = express();
app.use(express.json());

// Initialize LangChain.js agent
// All model requests go through Formation
const model = new ChatOpenAI({ 
    modelName: "gpt-4",
    configuration: {
        baseURL: "http://formation-api/v1",
        apiKey: "formation-managed"
    }
});

const tools = [new Calculator()];
const agent = await createOpenAIFunctionsAgent({
    llm: model,
    tools,
    prompt: "You are a helpful assistant"
});

const agentExecutor = new AgentExecutor({ agent, tools });

app.post('/run_task', async (req, res) => {
    const taskData = req.body;
    
    try {
        const result = await agentExecutor.invoke({
            input: taskData.parameters.prompt
        });
        
        res.json({
            task_id: taskData.task_id,
            status: "completed",
            result: { output: result.output },
            usage_metrics: {
                tokens_processed: result.output.split(' ').length,
                duration_seconds: 1.8
            }
        });
    } catch (error) {
        res.status(500).json({
            task_id: taskData.task_id,
            status: "failed",
            error: { code: "EXECUTION_ERROR", message: error.message }
        });
    }
});

app.get('/health', (req, res) => {
    res.json({ status: "healthy", framework: "langchain.js" });
});

app.listen(8080, '0.0.0.0');
```

### 3. Agno Framework Agent

```python
from agno import Agent, Model
from agno.tools import DuckDuckGoSearch, FileTools
from flask import Flask, request, jsonify

app = Flask(__name__)

# Initialize Agno agent
# Formation handles model routing
agent = Agent(
    model=Model.GPT4,
    tools=[DuckDuckGoSearch(), FileTools()],
    instructions="You are a helpful research assistant",
    storage_id="formation_agent_storage",
    # Formation manages model access
    api_base="http://formation-api/v1"
)

@app.route('/run_task', methods=['POST'])
def run_task():
    task_data = request.json
    
    try:
        # Execute Agno agent
        response = agent.run(
            message=task_data["parameters"]["prompt"],
            session_id=task_data.get("session_id", "default")
        )
        
        return jsonify({
            "task_id": task_data["task_id"],
            "status": "completed",
            "result": {"output": response.content},
            "usage_metrics": {
                "tokens_processed": response.metrics.get("tokens", 0),
                "duration_seconds": response.metrics.get("response_time", 0)
            }
        })
    except Exception as e:
        return jsonify({
            "task_id": task_data["task_id"],
            "status": "failed",
            "error": {"code": "EXECUTION_ERROR", "message": str(e)}
        }), 500

@app.route('/health', methods=['GET'])
def health():
    return jsonify({
        "status": "healthy", 
        "framework": "agno",
        "capabilities": ["research", "file_operations", "web_search"]
    })

if __name__ == '__main__':
    app.run(host='0.0.0.0', port=8080)
```

### 4. Custom Framework Integration

```python
from fastapi import FastAPI
from pydantic import BaseModel
import httpx

app = FastAPI()

class TaskRequest(BaseModel):
    task_id: str
    parameters: dict

class CustomAgent:
    def __init__(self):
        self.capabilities = ["text_processing", "analysis"]
        # Formation handles all external model access
        self.formation_client = httpx.AsyncClient(
            base_url="http://formation-api/v1"
        )
    
    async def process_task(self, prompt: str):
        # Call models through Formation's infrastructure
        response = await self.formation_client.post(
            "/models/gpt-4/inference",
            json={
                "messages": [{"role": "user", "content": prompt}],
                "max_tokens": 1000
            }
        )
        
        result = response.json()
        return result["choices"][0]["message"]["content"]

agent = CustomAgent()

@app.post("/run_task")
async def run_task(task: TaskRequest):
    try:
        result = await agent.process_task(task.parameters["prompt"])
        
        return {
            "task_id": task.task_id,
            "status": "completed",
            "result": {"output": result},
            "usage_metrics": {
                "compute_units": 1.0,
                "duration_seconds": 0.5
            }
        }
    except Exception as e:
        return {
            "task_id": task.task_id,
            "status": "failed",
            "error": {"code": "EXECUTION_ERROR", "message": str(e)}
        }

@app.get("/health")
async def health():
    return {
        "status": "healthy",
        "framework": "custom",
        "capabilities": agent.capabilities
    }

if __name__ == "__main__":
    import uvicorn
    uvicorn.run(app, host="0.0.0.0", port=8080)
```

## Testing and Validation

### Unit Testing

```python
import unittest
import json
from your_agent import app

class TestFormationAgent(unittest.TestCase):
    def setUp(self):
        self.app = app.test_client()
        self.app.testing = True
    
    def test_health_endpoint(self):
        """Test health endpoint returns correct format"""
        response = self.app.get('/health')
        self.assertEqual(response.status_code, 200)
        
        data = json.loads(response.data)
        self.assertIn('status', data)
        self.assertIn('version', data)
        self.assertIn('capabilities', data)
    
    def test_run_task_success(self):
        """Test successful task execution"""
        task_data = {
            "task_id": "test_123",
            "task_type": "text_generation",
            "parameters": {
                "prompt": "Hello world"
            }
        }
        
        response = self.app.post('/run_task', 
                               data=json.dumps(task_data),
                               content_type='application/json')
        
        self.assertEqual(response.status_code, 200)
        
        data = json.loads(response.data)
        self.assertEqual(data['task_id'], 'test_123')
        self.assertEqual(data['status'], 'completed')
        self.assertIn('result', data)
        self.assertIn('usage_metrics', data)
    
    def test_run_task_invalid_input(self):
        """Test error handling for invalid input"""
        response = self.app.post('/run_task',
                               data=json.dumps({}),
                               content_type='application/json')
        
        self.assertEqual(response.status_code, 400)
        
        data = json.loads(response.data)
        self.assertEqual(data['status'], 'failed')
        self.assertIn('error', data)

if __name__ == '__main__':
    unittest.main()
```

### Integration Testing

```bash
#!/bin/bash
# test-agent-integration.sh

# Test health endpoint
echo "Testing health endpoint..."
curl -X GET http://localhost:8080/health

# Test capabilities endpoint
echo "Testing capabilities endpoint..."
curl -X GET http://localhost:8080/capabilities

# Test task execution
echo "Testing task execution..."
curl -X POST http://localhost:8080/run_task \
  -H "Content-Type: application/json" \
  -d '{
    "task_id": "integration_test_1",
    "task_type": "text_generation",
    "parameters": {
      "prompt": "Write a short story about AI",
      "max_tokens": 500
    }
  }'
```

## Performance Requirements

### Response Time Targets

- **Health Check**: < 100ms
- **Simple Tasks**: < 5 seconds
- **Complex Tasks**: < 30 seconds
- **Timeout Handling**: Must respect timeout_seconds parameter

### Resource Efficiency

- **Memory Usage**: Monitor and report peak memory usage
- **CPU Utilization**: Optimize for efficient CPU usage
- **Network Bandwidth**: Minimize unnecessary network calls
- **Startup Time**: Container should start within 30 seconds

### Scalability Considerations

- **Concurrent Requests**: Handle multiple requests efficiently
- **Resource Cleanup**: Properly clean up resources after each task
- **Error Recovery**: Gracefully handle and recover from errors
- **Monitoring**: Provide detailed metrics for monitoring and debugging

## Security Requirements

### Input Sanitization

```python
import re
import html

def sanitize_input(text):
    """Sanitize user input to prevent injection attacks"""
    # Remove potentially dangerous characters
    text = re.sub(r'[<>"\']', '', text)
    
    # HTML escape
    text = html.escape(text)
    
    # Limit length
    if len(text) > 10000:
        text = text[:10000]
    
    return text
```

### Rate Limiting

```python
from collections import defaultdict
import time

class RateLimiter:
    def __init__(self, max_requests=100, window_seconds=60):
        self.max_requests = max_requests
        self.window_seconds = window_seconds
        self.requests = defaultdict(list)
    
    def is_allowed(self, requester_address):
        """Check if request is allowed based on rate limits"""
        now = time.time()
        window_start = now - self.window_seconds
        
        # Clean old requests
        self.requests[requester_address] = [
            req_time for req_time in self.requests[requester_address]
            if req_time > window_start
        ]
        
        # Check if under limit
        if len(self.requests[requester_address]) >= self.max_requests:
            return False
        
        # Add current request
        self.requests[requester_address].append(now)
        return True
```

## Deployment Considerations

### Container Requirements

Your agent must be packaged as a Docker container:

```dockerfile
FROM python:3.9-slim

# Install system dependencies
RUN apt-get update && apt-get install -y \
    gcc \
    && rm -rf /var/lib/apt/lists/*

# Set working directory
WORKDIR /app

# Copy requirements and install Python dependencies
COPY requirements.txt .
RUN pip install --no-cache-dir -r requirements.txt

# Copy application code
COPY . .

# Expose port
EXPOSE 8080

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
  CMD curl -f http://localhost:8080/health || exit 1

# Run application
CMD ["python", "app.py"]
```

### Environment Configuration

```python
import os

class Config:
    # Server configuration
    HOST = os.getenv('HOST', '0.0.0.0')
    PORT = int(os.getenv('PORT', 8080))
    
    # Agent configuration
    AGENT_ID = os.getenv('AGENT_ID', 'formation-agent')
    VERSION = os.getenv('VERSION', '1.0.0')
    
    # Formation configuration
    FORMATION_API_URL = os.getenv('FORMATION_API_URL', 'http://formation-api')
    DATABASE_URL = os.getenv('DATABASE_URL')
    
    # Performance configuration
    MAX_CONCURRENT_TASKS = int(os.getenv('MAX_CONCURRENT_TASKS', 10))
    TASK_TIMEOUT = int(os.getenv('TASK_TIMEOUT', 300))
    
    # Logging configuration
    LOG_LEVEL = os.getenv('LOG_LEVEL', 'INFO')
```

## Summary

This guide provides the technical specifications for building Formation-compatible AI agents. Key requirements:

### ✅ Required
- **HTTP API**: Implement `/run_task`, `/health`, and optionally `/capabilities` endpoints
- **Formation Model Access**: Route ALL external model requests through Formation's infrastructure
- **Usage Metrics**: Report accurate usage data for Formation's billing system
- **Containerization**: Package as Docker container for deployment

### ❌ Prohibited  
- **Direct External API Calls**: Cannot call OpenAI, Anthropic, etc. directly
- **Custom Billing Logic**: Formation handles all billing calculations
- **Direct Authentication**: Formation manages all authentication

### 🎯 Best Practices
- Use any framework (LangChain, Agno, custom) - Formation is framework-agnostic
- Optimize for performance to improve task assignment scores
- Implement comprehensive error handling and logging
- Follow security best practices for input validation

Follow these specifications to ensure your agent integrates seamlessly with Formation's decentralized network. 