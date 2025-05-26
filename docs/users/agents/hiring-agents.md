# Hiring and Using AI Agents

This comprehensive guide covers the complete agent hiring process, from selection to task execution, including advanced features and troubleshooting.

## Agent Hiring Process

### 1. Agent Discovery and Selection

#### Browse Available Agents

```bash
# List all public agents
curl -X GET "https://formation.ai/v1/agents" \
  -H "X-Formation-Address: 0x1234567890abcdef..." \
  -H "X-Formation-Signature: 0xabcdef..." \
  -H "X-Formation-Message: Formation authentication request"
```

#### Filter by Capabilities

```bash
# Find agents with specific capabilities
curl -X GET "https://formation.ai/v1/agents?capabilities=web_search,data_analysis" \
  -H "X-Formation-Address: 0x1234567890abcdef..." \
  -H "X-Formation-Signature: 0xabcdef..." \
  -H "X-Formation-Message: Formation authentication request"

# Filter by agent type
curl -X GET "https://formation.ai/v1/agents?type=CodeGenerator" \
  -H "X-Formation-Address: 0x1234567890abcdef..." \
  -H "X-Formation-Signature: 0xabcdef..." \
  -H "X-Formation-Message: Formation authentication request"

# Filter by price range (credits per request)
curl -X GET "https://formation.ai/v1/agents?max_price=100" \
  -H "X-Formation-Address: 0x1234567890abcdef..." \
  -H "X-Formation-Signature: 0xabcdef..." \
  -H "X-Formation-Message: Formation authentication request"
```

#### Evaluate Agent Details

```bash
# Get detailed information about a specific agent
curl -X GET "https://formation.ai/v1/agents/text-processor-v1" \
  -H "X-Formation-Address: 0x1234567890abcdef..." \
  -H "X-Formation-Signature: 0xabcdef..." \
  -H "X-Formation-Message: Formation authentication request"
```

**Response includes:**
```json
{
  "success": true,
  "agent": {
    "agent_id": "text-processor-v1",
    "name": "Advanced Text Processing Agent",
    "description": "Handles text generation, summarization, and translation tasks",
    "documentation": "# Text Processing Agent\n\nThis agent specializes in natural language processing tasks...",
    "version": "1.2.0",
    "agent_type": "Assistant",
    "framework": "LangChain",
    "runtime": "Python",
    "capabilities": [
      "text_generation",
      "summarization",
      "translation",
      "sentiment_analysis"
    ],
    "tools": [
      {
        "id": "web_search",
        "name": "Web Search",
        "description": "Search the web for information",
        "auth_required": false
      },
      {
        "id": "language_detector",
        "name": "Language Detection",
        "description": "Detect the language of input text",
        "auth_required": false
      }
    ],
    "supported_task_types": [
      {
        "type": "text_generation",
        "description": "Generate text based on prompts",
        "parameters": {
          "prompt": {"type": "string", "required": true},
          "max_tokens": {"type": "integer", "default": 1000, "min": 1, "max": 4000},
          "temperature": {"type": "number", "default": 0.7, "min": 0.0, "max": 2.0},
          "style": {"type": "string", "enum": ["formal", "casual", "technical"], "default": "casual"}
        }
      },
      {
        "type": "summarization",
        "description": "Summarize long text content",
        "parameters": {
          "text": {"type": "string", "required": true},
          "max_length": {"type": "integer", "default": 500, "min": 50, "max": 2000},
          "style": {"type": "string", "enum": ["bullet_points", "paragraph"], "default": "paragraph"}
        }
      }
    ],
    "pricing": {
      "model": "per_request",
      "base_cost": 50,
      "complexity_tiers": {
        "simple": 25,
        "standard": 50,
        "complex": 100
      },
      "currency": "credits"
    },
    "resource_requirements": {
      "min_vcpus": 1,
      "recommended_vcpus": 2,
      "min_memory_mb": 512,
      "recommended_memory_mb": 1024,
      "requires_gpu": false
    },
    "average_rating": 4.8,
    "deployment_count": 1250,
    "usage_count": 15000,
    "is_featured": true,
    "is_private": false,
    "owner_id": "0x9876543210fedcba...",
    "created_at": 1640995200,
    "updated_at": 1640995800
  }
}
```

