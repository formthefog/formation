# Getting Started with AI Agents

This guide walks you through everything you need to know to start hiring and using AI agents on the Formation network.

## What Are AI Agents?

AI agents on Formation are specialized services that can perform complex, multi-step tasks. Unlike simple AI models that just generate text, agents can:

- **Use Tools**: Access external APIs, databases, and services
- **Multi-Step Reasoning**: Break down complex problems into smaller tasks
- **Persistent Memory**: Remember context across conversations
- **Custom Logic**: Implement specialized business logic and workflows
- **Integration**: Connect with your existing systems and data

### Agents vs. Models

| Feature | AI Agents | AI Models |
|---------|-----------|-----------|
| **Complexity** | Multi-step workflows | Single inference requests |
| **Tools** | Can use external tools | Text-only input/output |
| **Memory** | Persistent across sessions | Stateless |
| **Customization** | Highly customizable | Fixed behavior |
| **Use Cases** | Complex tasks, automation | Text generation, analysis |

---

## Account Creation and Authentication

### 1. Understanding Formation Authentication

Formation uses **ECDSA signature authentication** instead of traditional API keys:

- **More Secure**: No API keys to leak or manage
- **Wallet Compatible**: Works with Ethereum wallets like MetaMask
- **Cryptographically Secure**: Uses the same security as blockchain transactions

### 2. Setting Up Your Account

#### Option A: Using an Ethereum Wallet (Recommended)

If you have an Ethereum wallet (MetaMask, WalletConnect, etc.):

1. **Connect Your Wallet**: Use your existing Ethereum address
2. **Sign Messages**: Sign authentication messages with your wallet
3. **Automatic Account**: Your Formation account is created automatically

#### Option B: Generate a New Key Pair

If you don't have an Ethereum wallet:

```bash
# Using Formation CLI (coming soon)
formation auth generate-key

# Using OpenSSL
openssl ecparam -genkey -name secp256k1 -noout -out private_key.pem
openssl ec -in private_key.pem -pubout -out public_key.pem
```

### 3. Authentication Process

Formation uses ECDSA signatures for all API requests:

```python
import hashlib
from eth_account import Account
import requests

# Your private key (keep this secure!)
private_key = "0x1234567890abcdef..."

# Create account from private key
account = Account.from_key(private_key)

# Create message to sign
message = "Formation authentication request"
message_hash = hashlib.sha256(message.encode()).hexdigest()

# Sign the message
signature = account.signHash(message_hash)

# Use in API requests
headers = {
    "X-Formation-Address": account.address,
    "X-Formation-Signature": signature.signature.hex(),
    "X-Formation-Message": message,
    "Content-Type": "application/json"
}
```

---

## Browsing Available Agents

### 1. List All Public Agents

Get a list of all publicly available agents:

```bash
curl -X GET "https://formation.ai/v1/agents" \
  -H "X-Formation-Address: 0x1234567890abcdef..." \
  -H "X-Formation-Signature: 0xabcdef..." \
  -H "X-Formation-Message: Formation authentication request"
```

**Response:**
```json
{
  "success": true,
  "count": 25,
  "agents": [
    {
      "agent_id": "text-processor-v1",
      "name": "Advanced Text Processing Agent",
      "description": "Handles text generation, summarization, and translation tasks",
      "owner_id": "0x9876543210fedcba...",
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
      "tags": ["nlp", "text", "language"],
      "average_rating": 4.8,
      "deployment_count": 1250,
      "usage_count": 15000,
      "is_featured": true,
      "is_private": false,
      "price_per_request": 50,
      "resource_requirements": {
        "min_vcpus": 1,
        "min_memory_mb": 512,
        "requires_gpu": false
      }
    }
  ]
}
```

### 2. Filter Agents by Category

