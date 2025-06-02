# Agent Usage Examples

This guide provides practical, working examples of using Formation AI agents in multiple programming languages and scenarios.

## Table of Contents

1. [cURL Examples](#curl-examples)
2. [Python Examples](#python-examples)
3. [JavaScript/Node.js Examples](#javascriptnodejs-examples)
4. [Real-World Use Cases](#real-world-use-cases)
5. [Integration Patterns](#integration-patterns)

---

## cURL Examples

### 1. Basic Agent Operations

#### List Available Agents

```bash
# List all public agents
curl -X GET "https://formation.ai/v1/agents" \
  -H "X-Formation-Address: 0x1234567890abcdef1234567890abcdef12345678" \
  -H "X-Formation-Signature: 0xabcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890ab" \
  -H "X-Formation-Message: Formation authentication request" \
  -H "Content-Type: application/json"
```

#### Get Agent Details

```bash
# Get detailed information about a specific agent
curl -X GET "https://formation.ai/v1/agents/text-processor-v1" \
  -H "X-Formation-Address: 0x1234567890abcdef1234567890abcdef12345678" \
  -H "X-Formation-Signature: 0xabcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890ab" \
  -H "X-Formation-Message: Formation authentication request" \
  -H "Content-Type: application/json"
```

#### Hire an Agent

```bash
# Hire a text processing agent
curl -X POST "https://formation.ai/v1/agents/text-processor-v1/hire" \
  -H "X-Formation-Address: 0x1234567890abcdef1234567890abcdef12345678" \
  -H "X-Formation-Signature: 0xabcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890ab" \
  -H "X-Formation-Message: Formation authentication request" \
  -H "Content-Type: application/json" \
  -d '{
    "metadata": {
      "purpose": "Content creation for marketing",
      "expected_usage": "daily"
    }
  }'
```

### 2. Task Submission Examples

#### Simple Text Generation

```bash
# Generate marketing copy
curl -X POST "https://formation.ai/v1/agents/text-processor-v1/run_task" \
  -H "X-Formation-Address: 0x1234567890abcdef1234567890abcdef12345678" \
  -H "X-Formation-Signature: 0xabcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890ab" \
  -H "X-Formation-Message: Formation authentication request" \
  -H "Content-Type: application/json" \
  -d '{
    "task": "Write a compelling product description for a new smartwatch",
    "parameters": {
      "tone": "enthusiastic",
      "length": "medium",
      "target_audience": "fitness_enthusiasts",
      "key_features": ["heart_rate_monitoring", "GPS", "waterproof", "7_day_battery"]
    }
  }'
```

#### Data Analysis Task

```bash
# Analyze business data
curl -X POST "https://formation.ai/v1/agents/data-analyst-v2/run_task" \
  -H "X-Formation-Address: 0x1234567890abcdef1234567890abcdef12345678" \
  -H "X-Formation-Signature: 0xabcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890ab" \
  -H "X-Formation-Message: Formation authentication request" \
  -H "Content-Type: application/json" \
  -d '{
    "task": "Analyze Q3 sales performance and identify trends",
    "parameters": {
      "data_source": "https://example.com/q3_sales.csv",
      "analysis_type": "comprehensive",
      "focus_areas": ["revenue_trends", "customer_segments", "product_performance"],
      "output_format": "executive_summary"
    },
    "timeout_seconds": 300
  }'
```

#### Code Generation Task

```bash
# Generate Python code
curl -X POST "https://formation.ai/v1/agents/code-generator-v3/run_task" \
  -H "X-Formation-Address: 0x1234567890abcdef1234567890abcdef12345678" \
  -H "X-Formation-Signature: 0xabcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890ab" \
  -H "X-Formation-Message: Formation authentication request" \
  -H "Content-Type: application/json" \
  -d '{
    "task": "Create a Python function to validate email addresses using regex",
    "parameters": {
      "language": "python",
      "include_tests": true,
      "include_documentation": true,
      "style": "pep8"
    }
  }'
```

---

## Python Examples

### 1. Formation Agent Client

```python
import hashlib
import requests
import json
import time
from eth_account import Account
from typing import Dict, Any, Optional

class FormationAgentClient:
    def __init__(self, private_key: str, base_url: str = "https://formation.ai"):
        self.base_url = base_url
        self.account = Account.from_key(private_key)
        self.session = requests.Session()
    
    def _generate_auth_headers(self, message: str = "Formation authentication request") -> Dict[str, str]:
        """Generate authentication headers for Formation API"""
        message_hash = hashlib.sha256(message.encode()).hexdigest()
        signature = self.account.signHash(message_hash)
        
        return {
            "X-Formation-Address": self.account.address,
            "X-Formation-Signature": signature.signature.hex(),
            "X-Formation-Message": message,
            "Content-Type": "application/json"
        }
    
    def list_agents(self, filters: Optional[Dict[str, Any]] = None) -> Dict[str, Any]:
        """List available agents with optional filters"""
        url = f"{self.base_url}/v1/agents"
        headers = self._generate_auth_headers()
        
        params = filters or {}
        response = self.session.get(url, headers=headers, params=params)
        response.raise_for_status()
        
        return response.json()
    
    def get_agent(self, agent_id: str) -> Dict[str, Any]:
        """Get detailed information about a specific agent"""
        url = f"{self.base_url}/v1/agents/{agent_id}"
        headers = self._generate_auth_headers()
        
        response = self.session.get(url, headers=headers)
        response.raise_for_status()
        
        return response.json()
    
    def hire_agent(self, agent_id: str, metadata: Optional[Dict[str, Any]] = None) -> Dict[str, Any]:
        """Hire an agent"""
        url = f"{self.base_url}/v1/agents/{agent_id}/hire"
        headers = self._generate_auth_headers()
        
        data = {"metadata": metadata or {}}
        response = self.session.post(url, headers=headers, json=data)
        response.raise_for_status()
        
        return response.json()
    
    def run_task(self, agent_id: str, task: str, parameters: Optional[Dict[str, Any]] = None, 
                 timeout_seconds: int = 300) -> Dict[str, Any]:
        """Submit a task to an agent"""
        url = f"{self.base_url}/v1/agents/{agent_id}/run_task"
        headers = self._generate_auth_headers()
        
        data = {
            "task": task,
            "parameters": parameters or {},
            "timeout_seconds": timeout_seconds
        }
        
        response = self.session.post(url, headers=headers, json=data)
        response.raise_for_status()
        
        return response.json()
    
    def get_account_balance(self) -> Dict[str, Any]:
        """Get current account balance and subscription info"""
        url = f"{self.base_url}/v1/account/{self.account.address}/balance"
        headers = self._generate_auth_headers()
        
        response = self.session.get(url, headers=headers)
        response.raise_for_status()
        
        return response.json()

# Example usage
if __name__ == "__main__":
    # Initialize client with your private key
    private_key = "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef12"
    client = FormationAgentClient(private_key)
    
    try:
        # Check account balance
        balance = client.get_account_balance()
        print(f"Available credits: {balance['account']['available_credits']}")
        
        # List available agents
        agents = client.list_agents({"type": "Assistant"})
        print(f"Found {agents['count']} assistant agents")
        
        # Hire a text processing agent
        hire_result = client.hire_agent("text-processor-v1", {
            "purpose": "Content creation",
            "expected_usage": "daily"
        })
        print(f"Hired agent: {hire_result['message']}")
        
        # Run a task
        task_result = client.run_task(
            "text-processor-v1",
            "Write a professional email announcing a new product launch",
            {
                "tone": "professional",
                "length": "medium",
                "product_name": "AI Analytics Platform",
                "target_audience": "enterprise_customers"
            }
        )
        
        print(f"Task completed: {task_result['status']}")
        print(f"Result: {task_result['completion'][:200]}...")
        print(f"Cost: {task_result['usage']['cost_credits']} credits")
        
    except requests.exceptions.RequestException as e:
        print(f"API Error: {e}")
    except Exception as e:
        print(f"Error: {e}")
```

### 2. Advanced Python Examples

#### Batch Processing with Async

```python
import asyncio
import aiohttp
import json
from typing import List, Dict, Any

class AsyncFormationClient:
    def __init__(self, private_key: str, base_url: str = "https://formation.ai"):
        self.base_url = base_url
        self.account = Account.from_key(private_key)
    
    def _generate_auth_headers(self) -> Dict[str, str]:
        """Generate authentication headers"""
        message = "Formation authentication request"
        message_hash = hashlib.sha256(message.encode()).hexdigest()
        signature = self.account.signHash(message_hash)
        
        return {
            "X-Formation-Address": self.account.address,
            "X-Formation-Signature": signature.signature.hex(),
            "X-Formation-Message": message,
            "Content-Type": "application/json"
        }
    
    async def run_task_async(self, session: aiohttp.ClientSession, agent_id: str, 
                           task: str, parameters: Dict[str, Any]) -> Dict[str, Any]:
        """Submit a task asynchronously"""
        url = f"{self.base_url}/v1/agents/{agent_id}/run_task"
        headers = self._generate_auth_headers()
        
        data = {
            "task": task,
            "parameters": parameters
        }
        
        async with session.post(url, headers=headers, json=data) as response:
            result = await response.json()
            return {
                "agent_id": agent_id,
                "task": task,
                "success": response.status == 200,
                "result": result
            }
    
    async def process_batch_tasks(self, tasks: List[Dict[str, Any]]) -> List[Dict[str, Any]]:
        """Process multiple tasks concurrently"""
        async with aiohttp.ClientSession() as session:
            coroutines = [
                self.run_task_async(
                    session,
                    task["agent_id"],
                    task["task"],
                    task["parameters"]
                )
                for task in tasks
            ]
            
            results = await asyncio.gather(*coroutines, return_exceptions=True)
            return results

# Example usage
async def main():
    private_key = "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef12"
    client = AsyncFormationClient(private_key)
    
    # Define batch tasks
    tasks = [
        {
            "agent_id": "text-processor-v1",
            "task": "Write a product description for a smartphone",
            "parameters": {"tone": "marketing", "length": "short"}
        },
        {
            "agent_id": "text-processor-v1",
            "task": "Write a product description for a laptop",
            "parameters": {"tone": "marketing", "length": "short"}
        },
        {
            "agent_id": "code-generator-v3",
            "task": "Create a Python function to calculate fibonacci numbers",
            "parameters": {"include_tests": True, "style": "pep8"}
        }
    ]
    
    # Process tasks concurrently
    results = await client.process_batch_tasks(tasks)
    
    for result in results:
        if result["success"]:
            print(f"✅ {result['agent_id']}: Task completed")
            print(f"   Cost: {result['result']['usage']['cost_credits']} credits")
        else:
            print(f"❌ {result['agent_id']}: Task failed")
            print(f"   Error: {result['result'].get('error', 'Unknown error')}")

# Run the async example
asyncio.run(main())
```

#### Streaming Response Handler

```python
import sseclient
import json

class StreamingTaskHandler:
    def __init__(self, client: FormationAgentClient):
        self.client = client
    
    def run_streaming_task(self, agent_id: str, task: str, parameters: Dict[str, Any]) -> None:
        """Run a task with streaming response"""
        url = f"{self.client.base_url}/v1/agents/{agent_id}/run_task"
        headers = self.client._generate_auth_headers()
        headers["Accept"] = "text/event-stream"
        
        data = {
            "task": task,
            "parameters": parameters,
            "streaming": True
        }
        
        response = self.client.session.post(url, headers=headers, json=data, stream=True)
        response.raise_for_status()
        
        client = sseclient.SSEClient(response)
        
        print(f"🚀 Starting streaming task for agent {agent_id}")
        print(f"📝 Task: {task}")
        print("📡 Streaming response:")
        print("-" * 50)
        
        for event in client.events():
            if event.data:
                try:
                    chunk_data = json.loads(event.data)
                    
                    if chunk_data.get("error"):
                        print(f"❌ Error: {chunk_data['error']}")
                        break
                    
                    content = chunk_data.get("content", "")
                    if content:
                        print(content, end="", flush=True)
                    
                    if chunk_data.get("is_final"):
                        print("\n" + "-" * 50)
                        usage = chunk_data.get("usage", {})
                        print(f"✅ Task completed!")
                        print(f"💰 Cost: {usage.get('cost_credits', 0)} credits")
                        print(f"⏱️  Duration: {usage.get('duration_ms', 0)}ms")
                        break
                        
                except json.JSONDecodeError:
                    print(f"Raw data: {event.data}")

# Example usage
client = FormationAgentClient("0x1234567890abcdef...")
streaming_handler = StreamingTaskHandler(client)

streaming_handler.run_streaming_task(
    "text-processor-v1",
    "Write a detailed blog post about the future of AI in healthcare",
    {
        "length": "long",
        "tone": "informative",
        "include_statistics": True,
        "target_audience": "healthcare_professionals"
    }
)
```

---

## JavaScript/Node.js Examples

### 1. Formation Agent SDK

```javascript
const crypto = require('crypto');
const { ethers } = require('ethers');
const axios = require('axios');

class FormationAgentClient {
    constructor(privateKey, baseUrl = 'https://formation.ai') {
        this.baseUrl = baseUrl;
        this.wallet = new ethers.Wallet(privateKey);
        this.axios = axios.create();
    }

    _generateAuthHeaders(message = 'Formation authentication request') {
        const messageHash = crypto.createHash('sha256').update(message).digest('hex');
        const signature = this.wallet.signMessage(messageHash);
        
        return {
            'X-Formation-Address': this.wallet.address,
            'X-Formation-Signature': signature,
            'X-Formation-Message': message,
            'Content-Type': 'application/json'
        };
    }

    async listAgents(filters = {}) {
        const url = `${this.baseUrl}/v1/agents`;
        const headers = this._generateAuthHeaders();
        
        try {
            const response = await this.axios.get(url, { headers, params: filters });
            return response.data;
        } catch (error) {
            throw new Error(`Failed to list agents: ${error.response?.data?.error || error.message}`);
        }
    }

    async getAgent(agentId) {
        const url = `${this.baseUrl}/v1/agents/${agentId}`;
        const headers = this._generateAuthHeaders();
        
        try {
            const response = await this.axios.get(url, { headers });
            return response.data;
        } catch (error) {
            throw new Error(`Failed to get agent: ${error.response?.data?.error || error.message}`);
        }
    }

    async hireAgent(agentId, metadata = {}) {
        const url = `${this.baseUrl}/v1/agents/${agentId}/hire`;
        const headers = this._generateAuthHeaders();
        
        try {
            const response = await this.axios.post(url, { metadata }, { headers });
            return response.data;
        } catch (error) {
            throw new Error(`Failed to hire agent: ${error.response?.data?.error || error.message}`);
        }
    }

    async runTask(agentId, task, parameters = {}, timeoutSeconds = 300) {
        const url = `${this.baseUrl}/v1/agents/${agentId}/run_task`;
        const headers = this._generateAuthHeaders();
        
        const data = {
            task,
            parameters,
            timeout_seconds: timeoutSeconds
        };
        
        try {
            const response = await this.axios.post(url, data, { headers });
            return response.data;
        } catch (error) {
            throw new Error(`Failed to run task: ${error.response?.data?.error || error.message}`);
        }
    }

    async getAccountBalance() {
        const url = `${this.baseUrl}/v1/account/${this.wallet.address}/balance`;
        const headers = this._generateAuthHeaders();
        
        try {
            const response = await this.axios.get(url, { headers });
            return response.data;
        } catch (error) {
            throw new Error(`Failed to get balance: ${error.response?.data?.error || error.message}`);
        }
    }
}

// Example usage
async function main() {
    const privateKey = '0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef12';
    const client = new FormationAgentClient(privateKey);
    
    try {
        // Check account balance
        const balance = await client.getAccountBalance();
        console.log(`Available credits: ${balance.account.available_credits}`);
        
        // List available agents
        const agents = await client.listAgents({ type: 'Assistant' });
        console.log(`Found ${agents.count} assistant agents`);
        
        // Hire a text processing agent
        const hireResult = await client.hireAgent('text-processor-v1', {
            purpose: 'Content creation',
            expected_usage: 'daily'
        });
        console.log(`Hired agent: ${hireResult.message}`);
        
        // Run a task
        const taskResult = await client.runTask(
            'text-processor-v1',
            'Write a compelling social media post about sustainable technology',
            {
                platform: 'linkedin',
                tone: 'professional',
                length: 'medium',
                include_hashtags: true
            }
        );
        
        console.log(`Task completed: ${taskResult.status}`);
        console.log(`Result: ${taskResult.completion.substring(0, 200)}...`);
        console.log(`Cost: ${taskResult.usage.cost_credits} credits`);
        
    } catch (error) {
        console.error('Error:', error.message);
    }
}

main();
```

### 2. React Integration Example

```jsx
import React, { useState, useEffect } from 'react';
import { FormationAgentClient } from './FormationAgentClient';

const AgentTaskInterface = () => {
    const [client, setClient] = useState(null);
    const [agents, setAgents] = useState([]);
    const [selectedAgent, setSelectedAgent] = useState('');
    const [task, setTask] = useState('');
    const [result, setResult] = useState(null);
    const [loading, setLoading] = useState(false);
    const [balance, setBalance] = useState(null);

    useEffect(() => {
        // Initialize client (in real app, get private key securely)
        const privateKey = process.env.REACT_APP_FORMATION_PRIVATE_KEY;
        if (privateKey) {
            const formationClient = new FormationAgentClient(privateKey);
            setClient(formationClient);
            loadInitialData(formationClient);
        }
    }, []);

    const loadInitialData = async (client) => {
        try {
            // Load account balance
            const balanceData = await client.getAccountBalance();
            setBalance(balanceData.account);

            // Load available agents
            const agentsData = await client.listAgents();
            setAgents(agentsData.agents);
        } catch (error) {
            console.error('Failed to load initial data:', error);
        }
    };

    const handleRunTask = async () => {
        if (!client || !selectedAgent || !task) return;

        setLoading(true);
        try {
            const result = await client.runTask(selectedAgent, task, {
                tone: 'professional',
                length: 'medium'
            });
            setResult(result);
            
            // Refresh balance
            const balanceData = await client.getAccountBalance();
            setBalance(balanceData.account);
        } catch (error) {
            setResult({ error: error.message });
        } finally {
            setLoading(false);
        }
    };

    return (
        <div className="agent-task-interface">
            <h2>Formation AI Agents</h2>
            
            {balance && (
                <div className="balance-info">
                    <p>Available Credits: {balance.available_credits}</p>
                    <p>Hired Agents: {balance.hired_agents?.length || 0}</p>
                </div>
            )}

            <div className="agent-selection">
                <label>Select Agent:</label>
                <select 
                    value={selectedAgent} 
                    onChange={(e) => setSelectedAgent(e.target.value)}
                >
                    <option value="">Choose an agent...</option>
                    {agents.map(agent => (
                        <option key={agent.agent_id} value={agent.agent_id}>
                            {agent.name} - {agent.price_per_request} credits
                        </option>
                    ))}
                </select>
            </div>

            <div className="task-input">
                <label>Task Description:</label>
                <textarea
                    value={task}
                    onChange={(e) => setTask(e.target.value)}
                    placeholder="Describe what you want the agent to do..."
                    rows={4}
                />
            </div>

            <button 
                onClick={handleRunTask} 
                disabled={loading || !selectedAgent || !task}
                className="run-task-btn"
            >
                {loading ? 'Processing...' : 'Run Task'}
            </button>

            {result && (
                <div className="task-result">
                    <h3>Result:</h3>
                    {result.error ? (
                        <div className="error">Error: {result.error}</div>
                    ) : (
                        <div>
                            <div className="result-content">
                                {result.completion}
                            </div>
                            <div className="result-meta">
                                <p>Status: {result.status}</p>
                                <p>Cost: {result.usage?.cost_credits} credits</p>
                                <p>Duration: {result.usage?.duration_ms}ms</p>
                            </div>
                        </div>
                    )}
                </div>
            )}
        </div>
    );
};

export default AgentTaskInterface;
```

### 3. Express.js API Integration

```javascript
const express = require('express');
const { FormationAgentClient } = require('./FormationAgentClient');

const app = express();
app.use(express.json());

// Initialize Formation client
const client = new FormationAgentClient(process.env.FORMATION_PRIVATE_KEY);

// Middleware to check authentication
const authenticateUser = (req, res, next) => {
    // Implement your user authentication logic here
    const userId = req.headers['x-user-id'];
    if (!userId) {
        return res.status(401).json({ error: 'Authentication required' });
    }
    req.userId = userId;
    next();
};

// Get available agents
app.get('/api/agents', authenticateUser, async (req, res) => {
    try {
        const agents = await client.listAgents(req.query);
        res.json(agents);
    } catch (error) {
        res.status(500).json({ error: error.message });
    }
});

// Hire an agent
app.post('/api/agents/:agentId/hire', authenticateUser, async (req, res) => {
    try {
        const { agentId } = req.params;
        const { metadata } = req.body;
        
        const result = await client.hireAgent(agentId, {
            ...metadata,
            user_id: req.userId
        });
        
        res.json(result);
    } catch (error) {
        res.status(500).json({ error: error.message });
    }
});

// Submit a task
app.post('/api/agents/:agentId/tasks', authenticateUser, async (req, res) => {
    try {
        const { agentId } = req.params;
        const { task, parameters } = req.body;
        
        if (!task) {
            return res.status(400).json({ error: 'Task description is required' });
        }
        
        const result = await client.runTask(agentId, task, parameters);
        
        // Log the task for audit purposes
        console.log(`User ${req.userId} ran task on agent ${agentId}: ${task.substring(0, 100)}...`);
        
        res.json(result);
    } catch (error) {
        res.status(500).json({ error: error.message });
    }
});

// Get account balance
app.get('/api/account/balance', authenticateUser, async (req, res) => {
    try {
        const balance = await client.getAccountBalance();
        res.json(balance);
    } catch (error) {
        res.status(500).json({ error: error.message });
    }
});

// Health check
app.get('/health', (req, res) => {
    res.json({ status: 'healthy', timestamp: new Date().toISOString() });
});

const PORT = process.env.PORT || 3000;
app.listen(PORT, () => {
    console.log(`Formation API server running on port ${PORT}`);
});
```

---

## Real-World Use Cases

### 1. Content Marketing Automation

```python
class ContentMarketingAutomation:
    def __init__(self, formation_client):
        self.client = formation_client
        self.content_agent = "content-creator-v2"
        self.research_agent = "research-agent-v3"
    
    async def create_marketing_campaign(self, product_info, target_audience):
        """Create a complete marketing campaign"""
        
        # Step 1: Research the market
        research_task = f"Research current trends and competitors for {product_info['category']}"
        research_result = await self.client.run_task(
            self.research_agent,
            research_task,
            {
                "depth": "comprehensive",
                "focus_areas": ["trends", "competitors", "audience_preferences"],
                "output_format": "structured_data"
            }
        )
        
        # Step 2: Generate blog post
        blog_task = f"Write a blog post about {product_info['name']}"
        blog_result = await self.client.run_task(
            self.content_agent,
            blog_task,
            {
                "tone": "informative",
                "length": "long",
                "target_audience": target_audience,
                "seo_keywords": product_info.get("keywords", []),
                "research_context": research_result["result"]
            }
        )
        
        # Step 3: Generate social media posts
        social_platforms = ["twitter", "linkedin", "facebook"]
        social_posts = {}
        
        for platform in social_platforms:
            social_task = f"Create a {platform} post promoting {product_info['name']}"
            social_result = await self.client.run_task(
                self.content_agent,
                social_task,
                {
                    "platform": platform,
                    "tone": "engaging",
                    "include_hashtags": True,
                    "call_to_action": True,
                    "product_context": product_info
                }
            )
            social_posts[platform] = social_result["completion"]
        
        # Step 4: Generate email campaign
        email_task = f"Create an email campaign for {product_info['name']}"
        email_result = await self.client.run_task(
            self.content_agent,
            email_task,
            {
                "email_type": "product_announcement",
                "tone": "professional",
                "target_audience": target_audience,
                "include_cta": True,
                "personalization": True
            }
        )
        
        return {
            "research": research_result["result"],
            "blog_post": blog_result["completion"],
            "social_posts": social_posts,
            "email_campaign": email_result["completion"],
            "total_cost": (
                research_result["usage"]["cost_credits"] +
                blog_result["usage"]["cost_credits"] +
                sum(result["usage"]["cost_credits"] for result in social_posts.values()) +
                email_result["usage"]["cost_credits"]
            )
        }

# Example usage
async def main():
    client = FormationAgentClient("0x...")
    automation = ContentMarketingAutomation(client)
    
    product_info = {
        "name": "EcoSmart Water Bottle",
        "category": "sustainable_products",
        "keywords": ["eco-friendly", "sustainable", "water bottle", "environment"]
    }
    
    campaign = await automation.create_marketing_campaign(
        product_info,
        "environmentally_conscious_consumers"
    )
    
    print(f"Campaign created! Total cost: {campaign['total_cost']} credits")
    print(f"Blog post: {campaign['blog_post'][:200]}...")
```

### 2. Customer Support Automation

```python
class CustomerSupportBot:
    def __init__(self, formation_client):
        self.client = formation_client
        self.support_agent = "customer-support-v2"
        self.escalation_agent = "technical-expert-v1"
    
    async def handle_customer_inquiry(self, inquiry, customer_context):
        """Handle a customer support inquiry"""
        
        # Step 1: Analyze the inquiry
        analysis_task = "Analyze this customer inquiry and categorize it"
        analysis_result = await self.client.run_task(
            self.support_agent,
            analysis_task,
            {
                "inquiry": inquiry,
                "customer_context": customer_context,
                "categorize": True,
                "urgency_assessment": True,
                "sentiment_analysis": True
            }
        )
        
        category = analysis_result["result"]["category"]
        urgency = analysis_result["result"]["urgency"]
        
        # Step 2: Generate appropriate response
        if urgency == "high" or category == "technical_issue":
            # Escalate to technical expert
            response_task = f"Provide expert technical support for: {inquiry}"
            response_result = await self.client.run_task(
                self.escalation_agent,
                response_task,
                {
                    "inquiry": inquiry,
                    "customer_context": customer_context,
                    "tone": "professional",
                    "include_troubleshooting": True,
                    "escalation_level": "technical_expert"
                }
            )
        else:
            # Handle with standard support agent
            response_task = f"Provide customer support response for: {inquiry}"
            response_result = await self.client.run_task(
                self.support_agent,
                response_task,
                {
                    "inquiry": inquiry,
                    "customer_context": customer_context,
                    "tone": "friendly",
                    "include_next_steps": True
                }
            )
        
        return {
            "analysis": analysis_result["result"],
            "response": response_result["completion"],
            "escalated": urgency == "high" or category == "technical_issue",
            "cost": analysis_result["usage"]["cost_credits"] + response_result["usage"]["cost_credits"]
        }

# Example usage
async def main():
    client = FormationAgentClient("0x...")
    support_bot = CustomerSupportBot(client)
    
    inquiry = "My app keeps crashing when I try to upload large files. I've tried restarting but it doesn't help."
    customer_context = {
        "subscription": "pro",
        "app_version": "2.1.4",
        "platform": "iOS",
        "previous_issues": []
    }
    
    result = await support_bot.handle_customer_inquiry(inquiry, customer_context)
    
    print(f"Category: {result['analysis']['category']}")
    print(f"Urgency: {result['analysis']['urgency']}")
    print(f"Escalated: {result['escalated']}")
    print(f"Response: {result['response']}")
    print(f"Cost: {result['cost']} credits")
```

### 3. Data Analysis Pipeline

```python
class DataAnalysisPipeline:
    def __init__(self, formation_client):
        self.client = formation_client
        self.data_agent = "data-analyst-v2"
        self.viz_agent = "visualization-expert-v1"
        self.report_agent = "report-generator-v2"
    
    async def analyze_business_data(self, data_sources, analysis_goals):
        """Complete data analysis pipeline"""
        
        # Step 1: Data analysis
        analysis_task = "Analyze business data and identify key insights"
        analysis_result = await self.client.run_task(
            self.data_agent,
            analysis_task,
            {
                "data_sources": data_sources,
                "analysis_goals": analysis_goals,
                "statistical_tests": True,
                "trend_analysis": True,
                "correlation_analysis": True,
                "output_format": "structured_insights"
            }
        )
        
        # Step 2: Create visualizations
        viz_task = "Create data visualizations for the analysis results"
        viz_result = await self.client.run_task(
            self.viz_agent,
            viz_task,
            {
                "analysis_results": analysis_result["result"],
                "chart_types": ["trend_lines", "bar_charts", "heatmaps"],
                "style": "professional",
                "include_annotations": True
            }
        )
        
        # Step 3: Generate executive report
        report_task = "Generate an executive summary report"
        report_result = await self.client.run_task(
            self.report_agent,
            report_task,
            {
                "analysis_insights": analysis_result["result"],
                "visualizations": viz_result["result"],
                "audience": "executives",
                "include_recommendations": True,
                "format": "executive_summary"
            }
        )
        
        return {
            "insights": analysis_result["result"],
            "visualizations": viz_result["result"],
            "executive_report": report_result["completion"],
            "total_cost": (
                analysis_result["usage"]["cost_credits"] +
                viz_result["usage"]["cost_credits"] +
                report_result["usage"]["cost_credits"]
            )
        }

# Example usage
async def main():
    client = FormationAgentClient("0x...")
    pipeline = DataAnalysisPipeline(client)
    
    data_sources = [
        "https://company.com/sales_data_q3.csv",
        "https://company.com/customer_feedback.json",
        "https://company.com/marketing_metrics.xlsx"
    ]
    
    analysis_goals = [
        "Identify sales trends and patterns",
        "Analyze customer satisfaction drivers",
        "Evaluate marketing campaign effectiveness",
        "Provide recommendations for Q4 strategy"
    ]
    
    results = await pipeline.analyze_business_data(data_sources, analysis_goals)
    
    print(f"Analysis completed! Total cost: {results['total_cost']} credits")
    print(f"Key insights: {len(results['insights']['key_findings'])} findings")
    print(f"Visualizations: {len(results['visualizations']['charts'])} charts created")
```

---

## Integration Patterns

### 1. Webhook Integration

```python
from flask import Flask, request, jsonify
import asyncio
from threading import Thread

app = Flask(__name__)

class WebhookIntegration:
    def __init__(self, formation_client):
        self.client = formation_client
        self.pending_tasks = {}
    
    def process_webhook_async(self, webhook_data):
        """Process webhook in background"""
        loop = asyncio.new_event_loop()
        asyncio.set_event_loop(loop)
        
        try:
            result = loop.run_until_complete(
                self.handle_webhook(webhook_data)
            )
            # Send result back to originating system
            self.send_result_callback(webhook_data['callback_url'], result)
        finally:
            loop.close()
    
    async def handle_webhook(self, webhook_data):
        """Handle incoming webhook"""
        event_type = webhook_data.get('event_type')
        
        if event_type == 'content_request':
            return await self.handle_content_request(webhook_data)
        elif event_type == 'data_analysis_request':
            return await self.handle_analysis_request(webhook_data)
        else:
            return {"error": f"Unknown event type: {event_type}"}
    
    async def handle_content_request(self, data):
        """Handle content creation request"""
        result = await self.client.run_task(
            "content-creator-v2",
            data['content_brief'],
            data.get('parameters', {})
        )
        return result
    
    def send_result_callback(self, callback_url, result):
        """Send result back to originating system"""
        import requests
        try:
            requests.post(callback_url, json=result, timeout=30)
        except Exception as e:
            print(f"Failed to send callback: {e}")

# Flask webhook endpoint
webhook_handler = WebhookIntegration(FormationAgentClient("0x..."))

@app.route('/webhook/formation', methods=['POST'])
def handle_formation_webhook():
    webhook_data = request.json
    
    # Process webhook asynchronously
    thread = Thread(
        target=webhook_handler.process_webhook_async,
        args=(webhook_data,)
    )
    thread.start()
    
    return jsonify({"status": "accepted", "message": "Processing request"})

if __name__ == '__main__':
    app.run(host='0.0.0.0', port=5000)
```

### 2. Queue-Based Processing

```python
import redis
import json
from celery import Celery

# Celery configuration
app = Celery('formation_tasks', broker='redis://localhost:6379')

@app.task
def process_agent_task(agent_id, task, parameters):
    """Celery task for processing Formation agent requests"""
    client = FormationAgentClient(os.environ['FORMATION_PRIVATE_KEY'])
    
    try:
        result = client.run_task(agent_id, task, parameters)
        return {
            "success": True,
            "result": result,
            "cost": result["usage"]["cost_credits"]
        }
    except Exception as e:
        return {
            "success": False,
            "error": str(e)
        }

class QueuedTaskManager:
    def __init__(self):
        self.redis_client = redis.Redis(host='localhost', port=6379, db=0)
    
    def submit_task(self, agent_id, task, parameters, priority='normal'):
        """Submit task to queue"""
        task_id = str(uuid.uuid4())
        
        # Submit to Celery
        celery_task = process_agent_task.delay(agent_id, task, parameters)
        
        # Store task info in Redis
        task_info = {
            "task_id": task_id,
            "celery_task_id": celery_task.id,
            "agent_id": agent_id,
            "status": "queued",
            "submitted_at": time.time()
        }
        
        self.redis_client.setex(
            f"task:{task_id}",
            3600,  # 1 hour TTL
            json.dumps(task_info)
        )
        
        return task_id
    
    def get_task_status(self, task_id):
        """Get task status"""
        task_info = self.redis_client.get(f"task:{task_id}")
        if not task_info:
            return {"error": "Task not found"}
        
        task_data = json.loads(task_info)
        celery_task = process_agent_task.AsyncResult(task_data["celery_task_id"])
        
        if celery_task.ready():
            result = celery_task.result
            task_data["status"] = "completed" if result["success"] else "failed"
            task_data["result"] = result
        else:
            task_data["status"] = "processing"
        
        return task_data

# Example usage
task_manager = QueuedTaskManager()

# Submit task
task_id = task_manager.submit_task(
    "text-processor-v1",
    "Write a product review for a new smartphone",
    {"tone": "balanced", "length": "medium"}
)

# Check status
status = task_manager.get_task_status(task_id)
print(f"Task status: {status['status']}")
```

---

## Best Practices

### 1. Error Handling and Retry Logic

```python
import time
import random
from functools import wraps

def retry_on_failure(max_retries=3, backoff_factor=2):
    """Decorator for retrying failed agent tasks"""
    def decorator(func):
        @wraps(func)
        async def wrapper(*args, **kwargs):
            last_exception = None
            
            for attempt in range(max_retries + 1):
                try:
                    return await func(*args, **kwargs)
                except Exception as e:
                    last_exception = e
                    
                    if attempt < max_retries:
                        delay = backoff_factor ** attempt + random.uniform(0, 1)
                        print(f"Attempt {attempt + 1} failed: {e}. Retrying in {delay:.2f}s...")
                        await asyncio.sleep(delay)
                    else:
                        print(f"All {max_retries + 1} attempts failed")
            
            raise last_exception
        return wrapper
    return decorator

class RobustAgentClient:
    def __init__(self, formation_client):
        self.client = formation_client
    
    @retry_on_failure(max_retries=3)
    async def run_task_with_retry(self, agent_id, task, parameters):
        """Run task with automatic retry on failure"""
        return await self.client.run_task(agent_id, task, parameters)
    
    async def run_task_with_fallback(self, primary_agent, fallback_agents, task, parameters):
        """Run task with fallback agents"""
        agents_to_try = [primary_agent] + fallback_agents
        
        for agent_id in agents_to_try:
            try:
                result = await self.run_task_with_retry(agent_id, task, parameters)
                return {
                    "success": True,
                    "agent_used": agent_id,
                    "result": result
                }
            except Exception as e:
                print(f"Agent {agent_id} failed: {e}")
                continue
        
        return {
            "success": False,
            "error": "All agents failed",
            "agents_attempted": agents_to_try
        }
```

### 2. Cost Monitoring

```python
class CostMonitor:
    def __init__(self, budget_limit=1000):
        self.budget_limit = budget_limit
        self.current_spend = 0
        self.transactions = []
    
    def track_transaction(self, agent_id, task, cost):
        """Track a transaction"""
        self.current_spend += cost
        self.transactions.append({
            "timestamp": time.time(),
            "agent_id": agent_id,
            "task": task[:100],  # Truncate for storage
            "cost": cost
        })
        
        if self.current_spend > self.budget_limit:
            raise Exception(f"Budget limit exceeded! Spent: {self.current_spend}, Limit: {self.budget_limit}")
    
    def get_spending_report(self):
        """Generate spending report"""
        agent_costs = {}
        for tx in self.transactions:
            agent_id = tx["agent_id"]
            agent_costs[agent_id] = agent_costs.get(agent_id, 0) + tx["cost"]
        
        return {
            "total_spend": self.current_spend,
            "remaining_budget": self.budget_limit - self.current_spend,
            "transaction_count": len(self.transactions),
            "average_cost_per_transaction": self.current_spend / len(self.transactions) if self.transactions else 0,
            "spend_by_agent": agent_costs
        }

# Usage with cost monitoring
cost_monitor = CostMonitor(budget_limit=500)

async def run_monitored_task(client, agent_id, task, parameters):
    """Run task with cost monitoring"""
    result = await client.run_task(agent_id, task, parameters)
    cost = result["usage"]["cost_credits"]
    
    cost_monitor.track_transaction(agent_id, task, cost)
    
    return result
```

---

**Ready to integrate Formation agents into your applications?** Start with the basic examples and gradually add more sophisticated features like error handling, cost monitoring, and batch processing! 🚀 