### 2. Pre-Hiring Checks

#### Check Account Eligibility

```python
import requests
import json

def check_hiring_eligibility(agent_id, user_address, headers):
    """
    Check if you can hire a specific agent
    """
    # Get account information
    account_response = requests.get(
        f"https://formation.ai/v1/account/{user_address}/balance",
        headers=headers
    )
    
    if account_response.status_code != 200:
        return {"eligible": False, "reason": "Account not found"}
    
    account_data = account_response.json()
    
    # Get agent information
    agent_response = requests.get(
        f"https://formation.ai/v1/agents/{agent_id}",
        headers=headers
    )
    
    if agent_response.status_code != 200:
        return {"eligible": False, "reason": "Agent not found"}
    
    agent_data = agent_response.json()["agent"]
    
    # Check subscription limits
    subscription = account_data.get("subscription", {})
    hired_agents = len(account_data.get("hired_agents", []))
    
    # Subscription tier limits
    tier_limits = {
        "Free": {"max_agents": 2, "included": 1},
        "Pro": {"max_agents": 5, "included": 3},
        "Pro Plus": {"max_agents": 10, "included": 5},
        "Power": {"max_agents": 20, "included": 10},
        "Power Plus": {"max_agents": 50, "included": 25}
    }
    
    current_tier = subscription.get("tier", "Free")
    limits = tier_limits.get(current_tier, tier_limits["Free"])
    
    if hired_agents >= limits["max_agents"]:
        return {
            "eligible": False, 
            "reason": f"Maximum agents reached for {current_tier} tier ({limits['max_agents']})"
        }
    
    # Check if additional agent cost is required
    if hired_agents >= limits["included"]:
        additional_cost = 10 - (2 * (["Free", "Pro", "Pro Plus", "Power", "Power Plus"].index(current_tier)))
        available_credits = account_data.get("available_credits", 0)
        
        if available_credits < additional_cost:
            return {
                "eligible": False,
                "reason": f"Insufficient credits for additional agent. Required: {additional_cost}, Available: {available_credits}"
            }
    
    # Check if agent is premium and user has access
    is_premium = "premium" in agent_data.get("tags", []) or agent_data.get("price_per_request", 0) > 100
    if is_premium and current_tier in ["Free"]:
        return {
            "eligible": False,
            "reason": "Premium agents require Pro tier or higher"
        }
    
    return {
        "eligible": True,
        "additional_cost": additional_cost if hired_agents >= limits["included"] else 0,
        "remaining_slots": limits["max_agents"] - hired_agents
    }

# Example usage
headers = {
    "X-Formation-Address": "0x1234567890abcdef...",
    "X-Formation-Signature": "0xabcdef...",
    "X-Formation-Message": "Formation authentication request"
}

eligibility = check_hiring_eligibility("text-processor-v1", "0x1234567890abcdef...", headers)
print(json.dumps(eligibility, indent=2))
```

### 3. Hiring an Agent

#### Basic Agent Hiring

```bash
curl -X POST "https://formation.ai/v1/agents/text-processor-v1/hire" \
  -H "X-Formation-Address: 0x1234567890abcdef..." \
  -H "X-Formation-Signature: 0xabcdef..." \
  -H "X-Formation-Message: Formation authentication request" \
  -H "Content-Type: application/json" \
  -d '{
    "metadata": {
      "purpose": "Content creation for marketing campaigns",
      "expected_usage": "daily",
      "budget_limit": 1000
    }
  }'
```

#### Advanced Hiring with Configuration

```bash
curl -X POST "https://formation.ai/v1/agents/data-analyst-v2/hire" \
  -H "X-Formation-Address: 0x1234567890abcdef..." \
  -H "X-Formation-Signature: 0xabcdef..." \
  -H "X-Formation-Message: Formation authentication request" \
  -H "Content-Type: application/json" \
  -d '{
    "metadata": {
      "purpose": "Business intelligence and reporting",
      "expected_usage": "weekly",
      "budget_limit": 2000,
      "priority": "high"
    },
    "configuration": {
      "default_output_format": "executive_summary",
      "data_retention_days": 30,
      "notification_webhook": "https://myapp.com/webhooks/agent-updates"
    }
  }'
```