```bash
# Get only code generation agents
curl -X GET "https://formation.ai/v1/agents?category=CodeGenerator" \
  -H "X-Formation-Address: 0x1234567890abcdef..." \
  -H "X-Formation-Signature: 0xabcdef..." \
  -H "X-Formation-Message: Formation authentication request"

# Get agents with specific capabilities
curl -X GET "https://formation.ai/v1/agents?capabilities=web_search,data_analysis" \
  -H "X-Formation-Address: 0x1234567890abcdef..." \
  -H "X-Formation-Signature: 0xabcdef..." \
  -H "X-Formation-Message: Formation authentication request"
```

### 3. Get Detailed Agent Information

```bash
curl -X GET "https://formation.ai/v1/agents/text-processor-v1" \
  -H "X-Formation-Address: 0x1234567890abcdef..." \
  -H "X-Formation-Signature: 0xabcdef..." \
  -H "X-Formation-Message: Formation authentication request"
```

**Response:**
```json
{
  "success": true,
  "agent": {
    "agent_id": "text-processor-v1",
    "name": "Advanced Text Processing Agent",
    "description": "Handles text generation, summarization, and translation tasks",
    "documentation": "# Text Processing Agent\n\nThis agent specializes in...",
    "capabilities": [
      "text_generation",
      "summarization",
      "translation"
    ],
    "supported_task_types": [
      {
        "type": "text_generation",
        "description": "Generate text based on prompts",
        "parameters": {
          "prompt": {"type": "string", "required": true},
          "max_tokens": {"type": "integer", "default": 1000},
          "temperature": {"type": "number", "default": 0.7}
        }
      }
    ],
    "pricing": {
      "model": "per_request",
      "base_cost": 50,
      "currency": "credits"
    },
    "tools": [
      {
        "id": "web_search",
        "name": "Web Search",
        "description": "Search the web for information",
        "auth_required": false
      }
    ]
  }
}
```

---

## Understanding Agent Capabilities and Pricing

### 1. Agent Types

Formation hosts various types of agents:

#### **Assistant Agents**
- General-purpose conversational agents
- Good for Q&A, writing, and basic tasks
- Usually lower cost, fast response times

#### **Specialist Agents**
- **Code Generators**: Programming assistance, debugging
- **Data Analysts**: Process spreadsheets, generate insights
- **Content Creators**: Writing, marketing copy, social media
- **Researchers**: Information gathering, fact-checking

#### **Workflow Agents**
- Multi-step process automation
- Can orchestrate multiple tools and services
- Higher cost but can handle complex tasks

### 2. Pricing Models

Agents use different pricing strategies:

#### **Per-Request Pricing**
```json
{
  "pricing": {
    "model": "per_request",
    "base_cost": 50,
    "complexity_tiers": {
      "simple": 25,
      "standard": 50,
      "complex": 100
    },
    "currency": "credits"
  }
}
```

#### **Token-Based Pricing**
```json
{
  "pricing": {
    "model": "per_token",
    "input_rate": 1.0,
    "output_rate": 1.5,
    "currency": "credits_per_1k_tokens"
  }
}
```

#### **Time-Based Pricing**
```json
{
  "pricing": {
    "model": "per_minute",
    "rate": 10,
    "minimum_duration": 1,
    "currency": "credits_per_minute"
  }
}
```

### 3. Capability Assessment

Before hiring an agent, check if it can handle your use case:

