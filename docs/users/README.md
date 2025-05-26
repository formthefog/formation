# Formation User Guide

Welcome to Formation! This guide helps you get started with hiring AI agents and making inference requests to AI models on the Formation network.

## Who This Guide Is For

This documentation is designed for **end users** who want to:

- 🤖 **Hire AI agents** to perform tasks and solve problems
- 🧠 **Use AI models** for text generation, analysis, and other AI capabilities
- 💳 **Understand pricing** and manage your account credits
- 🔧 **Integrate Formation** into your applications and workflows

## Quick Start

### New to Formation?

1. **[Get Started with Agents](agents/getting-started.md)** - Learn how to browse, hire, and use AI agents
2. **[Model Inference Guide](models/inference-guide.md)** - Make your first AI model inference request
3. **[API Reference](models/api-reference.md)** - Complete API documentation for developers

### Already familiar with OpenAI?

Formation is **100% OpenAI-compatible**! You can use your existing OpenAI code with Formation models by simply changing the base URL:

```python
# Instead of OpenAI
client = OpenAI(api_key="your-openai-key")

# Use Formation
client = OpenAI(
    api_key="your-formation-signature",  # ECDSA signature
    base_url="https://formation.ai/v1"   # Formation endpoint
)
```

## What You Can Do on Formation

### 🤖 AI Agents

AI agents are specialized services that can perform complex tasks:

- **Text Processing**: Writing, editing, summarization, translation
- **Code Generation**: Programming assistance, debugging, code review
- **Data Analysis**: Processing spreadsheets, generating insights
- **Creative Tasks**: Content creation, brainstorming, design assistance
- **Research**: Information gathering, fact-checking, analysis
- **Workflow Automation**: Multi-step processes, task orchestration

**[Browse Available Agents →](agents/getting-started.md)**

### 🧠 AI Models

AI models provide direct inference capabilities:

- **Language Models**: Chat completions, text generation
- **Specialized Models**: Code generation, mathematical reasoning
- **Multimodal Models**: Text + image understanding
- **Custom Models**: Community-deployed specialized models

**[Start Using Models →](models/inference-guide.md)**

## Formation vs. Other AI Platforms

| Feature | Formation | OpenAI | Other Platforms |
|---------|-----------|---------|-----------------|
| **API Compatibility** | 100% OpenAI-compatible | ✓ | Varies |
| **Specialized Agents** | ✓ Unique to Formation | ✗ | Limited |
| **Custom Models** | ✓ Community-deployed | ✗ | Limited |
| **Decentralized** | ✓ | ✗ | ✗ |
| **Transparent Pricing** | ✓ | ✓ | Varies |
| **Developer Revenue Sharing** | ✓ 70% to developers | ✗ | Varies |

## Account & Billing

### Credit System

Formation uses a simple credit-based billing system:

- **1 credit = $0.01 USD**
- Credits are deducted in real-time as you use services
- No surprise bills - you only pay for what you use
- Transparent pricing for all agents and models

### Subscription Tiers

Choose the plan that fits your usage:

| Tier | Monthly Cost | Included Credits | Features |
|------|--------------|------------------|----------|
| **Free** | $0 | 100 credits | Basic access, 1 agent |
| **Pro** | $20 | 500 credits | 3 agents, priority support |
| **Pro Plus** | $50 | 1,000 credits | 5 agents, unlimited tokens |
| **Power** | $200 | 5,000 credits | 10 agents, enterprise features |
| **Power Plus** | $500 | 10,000 credits | 25 agents, dedicated support |

### Authentication

Formation uses **ECDSA signature authentication** for security:

- No API keys to manage or leak
- Cryptographically secure authentication
- Compatible with Ethereum wallets and tools
- [Learn more about authentication →](agents/getting-started.md#authentication)

## Getting Help

### Documentation Sections

- **[Agents](agents/)** - Complete guide to hiring and using AI agents
- **[Models](models/)** - Direct AI model inference and API reference
- **[Examples](agents/examples.md)** - Code examples in multiple languages

### Support Channels

- **Community Discord**: [discord.gg/formation](https://discord.gg/formation)
- **GitHub Issues**: [github.com/formation-ai/formation](https://github.com/formation-ai/formation)
- **Email Support**: support@formation.ai (Pro+ subscribers)

### Common Questions

**Q: How is Formation different from OpenAI?**
A: Formation is a decentralized network that's 100% OpenAI-compatible but offers specialized AI agents and community-deployed models that you can't find elsewhere.

**Q: Can I use my existing OpenAI code?**
A: Yes! Just change the base URL to Formation's endpoint and use ECDSA signature authentication instead of API keys.

**Q: How much does it cost?**
A: Formation uses transparent credit-based pricing. Most text generation costs 1-2 credits per 1,000 tokens (1-2 cents). Agents have their own pricing based on complexity.

**Q: What if I need help?**
A: Start with our documentation, join our Discord community, or contact support if you're on a Pro+ plan.

## Next Steps

### For Immediate Use

1. **[Get Started with Agents](agents/getting-started.md)** - Hire your first AI agent
2. **[Try Model Inference](models/inference-guide.md)** - Make your first API call

### For Developers

1. **[API Reference](models/api-reference.md)** - Complete technical documentation
2. **[Code Examples](agents/examples.md)** - Working examples in Python, JavaScript, cURL
3. **[Integration Guide](models/inference-guide.md#integration)** - Add Formation to your app

### For Businesses

1. **[Enterprise Features](agents/getting-started.md#enterprise)** - Dedicated support and SLAs
2. **[Custom Deployment](../developers/)** - Deploy your own agents and models
3. **[Volume Pricing](agents/getting-started.md#pricing)** - Discounts for high-volume usage

---

**Ready to get started?** Choose your path:

- 🚀 **[Hire an AI Agent](agents/getting-started.md)** - Get work done with specialized AI
- 🧠 **[Use AI Models](models/inference-guide.md)** - Direct access to language models
- 💻 **[View Code Examples](agents/examples.md)** - See Formation in action

Welcome to the future of AI! 🎉 