**Successful Hiring Response:**
```json
{
  "success": true,
  "message": "Successfully hired agent text-processor-v1",
  "agent": {
    "id": "text-processor-v1",
    "name": "Advanced Text Processing Agent",
    "description": "Handles text generation, summarization, and translation tasks",
    "version": "1.2.0"
  },
  "transaction": {
    "cost_credits": 0,
    "reason": "Within subscription allowance"
  },
  "account_status": {
    "credits_remaining": 1000,
    "hired_agent_count": 2,
    "subscription_agents_used": 2,
    "subscription_agents_remaining": 1
  }
}
```

---

## Task Submission

### 1. Basic Task Submission

#### Simple Text Generation Task

```bash
curl -X POST "https://formation.ai/v1/agents/text-processor-v1/run_task" \
  -H "X-Formation-Address: 0x1234567890abcdef..." \
  -H "X-Formation-Signature: 0xabcdef..." \
  -H "X-Formation-Message: Formation authentication request" \
  -H "Content-Type: application/json" \
  -d '{
    "task": "Write a professional email introducing our new AI-powered analytics platform",
    "parameters": {
      "tone": "professional",
      "length": "medium",
      "include_call_to_action": true,
      "target_audience": "enterprise_customers"
    }
  }'
```

#### Complex Multi-Parameter Task

```bash
curl -X POST "https://formation.ai/v1/agents/research-agent-v3/run_task" \
  -H "X-Formation-Address: 0x1234567890abcdef..." \
  -H "X-Formation-Signature: 0xabcdef..." \
  -H "X-Formation-Message: Formation authentication request" \
  -H "Content-Type: application/json" \
  -d '{
    "task": "Research and analyze the competitive landscape for AI-powered customer service solutions",
    "parameters": {
      "research_depth": "comprehensive",
      "focus_areas": ["pricing", "features", "market_share", "customer_reviews"],
      "competitors": ["Zendesk", "Intercom", "Freshworks", "ServiceNow"],
      "output_format": "structured_report",
      "include_charts": true,
      "data_sources": ["web_search", "industry_reports", "social_media"],
      "time_frame": "last_12_months"
    },
    "timeout_seconds": 600,
    "priority": "high"
  }'
```

### 2. Advanced Task Features

#### Streaming Responses

```python
import requests
import json
import sseclient

def stream_agent_task(agent_id, task_data, headers):
    """
    Submit a task with streaming response
    """
    # Add streaming flag to task data
    task_data["streaming"] = True
    
    response = requests.post(
        f"https://formation.ai/v1/agents/{agent_id}/run_task",
        headers={**headers, "Accept": "text/event-stream"},
        json=task_data,
        stream=True
    )
    
    if response.status_code == 200:
        client = sseclient.SSEClient(response)
        
        for event in client.events():
            if event.data:
                try:
                    chunk_data = json.loads(event.data)
                    
                    if chunk_data.get("is_final"):
                        print(f"Final chunk: {chunk_data}")
                        break
                    else:
                        print(f"Chunk: {chunk_data.get('content', '')}")
                        
                except json.JSONDecodeError:
                    print(f"Raw data: {event.data}")
    else:
        print(f"Error: {response.status_code} - {response.text}")

# Example usage
task_data = {
    "task": "Write a detailed analysis of renewable energy trends",
    "parameters": {
        "length": "long",
        "include_statistics": True,
        "format": "markdown"
    }
}

stream_agent_task("research-agent-v3", task_data, headers)
```

#### Task with File Uploads

```python
import requests

def submit_task_with_files(agent_id, task_data, files, headers):
    """
    Submit a task with file attachments
    """
    # Prepare multipart form data
    form_data = {
        'task_data': (None, json.dumps(task_data), 'application/json')
    }
    
    # Add files
    for i, file_path in enumerate(files):
        with open(file_path, 'rb') as f:
            form_data[f'file_{i}'] = (file_path, f.read(), 'application/octet-stream')
    
    response = requests.post(
        f"https://formation.ai/v1/agents/{agent_id}/run_task",
        headers={k: v for k, v in headers.items() if k != 'Content-Type'},  # Remove Content-Type for multipart
        files=form_data
    )
    
    return response.json()

# Example usage
task_data = {
    "task": "Analyze the data in the uploaded spreadsheet and generate insights",
    "parameters": {
        "analysis_type": "trend_analysis",
        "output_format": "executive_summary"
    }
}

files = ["sales_data.csv", "customer_feedback.xlsx"]
result = submit_task_with_files("data-analyst-v2", task_data, files, headers)
print(json.dumps(result, indent=2))
```