```python
def assess_agent_suitability(agent_data, your_requirements):
    """
    Assess if an agent is suitable for your needs
    """
    score = 0
    
    # Check capabilities match
    agent_capabilities = set(agent_data["capabilities"])
    required_capabilities = set(your_requirements["capabilities"])
    
    if required_capabilities.issubset(agent_capabilities):
        score += 40
    
    # Check resource requirements
    if agent_data["resource_requirements"]["requires_gpu"] and not your_requirements.get("gpu_available", False):
        return 0  # Can't run without GPU
    
    # Check pricing fits budget
    estimated_cost = estimate_cost(agent_data["pricing"], your_requirements["expected_usage"])
    if estimated_cost <= your_requirements["budget"]:
        score += 30
    
    # Check ratings and reliability
    if agent_data.get("average_rating", 0) >= 4.0:
        score += 20
    
    if agent_data.get("deployment_count", 0) >= 100:
        score += 10
    
    return score

# Example usage
requirements = {
    "capabilities": ["text_generation", "web_search"],
    "expected_usage": {"requests_per_day": 100},
    "budget": 500,  # credits per month
    "gpu_available": False
}

suitability_score = assess_agent_suitability(agent_data, requirements)
print(f"Agent suitability score: {suitability_score}/100")
```

---

## Hiring Your First Agent

### 1. Check Your Account Balance

Before hiring an agent, ensure you have sufficient credits:

```bash
curl -X GET "https://formation.ai/v1/account/0x1234567890abcdef.../balance" \
  -H "X-Formation-Address: 0x1234567890abcdef..." \
  -H "X-Formation-Signature: 0xabcdef..." \
  -H "X-Formation-Message: Formation authentication request"
```

**Response:**
```json
{
  "success": true,
  "account": {
    "address": "0x1234567890abcdef...",
    "available_credits": 1000,
    "subscription": {
      "tier": "Pro",
      "status": "Active",
      "included_agents": 3,
      "hired_agents": 1,
      "remaining_agents": 2
    },
    "hired_agents": ["text-processor-v1"]
  }
}
```

### 2. Hire an Agent

```bash
curl -X POST "https://formation.ai/v1/agents/text-processor-v1/hire" \
  -H "X-Formation-Address: 0x1234567890abcdef..." \
  -H "X-Formation-Signature: 0xabcdef..." \
  -H "X-Formation-Message: Formation authentication request" \
  -H "Content-Type: application/json" \
  -d '{
    "metadata": {
      "purpose": "Content creation for blog",
      "expected_usage": "daily"
    }
  }'
```

**Response:**
```json
{
  "success": true,
  "message": "Successfully hired agent text-processor-v1",
  "agent": {
    "id": "text-processor-v1",
    "name": "Advanced Text Processing Agent",
    "description": "Handles text generation, summarization, and translation tasks"
  },
  "credits_remaining": 950,
  "hired_agent_count": 2
}
```

### 3. Subscription Tier Limits

Different subscription tiers have different agent limits:

| Tier | Included Agents | Max Total Agents | Additional Agent Cost |
|------|-----------------|------------------|----------------------|
| **Free** | 1 | 2 | 10 credits |
| **Pro** | 3 | 5 | 8 credits |
| **Pro Plus** | 5 | 10 | 6 credits |
| **Power** | 10 | 20 | 4 credits |
| **Power Plus** | 25 | 50 | 2 credits |

### 4. Managing Hired Agents

#### List Your Hired Agents
```bash
curl -X GET "https://formation.ai/v1/account/0x1234567890abcdef.../agents" \
  -H "X-Formation-Address: 0x1234567890abcdef..." \
  -H "X-Formation-Signature: 0xabcdef..." \
  -H "X-Formation-Message: Formation authentication request"
```

#### Fire an Agent (Stop Hiring)
```bash
curl -X POST "https://formation.ai/v1/agents/text-processor-v1/fire" \
  -H "X-Formation-Address: 0x1234567890abcdef..." \
  -H "X-Formation-Signature: 0xabcdef..." \
  -H "X-Formation-Message: Formation authentication request"
```

---

## Making Your First Agent Request

Once you've hired an agent, you can start using it:

### 1. Simple Task Request

```bash
curl -X POST "https://formation.ai/v1/agents/text-processor-v1/run_task" \
  -H "X-Formation-Address: 0x1234567890abcdef..." \
  -H "X-Formation-Signature: 0xabcdef..." \
  -H "X-Formation-Message: Formation authentication request" \
  -H "Content-Type: application/json" \
  -d '{
    "task": "Write a professional email introducing our new product launch",
    "parameters": {
      "tone": "professional",
      "length": "medium",
      "include_call_to_action": true
    }
  }'
```

