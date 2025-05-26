# Formation Agent Examples

This guide provides complete, working examples of Formation agents that you can use as templates for your own projects. Each example includes full source code, containerization, and deployment configurations.

## Overview

We'll cover three types of agents:

1. **[Simple Echo Agent](#1-simple-echo-agent)** - Basic HTTP API agent for learning Formation integration
2. **[LLM-Powered Agent](#2-llm-powered-agent)** - AI agent using Formation's model infrastructure  
3. **[Multi-Step Workflow Agent](#3-multi-step-workflow-agent)** - Complex agent with multiple processing steps

Each example follows Formation's best practices:
- ✅ **Containerized deployment** with Docker
- ✅ **Formation model integration** (where applicable)
- ✅ **Proper error handling** and logging
- ✅ **Complete deployment configuration**

---

## 1. Simple Echo Agent

A basic agent that echoes back user input with some processing. Perfect for learning Formation's agent architecture.

### Features
- HTTP API with required endpoints (`/run_task`, `/health`, `/capabilities`)
- Input validation and error handling
- Usage metrics reporting
- Docker containerization

### Project Structure

```
echo-agent/
├── Formfile
├── docker-compose.yml
├── agent-container/
│   ├── Dockerfile
│   ├── requirements.txt
│   └── agent/
│       ├── main.py
│       ├── handlers.py
│       └── utils.py
└── README.md
```

### Source Code

#### `agent-container/agent/main.py`

```python
#!/usr/bin/env python3
"""
Formation Echo Agent - Simple example agent
"""

import time
import logging
from flask import Flask, request, jsonify
from handlers import EchoHandler
from utils import UsageTracker, validate_task_request

# Configure logging
logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s - %(name)s - %(levelname)s - %(message)s'
)
logger = logging.getLogger(__name__)

app = Flask(__name__)
handler = EchoHandler()

# Track startup time for uptime calculation
start_time = time.time()

@app.route('/run_task', methods=['POST'])
def run_task():
    """Main task execution endpoint"""
    try:
        task_data = request.json
        logger.info(f"Received task: {task_data.get('task_id', 'unknown')}")
        
        # Validate request
        validation_error = validate_task_request(task_data)
        if validation_error:
            return jsonify({
                "task_id": task_data.get("task_id", "unknown"),
                "status": "failed",
                "error": {
                    "code": "INVALID_INPUT",
                    "message": validation_error
                }
            }), 400
        
        # Start usage tracking
        tracker = UsageTracker()
        tracker.start_tracking()
        
        # Process task
        result = handler.process_task(task_data)
        
        # Stop tracking and get metrics
        usage_metrics = tracker.stop_tracking()
        
        response = {
            "task_id": task_data["task_id"],
            "status": "completed",
            "result": result,
            "usage_metrics": usage_metrics
        }
        
        logger.info(f"Task completed: {task_data['task_id']}")
        return jsonify(response)
        
    except Exception as e:
        logger.error(f"Task processing failed: {str(e)}")
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
    """Health check endpoint"""
    uptime = time.time() - start_time
    return jsonify({
        "status": "healthy",
        "version": "1.0.0",
        "uptime_seconds": int(uptime),
        "capabilities": ["echo", "text_processing"],
        "framework": "custom"
    })

@app.route('/capabilities', methods=['GET'])
def capabilities():
    """Agent capabilities endpoint"""
    return jsonify({
        "agent_id": "echo-agent-v1",
        "name": "Formation Echo Agent",
        "description": "Simple echo agent for testing and learning",
        "version": "1.0.0",
        "supported_task_types": [
            {
                "type": "echo",
                "description": "Echo back the input with optional processing",
                "parameters": {
                    "message": {"type": "string", "required": True},
                    "uppercase": {"type": "boolean", "default": False},
                    "reverse": {"type": "boolean", "default": False}
                }
            }
        ],
        "resource_requirements": {
            "min_memory_mb": 128,
            "min_cpu_cores": 1,
            "gpu_required": False
        }
    })

if __name__ == '__main__':
    logger.info("Starting Formation Echo Agent...")
    app.run(host='0.0.0.0', port=8080, debug=False)
```

#### `agent-container/agent/handlers.py`

```python
"""
Task handlers for the Echo Agent
"""

import time
import logging

logger = logging.getLogger(__name__)

class EchoHandler:
    """Handles echo tasks"""
    
    def process_task(self, task_data):
        """Process an echo task"""
        task_type = task_data.get("task_type", "echo")
        parameters = task_data.get("parameters", {})
        
        if task_type == "echo":
            return self._handle_echo(parameters)
        else:
            raise ValueError(f"Unsupported task type: {task_type}")
    
    def _handle_echo(self, parameters):
        """Handle echo task"""
        message = parameters.get("message", "")
        uppercase = parameters.get("uppercase", False)
        reverse = parameters.get("reverse", False)
        
        # Process the message
        processed_message = message
        
        if reverse:
            processed_message = processed_message[::-1]
        
        if uppercase:
            processed_message = processed_message.upper()
        
        # Add some processing metadata
        word_count = len(message.split())
        char_count = len(message)
        
        return {
            "output": processed_message,
            "metadata": {
                "original_message": message,
                "word_count": word_count,
                "character_count": char_count,
                "processing_applied": {
                    "uppercase": uppercase,
                    "reverse": reverse
                },
                "processing_time_ms": 1  # Minimal processing time
            }
        }
```

#### `agent-container/agent/utils.py`

```python
"""
Utility functions for the Echo Agent
"""

import time
import psutil
import threading

def validate_task_request(task_data):
    """Validate incoming task request"""
    if not task_data:
        return "Request body is required"
    
    if "task_id" not in task_data:
        return "task_id is required"
    
    if "task_type" not in task_data:
        return "task_type is required"
    
    if "parameters" not in task_data:
        return "parameters are required"
    
    # Validate echo-specific parameters
    task_type = task_data.get("task_type")
    if task_type == "echo":
        parameters = task_data.get("parameters", {})
        if "message" not in parameters:
            return "message parameter is required for echo tasks"
        
        if not isinstance(parameters["message"], str):
            return "message must be a string"
    
    return None

class UsageTracker:
    """Track resource usage for billing purposes"""
    
    def __init__(self):
        self.start_time = None
        self.peak_memory = 0
        self.monitoring = False
        
    def start_tracking(self):
        """Start tracking resource usage"""
        self.start_time = time.time()
        self.peak_memory = 0
        self.monitoring = True
        
        # Start memory monitoring in background
        threading.Thread(target=self._monitor_memory, daemon=True).start()
        
    def stop_tracking(self):
        """Stop tracking and return metrics"""
        self.monitoring = False
        end_time = time.time()
        
        duration = end_time - self.start_time
        
        return {
            "compute_units": 0.1,  # Minimal compute for echo
            "memory_mb": self.peak_memory,
            "duration_seconds": round(duration, 3),
            "tokens_processed": 0,  # No tokens in echo agent
            "api_calls": 0
        }
    
    def _monitor_memory(self):
        """Monitor memory usage in background"""
        process = psutil.Process()
        while self.monitoring:
            try:
                memory_mb = process.memory_info().rss / 1024 / 1024
                self.peak_memory = max(self.peak_memory, memory_mb)
                time.sleep(0.1)
            except:
                break
```

### Container Configuration

#### `agent-container/Dockerfile`

```dockerfile
FROM python:3.9-slim

WORKDIR /app

# Install system dependencies
RUN apt-get update && apt-get install -y \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Install Python dependencies
COPY requirements.txt .
RUN pip install --no-cache-dir -r requirements.txt

# Copy agent code
COPY agent/ ./agent/

# Expose port
EXPOSE 8080

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
  CMD curl -f http://localhost:8080/health || exit 1

# Run agent
CMD ["python", "agent/main.py"]
```

#### `agent-container/requirements.txt`

```
flask==2.3.3
psutil==5.9.5
requests==2.31.0
```

#### `docker-compose.yml`

```yaml
version: '3.8'

services:
  echo-agent:
    build: ./agent-container
    container_name: formation-echo-agent
    ports:
      - "8080:8080"
    environment:
      - LOG_LEVEL=info
    restart: unless-stopped
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:8080/health"]
      interval: 30s
      timeout: 10s
      retries: 3
      start_period: 40s
```

### Deployment Configuration

#### `Formfile`

```
NAME echo-agent
DESCRIPTION "Simple echo agent for Formation network"

# System Resources
VCPU 1
MEM 1024
DISK 10

# User Configuration
USER username:agent passwd:securepass sudo:true ssh_authorized_keys:"ssh-rsa AAAAB3NzaC1yc2E..."

# Base System
FROM ubuntu:22.04

# Install Docker
RUN apt-get update && apt-get install -y docker.io curl
RUN systemctl enable docker

# Copy agent container and configuration
COPY ./agent-container /app/agent-container
COPY ./docker-compose.yml /app/docker-compose.yml

# Set working directory
WORKDIR /app

# Start Docker service and run agent container
ENTRYPOINT ["sh", "-c", "service docker start && docker-compose up -d"]
```

### Testing the Echo Agent

```bash
# Test health endpoint
curl http://localhost:8080/health

# Test capabilities
curl http://localhost:8080/capabilities

# Test echo task
curl -X POST http://localhost:8080/run_task \
  -H "Content-Type: application/json" \
  -d '{
    "task_id": "test_echo_1",
    "task_type": "echo",
    "parameters": {
      "message": "Hello Formation!",
      "uppercase": true,
      "reverse": false
    }
  }'

# Expected response:
{
  "task_id": "test_echo_1",
  "status": "completed",
  "result": {
    "output": "HELLO FORMATION!",
    "metadata": {
      "original_message": "Hello Formation!",
      "word_count": 2,
      "character_count": 16,
      "processing_applied": {
        "uppercase": true,
        "reverse": false
      }
    }
  },
  "usage_metrics": {
    "compute_units": 0.1,
    "memory_mb": 45.2,
    "duration_seconds": 0.003
  }
}
```

---

## 2. LLM-Powered Agent

An AI agent that uses Formation's model infrastructure to provide intelligent text processing capabilities.

### Features
- Integration with Formation's model routing
- Multiple AI capabilities (text generation, summarization, Q&A)
- Proper error handling for model failures
- Token usage tracking

### Project Structure

```
llm-agent/
├── Formfile
├── docker-compose.yml
├── agent-container/
│   ├── Dockerfile
│   ├── requirements.txt
│   └── agent/
│       ├── main.py
│       ├── handlers.py
│       ├── formation_client.py
│       └── utils.py
└── README.md
```

### Source Code

#### `agent-container/agent/main.py`

```python
#!/usr/bin/env python3
"""
Formation LLM Agent - AI-powered agent using Formation's model infrastructure
"""

import time
import logging
import os
from flask import Flask, request, jsonify
from handlers import LLMHandler
from utils import UsageTracker, validate_task_request

# Configure logging
logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s - %(name)s - %(levelname)s - %(message)s'
)
logger = logging.getLogger(__name__)

app = Flask(__name__)

# Initialize handler with Formation API configuration
formation_api_url = os.getenv('FORMATION_API_URL', 'http://formation-api:3004')
handler = LLMHandler(formation_api_url)

# Track startup time
start_time = time.time()

@app.route('/run_task', methods=['POST'])
def run_task():
    """Main task execution endpoint"""
    try:
        task_data = request.json
        logger.info(f"Received LLM task: {task_data.get('task_id', 'unknown')}")
        
        # Validate request
        validation_error = validate_task_request(task_data)
        if validation_error:
            return jsonify({
                "task_id": task_data.get("task_id", "unknown"),
                "status": "failed",
                "error": {
                    "code": "INVALID_INPUT",
                    "message": validation_error
                }
            }), 400
        
        # Start usage tracking
        tracker = UsageTracker()
        tracker.start_tracking()
        
        # Process task
        result = handler.process_task(task_data)
        
        # Stop tracking and get metrics
        usage_metrics = tracker.stop_tracking()
        
        # Add token count from result if available
        if "metadata" in result and "tokens_used" in result["metadata"]:
            usage_metrics["tokens_processed"] = result["metadata"]["tokens_used"]
        
        response = {
            "task_id": task_data["task_id"],
            "status": "completed",
            "result": result,
            "usage_metrics": usage_metrics
        }
        
        logger.info(f"LLM task completed: {task_data['task_id']}")
        return jsonify(response)
        
    except Exception as e:
        logger.error(f"LLM task processing failed: {str(e)}")
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
    """Health check endpoint"""
    uptime = time.time() - start_time
    
    # Check Formation API connectivity
    formation_status = handler.check_formation_connectivity()
    
    return jsonify({
        "status": "healthy" if formation_status else "degraded",
        "version": "1.0.0",
        "uptime_seconds": int(uptime),
        "capabilities": ["text_generation", "summarization", "question_answering"],
        "framework": "formation_llm",
        "dependencies": {
            "formation_api": "connected" if formation_status else "disconnected"
        }
    })

@app.route('/capabilities', methods=['GET'])
def capabilities():
    """Agent capabilities endpoint"""
    return jsonify({
        "agent_id": "llm-agent-v1",
        "name": "Formation LLM Agent",
        "description": "AI agent powered by Formation's model infrastructure",
        "version": "1.0.0",
        "supported_task_types": [
            {
                "type": "text_generation",
                "description": "Generate text based on prompts",
                "parameters": {
                    "prompt": {"type": "string", "required": True},
                    "max_tokens": {"type": "integer", "default": 1000},
                    "temperature": {"type": "number", "default": 0.7},
                    "model": {"type": "string", "default": "gpt-4"}
                }
            },
            {
                "type": "summarization",
                "description": "Summarize long text content",
                "parameters": {
                    "text": {"type": "string", "required": True},
                    "max_length": {"type": "integer", "default": 500},
                    "model": {"type": "string", "default": "gpt-4"}
                }
            },
            {
                "type": "question_answering",
                "description": "Answer questions based on context",
                "parameters": {
                    "question": {"type": "string", "required": True},
                    "context": {"type": "string", "required": False},
                    "model": {"type": "string", "default": "gpt-4"}
                }
            }
        ],
        "resource_requirements": {
            "min_memory_mb": 512,
            "min_cpu_cores": 1,
            "gpu_required": False
        }
    })

if __name__ == '__main__':
    logger.info("Starting Formation LLM Agent...")
    app.run(host='0.0.0.0', port=8080, debug=False)
```

#### `agent-container/agent/handlers.py`

```python
"""
Task handlers for the LLM Agent
"""

import logging
from formation_client import FormationModelClient

logger = logging.getLogger(__name__)

class LLMHandler:
    """Handles LLM-powered tasks"""
    
    def __init__(self, formation_api_url):
        self.formation_client = FormationModelClient(formation_api_url)
    
    def process_task(self, task_data):
        """Process an LLM task"""
        task_type = task_data.get("task_type")
        parameters = task_data.get("parameters", {})
        
        if task_type == "text_generation":
            return self._handle_text_generation(parameters)
        elif task_type == "summarization":
            return self._handle_summarization(parameters)
        elif task_type == "question_answering":
            return self._handle_question_answering(parameters)
        else:
            raise ValueError(f"Unsupported task type: {task_type}")
    
    def _handle_text_generation(self, parameters):
        """Handle text generation task"""
        prompt = parameters.get("prompt")
        max_tokens = parameters.get("max_tokens", 1000)
        temperature = parameters.get("temperature", 0.7)
        model = parameters.get("model", "gpt-4")
        
        # Call Formation's model infrastructure
        response = self.formation_client.call_model(
            model_id=model,
            messages=[{"role": "user", "content": prompt}],
            max_tokens=max_tokens,
            temperature=temperature
        )
        
        return {
            "output": response["content"],
            "metadata": {
                "model_used": model,
                "tokens_used": response.get("tokens_used", 0),
                "finish_reason": response.get("finish_reason", "completed"),
                "processing_time_ms": response.get("processing_time_ms", 0)
            }
        }
    
    def _handle_summarization(self, parameters):
        """Handle text summarization task"""
        text = parameters.get("text")
        max_length = parameters.get("max_length", 500)
        model = parameters.get("model", "gpt-4")
        
        # Create summarization prompt
        prompt = f"""Please summarize the following text in approximately {max_length} characters or less:

{text}

Summary:"""
        
        response = self.formation_client.call_model(
            model_id=model,
            messages=[{"role": "user", "content": prompt}],
            max_tokens=max_length // 4,  # Rough token estimate
            temperature=0.3  # Lower temperature for more focused summaries
        )
        
        return {
            "output": response["content"],
            "metadata": {
                "model_used": model,
                "original_length": len(text),
                "summary_length": len(response["content"]),
                "tokens_used": response.get("tokens_used", 0),
                "compression_ratio": round(len(response["content"]) / len(text), 2)
            }
        }
    
    def _handle_question_answering(self, parameters):
        """Handle question answering task"""
        question = parameters.get("question")
        context = parameters.get("context", "")
        model = parameters.get("model", "gpt-4")
        
        # Create Q&A prompt
        if context:
            prompt = f"""Based on the following context, please answer the question:

Context: {context}

Question: {question}

Answer:"""
        else:
            prompt = f"Question: {question}\n\nAnswer:"
        
        response = self.formation_client.call_model(
            model_id=model,
            messages=[{"role": "user", "content": prompt}],
            max_tokens=1000,
            temperature=0.1  # Very low temperature for factual answers
        )
        
        return {
            "output": response["content"],
            "metadata": {
                "model_used": model,
                "question": question,
                "has_context": bool(context),
                "tokens_used": response.get("tokens_used", 0),
                "confidence": "high" if context else "medium"
            }
        }
    
    def check_formation_connectivity(self):
        """Check if Formation API is accessible"""
        try:
            return self.formation_client.health_check()
        except Exception as e:
            logger.error(f"Formation connectivity check failed: {e}")
            return False
```

#### `agent-container/agent/formation_client.py`

```python
"""
Formation Model Client - Interface to Formation's model infrastructure
"""

import requests
import logging
import time

logger = logging.getLogger(__name__)

class FormationModelClient:
    """Client for Formation's model infrastructure"""
    
    def __init__(self, base_url):
        self.base_url = base_url.rstrip('/')
        self.session = requests.Session()
        self.session.timeout = 30
    
    def call_model(self, model_id, messages, **kwargs):
        """Call a model through Formation's infrastructure"""
        start_time = time.time()
        
        try:
            # Formation routes all model requests
            response = self.session.post(
                f"{self.base_url}/v1/models/{model_id}/inference",
                json={
                    "messages": messages,
                    "max_tokens": kwargs.get("max_tokens", 1000),
                    "temperature": kwargs.get("temperature", 0.7),
                    "stream": False
                },
                headers={
                    "Content-Type": "application/json"
                }
            )
            
            response.raise_for_status()
            result = response.json()
            
            processing_time = int((time.time() - start_time) * 1000)
            
            # Extract content from OpenAI-compatible response
            if "choices" in result and len(result["choices"]) > 0:
                content = result["choices"][0]["message"]["content"]
                tokens_used = result.get("usage", {}).get("total_tokens", 0)
                finish_reason = result["choices"][0].get("finish_reason", "completed")
            else:
                raise ValueError("Invalid response format from Formation model API")
            
            return {
                "content": content,
                "tokens_used": tokens_used,
                "finish_reason": finish_reason,
                "processing_time_ms": processing_time
            }
            
        except requests.exceptions.RequestException as e:
            logger.error(f"Formation model API request failed: {e}")
            raise Exception(f"Model API request failed: {str(e)}")
        except Exception as e:
            logger.error(f"Formation model call failed: {e}")
            raise Exception(f"Model call failed: {str(e)}")
    
    def health_check(self):
        """Check Formation API health"""
        try:
            response = self.session.get(f"{self.base_url}/health", timeout=5)
            return response.status_code == 200
        except:
            return False
```

#### `agent-container/agent/utils.py`

```python
"""
Utility functions for the LLM Agent
"""

import time
import psutil
import threading

def validate_task_request(task_data):
    """Validate incoming task request"""
    if not task_data:
        return "Request body is required"
    
    required_fields = ["task_id", "task_type", "parameters"]
    for field in required_fields:
        if field not in task_data:
            return f"{field} is required"
    
    task_type = task_data.get("task_type")
    parameters = task_data.get("parameters", {})
    
    # Validate task-specific parameters
    if task_type == "text_generation":
        if "prompt" not in parameters:
            return "prompt parameter is required for text_generation"
        if not isinstance(parameters["prompt"], str):
            return "prompt must be a string"
    
    elif task_type == "summarization":
        if "text" not in parameters:
            return "text parameter is required for summarization"
        if not isinstance(parameters["text"], str):
            return "text must be a string"
    
    elif task_type == "question_answering":
        if "question" not in parameters:
            return "question parameter is required for question_answering"
        if not isinstance(parameters["question"], str):
            return "question must be a string"
    
    return None

class UsageTracker:
    """Track resource usage for billing purposes"""
    
    def __init__(self):
        self.start_time = None
        self.peak_memory = 0
        self.monitoring = False
        
    def start_tracking(self):
        """Start tracking resource usage"""
        self.start_time = time.time()
        self.peak_memory = 0
        self.monitoring = True
        
        # Start memory monitoring in background
        threading.Thread(target=self._monitor_memory, daemon=True).start()
        
    def stop_tracking(self):
        """Stop tracking and return metrics"""
        self.monitoring = False
        end_time = time.time()
        
        duration = end_time - self.start_time
        
        return {
            "compute_units": round(duration * 2.0, 2),  # Higher compute for LLM tasks
            "memory_mb": self.peak_memory,
            "duration_seconds": round(duration, 3),
            "tokens_processed": 0,  # Will be updated by handler
            "api_calls": 1
        }
    
    def _monitor_memory(self):
        """Monitor memory usage in background"""
        process = psutil.Process()
        while self.monitoring:
            try:
                memory_mb = process.memory_info().rss / 1024 / 1024
                self.peak_memory = max(self.peak_memory, memory_mb)
                time.sleep(0.1)
            except:
                break
```

### Container Configuration

#### `agent-container/requirements.txt`

```
flask==2.3.3
requests==2.31.0
psutil==5.9.5
```

#### `docker-compose.yml`

```yaml
version: '3.8'

services:
  llm-agent:
    build: ./agent-container
    container_name: formation-llm-agent
    ports:
      - "8080:8080"
    environment:
      - FORMATION_API_URL=http://formation-api:3004
      - LOG_LEVEL=info
    restart: unless-stopped
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:8080/health"]
      interval: 30s
      timeout: 10s
      retries: 3
      start_period: 40s
```

### Deployment Configuration

#### `Formfile`

```
NAME llm-agent
DESCRIPTION "LLM-powered agent using Formation's model infrastructure"

# System Resources (higher for LLM processing)
VCPU 2
MEM 2048
DISK 15

# User Configuration
USER username:agent passwd:securepass sudo:true ssh_authorized_keys:"ssh-rsa AAAAB3NzaC1yc2E..."

# Base System
FROM ubuntu:22.04

# Install Docker
RUN apt-get update && apt-get install -y docker.io curl
RUN systemctl enable docker

# Copy agent container and configuration
COPY ./agent-container /app/agent-container
COPY ./docker-compose.yml /app/docker-compose.yml

# Set working directory
WORKDIR /app

# Start Docker service and run agent container
ENTRYPOINT ["sh", "-c", "service docker start && docker-compose up -d"]
```

### Testing the LLM Agent

```bash
# Test text generation
curl -X POST http://localhost:8080/run_task \
  -H "Content-Type: application/json" \
  -d '{
    "task_id": "test_generation_1",
    "task_type": "text_generation",
    "parameters": {
      "prompt": "Explain quantum computing in simple terms",
      "max_tokens": 500,
      "temperature": 0.7
    }
  }'

# Test summarization
curl -X POST http://localhost:8080/run_task \
  -H "Content-Type: application/json" \
  -d '{
    "task_id": "test_summary_1",
    "task_type": "summarization",
    "parameters": {
      "text": "Long article text here...",
      "max_length": 200
    }
  }'

# Test question answering
curl -X POST http://localhost:8080/run_task \
  -H "Content-Type: application/json" \
  -d '{
    "task_id": "test_qa_1",
    "task_type": "question_answering",
    "parameters": {
      "question": "What is the capital of France?",
      "context": "France is a country in Europe. Its capital city is Paris."
    }
  }'
```

---

## 3. Multi-Step Workflow Agent

A sophisticated agent that performs complex, multi-step workflows with state management and error recovery.

### Features
- Multi-step task processing with state management
- Error recovery and retry logic
- Progress tracking and intermediate results
- Integration with multiple Formation services

### Project Structure

```
workflow-agent/
├── Formfile
├── docker-compose.yml
├── agent-container/
│   ├── Dockerfile
│   ├── requirements.txt
│   └── agent/
│       ├── main.py
│       ├── workflow_engine.py
│       ├── steps/
│       │   ├── __init__.py
│       │   ├── data_processing.py
│       │   ├── analysis.py
│       │   └── reporting.py
│       ├── formation_client.py
│       └── utils.py
└── README.md
```

### Source Code

#### `agent-container/agent/main.py`

```python
#!/usr/bin/env python3
"""
Formation Workflow Agent - Multi-step workflow processing
"""

import time
import logging
import os
from flask import Flask, request, jsonify
from workflow_engine import WorkflowEngine
from utils import validate_workflow_request

# Configure logging
logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s - %(name)s - %(levelname)s - %(message)s'
)
logger = logging.getLogger(__name__)

app = Flask(__name__)

# Initialize workflow engine
formation_api_url = os.getenv('FORMATION_API_URL', 'http://formation-api:3004')
workflow_engine = WorkflowEngine(formation_api_url)

# Track startup time
start_time = time.time()

@app.route('/run_task', methods=['POST'])
def run_task():
    """Main task execution endpoint"""
    try:
        task_data = request.json
        logger.info(f"Received workflow task: {task_data.get('task_id', 'unknown')}")
        
        # Validate request
        validation_error = validate_workflow_request(task_data)
        if validation_error:
            return jsonify({
                "task_id": task_data.get("task_id", "unknown"),
                "status": "failed",
                "error": {
                    "code": "INVALID_INPUT",
                    "message": validation_error
                }
            }), 400
        
        # Execute workflow
        result = workflow_engine.execute_workflow(task_data)
        
        logger.info(f"Workflow completed: {task_data['task_id']}")
        return jsonify(result)
        
    except Exception as e:
        logger.error(f"Workflow execution failed: {str(e)}")
        return jsonify({
            "task_id": task_data.get("task_id", "unknown"),
            "status": "failed",
            "error": {
                "code": "WORKFLOW_ERROR",
                "message": str(e)
            }
        }), 500

@app.route('/workflow_status/<task_id>', methods=['GET'])
def workflow_status(task_id):
    """Get workflow execution status"""
    status = workflow_engine.get_workflow_status(task_id)
    if status:
        return jsonify(status)
    else:
        return jsonify({"error": "Workflow not found"}), 404

@app.route('/health', methods=['GET'])
def health():
    """Health check endpoint"""
    uptime = time.time() - start_time
    
    return jsonify({
        "status": "healthy",
        "version": "1.0.0",
        "uptime_seconds": int(uptime),
        "capabilities": ["data_analysis_workflow", "report_generation", "multi_step_processing"],
        "framework": "formation_workflow",
        "active_workflows": workflow_engine.get_active_workflow_count()
    })

@app.route('/capabilities', methods=['GET'])
def capabilities():
    """Agent capabilities endpoint"""
    return jsonify({
        "agent_id": "workflow-agent-v1",
        "name": "Formation Workflow Agent",
        "description": "Multi-step workflow processing agent",
        "version": "1.0.0",
        "supported_task_types": [
            {
                "type": "data_analysis_workflow",
                "description": "Complete data analysis workflow with multiple steps",
                "parameters": {
                    "data": {"type": "string", "required": True},
                    "analysis_type": {"type": "string", "default": "comprehensive"},
                    "output_format": {"type": "string", "default": "report"}
                }
            }
        ],
        "workflow_steps": [
            "data_validation",
            "data_processing", 
            "analysis",
            "report_generation"
        ],
        "resource_requirements": {
            "min_memory_mb": 1024,
            "min_cpu_cores": 2,
            "gpu_required": False
        }
    })

if __name__ == '__main__':
    logger.info("Starting Formation Workflow Agent...")
    app.run(host='0.0.0.0', port=8080, debug=False)
```

#### `agent-container/agent/workflow_engine.py`

```python
"""
Workflow execution engine for multi-step processing
"""

import time
import logging
import uuid
from typing import Dict, Any, List
from steps.data_processing import DataProcessingStep
from steps.analysis import AnalysisStep
from steps.reporting import ReportingStep
from formation_client import FormationModelClient

logger = logging.getLogger(__name__)

class WorkflowEngine:
    """Manages workflow execution with state tracking"""
    
    def __init__(self, formation_api_url):
        self.formation_client = FormationModelClient(formation_api_url)
        self.active_workflows = {}
        
        # Initialize workflow steps
        self.steps = {
            "data_processing": DataProcessingStep(self.formation_client),
            "analysis": AnalysisStep(self.formation_client),
            "reporting": ReportingStep(self.formation_client)
        }
    
    def execute_workflow(self, task_data):
        """Execute a complete workflow"""
        task_id = task_data["task_id"]
        workflow_type = task_data.get("task_type", "data_analysis_workflow")
        parameters = task_data.get("parameters", {})
        
        # Initialize workflow state
        workflow_state = {
            "task_id": task_id,
            "workflow_type": workflow_type,
            "status": "running",
            "current_step": 0,
            "steps": [],
            "start_time": time.time(),
            "parameters": parameters,
            "results": {},
            "errors": []
        }
        
        self.active_workflows[task_id] = workflow_state
        
        try:
            if workflow_type == "data_analysis_workflow":
                return self._execute_data_analysis_workflow(workflow_state)
            else:
                raise ValueError(f"Unsupported workflow type: {workflow_type}")
                
        except Exception as e:
            workflow_state["status"] = "failed"
            workflow_state["errors"].append(str(e))
            logger.error(f"Workflow {task_id} failed: {e}")
            raise
        finally:
            workflow_state["end_time"] = time.time()
            workflow_state["duration"] = workflow_state["end_time"] - workflow_state["start_time"]
    
    def _execute_data_analysis_workflow(self, workflow_state):
        """Execute data analysis workflow"""
        task_id = workflow_state["task_id"]
        parameters = workflow_state["parameters"]
        
        # Define workflow steps
        workflow_steps = [
            ("data_validation", self._validate_data),
            ("data_processing", self.steps["data_processing"].execute),
            ("analysis", self.steps["analysis"].execute),
            ("reporting", self.steps["reporting"].execute)
        ]
        
        workflow_state["steps"] = [step[0] for step in workflow_steps]
        
        # Execute each step
        for i, (step_name, step_function) in enumerate(workflow_steps):
            workflow_state["current_step"] = i
            logger.info(f"Executing step {i+1}/{len(workflow_steps)}: {step_name}")
            
            try:
                step_result = step_function(parameters, workflow_state["results"])
                workflow_state["results"][step_name] = step_result
                logger.info(f"Step {step_name} completed successfully")
                
            except Exception as e:
                error_msg = f"Step {step_name} failed: {str(e)}"
                workflow_state["errors"].append(error_msg)
                logger.error(error_msg)
                
                # Attempt recovery for certain steps
                if self._can_recover(step_name, e):
                    logger.info(f"Attempting recovery for step {step_name}")
                    step_result = self._recover_step(step_name, parameters, workflow_state["results"], e)
                    workflow_state["results"][step_name] = step_result
                else:
                    raise
        
        # Mark workflow as completed
        workflow_state["status"] = "completed"
        
        # Compile final result
        final_result = {
            "task_id": task_id,
            "status": "completed",
            "result": {
                "output": workflow_state["results"].get("reporting", {}).get("final_report", ""),
                "metadata": {
                    "workflow_type": workflow_state["workflow_type"],
                    "steps_completed": len(workflow_state["steps"]),
                    "total_duration": workflow_state["duration"],
                    "step_results": {k: v.get("summary", "") for k, v in workflow_state["results"].items()}
                }
            },
            "usage_metrics": self._calculate_usage_metrics(workflow_state)
        }
        
        return final_result
    
    def _validate_data(self, parameters, previous_results):
        """Validate input data"""
        data = parameters.get("data", "")
        
        if not data:
            raise ValueError("No data provided for analysis")
        
        if len(data) < 10:
            raise ValueError("Data too short for meaningful analysis")
        
        return {
            "status": "valid",
            "data_length": len(data),
            "data_type": "text",
            "summary": f"Validated {len(data)} characters of text data"
        }
    
    def _can_recover(self, step_name, error):
        """Check if a step can be recovered from error"""
        recoverable_steps = ["data_processing", "analysis"]
        return step_name in recoverable_steps
    
    def _recover_step(self, step_name, parameters, previous_results, error):
        """Attempt to recover from step failure"""
        logger.info(f"Recovering step {step_name} from error: {error}")
        
        if step_name == "data_processing":
            # Use simplified processing
            return {
                "status": "recovered",
                "processed_data": parameters.get("data", ""),
                "processing_method": "simplified",
                "summary": "Data processing recovered with simplified method"
            }
        
        elif step_name == "analysis":
            # Use basic analysis
            return {
                "status": "recovered", 
                "analysis_result": "Basic analysis completed",
                "confidence": "low",
                "summary": "Analysis recovered with basic method"
            }
        
        return {"status": "failed", "summary": f"Could not recover step {step_name}"}
    
    def _calculate_usage_metrics(self, workflow_state):
        """Calculate usage metrics for the workflow"""
        duration = workflow_state.get("duration", 0)
        steps_count = len(workflow_state["steps"])
        
        return {
            "compute_units": round(duration * steps_count * 0.5, 2),
            "memory_mb": 256,  # Estimated peak memory
            "duration_seconds": round(duration, 3),
            "tokens_processed": sum(
                result.get("tokens_used", 0) 
                for result in workflow_state["results"].values()
                if isinstance(result, dict)
            ),
            "api_calls": steps_count,
            "workflow_steps": steps_count
        }
    
    def get_workflow_status(self, task_id):
        """Get current status of a workflow"""
        return self.active_workflows.get(task_id)
    
    def get_active_workflow_count(self):
        """Get count of active workflows"""
        return len([w for w in self.active_workflows.values() if w["status"] == "running"])
```

#### `agent-container/agent/steps/data_processing.py`

```python
"""
Data processing step for workflows
"""

import logging
import time

logger = logging.getLogger(__name__)

class DataProcessingStep:
    """Handles data processing in workflows"""
    
    def __init__(self, formation_client):
        self.formation_client = formation_client
    
    def execute(self, parameters, previous_results):
        """Execute data processing step"""
        start_time = time.time()
        
        data = parameters.get("data", "")
        processing_type = parameters.get("processing_type", "standard")
        
        logger.info(f"Processing {len(data)} characters with {processing_type} method")
        
        # Simulate data processing
        processed_data = self._process_data(data, processing_type)
        
        processing_time = time.time() - start_time
        
        return {
            "status": "completed",
            "processed_data": processed_data,
            "processing_method": processing_type,
            "original_length": len(data),
            "processed_length": len(processed_data),
            "processing_time": processing_time,
            "summary": f"Processed {len(data)} characters using {processing_type} method"
        }
    
    def _process_data(self, data, processing_type):
        """Process the data based on type"""
        if processing_type == "standard":
            # Standard processing: clean and normalize
            processed = data.strip().lower()
            # Remove extra whitespace
            processed = ' '.join(processed.split())
            return processed
        
        elif processing_type == "advanced":
            # Advanced processing with LLM
            try:
                response = self.formation_client.call_model(
                    model_id="gpt-4",
                    messages=[{
                        "role": "user", 
                        "content": f"Clean and normalize this text data:\n\n{data}"
                    }],
                    max_tokens=len(data) + 100,
                    temperature=0.1
                )
                return response["content"]
            except Exception as e:
                logger.warning(f"Advanced processing failed, falling back to standard: {e}")
                return self._process_data(data, "standard")
        
        else:
            return data  # No processing
```

#### `agent-container/agent/steps/analysis.py`

```python
"""
Analysis step for workflows
"""

import logging
import time

logger = logging.getLogger(__name__)

class AnalysisStep:
    """Handles analysis in workflows"""
    
    def __init__(self, formation_client):
        self.formation_client = formation_client
    
    def execute(self, parameters, previous_results):
        """Execute analysis step"""
        start_time = time.time()
        
        # Get processed data from previous step
        data_result = previous_results.get("data_processing", {})
        processed_data = data_result.get("processed_data", parameters.get("data", ""))
        
        analysis_type = parameters.get("analysis_type", "comprehensive")
        
        logger.info(f"Analyzing data with {analysis_type} analysis")
        
        # Perform analysis
        analysis_result = self._analyze_data(processed_data, analysis_type)
        
        analysis_time = time.time() - start_time
        
        return {
            "status": "completed",
            "analysis_result": analysis_result,
            "analysis_type": analysis_type,
            "data_analyzed": len(processed_data),
            "analysis_time": analysis_time,
            "tokens_used": analysis_result.get("tokens_used", 0),
            "summary": f"Completed {analysis_type} analysis of {len(processed_data)} characters"
        }
    
    def _analyze_data(self, data, analysis_type):
        """Analyze the data"""
        if analysis_type == "basic":
            # Basic statistical analysis
            word_count = len(data.split())
            char_count = len(data)
            
            return {
                "type": "basic",
                "word_count": word_count,
                "character_count": char_count,
                "average_word_length": round(char_count / max(word_count, 1), 2),
                "insights": f"Text contains {word_count} words and {char_count} characters"
            }
        
        elif analysis_type == "comprehensive":
            # LLM-powered comprehensive analysis
            try:
                response = self.formation_client.call_model(
                    model_id="gpt-4",
                    messages=[{
                        "role": "user",
                        "content": f"""Perform a comprehensive analysis of this text data. Include:
1. Key themes and topics
2. Sentiment analysis
3. Writing style assessment
4. Main insights

Text to analyze:
{data}

Provide a structured analysis:"""
                    }],
                    max_tokens=1000,
                    temperature=0.3
                )
                
                return {
                    "type": "comprehensive",
                    "llm_analysis": response["content"],
                    "tokens_used": response.get("tokens_used", 0),
                    "model_used": "gpt-4",
                    "confidence": "high"
                }
                
            except Exception as e:
                logger.warning(f"Comprehensive analysis failed, falling back to basic: {e}")
                return self._analyze_data(data, "basic")
        
        else:
            return {"type": "none", "message": "No analysis performed"}
```

#### `agent-container/agent/steps/reporting.py`

```python
"""
Reporting step for workflows
"""

import logging
import time
import json

logger = logging.getLogger(__name__)

class ReportingStep:
    """Handles report generation in workflows"""
    
    def __init__(self, formation_client):
        self.formation_client = formation_client
    
    def execute(self, parameters, previous_results):
        """Execute reporting step"""
        start_time = time.time()
        
        output_format = parameters.get("output_format", "report")
        
        logger.info(f"Generating {output_format} format report")
        
        # Generate report
        report = self._generate_report(previous_results, output_format)
        
        reporting_time = time.time() - start_time
        
        return {
            "status": "completed",
            "final_report": report,
            "output_format": output_format,
            "reporting_time": reporting_time,
            "report_length": len(report),
            "summary": f"Generated {output_format} report ({len(report)} characters)"
        }
    
    def _generate_report(self, previous_results, output_format):
        """Generate the final report"""
        if output_format == "json":
            return json.dumps(previous_results, indent=2)
        
        elif output_format == "summary":
            summaries = []
            for step, result in previous_results.items():
                if isinstance(result, dict) and "summary" in result:
                    summaries.append(f"- {step}: {result['summary']}")
            
            return "Workflow Summary:\n" + "\n".join(summaries)
        
        elif output_format == "report":
            # Generate comprehensive report using LLM
            try:
                context = json.dumps(previous_results, indent=2)
                
                response = self.formation_client.call_model(
                    model_id="gpt-4",
                    messages=[{
                        "role": "user",
                        "content": f"""Generate a comprehensive report based on this workflow execution data:

{context}

Create a well-structured report that includes:
1. Executive Summary
2. Data Processing Results
3. Analysis Findings
4. Conclusions and Recommendations

Format as a professional report:"""
                    }],
                    max_tokens=1500,
                    temperature=0.2
                )
                
                return response["content"]
                
            except Exception as e:
                logger.warning(f"LLM report generation failed, using summary format: {e}")
                return self._generate_report(previous_results, "summary")
        
        else:
            return str(previous_results)
```

### Container Configuration

#### `agent-container/requirements.txt`

```
flask==2.3.3
requests==2.31.0
psutil==5.9.5
```

#### `docker-compose.yml`

```yaml
version: '3.8'

services:
  workflow-agent:
    build: ./agent-container
    container_name: formation-workflow-agent
    ports:
      - "8080:8080"
    environment:
      - FORMATION_API_URL=http://formation-api:3004
      - LOG_LEVEL=info
    restart: unless-stopped
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:8080/health"]
      interval: 30s
      timeout: 10s
      retries: 3
      start_period: 40s
```

### Deployment Configuration

#### `Formfile`

```
NAME workflow-agent
DESCRIPTION "Multi-step workflow processing agent"

# System Resources (higher for complex workflows)
VCPU 2
MEM 2048
DISK 20

# User Configuration
USER username:agent passwd:securepass sudo:true ssh_authorized_keys:"ssh-rsa AAAAB3NzaC1yc2E..."

# Base System
FROM ubuntu:22.04

# Install Docker
RUN apt-get update && apt-get install -y docker.io curl
RUN systemctl enable docker

# Copy agent container and configuration
COPY ./agent-container /app/agent-container
COPY ./docker-compose.yml /app/docker-compose.yml

# Set working directory
WORKDIR /app

# Start Docker service and run agent container
ENTRYPOINT ["sh", "-c", "service docker start && docker-compose up -d"]
```

### Testing the Workflow Agent

```bash
# Test comprehensive workflow
curl -X POST http://localhost:8080/run_task \
  -H "Content-Type: application/json" \
  -d '{
    "task_id": "workflow_test_1",
    "task_type": "data_analysis_workflow",
    "parameters": {
      "data": "This is sample data for analysis. It contains multiple sentences and various topics that can be analyzed for insights and patterns.",
      "analysis_type": "comprehensive",
      "output_format": "report"
    }
  }'

# Check workflow status
curl http://localhost:8080/workflow_status/workflow_test_1

# Test health with workflow info
curl http://localhost:8080/health
```

---

## Deployment Instructions

### Quick Deployment

For any of the examples above:

1. **Clone the example**:
   ```bash
   mkdir my-formation-agent
   cd my-formation-agent
   # Copy the example files
   ```

2. **Build and deploy**:
   ```bash
   # Using form-cli
   form pack build
   form pack ship
   
   # Or using the wizard
   form pack wizard
   ```

3. **Test your agent**:
   ```bash
   # Get instance IPs
   form manage get-ips <build_id>
   
   # Test the agent
   curl http://<formnet_ip>:8080/health
   ```

### Customization Tips

- **Modify the handlers** to implement your specific business logic
- **Add new task types** by extending the handler classes
- **Integrate additional services** by adding them to docker-compose.yml
- **Adjust resource requirements** in the Formfile based on your needs
- **Add monitoring and logging** for production deployments

### Best Practices

- **Start with the Echo Agent** to understand Formation's architecture
- **Use the LLM Agent** as a template for AI-powered applications
- **Build on the Workflow Agent** for complex, multi-step processes
- **Test thoroughly** before deploying to production
- **Monitor resource usage** and optimize accordingly

---

## Next Steps

1. **Choose an example** that matches your use case
2. **Customize the code** for your specific requirements
3. **Test locally** using Docker Compose
4. **Deploy to Formation** using the provided Formfiles
5. **Monitor and optimize** your agent's performance

For more advanced topics, see:
- **[Building Agents](building-agents.md)**: Technical architecture details
- **[Deployment Guide](deployment.md)**: Advanced deployment scenarios
- **[Monetization Guide](../monetization/)**: Pricing and billing strategies

---

**Ready to build?** Start with the Echo Agent and work your way up to more complex examples! 🚀 