---

## Response Formats

### 1. Non-Streaming Response

#### Successful Task Completion

```json
{
  "task_id": "task_abc123def456",
  "agent_id": "text-processor-v1",
  "status": "completed",
  "completion": "Subject: Introducing Our Revolutionary AI-Powered Analytics Platform\n\nDear [Customer Name],\n\nI hope this email finds you well. I'm excited to share some groundbreaking news that will transform how you approach data analytics...",
  "result": {
    "output": "Subject: Introducing Our Revolutionary AI-Powered Analytics Platform...",
    "metadata": {
      "word_count": 245,
      "reading_time_minutes": 1.2,
      "tone_analysis": "professional",
      "sentiment_score": 0.8,
      "key_topics": ["AI", "analytics", "platform", "innovation"]
    }
  },
  "usage": {
    "prompt_tokens": 85,
    "completion_tokens": 245,
    "total_tokens": 330,
    "duration_ms": 3200,
    "billable_duration_ms": 3200,
    "cost_credits": 4
  },
  "timestamp": 1640995200,
  "duration_ms": 3200
}
```

#### Task with Structured Output

```json
{
  "task_id": "task_xyz789abc123",
  "agent_id": "research-agent-v3",
  "status": "completed",
  "completion": null,
  "result": {
    "report": {
      "title": "Competitive Landscape Analysis: AI-Powered Customer Service Solutions",
      "executive_summary": "The AI-powered customer service market is experiencing rapid growth...",
      "competitors": [
        {
          "name": "Zendesk",
          "market_share": "23%",
          "pricing": "$19-$199/agent/month",
          "key_features": ["AI Answer Bot", "Predictive Analytics", "Omnichannel Support"],
          "strengths": ["Market leader", "Extensive integrations"],
          "weaknesses": ["Higher pricing", "Complex setup"]
        }
      ],
      "market_trends": [
        "Increased adoption of conversational AI",
        "Focus on proactive customer engagement",
        "Integration with voice assistants"
      ],
      "recommendations": [
        "Focus on ease of implementation",
        "Competitive pricing strategy",
        "Emphasize AI accuracy and learning capabilities"
      ]
    },
    "charts": [
      {
        "type": "market_share_pie",
        "url": "https://formation.ai/generated/charts/market_share_abc123.png",
        "description": "Market share distribution among top competitors"
      }
    ],
    "sources": [
      "https://www.gartner.com/en/documents/4008681",
      "https://www.forrester.com/report/customer-service-ai-2023",
      "https://techcrunch.com/2023/ai-customer-service-trends"
    ]
  },
  "usage": {
    "prompt_tokens": 450,
    "completion_tokens": 1200,
    "total_tokens": 1650,
    "duration_ms": 45000,
    "billable_duration_ms": 45000,
    "cost_credits": 25,
    "external_api_calls": 15,
    "web_searches": 8
  },
  "timestamp": 1640995800,
  "duration_ms": 45000
}
```

### 2. Streaming Response Format

#### Stream Chunk Structure

```json
{
  "task_id": "task_stream123",
  "agent_id": "text-processor-v1",
  "chunk_id": "chunk-001",
  "content": "Subject: Introducing Our Revolutionary",
  "is_final": false,
  "error": null,
  "metadata": {
    "chunk_index": 1,
    "estimated_completion": 0.15
  },
  "timestamp": 1640995200100
}
```

#### Final Stream Chunk

```json
{
  "task_id": "task_stream123",
  "agent_id": "text-processor-v1",
  "chunk_id": "chunk-final",
  "content": "",
  "is_final": true,
  "error": null,
  "metadata": {
    "total_chunks": 15,
    "completion_reason": "finished"
  },
  "usage": {
    "prompt_tokens": 85,
    "completion_tokens": 245,
    "total_tokens": 330,
    "duration_ms": 3200,
    "cost_credits": 4
  },
  "timestamp": 1640995203200
}
```