**Response:**
```json
{
  "task_id": "task_abc123",
  "agent_id": "text-processor-v1",
  "status": "completed",
  "completion": "Subject: Introducing Our Revolutionary New Product\n\nDear [Name],\n\nI hope this email finds you well. I'm excited to share some fantastic news with you...",
  "usage": {
    "prompt_tokens": 45,
    "completion_tokens": 180,
    "total_tokens": 225,
    "duration_ms": 2500,
    "cost_credits": 3
  },
  "timestamp": 1640995200
}
```

### 2. Complex Task with Multiple Parameters

```bash
curl -X POST "https://formation.ai/v1/agents/data-analyst-v2/run_task" \
  -H "X-Formation-Address: 0x1234567890abcdef..." \
  -H "X-Formation-Signature: 0xabcdef..." \
  -H "X-Formation-Message: Formation authentication request" \
  -H "Content-Type: application/json" \
  -d '{
    "task": "Analyze sales data and generate insights",
    "parameters": {
      "data_source": "https://example.com/sales_data.csv",
      "analysis_type": "trend_analysis",
      "time_period": "last_quarter",
      "output_format": "executive_summary"
    }
  }'
```

---

## Best Practices

### 1. Agent Selection

- **Start Simple**: Begin with general-purpose agents before moving to specialists
- **Check Reviews**: Look at ratings and deployment counts
- **Test Small**: Try small tasks before committing to large projects
- **Read Documentation**: Review agent capabilities and limitations

### 2. Cost Management

- **Monitor Usage**: Track your credit consumption
- **Set Budgets**: Use subscription tiers to control costs
- **Optimize Requests**: Be specific in your task descriptions to avoid retries

### 3. Security

- **Protect Private Keys**: Never share your private keys
- **Use Environment Variables**: Store credentials securely
- **Rotate Keys**: Periodically generate new key pairs
- **Monitor Access**: Review your account activity regularly

### 4. Task Design

- **Be Specific**: Clear instructions lead to better results
- **Provide Context**: Include relevant background information
- **Set Expectations**: Specify desired output format and length
- **Handle Errors**: Implement retry logic for failed tasks

---

## Troubleshooting

### Common Issues

#### "Agent not found" Error
```json
{
  "error": "Agent with ID text-processor-v1 not found"
}
```
**Solution**: Check the agent ID spelling and ensure the agent is still available.

#### "Insufficient credits" Error
```json
{
  "error": "Insufficient credits. Required: 50, Available: 25"
}
```
**Solution**: Add credits to your account or upgrade your subscription.

#### "Authentication failed" Error
```json
{
  "error": "Invalid signature or address"
}
```
**Solution**: Verify your signature generation and ensure the message matches.

#### "Agent hire limit exceeded" Error
```json
{
  "error": "Maximum agents reached for your subscription tier"
}
```
**Solution**: Upgrade your subscription or fire unused agents.

### Getting Help

- **Documentation**: Check agent-specific documentation
- **Community**: Join our Discord for community support
- **Support**: Contact support@formation.ai for technical issues
- **Status Page**: Check status.formation.ai for service status

---

## Next Steps

Now that you know the basics:

1. **[Learn Advanced Agent Usage](hiring-agents.md)** - Detailed guide to agent hiring and management
2. **[Explore Code Examples](examples.md)** - Working examples in multiple programming languages
3. **[Try Model Inference](../models/inference-guide.md)** - Direct access to AI models
4. **[Build Your Own Agent](../../developers/agents/building-agents.md)** - Create and deploy custom agents

---

**Ready to hire your first agent?** Start with a simple text processing agent and experiment with different tasks to get familiar with the platform! 🚀 