### 3. Error Response Formats

#### Task Execution Error

```json
{
  "task_id": "task_error123",
  "agent_id": "text-processor-v1",
  "status": "failed",
  "completion": null,
  "error": {
    "code": "INVALID_PARAMETERS",
    "message": "The 'temperature' parameter must be between 0.0 and 2.0",
    "details": {
      "field": "parameters.temperature",
      "provided_value": 3.5,
      "valid_range": "0.0 - 2.0"
    }
  },
  "partial_usage": {
    "duration_ms": 150,
    "cost_credits": 0
  },
  "timestamp": 1640995200
}
```

#### Agent Unavailable Error

```json
{
  "task_id": "task_unavailable123",
  "agent_id": "text-processor-v1",
  "status": "failed",
  "error": {
    "code": "AGENT_UNAVAILABLE",
    "message": "No available instances for agent text-processor-v1. Please try again later.",
    "details": {
      "retry_after_seconds": 30,
      "estimated_wait_time": "1-2 minutes"
    }
  },
  "timestamp": 1640995200
}
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
    "details": {
      "provided_address": "0x1234567890abcdef...",
      "signature_valid": false
    }
  }
}
```

**Solution:**
```python
# Verify signature generation
import hashlib
from eth_account import Account

def generate_valid_signature(private_key, message):
    account = Account.from_key(private_key)
    message_hash = hashlib.sha256(message.encode()).hexdigest()
    signature = account.signHash(message_hash)
    
    return {
        "address": account.address,
        "signature": signature.signature.hex(),
        "message": message
    }
```

#### Insufficient Credits

```json
{
  "error": {
    "code": "INSUFFICIENT_CREDITS",
    "message": "Insufficient credits for task execution",
    "details": {
      "required_credits": 50,
      "available_credits": 25,
      "shortfall": 25
    }
  }
}
```

**Solution:**
```python
def handle_insufficient_credits(error_details):
    shortfall = error_details["shortfall"]
    
    print(f"Need {shortfall} more credits")
    print("Options:")
    print("1. Add credits to your account")
    print("2. Upgrade your subscription")
    print("3. Use a less expensive agent")
    
    # Suggest credit purchase
    suggested_purchase = max(shortfall * 2, 100)  # Buy at least 2x needed
    print(f"Suggested credit purchase: {suggested_purchase} credits")
```

#### Agent Not Hired

```json
{
  "error": {
    "code": "AGENT_NOT_HIRED",
    "message": "You must hire this agent before using it",
    "details": {
      "agent_id": "text-processor-v1",
      "hire_endpoint": "/v1/agents/text-processor-v1/hire"
    }
  }
}
```

### 2. Retry Logic Implementation

```python
import time
import random
from typing import Dict, Any, Optional

class AgentTaskRetryHandler:
    def __init__(self, max_retries: int = 3, base_delay: float = 1.0):
        self.max_retries = max_retries
        self.base_delay = base_delay
        
        # Retryable error codes
        self.retryable_errors = {
            "AGENT_UNAVAILABLE",
            "TIMEOUT",
            "RATE_LIMITED",
            "TEMPORARY_ERROR",
            "SERVICE_UNAVAILABLE"
        }
    
    def should_retry(self, error_code: str, attempt: int) -> bool:
        """Determine if an error should be retried"""
        return (
            attempt < self.max_retries and 
            error_code in self.retryable_errors
        )
    
    def calculate_delay(self, attempt: int) -> float:
        """Calculate exponential backoff delay with jitter"""
        delay = self.base_delay * (2 ** attempt)
        jitter = random.uniform(0.1, 0.3) * delay
        return delay + jitter
    
    def execute_with_retry(self, task_func, *args, **kwargs) -> Dict[str, Any]:
        """Execute a task with retry logic"""
        last_error = None
        
        for attempt in range(self.max_retries + 1):
            try:
                result = task_func(*args, **kwargs)
                
                # Check if result indicates an error
                if isinstance(result, dict) and "error" in result:
                    error_code = result["error"].get("code", "UNKNOWN")
                    
                    if self.should_retry(error_code, attempt):
                        delay = self.calculate_delay(attempt)
                        print(f"Attempt {attempt + 1} failed with {error_code}. Retrying in {delay:.2f}s...")
                        time.sleep(delay)
                        last_error = result
                        continue
                    else:
                        return result  # Non-retryable error or max retries reached
                
                return result  # Success
                
            except Exception as e:
                if attempt < self.max_retries:
                    delay = self.calculate_delay(attempt)
                    print(f"Attempt {attempt + 1} failed with exception. Retrying in {delay:.2f}s...")
                    time.sleep(delay)
                    last_error = {"error": {"code": "EXCEPTION", "message": str(e)}}
                else:
                    raise e
        
        return last_error or {"error": {"code": "MAX_RETRIES_EXCEEDED", "message": "All retry attempts failed"}}

# Usage example
def submit_agent_task(agent_id: str, task_data: Dict[str, Any], headers: Dict[str, str]) -> Dict[str, Any]:
    response = requests.post(
        f"https://formation.ai/v1/agents/{agent_id}/run_task",
        headers=headers,
        json=task_data,
        timeout=30
    )
    return response.json()

# Execute with retry
retry_handler = AgentTaskRetryHandler(max_retries=3, base_delay=2.0)

result = retry_handler.execute_with_retry(
    submit_agent_task,
    "text-processor-v1",
    {"task": "Generate a product description", "parameters": {"length": "medium"}},
    headers
)

print(json.dumps(result, indent=2))
```

### 3. Error Recovery Strategies

#### Graceful Degradation

```python
def execute_task_with_fallback(primary_agent: str, fallback_agents: list, task_data: dict, headers: dict):
    """
    Try primary agent, fall back to alternatives if needed
    """
    agents_to_try = [primary_agent] + fallback_agents
    
    for agent_id in agents_to_try:
        try:
            result = submit_agent_task(agent_id, task_data, headers)
            
            if "error" not in result:
                return {
                    "success": True,
                    "agent_used": agent_id,
                    "result": result
                }
            
            error_code = result["error"].get("code")
            
            # Don't try fallbacks for certain errors
            if error_code in ["AUTHENTICATION_FAILED", "AGENT_NOT_HIRED", "INVALID_PARAMETERS"]:
                return {
                    "success": False,
                    "error": result["error"],
                    "agent_attempted": agent_id
                }
                
        except Exception as e:
            print(f"Failed to use agent {agent_id}: {e}")
            continue
    
    return {
        "success": False,
        "error": {"code": "ALL_AGENTS_FAILED", "message": "All agents failed to process the task"},
        "agents_attempted": agents_to_try
    }

# Example usage
result = execute_task_with_fallback(
    primary_agent="premium-text-processor-v2",
    fallback_agents=["text-processor-v1", "basic-text-agent"],
    task_data={"task": "Write a blog post about AI trends"},
    headers=headers
)
```

#### Partial Result Handling

```python
def handle_partial_results(task_result: dict) -> dict:
    """
    Extract useful information even from failed tasks
    """
    if task_result.get("status") == "failed":
        # Check if there's partial output
        partial_usage = task_result.get("partial_usage", {})
        
        if partial_usage.get("duration_ms", 0) > 1000:  # Task ran for a while
            return {
                "status": "partial_failure",
                "message": "Task failed but may have produced partial results",
                "suggestion": "Try breaking the task into smaller parts",
                "cost_incurred": partial_usage.get("cost_credits", 0)
            }
    
    return task_result
```

---

## Advanced Features

### 1. Batch Task Processing

```python
import asyncio
import aiohttp
from typing import List, Dict, Any

class BatchTaskProcessor:
    def __init__(self, max_concurrent: int = 5):
        self.max_concurrent = max_concurrent
        self.semaphore = asyncio.Semaphore(max_concurrent)
    
    async def submit_single_task(self, session: aiohttp.ClientSession, agent_id: str, task_data: dict, headers: dict) -> dict:
        """Submit a single task asynchronously"""
        async with self.semaphore:
            try:
                async with session.post(
                    f"https://formation.ai/v1/agents/{agent_id}/run_task",
                    headers=headers,
                    json=task_data
                ) as response:
                    result = await response.json()
                    return {
                        "task_id": task_data.get("task_id", "unknown"),
                        "agent_id": agent_id,
                        "success": response.status == 200,
                        "result": result
                    }
            except Exception as e:
                return {
                    "task_id": task_data.get("task_id", "unknown"),
                    "agent_id": agent_id,
                    "success": False,
                    "error": str(e)
                }
    
    async def process_batch(self, tasks: List[Dict[str, Any]], headers: dict) -> List[dict]:
        """Process multiple tasks concurrently"""
        async with aiohttp.ClientSession() as session:
            coroutines = [
                self.submit_single_task(
                    session,
                    task["agent_id"],
                    task["task_data"],
                    headers
                )
                for task in tasks
            ]
            
            results = await asyncio.gather(*coroutines, return_exceptions=True)
            return results

# Example usage
async def main():
    batch_processor = BatchTaskProcessor(max_concurrent=3)
    
    tasks = [
        {
            "agent_id": "text-processor-v1",
            "task_data": {
                "task": "Write a product description for a smartphone",
                "parameters": {"length": "short", "tone": "marketing"}
            }
        },
        {
            "agent_id": "text-processor-v1",
            "task_data": {
                "task": "Write a product description for a laptop",
                "parameters": {"length": "short", "tone": "marketing"}
            }
        },
        {
            "agent_id": "research-agent-v3",
            "task_data": {
                "task": "Research latest smartphone trends",
                "parameters": {"depth": "summary", "sources": ["web_search"]}
            }
        }
    ]
    
    results = await batch_processor.process_batch(tasks, headers)
    
    for result in results:
        print(f"Task {result['task_id']}: {'Success' if result['success'] else 'Failed'}")

# Run the batch processing
asyncio.run(main())
```

### 2. Task Progress Monitoring

```python
def monitor_long_running_task(task_id: str, agent_id: str, headers: dict, poll_interval: int = 5):
    """
    Monitor a long-running task's progress
    """
    start_time = time.time()
    
    while True:
        try:
            # Check task status
            response = requests.get(
                f"https://formation.ai/v1/agents/{agent_id}/tasks/{task_id}/status",
                headers=headers
            )
            
            if response.status_code == 200:
                status_data = response.json()
                status = status_data.get("status")
                
                print(f"Task {task_id} status: {status}")
                
                if status in ["completed", "failed", "cancelled"]:
                    return status_data
                
                # Show progress if available
                progress = status_data.get("progress", {})
                if progress:
                    percentage = progress.get("percentage", 0)
                    current_step = progress.get("current_step", "Processing")
                    print(f"Progress: {percentage}% - {current_step}")
                
            else:
                print(f"Failed to get status: {response.status_code}")
            
            # Wait before next poll
            time.sleep(poll_interval)
            
            # Timeout after 10 minutes
            if time.time() - start_time > 600:
                print("Monitoring timeout reached")
                break
                
        except KeyboardInterrupt:
            print("Monitoring interrupted by user")
            break
        except Exception as e:
            print(f"Error monitoring task: {e}")
            time.sleep(poll_interval)
```

---

## Best Practices

### 1. Task Design

- **Be Specific**: Provide clear, detailed instructions
- **Set Appropriate Timeouts**: Allow sufficient time for complex tasks
- **Use Structured Parameters**: Organize parameters logically
- **Provide Context**: Include relevant background information

### 2. Error Handling

- **Implement Retry Logic**: Handle transient failures gracefully
- **Use Fallback Agents**: Have backup options for critical tasks
- **Monitor Costs**: Track credit usage to avoid surprises
- **Log Everything**: Maintain detailed logs for debugging

### 3. Performance Optimization

- **Batch Similar Tasks**: Process multiple tasks concurrently
- **Cache Results**: Store frequently used outputs
- **Choose Appropriate Agents**: Match agent capabilities to task requirements
- **Monitor Response Times**: Track performance metrics

---

## Next Steps

- **[Explore Code Examples](examples.md)** - Working examples in multiple programming languages
- **[Try Model Inference](../models/inference-guide.md)** - Direct access to AI models
- **[Build Your Own Agent](../../developers/agents/building-agents.md)** - Create custom agents

---

**Ready to master agent hiring?** Start with simple tasks and gradually work up to more complex workflows! 🚀 