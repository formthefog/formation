# Formation Model Examples

Practical, working examples for using Formation AI models in your applications. All examples include complete code that you can run immediately.

## Table of Contents

1. [Chat Completion Examples](#chat-completion-examples)
2. [Streaming Responses](#streaming-responses)
3. [Model Listing and Capabilities](#model-listing-and-capabilities)
4. [Different Model Types](#different-model-types)
5. [Integration Patterns](#integration-patterns)
6. [Error Handling](#error-handling)
7. [Production Examples](#production-examples)

---

## Chat Completion Examples

### 1. Basic Chat Completion (Python)

```python
import hashlib
import requests
from eth_account import Account

class FormationModelClient:
    def __init__(self, private_key):
        self.base_url = "https://formation.ai"
        self.account = Account.from_key(private_key)
        self.session = requests.Session()
    
    def _generate_auth_headers(self):
        """Generate authentication headers for Formation API"""
        message = "Formation authentication request"
        message_hash = hashlib.sha256(message.encode()).hexdigest()
        signature = self.account.signHash(message_hash)
        
        return {
            "X-Formation-Address": self.account.address,
            "X-Formation-Signature": signature.signature.hex(),
            "X-Formation-Message": message,
            "Content-Type": "application/json"
        }
    
    def chat_completion(self, model_id, messages, **kwargs):
        """Create a chat completion"""
        headers = self._generate_auth_headers()
        
        payload = {
            "messages": messages,
            "max_tokens": kwargs.get("max_tokens", 1000),
            "temperature": kwargs.get("temperature", 0.7),
            "top_p": kwargs.get("top_p", 1.0),
            "stream": kwargs.get("stream", False)
        }
        
        response = self.session.post(
            f"{self.base_url}/v1/models/{model_id}/inference",
            headers=headers,
            json=payload
        )
        
        response.raise_for_status()
        return response.json()

# Example usage
if __name__ == "__main__":
    # Initialize client with your private key
    private_key = "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef12"
    client = FormationModelClient(private_key)
    
    # Simple chat completion
    messages = [
        {"role": "system", "content": "You are a helpful AI assistant."},
        {"role": "user", "content": "Explain quantum computing in simple terms."}
    ]
    
    result = client.chat_completion("llama2-7b-chat", messages, max_tokens=500)
    
    print("Response:")
    print(result["choices"][0]["message"]["content"])
    print(f"\nTokens used: {result['usage']['total_tokens']}")
    print(f"Cost: {result['usage'].get('cost_credits', 'N/A')} credits")
```

### 2. Multi-Turn Conversation (Python)

```python
def interactive_chat(client, model_id):
    """Interactive chat session with a model"""
    
    conversation = [
        {"role": "system", "content": "You are a helpful AI assistant. Be concise but informative."}
    ]
    
    print("🤖 Formation AI Chat (type 'quit' to exit)")
    print("-" * 50)
    
    while True:
        user_input = input("\n👤 You: ").strip()
        
        if user_input.lower() in ['quit', 'exit', 'bye']:
            print("👋 Goodbye!")
            break
        
        if not user_input:
            continue
        
        # Add user message to conversation
        conversation.append({"role": "user", "content": user_input})
        
        try:
            # Get AI response
            result = client.chat_completion(
                model_id, 
                conversation, 
                max_tokens=300,
                temperature=0.7
            )
            
            ai_response = result["choices"][0]["message"]["content"]
            
            # Add AI response to conversation
            conversation.append({"role": "assistant", "content": ai_response})
            
            print(f"\n🤖 AI: {ai_response}")
            
            # Show usage info
            usage = result["usage"]
            print(f"\n💰 Tokens: {usage['total_tokens']} | Cost: {usage.get('cost_credits', 'N/A')} credits")
            
        except Exception as e:
            print(f"\n❌ Error: {e}")

# Run interactive chat
client = FormationModelClient("0x1234567890abcdef...")
interactive_chat(client, "llama2-7b-chat")
```

### 3. Chat Completion (JavaScript/Node.js)

```javascript
const crypto = require('crypto');
const { ethers } = require('ethers');
const axios = require('axios');

class FormationModelClient {
    constructor(privateKey) {
        this.baseUrl = 'https://formation.ai';
        this.wallet = new ethers.Wallet(privateKey);
        this.axios = axios.create();
    }

    _generateAuthHeaders() {
        const message = 'Formation authentication request';
        const messageHash = crypto.createHash('sha256').update(message).digest('hex');
        const signature = this.wallet.signMessage(messageHash);
        
        return {
            'X-Formation-Address': this.wallet.address,
            'X-Formation-Signature': signature,
            'X-Formation-Message': message,
            'Content-Type': 'application/json'
        };
    }

    async chatCompletion(modelId, messages, options = {}) {
        const headers = this._generateAuthHeaders();
        
        const payload = {
            messages,
            max_tokens: options.maxTokens || 1000,
            temperature: options.temperature || 0.7,
            top_p: options.topP || 1.0,
            stream: options.stream || false
        };

        try {
            const response = await this.axios.post(
                `${this.baseUrl}/v1/models/${modelId}/inference`,
                payload,
                { headers }
            );
            
            return response.data;
        } catch (error) {
            throw new Error(`Formation API error: ${error.response?.data?.error?.message || error.message}`);
        }
    }
}

// Example usage
async function main() {
    const client = new FormationModelClient('0x1234567890abcdef...');
    
    const messages = [
        { role: 'system', content: 'You are a helpful coding assistant.' },
        { role: 'user', content: 'Write a Python function to calculate the factorial of a number.' }
    ];
    
    try {
        const result = await client.chatCompletion('llama2-7b-chat', messages, {
            maxTokens: 500,
            temperature: 0.3
        });
        
        console.log('Response:');
        console.log(result.choices[0].message.content);
        console.log(`\nTokens used: ${result.usage.total_tokens}`);
        console.log(`Cost: ${result.usage.cost_credits || 'N/A'} credits`);
        
    } catch (error) {
        console.error('Error:', error.message);
    }
}

main();
```

### 4. Chat Completion (cURL)

```bash
#!/bin/bash

# Set your authentication details
FORMATION_ADDRESS="0x1234567890abcdef1234567890abcdef12345678"
FORMATION_SIGNATURE="0xabcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890ab"
FORMATION_MESSAGE="Formation authentication request"

# Simple chat completion
curl -X POST "https://formation.ai/v1/models/llama2-7b-chat/inference" \
  -H "X-Formation-Address: $FORMATION_ADDRESS" \
  -H "X-Formation-Signature: $FORMATION_SIGNATURE" \
  -H "X-Formation-Message: $FORMATION_MESSAGE" \
  -H "Content-Type: application/json" \
  -d '{
    "messages": [
      {
        "role": "system",
        "content": "You are a helpful assistant."
      },
      {
        "role": "user",
        "content": "What are the benefits of renewable energy?"
      }
    ],
    "max_tokens": 500,
    "temperature": 0.7
  }' | jq '.'
```

---

## Streaming Responses

### 1. Streaming Chat (Python)

```python
import json
import sseclient

def stream_chat_completion(client, model_id, messages, **kwargs):
    """Stream a chat completion response"""
    
    headers = client._generate_auth_headers()
    headers["Accept"] = "text/event-stream"
    
    payload = {
        "messages": messages,
        "max_tokens": kwargs.get("max_tokens", 1000),
        "temperature": kwargs.get("temperature", 0.7),
        "stream": True
    }
    
    response = client.session.post(
        f"{client.base_url}/v1/models/{model_id}/inference",
        headers=headers,
        json=payload,
        stream=True
    )
    
    response.raise_for_status()
    
    print("🤖 AI: ", end="", flush=True)
    
    client_stream = sseclient.SSEClient(response)
    full_response = ""
    
    for event in client_stream.events():
        if event.data:
            if event.data == '[DONE]':
                break
                
            try:
                chunk_data = json.loads(event.data)
                
                if 'choices' in chunk_data and len(chunk_data['choices']) > 0:
                    delta = chunk_data['choices'][0].get('delta', {})
                    content = delta.get('content', '')
                    
                    if content:
                        print(content, end="", flush=True)
                        full_response += content
                        
                    # Check if this is the final chunk
                    if chunk_data['choices'][0].get('finish_reason'):
                        usage = chunk_data.get('usage', {})
                        print(f"\n\n💰 Tokens: {usage.get('total_tokens', 'N/A')} | Cost: {usage.get('cost_credits', 'N/A')} credits")
                        break
                        
            except json.JSONDecodeError:
                continue
    
    return full_response

# Example usage
client = FormationModelClient("0x1234567890abcdef...")

messages = [
    {"role": "system", "content": "You are a creative writing assistant."},
    {"role": "user", "content": "Write a short story about a robot discovering emotions."}
]

story = stream_chat_completion(client, "llama2-7b-chat", messages, max_tokens=800)
```

### 2. Streaming with Progress Tracking (Python)

```python
import time
from typing import Generator, Dict, Any

def stream_with_progress(client, model_id, messages, **kwargs) -> Generator[Dict[str, Any], None, None]:
    """Stream response with progress tracking"""
    
    headers = client._generate_auth_headers()
    headers["Accept"] = "text/event-stream"
    
    payload = {
        "messages": messages,
        "max_tokens": kwargs.get("max_tokens", 1000),
        "temperature": kwargs.get("temperature", 0.7),
        "stream": True
    }
    
    response = client.session.post(
        f"{client.base_url}/v1/models/{model_id}/inference",
        headers=headers,
        json=payload,
        stream=True
    )
    
    response.raise_for_status()
    
    start_time = time.time()
    total_tokens = 0
    content_length = 0
    
    client_stream = sseclient.SSEClient(response)
    
    for event in client_stream.events():
        if event.data:
            if event.data == '[DONE]':
                yield {
                    "type": "done",
                    "total_time": time.time() - start_time,
                    "total_tokens": total_tokens,
                    "content_length": content_length
                }
                break
                
            try:
                chunk_data = json.loads(event.data)
                
                if 'choices' in chunk_data and len(chunk_data['choices']) > 0:
                    delta = chunk_data['choices'][0].get('delta', {})
                    content = delta.get('content', '')
                    
                    if content:
                        content_length += len(content)
                        
                        yield {
                            "type": "content",
                            "content": content,
                            "elapsed_time": time.time() - start_time,
                            "content_length": content_length
                        }
                    
                    # Check for usage info
                    if 'usage' in chunk_data:
                        total_tokens = chunk_data['usage'].get('total_tokens', 0)
                        
                    # Check if finished
                    if chunk_data['choices'][0].get('finish_reason'):
                        yield {
                            "type": "finished",
                            "finish_reason": chunk_data['choices'][0]['finish_reason'],
                            "usage": chunk_data.get('usage', {}),
                            "elapsed_time": time.time() - start_time
                        }
                        
            except json.JSONDecodeError:
                continue

# Example usage with progress bar
from tqdm import tqdm

def chat_with_progress_bar(client, model_id, messages):
    """Chat with a visual progress bar"""
    
    print("🤖 Generating response...")
    
    with tqdm(desc="Streaming", unit="chars") as pbar:
        full_response = ""
        
        for chunk in stream_with_progress(client, model_id, messages):
            if chunk["type"] == "content":
                content = chunk["content"]
                full_response += content
                pbar.update(len(content))
                pbar.set_postfix({
                    "time": f"{chunk['elapsed_time']:.1f}s",
                    "chars": chunk["content_length"]
                })
                
            elif chunk["type"] == "finished":
                usage = chunk["usage"]
                pbar.set_postfix({
                    "time": f"{chunk['elapsed_time']:.1f}s",
                    "tokens": usage.get("total_tokens", "N/A"),
                    "cost": f"{usage.get('cost_credits', 'N/A')} credits"
                })
                
            elif chunk["type"] == "done":
                pbar.close()
                break
    
    print(f"\n🤖 AI: {full_response}")
    return full_response

# Usage
messages = [
    {"role": "user", "content": "Explain machine learning in detail."}
]

response = chat_with_progress_bar(client, "llama2-7b-chat", messages)
```

### 3. Streaming Chat (JavaScript)

```javascript
async function streamChatCompletion(client, modelId, messages, options = {}) {
    const headers = {
        ...client._generateAuthHeaders(),
        'Accept': 'text/event-stream'
    };
    
    const payload = {
        messages,
        max_tokens: options.maxTokens || 1000,
        temperature: options.temperature || 0.7,
        stream: true
    };

    const response = await fetch(`${client.baseUrl}/v1/models/${modelId}/inference`, {
        method: 'POST',
        headers,
        body: JSON.stringify(payload)
    });

    if (!response.ok) {
        throw new Error(`HTTP ${response.status}: ${response.statusText}`);
    }

    const reader = response.body.getReader();
    const decoder = new TextDecoder();
    
    let fullResponse = '';
    
    console.log('🤖 AI: ');
    
    try {
        while (true) {
            const { done, value } = await reader.read();
            
            if (done) break;
            
            const chunk = decoder.decode(value);
            const lines = chunk.split('\n');
            
            for (const line of lines) {
                if (line.startsWith('data: ')) {
                    const data = line.slice(6);
                    
                    if (data === '[DONE]') {
                        console.log('\n');
                        return fullResponse;
                    }
                    
                    try {
                        const parsed = JSON.parse(data);
                        
                        if (parsed.choices && parsed.choices[0].delta.content) {
                            const content = parsed.choices[0].delta.content;
                            process.stdout.write(content);
                            fullResponse += content;
                        }
                        
                        if (parsed.choices && parsed.choices[0].finish_reason) {
                            const usage = parsed.usage || {};
                            console.log(`\n\n💰 Tokens: ${usage.total_tokens || 'N/A'} | Cost: ${usage.cost_credits || 'N/A'} credits`);
                        }
                        
                    } catch (e) {
                        // Skip invalid JSON
                    }
                }
            }
        }
    } finally {
        reader.releaseLock();
    }
    
    return fullResponse;
}

// Example usage
async function main() {
    const client = new FormationModelClient('0x1234567890abcdef...');
    
    const messages = [
        { role: 'system', content: 'You are a helpful assistant.' },
        { role: 'user', content: 'Tell me about the history of artificial intelligence.' }
    ];
    
    try {
        const response = await streamChatCompletion(client, 'llama2-7b-chat', messages, {
            maxTokens: 600,
            temperature: 0.8
        });
        
        console.log('\n✅ Streaming complete!');
        
    } catch (error) {
        console.error('❌ Error:', error.message);
    }
}

main();
```

---

## Model Listing and Capabilities

### 1. List All Models (Python)

```python
def list_all_models(client):
    """List all available models with detailed information"""
    
    headers = client._generate_auth_headers()
    
    response = client.session.get(
        f"{client.base_url}/v1/models",
        headers=headers
    )
    
    response.raise_for_status()
    data = response.json()
    
    print(f"📋 Found {data['total']} models:")
    print("=" * 80)
    
    for model in data['models']:
        print(f"\n🤖 {model['name']} ({model['id']})")
        print(f"   Type: {model['type']}")
        print(f"   Owner: {model['owner_id'][:10]}...")
        print(f"   Private: {'Yes' if model['is_private'] else 'No'}")
        
        # Pricing info
        pricing = model['pricing']
        if pricing['model'] == 'per_token':
            print(f"   Pricing: {pricing['input_rate']}/{pricing['output_rate']} credits per 1k tokens")
        else:
            print(f"   Pricing: {pricing.get('base_cost', 'N/A')} credits per request")
        
        # Capabilities
        capabilities = model.get('capabilities', [])
        print(f"   Capabilities: {', '.join(capabilities)}")
        
        # Limits
        print(f"   Max tokens: {model.get('max_tokens', 'N/A')}")
        print(f"   Context length: {model.get('context_length', 'N/A')}")
    
    return data['models']

# Example usage
client = FormationModelClient("0x1234567890abcdef...")
models = list_all_models(client)
```

### 2. Filter Models by Type (Python)

```python
def find_models_by_type(client, model_type):
    """Find models of a specific type"""
    
    headers = client._generate_auth_headers()
    
    response = client.session.get(
        f"{client.base_url}/v1/models",
        headers=headers
    )
    
    response.raise_for_status()
    data = response.json()
    
    filtered_models = [
        model for model in data['models'] 
        if model['type'] == model_type
    ]
    
    print(f"🔍 Found {len(filtered_models)} {model_type} models:")
    
    for model in filtered_models:
        pricing = model['pricing']
        price_info = f"{pricing['input_rate']}/{pricing['output_rate']} credits/1k tokens" if pricing['model'] == 'per_token' else f"{pricing.get('base_cost', 'N/A')} credits/request"
        
        print(f"  • {model['name']} ({model['id']}) - {price_info}")
    
    return filtered_models

# Example usage
text_models = find_models_by_type(client, "text_generation")
image_models = find_models_by_type(client, "image_generation")
```

### 3. Get Model Details (Python)

```python
def get_model_details(client, model_id):
    """Get detailed information about a specific model"""
    
    headers = client._generate_auth_headers()
    
    response = client.session.get(
        f"{client.base_url}/v1/models/{model_id}",
        headers=headers
    )
    
    response.raise_for_status()
    data = response.json()
    model = data['model']
    
    print(f"🤖 Model Details: {model['name']}")
    print("=" * 50)
    print(f"ID: {model['id']}")
    print(f"Type: {model['type']}")
    print(f"Description: {model['description']}")
    print(f"Owner: {model['owner_id']}")
    print(f"Version: {model.get('version', 'N/A')}")
    print(f"License: {model.get('license', 'N/A')}")
    
    # Parameters
    if 'parameters' in model:
        params = model['parameters']
        print(f"\nParameters:")
        for key, value in params.items():
            print(f"  {key}: {value}")
    
    # Capabilities
    capabilities = model.get('capabilities', [])
    print(f"\nCapabilities: {', '.join(capabilities)}")
    
    # Supported languages
    languages = model.get('supported_languages', [])
    if languages:
        print(f"Supported languages: {', '.join(languages)}")
    
    # Pricing
    pricing = model['pricing']
    print(f"\nPricing:")
    print(f"  Model: {pricing['model']}")
    if pricing['model'] == 'per_token':
        print(f"  Input rate: {pricing['input_rate']} credits per 1k tokens")
        print(f"  Output rate: {pricing['output_rate']} credits per 1k tokens")
    else:
        print(f"  Base cost: {pricing.get('base_cost', 'N/A')} credits per request")
    
    # Limits
    print(f"\nLimits:")
    print(f"  Max tokens: {model.get('max_tokens', 'N/A')}")
    print(f"  Context length: {model.get('context_length', 'N/A')}")
    
    # Tags
    tags = model.get('tags', [])
    if tags:
        print(f"\nTags: {', '.join(tags)}")
    
    return model

# Example usage
model_details = get_model_details(client, "llama2-7b-chat")
```

### 4. Model Comparison Tool (Python)

```python
def compare_models(client, model_ids):
    """Compare multiple models side by side"""
    
    models = []
    for model_id in model_ids:
        try:
            model = get_model_details(client, model_id)
            models.append(model)
        except Exception as e:
            print(f"❌ Could not fetch {model_id}: {e}")
    
    if not models:
        print("No models to compare")
        return
    
    print(f"\n📊 Comparing {len(models)} models:")
    print("=" * 100)
    
    # Header
    print(f"{'Attribute':<20} ", end="")
    for model in models:
        print(f"{model['name'][:15]:<17}", end="")
    print()
    
    print("-" * 100)
    
    # Comparison rows
    attributes = [
        ('ID', 'id'),
        ('Type', 'type'),
        ('Max Tokens', 'max_tokens'),
        ('Context Length', 'context_length'),
        ('Private', 'is_private'),
        ('Version', 'version')
    ]
    
    for attr_name, attr_key in attributes:
        print(f"{attr_name:<20} ", end="")
        for model in models:
            value = model.get(attr_key, 'N/A')
            if isinstance(value, bool):
                value = 'Yes' if value else 'No'
            print(f"{str(value)[:15]:<17}", end="")
        print()
    
    # Pricing comparison
    print(f"{'Pricing':<20} ", end="")
    for model in models:
        pricing = model['pricing']
        if pricing['model'] == 'per_token':
            price_str = f"{pricing['input_rate']}/{pricing['output_rate']}"
        else:
            price_str = str(pricing.get('base_cost', 'N/A'))
        print(f"{price_str[:15]:<17}", end="")
    print()
    
    # Capabilities comparison
    print(f"{'Capabilities':<20} ", end="")
    for model in models:
        caps = model.get('capabilities', [])
        caps_str = ', '.join(caps[:2])  # Show first 2 capabilities
        if len(caps) > 2:
            caps_str += f" (+{len(caps)-2})"
        print(f"{caps_str[:15]:<17}", end="")
    print()

# Example usage
compare_models(client, ["llama2-7b-chat", "gpt-3.5-turbo", "claude-3-sonnet"])
```

---

## Different Model Types

### 1. Text Generation Models

```python
def text_completion_example(client):
    """Example using text completion models"""
    
    # For models that support text completion (not chat)
    headers = client._generate_auth_headers()
    
    payload = {
        "prompt": "The future of artificial intelligence is",
        "max_tokens": 200,
        "temperature": 0.8,
        "top_p": 0.9,
        "stop": ["\n\n"]
    }
    
    response = client.session.post(
        f"{client.base_url}/v1/models/gpt-3.5-turbo-instruct/inference",
        headers=headers,
        json=payload
    )
    
    response.raise_for_status()
    result = response.json()
    
    print("📝 Text Completion:")
    print(f"Prompt: {payload['prompt']}")
    print(f"Completion: {result['choices'][0]['text']}")
    print(f"Tokens: {result['usage']['total_tokens']}")
    
    return result

# Example usage
text_result = text_completion_example(client)
```

### 2. Image Generation Models

```python
def image_generation_example(client):
    """Example using image generation models"""
    
    headers = client._generate_auth_headers()
    
    payload = {
        "prompt": "A beautiful sunset over a mountain lake, photorealistic, 4k",
        "n": 1,
        "size": "1024x1024",
        "response_format": "url"
    }
    
    response = client.session.post(
        f"{client.base_url}/v1/models/stable-diffusion-xl/inference",
        headers=headers,
        json=payload
    )
    
    response.raise_for_status()
    result = response.json()
    
    print("🎨 Image Generation:")
    print(f"Prompt: {payload['prompt']}")
    print(f"Image URL: {result['data'][0]['url']}")
    
    # Download and save the image
    import requests
    from PIL import Image
    from io import BytesIO
    
    image_response = requests.get(result['data'][0]['url'])
    image = Image.open(BytesIO(image_response.content))
    image.save("generated_image.png")
    print("💾 Image saved as 'generated_image.png'")
    
    return result

# Example usage (if you have access to image generation models)
# image_result = image_generation_example(client)
```

### 3. Embedding Models

```python
def embedding_example(client):
    """Example using embedding models"""
    
    headers = client._generate_auth_headers()
    
    texts = [
        "The quick brown fox jumps over the lazy dog",
        "Machine learning is a subset of artificial intelligence",
        "Python is a popular programming language"
    ]
    
    payload = {
        "input": texts,
        "encoding_format": "float"
    }
    
    response = client.session.post(
        f"{client.base_url}/v1/models/text-embedding-ada-002/inference",
        headers=headers,
        json=payload
    )
    
    response.raise_for_status()
    result = response.json()
    
    print("🔢 Text Embeddings:")
    for i, embedding_data in enumerate(result['data']):
        embedding = embedding_data['embedding']
        print(f"Text {i+1}: {texts[i]}")
        print(f"Embedding dimensions: {len(embedding)}")
        print(f"First 5 values: {embedding[:5]}")
        print()
    
    print(f"Total tokens: {result['usage']['total_tokens']}")
    
    return result

# Example usage (if you have access to embedding models)
# embedding_result = embedding_example(client)
```

### 4. Function Calling Models

```python
def function_calling_example(client):
    """Example using models with function calling capabilities"""
    
    # Define available functions
    functions = [
        {
            "name": "get_weather",
            "description": "Get the current weather for a location",
            "parameters": {
                "type": "object",
                "properties": {
                    "location": {
                        "type": "string",
                        "description": "The city and state, e.g. San Francisco, CA"
                    },
                    "unit": {
                        "type": "string",
                        "enum": ["celsius", "fahrenheit"],
                        "description": "The temperature unit"
                    }
                },
                "required": ["location"]
            }
        }
    ]
    
    messages = [
        {"role": "system", "content": "You are a helpful assistant with access to weather information."},
        {"role": "user", "content": "What's the weather like in New York?"}
    ]
    
    headers = client._generate_auth_headers()
    
    payload = {
        "messages": messages,
        "functions": functions,
        "function_call": "auto",
        "max_tokens": 500
    }
    
    response = client.session.post(
        f"{client.base_url}/v1/models/gpt-4-function-calling/inference",
        headers=headers,
        json=payload
    )
    
    response.raise_for_status()
    result = response.json()
    
    choice = result['choices'][0]
    
    if choice['message'].get('function_call'):
        function_call = choice['message']['function_call']
        print("🔧 Function Call:")
        print(f"Function: {function_call['name']}")
        print(f"Arguments: {function_call['arguments']}")
        
        # In a real application, you would execute the function here
        # and then send the result back to the model
        
    else:
        print("💬 Regular Response:")
        print(choice['message']['content'])
    
    return result

# Example usage (if you have access to function calling models)
# function_result = function_calling_example(client)
```

---

## Integration Patterns

### 1. Web Application Integration (Flask)

```python
from flask import Flask, request, jsonify, render_template_string
import json

app = Flask(__name__)

# Initialize Formation client
formation_client = FormationModelClient("0x1234567890abcdef...")

# HTML template for the chat interface
CHAT_TEMPLATE = """
<!DOCTYPE html>
<html>
<head>
    <title>Formation AI Chat</title>
    <style>
        body { font-family: Arial, sans-serif; max-width: 800px; margin: 0 auto; padding: 20px; }
        .chat-container { border: 1px solid #ddd; height: 400px; overflow-y: scroll; padding: 10px; margin-bottom: 10px; }
        .message { margin-bottom: 10px; }
        .user { color: blue; }
        .assistant { color: green; }
        .system { color: gray; font-style: italic; }
        input[type="text"] { width: 70%; padding: 10px; }
        button { padding: 10px 20px; }
    </style>
</head>
<body>
    <h1>Formation AI Chat</h1>
    <div id="chat-container" class="chat-container"></div>
    <input type="text" id="message-input" placeholder="Type your message..." onkeypress="handleKeyPress(event)">
    <button onclick="sendMessage()">Send</button>
    
    <script>
        let conversation = [
            {"role": "system", "content": "You are a helpful AI assistant."}
        ];
        
        function addMessage(role, content) {
            const container = document.getElementById('chat-container');
            const messageDiv = document.createElement('div');
            messageDiv.className = `message ${role}`;
            messageDiv.innerHTML = `<strong>${role}:</strong> ${content}`;
            container.appendChild(messageDiv);
            container.scrollTop = container.scrollHeight;
        }
        
        function handleKeyPress(event) {
            if (event.key === 'Enter') {
                sendMessage();
            }
        }
        
        async function sendMessage() {
            const input = document.getElementById('message-input');
            const message = input.value.trim();
            
            if (!message) return;
            
            addMessage('user', message);
            input.value = '';
            
            try {
                const response = await fetch('/chat', {
                    method: 'POST',
                    headers: {
                        'Content-Type': 'application/json'
                    },
                    body: JSON.stringify({
                        message: message,
                        conversation: conversation
                    })
                });
                
                const data = await response.json();
                
                if (data.success) {
                    addMessage('assistant', data.response);
                    conversation = data.conversation;
                } else {
                    addMessage('system', 'Error: ' + data.error);
                }
                
            } catch (error) {
                addMessage('system', 'Error: ' + error.message);
            }
        }
    </script>
</body>
</html>
"""

@app.route('/')
def index():
    return render_template_string(CHAT_TEMPLATE)

@app.route('/chat', methods=['POST'])
def chat():
    try:
        data = request.json
        message = data['message']
        conversation = data['conversation']
        
        # Add user message to conversation
        conversation.append({"role": "user", "content": message})
        
        # Get AI response
        result = formation_client.chat_completion(
            "llama2-7b-chat",
            conversation,
            max_tokens=500,
            temperature=0.7
        )
        
        ai_response = result['choices'][0]['message']['content']
        
        # Add AI response to conversation
        conversation.append({"role": "assistant", "content": ai_response})
        
        return jsonify({
            "success": True,
            "response": ai_response,
            "conversation": conversation,
            "usage": result['usage']
        })
        
    except Exception as e:
        return jsonify({
            "success": False,
            "error": str(e)
        }), 500

@app.route('/models')
def list_models():
    try:
        headers = formation_client._generate_auth_headers()
        response = formation_client.session.get(
            f"{formation_client.base_url}/v1/models",
            headers=headers
        )
        response.raise_for_status()
        return response.json()
    except Exception as e:
        return jsonify({"error": str(e)}), 500

if __name__ == '__main__':
    app.run(debug=True)
```

### 2. Batch Processing System

```python
import asyncio
import aiohttp
import json
from typing import List, Dict, Any
import time

class BatchProcessor:
    def __init__(self, formation_client, max_concurrent=5):
        self.client = formation_client
        self.max_concurrent = max_concurrent
        self.semaphore = asyncio.Semaphore(max_concurrent)
    
    async def process_single_request(self, session, model_id, messages, request_id):
        """Process a single inference request"""
        async with self.semaphore:
            try:
                headers = self.client._generate_auth_headers()
                
                payload = {
                    "messages": messages,
                    "max_tokens": 500,
                    "temperature": 0.7
                }
                
                async with session.post(
                    f"{self.client.base_url}/v1/models/{model_id}/inference",
                    headers=headers,
                    json=payload
                ) as response:
                    
                    if response.status == 200:
                        result = await response.json()
                        return {
                            "request_id": request_id,
                            "success": True,
                            "result": result,
                            "response": result['choices'][0]['message']['content'],
                            "usage": result['usage']
                        }
                    else:
                        error_text = await response.text()
                        return {
                            "request_id": request_id,
                            "success": False,
                            "error": f"HTTP {response.status}: {error_text}"
                        }
                        
            except Exception as e:
                return {
                    "request_id": request_id,
                    "success": False,
                    "error": str(e)
                }
    
    async def process_batch(self, requests: List[Dict[str, Any]]):
        """Process multiple requests concurrently"""
        
        print(f"🚀 Processing {len(requests)} requests with max {self.max_concurrent} concurrent...")
        
        async with aiohttp.ClientSession() as session:
            tasks = [
                self.process_single_request(
                    session,
                    req["model_id"],
                    req["messages"],
                    req["request_id"]
                )
                for req in requests
            ]
            
            start_time = time.time()
            results = await asyncio.gather(*tasks, return_exceptions=True)
            end_time = time.time()
            
            # Process results
            successful = sum(1 for r in results if isinstance(r, dict) and r.get("success"))
            failed = len(results) - successful
            total_tokens = sum(r.get("usage", {}).get("total_tokens", 0) for r in results if isinstance(r, dict) and r.get("success"))
            total_cost = sum(r.get("usage", {}).get("cost_credits", 0) for r in results if isinstance(r, dict) and r.get("success"))
            
            print(f"✅ Batch complete in {end_time - start_time:.2f}s")
            print(f"📊 Results: {successful} successful, {failed} failed")
            print(f"💰 Total: {total_tokens} tokens, {total_cost} credits")
            
            return results

# Example usage
async def main():
    client = FormationModelClient("0x1234567890abcdef...")
    processor = BatchProcessor(client, max_concurrent=3)
    
    # Prepare batch requests
    requests = [
        {
            "request_id": f"req_{i}",
            "model_id": "llama2-7b-chat",
            "messages": [
                {"role": "system", "content": "You are a helpful assistant."},
                {"role": "user", "content": f"Tell me about topic number {i}"}
            ]
        }
        for i in range(10)
    ]
    
    # Process batch
    results = await processor.process_batch(requests)
    
    # Print results
    for result in results:
        if isinstance(result, dict):
            if result["success"]:
                print(f"✅ {result['request_id']}: {result['response'][:100]}...")
            else:
                print(f"❌ {result['request_id']}: {result['error']}")

# Run the batch processor
asyncio.run(main())
```

### 3. Real-time Chat Application (WebSocket)

```python
import asyncio
import websockets
import json
from websockets.server import serve

class FormationChatServer:
    def __init__(self, formation_client):
        self.client = formation_client
        self.active_connections = set()
    
    async def register(self, websocket):
        """Register a new client connection"""
        self.active_connections.add(websocket)
        print(f"👤 Client connected. Total: {len(self.active_connections)}")
    
    async def unregister(self, websocket):
        """Unregister a client connection"""
        self.active_connections.discard(websocket)
        print(f"👋 Client disconnected. Total: {len(self.active_connections)}")
    
    async def handle_message(self, websocket, message_data):
        """Handle incoming message from client"""
        try:
            if message_data["type"] == "chat":
                # Get AI response
                result = self.client.chat_completion(
                    message_data["model_id"],
                    message_data["messages"],
                    max_tokens=message_data.get("max_tokens", 500),
                    temperature=message_data.get("temperature", 0.7)
                )
                
                # Send response back to client
                response = {
                    "type": "chat_response",
                    "request_id": message_data.get("request_id"),
                    "response": result["choices"][0]["message"]["content"],
                    "usage": result["usage"]
                }
                
                await websocket.send(json.dumps(response))
                
            elif message_data["type"] == "list_models":
                # Get available models
                headers = self.client._generate_auth_headers()
                response = self.client.session.get(
                    f"{self.client.base_url}/v1/models",
                    headers=headers
                )
                response.raise_for_status()
                models_data = response.json()
                
                response = {
                    "type": "models_list",
                    "request_id": message_data.get("request_id"),
                    "models": models_data["models"]
                }
                
                await websocket.send(json.dumps(response))
                
        except Exception as e:
            error_response = {
                "type": "error",
                "request_id": message_data.get("request_id"),
                "error": str(e)
            }
            await websocket.send(json.dumps(error_response))
    
    async def handle_client(self, websocket, path):
        """Handle a client connection"""
        await self.register(websocket)
        
        try:
            async for message in websocket:
                try:
                    message_data = json.loads(message)
                    await self.handle_message(websocket, message_data)
                except json.JSONDecodeError:
                    await websocket.send(json.dumps({
                        "type": "error",
                        "error": "Invalid JSON message"
                    }))
                except Exception as e:
                    await websocket.send(json.dumps({
                        "type": "error",
                        "error": str(e)
                    }))
        finally:
            await self.unregister(websocket)

# Start the WebSocket server
async def main():
    client = FormationModelClient("0x1234567890abcdef...")
    chat_server = FormationChatServer(client)
    
    print("🚀 Starting Formation Chat WebSocket server on ws://localhost:8765")
    
    async with serve(chat_server.handle_client, "localhost", 8765):
        await asyncio.Future()  # Run forever

# Client example (JavaScript)
CLIENT_HTML = """
<!DOCTYPE html>
<html>
<head>
    <title>Formation WebSocket Chat</title>
</head>
<body>
    <div id="messages"></div>
    <input type="text" id="messageInput" placeholder="Type a message...">
    <button onclick="sendMessage()">Send</button>
    
    <script>
        const ws = new WebSocket('ws://localhost:8765');
        let requestId = 0;
        
        ws.onmessage = function(event) {
            const data = JSON.parse(event.data);
            const messagesDiv = document.getElementById('messages');
            
            if (data.type === 'chat_response') {
                messagesDiv.innerHTML += `<div><strong>AI:</strong> ${data.response}</div>`;
                messagesDiv.innerHTML += `<div><small>Tokens: ${data.usage.total_tokens}, Cost: ${data.usage.cost_credits} credits</small></div>`;
            } else if (data.type === 'error') {
                messagesDiv.innerHTML += `<div style="color: red;"><strong>Error:</strong> ${data.error}</div>`;
            }
            
            messagesDiv.scrollTop = messagesDiv.scrollHeight;
        };
        
        function sendMessage() {
            const input = document.getElementById('messageInput');
            const message = input.value.trim();
            
            if (!message) return;
            
            const messagesDiv = document.getElementById('messages');
            messagesDiv.innerHTML += `<div><strong>You:</strong> ${message}</div>`;
            
            ws.send(JSON.stringify({
                type: 'chat',
                request_id: ++requestId,
                model_id: 'llama2-7b-chat',
                messages: [
                    {role: 'system', content: 'You are a helpful assistant.'},
                    {role: 'user', content: message}
                ]
            }));
            
            input.value = '';
        }
        
        document.getElementById('messageInput').addEventListener('keypress', function(e) {
            if (e.key === 'Enter') {
                sendMessage();
            }
        });
    </script>
</body>
</html>
"""

if __name__ == "__main__":
    asyncio.run(main())
```

---

## Error Handling

### 1. Comprehensive Error Handling

```python
import time
import random
from typing import Optional, Dict, Any

class FormationError(Exception):
    """Base exception for Formation API errors"""
    pass

class AuthenticationError(FormationError):
    """Authentication failed"""
    pass

class RateLimitError(FormationError):
    """Rate limit exceeded"""
    def __init__(self, message, retry_after=None):
        super().__init__(message)
        self.retry_after = retry_after

class InsufficientCreditsError(FormationError):
    """Insufficient credits"""
    def __init__(self, message, required_credits=None, available_credits=None):
        super().__init__(message)
        self.required_credits = required_credits
        self.available_credits = available_credits

class ModelError(FormationError):
    """Model-related error"""
    pass

class RobustFormationClient:
    def __init__(self, private_key, max_retries=3, base_delay=1.0):
        self.client = FormationModelClient(private_key)
        self.max_retries = max_retries
        self.base_delay = base_delay
    
    def _handle_error_response(self, response):
        """Convert HTTP error responses to appropriate exceptions"""
        try:
            error_data = response.json()
            error = error_data.get("error", {})
            code = error.get("code", "UNKNOWN_ERROR")
            message = error.get("message", "Unknown error occurred")
            
            if response.status_code == 401:
                raise AuthenticationError(message)
            elif response.status_code == 429:
                retry_after = error.get("details", {}).get("retry_after", 60)
                raise RateLimitError(message, retry_after)
            elif code == "INSUFFICIENT_CREDITS":
                details = error.get("details", {})
                raise InsufficientCreditsError(
                    message,
                    details.get("required_credits"),
                    details.get("available_credits")
                )
            elif code in ["MODEL_NOT_FOUND", "MODEL_UNAVAILABLE"]:
                raise ModelError(message)
            else:
                raise FormationError(f"{code}: {message}")
                
        except ValueError:  # JSON decode error
            raise FormationError(f"HTTP {response.status_code}: {response.text}")
    
    def _should_retry(self, exception, attempt):
        """Determine if an error should be retried"""
        if attempt >= self.max_retries:
            return False
        
        # Retry on rate limits, server errors, and network issues
        if isinstance(exception, (RateLimitError, ConnectionError)):
            return True
        
        if isinstance(exception, FormationError):
            return False  # Don't retry client errors
        
        return True  # Retry on other exceptions
    
    def _calculate_delay(self, attempt, exception=None):
        """Calculate delay for exponential backoff"""
        if isinstance(exception, RateLimitError) and exception.retry_after:
            return exception.retry_after
        
        delay = self.base_delay * (2 ** attempt)
        jitter = random.uniform(0.1, 0.3) * delay
        return delay + jitter
    
    def chat_completion_with_retry(self, model_id, messages, **kwargs):
        """Chat completion with automatic retry logic"""
        last_exception = None
        
        for attempt in range(self.max_retries + 1):
            try:
                return self.client.chat_completion(model_id, messages, **kwargs)
                
            except Exception as e:
                last_exception = e
                
                # Convert HTTP errors to appropriate exceptions
                if hasattr(e, 'response'):
                    self._handle_error_response(e.response)
                
                if not self._should_retry(e, attempt):
                    raise e
                
                if attempt < self.max_retries:
                    delay = self._calculate_delay(attempt, e)
                    print(f"⚠️  Attempt {attempt + 1} failed: {e}")
                    print(f"🔄 Retrying in {delay:.2f} seconds...")
                    time.sleep(delay)
        
        raise last_exception
    
    def safe_chat_completion(self, model_id, messages, fallback_models=None, **kwargs):
        """Chat completion with fallback models"""
        models_to_try = [model_id]
        if fallback_models:
            models_to_try.extend(fallback_models)
        
        last_error = None
        
        for model in models_to_try:
            try:
                result = self.chat_completion_with_retry(model, messages, **kwargs)
                
                if model != model_id:
                    print(f"✅ Used fallback model: {model}")
                
                return result
                
            except Exception as e:
                last_error = e
                print(f"❌ Model {model} failed: {e}")
                
                # Don't try fallbacks for authentication or credit errors
                if isinstance(e, (AuthenticationError, InsufficientCreditsError)):
                    raise e
                
                continue
        
        raise last_error or FormationError("All models failed")

# Example usage with error handling
def robust_chat_example():
    """Example of robust chat with comprehensive error handling"""
    
    try:
        client = RobustFormationClient("0x1234567890abcdef...")
        
        messages = [
            {"role": "system", "content": "You are a helpful assistant."},
            {"role": "user", "content": "Explain machine learning briefly."}
        ]
        
        # Try with fallback models
        result = client.safe_chat_completion(
            "llama2-7b-chat",
            messages,
            fallback_models=["gpt-3.5-turbo", "claude-3-sonnet"],
            max_tokens=300
        )
        
        print("✅ Success!")
        print(f"Response: {result['choices'][0]['message']['content']}")
        print(f"Usage: {result['usage']}")
        
    except AuthenticationError as e:
        print(f"🔐 Authentication failed: {e}")
        print("Please check your private key and signature generation.")
        
    except InsufficientCreditsError as e:
        print(f"💳 Insufficient credits: {e}")
        print(f"Required: {e.required_credits}, Available: {e.available_credits}")
        print("Please add credits to your account.")
        
    except RateLimitError as e:
        print(f"⏱️  Rate limited: {e}")
        print(f"Retry after: {e.retry_after} seconds")
        
    except ModelError as e:
        print(f"🤖 Model error: {e}")
        print("The requested model may be unavailable or not found.")
        
    except FormationError as e:
        print(f"🚨 Formation API error: {e}")
        
    except Exception as e:
        print(f"💥 Unexpected error: {e}")

# Run the robust example
robust_chat_example()
```

---

## Production Examples

### 1. Production-Ready Chat Service

```python
import logging
import os
import time
from typing import Optional, Dict, Any, List
from dataclasses import dataclass
from datetime import datetime, timedelta
import redis
import json

# Configure logging
logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s - %(name)s - %(levelname)s - %(message)s'
)
logger = logging.getLogger(__name__)

@dataclass
class ChatSession:
    session_id: str
    user_id: str
    model_id: str
    conversation: List[Dict[str, str]]
    created_at: datetime
    last_activity: datetime
    total_tokens: int = 0
    total_cost: float = 0.0

class ProductionChatService:
    def __init__(self, private_key: str, redis_url: str = "redis://localhost:6379"):
        self.client = RobustFormationClient(private_key)
        self.redis_client = redis.from_url(redis_url)
        self.session_timeout = timedelta(hours=1)
        
        # Rate limiting (requests per minute per user)
        self.rate_limit = 60
        
        logger.info("Production chat service initialized")
    
    def _get_rate_limit_key(self, user_id: str) -> str:
        """Get Redis key for rate limiting"""
        minute = int(time.time() / 60)
        return f"rate_limit:{user_id}:{minute}"
    
    def _check_rate_limit(self, user_id: str) -> bool:
        """Check if user is within rate limits"""
        key = self._get_rate_limit_key(user_id)
        current = self.redis_client.get(key)
        
        if current is None:
            self.redis_client.setex(key, 60, 1)
            return True
        
        if int(current) >= self.rate_limit:
            return False
        
        self.redis_client.incr(key)
        return True
    
    def _get_session_key(self, session_id: str) -> str:
        """Get Redis key for session storage"""
        return f"chat_session:{session_id}"
    
    def _save_session(self, session: ChatSession):
        """Save session to Redis"""
        key = self._get_session_key(session.session_id)
        data = {
            "session_id": session.session_id,
            "user_id": session.user_id,
            "model_id": session.model_id,
            "conversation": session.conversation,
            "created_at": session.created_at.isoformat(),
            "last_activity": session.last_activity.isoformat(),
            "total_tokens": session.total_tokens,
            "total_cost": session.total_cost
        }
        
        self.redis_client.setex(
            key,
            int(self.session_timeout.total_seconds()),
            json.dumps(data)
        )
    
    def _load_session(self, session_id: str) -> Optional[ChatSession]:
        """Load session from Redis"""
        key = self._get_session_key(session_id)
        data = self.redis_client.get(key)
        
        if not data:
            return None
        
        try:
            session_data = json.loads(data)
            return ChatSession(
                session_id=session_data["session_id"],
                user_id=session_data["user_id"],
                model_id=session_data["model_id"],
                conversation=session_data["conversation"],
                created_at=datetime.fromisoformat(session_data["created_at"]),
                last_activity=datetime.fromisoformat(session_data["last_activity"]),
                total_tokens=session_data["total_tokens"],
                total_cost=session_data["total_cost"]
            )
        except (json.JSONDecodeError, KeyError) as e:
            logger.error(f"Failed to load session {session_id}: {e}")
            return None
    
    def create_session(self, user_id: str, model_id: str, system_prompt: str = None) -> str:
        """Create a new chat session"""
        import uuid
        
        session_id = str(uuid.uuid4())
        now = datetime.now()
        
        conversation = []
        if system_prompt:
            conversation.append({"role": "system", "content": system_prompt})
        
        session = ChatSession(
            session_id=session_id,
            user_id=user_id,
            model_id=model_id,
            conversation=conversation,
            created_at=now,
            last_activity=now
        )
        
        self._save_session(session)
        
        logger.info(f"Created session {session_id} for user {user_id}")
        return session_id
    
    def send_message(self, session_id: str, message: str, **kwargs) -> Dict[str, Any]:
        """Send a message in a chat session"""
        
        # Load session
        session = self._load_session(session_id)
        if not session:
            raise ValueError(f"Session {session_id} not found or expired")
        
        # Check rate limits
        if not self._check_rate_limit(session.user_id):
            raise RateLimitError(f"Rate limit exceeded for user {session.user_id}")
        
        try:
            # Add user message to conversation
            session.conversation.append({"role": "user", "content": message})
            
            # Get AI response
            result = self.client.chat_completion_with_retry(
                session.model_id,
                session.conversation,
                max_tokens=kwargs.get("max_tokens", 500),
                temperature=kwargs.get("temperature", 0.7)
            )
            
            ai_response = result["choices"][0]["message"]["content"]
            usage = result["usage"]
            
            # Add AI response to conversation
            session.conversation.append({"role": "assistant", "content": ai_response})
            
            # Update session metrics
            session.last_activity = datetime.now()
            session.total_tokens += usage["total_tokens"]
            session.total_cost += usage.get("cost_credits", 0)
            
            # Save updated session
            self._save_session(session)
            
            # Log the interaction
            logger.info(f"Session {session_id}: {usage['total_tokens']} tokens, {usage.get('cost_credits', 0)} credits")
            
            return {
                "success": True,
                "response": ai_response,
                "usage": usage,
                "session_stats": {
                    "total_tokens": session.total_tokens,
                    "total_cost": session.total_cost,
                    "message_count": len([m for m in session.conversation if m["role"] != "system"])
                }
            }
            
        except Exception as e:
            logger.error(f"Error in session {session_id}: {e}")
            raise
    
    def get_session_info(self, session_id: str) -> Optional[Dict[str, Any]]:
        """Get session information"""
        session = self._load_session(session_id)
        if not session:
            return None
        
        return {
            "session_id": session.session_id,
            "user_id": session.user_id,
            "model_id": session.model_id,
            "created_at": session.created_at.isoformat(),
            "last_activity": session.last_activity.isoformat(),
            "message_count": len([m for m in session.conversation if m["role"] != "system"]),
            "total_tokens": session.total_tokens,
            "total_cost": session.total_cost
        }
    
    def delete_session(self, session_id: str) -> bool:
        """Delete a chat session"""
        key = self._get_session_key(session_id)
        deleted = self.redis_client.delete(key)
        
        if deleted:
            logger.info(f"Deleted session {session_id}")
        
        return bool(deleted)

# Example usage
def production_example():
    """Example of production chat service usage"""
    
    # Initialize service
    service = ProductionChatService(
        private_key=os.getenv("FORMATION_PRIVATE_KEY"),
        redis_url=os.getenv("REDIS_URL", "redis://localhost:6379")
    )
    
    try:
        # Create a new session
        session_id = service.create_session(
            user_id="user_123",
            model_id="llama2-7b-chat",
            system_prompt="You are a helpful customer service assistant."
        )
        
        print(f"Created session: {session_id}")
        
        # Send messages
        messages = [
            "Hello, I need help with my account",
            "I forgot my password",
            "How can I reset it?"
        ]
        
        for message in messages:
            print(f"\n👤 User: {message}")
            
            result = service.send_message(session_id, message)
            
            if result["success"]:
                print(f"🤖 AI: {result['response']}")
                print(f"💰 Usage: {result['usage']['total_tokens']} tokens, {result['usage'].get('cost_credits', 0)} credits")
                print(f"📊 Session stats: {result['session_stats']}")
            else:
                print(f"❌ Error: {result.get('error')}")
        
        # Get session info
        info = service.get_session_info(session_id)
        print(f"\n📋 Session info: {info}")
        
        # Clean up
        service.delete_session(session_id)
        print(f"🗑️  Deleted session: {session_id}")
        
    except Exception as e:
        print(f"💥 Error: {e}")

if __name__ == "__main__":
    production_example()
```

### 2. Monitoring and Analytics

```python
import time
import json
from datetime import datetime, timedelta
from collections import defaultdict, Counter
from typing import Dict, List, Any
import matplotlib.pyplot as plt
import pandas as pd

class FormationAnalytics:
    def __init__(self, redis_client):
        self.redis = redis_client
    
    def log_request(self, user_id: str, model_id: str, tokens: int, cost: float, duration: float, success: bool):
        """Log a request for analytics"""
        timestamp = int(time.time())
        
        log_entry = {
            "timestamp": timestamp,
            "user_id": user_id,
            "model_id": model_id,
            "tokens": tokens,
            "cost": cost,
            "duration": duration,
            "success": success
        }
        
        # Store in Redis with TTL of 30 days
        key = f"analytics:request:{timestamp}:{user_id}"
        self.redis.setex(key, 30 * 24 * 3600, json.dumps(log_entry))
        
        # Update counters
        date_key = datetime.fromtimestamp(timestamp).strftime("%Y-%m-%d")
        self.redis.hincrby(f"analytics:daily:{date_key}", "total_requests", 1)
        self.redis.hincrby(f"analytics:daily:{date_key}", "total_tokens", tokens)
        self.redis.hincrby(f"analytics:daily:{date_key}", "total_cost", int(cost * 100))  # Store as cents
        
        if success:
            self.redis.hincrby(f"analytics:daily:{date_key}", "successful_requests", 1)
        else:
            self.redis.hincrby(f"analytics:daily:{date_key}", "failed_requests", 1)
        
        # Model-specific counters
        self.redis.hincrby(f"analytics:model:{model_id}:{date_key}", "requests", 1)
        self.redis.hincrby(f"analytics:model:{model_id}:{date_key}", "tokens", tokens)
    
    def get_daily_stats(self, date: str) -> Dict[str, Any]:
        """Get daily statistics"""
        key = f"analytics:daily:{date}"
        data = self.redis.hgetall(key)
        
        if not data:
            return {}
        
        return {
            "date": date,
            "total_requests": int(data.get(b"total_requests", 0)),
            "successful_requests": int(data.get(b"successful_requests", 0)),
            "failed_requests": int(data.get(b"failed_requests", 0)),
            "total_tokens": int(data.get(b"total_tokens", 0)),
            "total_cost": int(data.get(b"total_cost", 0)) / 100.0,  # Convert back from cents
            "success_rate": int(data.get(b"successful_requests", 0)) / max(int(data.get(b"total_requests", 1)), 1)
        }
    
    def get_model_stats(self, model_id: str, date: str) -> Dict[str, Any]:
        """Get model-specific statistics"""
        key = f"analytics:model:{model_id}:{date}"
        data = self.redis.hgetall(key)
        
        if not data:
            return {}
        
        return {
            "model_id": model_id,
            "date": date,
            "requests": int(data.get(b"requests", 0)),
            "tokens": int(data.get(b"tokens", 0))
        }
    
    def generate_report(self, days: int = 7) -> Dict[str, Any]:
        """Generate a comprehensive analytics report"""
        end_date = datetime.now()
        start_date = end_date - timedelta(days=days)
        
        daily_stats = []
        total_requests = 0
        total_tokens = 0
        total_cost = 0.0
        
        # Collect daily stats
        current_date = start_date
        while current_date <= end_date:
            date_str = current_date.strftime("%Y-%m-%d")
            stats = self.get_daily_stats(date_str)
            
            if stats:
                daily_stats.append(stats)
                total_requests += stats["total_requests"]
                total_tokens += stats["total_tokens"]
                total_cost += stats["total_cost"]
            
            current_date += timedelta(days=1)
        
        # Calculate averages
        avg_requests_per_day = total_requests / days if days > 0 else 0
        avg_tokens_per_request = total_tokens / total_requests if total_requests > 0 else 0
        avg_cost_per_request = total_cost / total_requests if total_requests > 0 else 0
        
        return {
            "period": {
                "start_date": start_date.strftime("%Y-%m-%d"),
                "end_date": end_date.strftime("%Y-%m-%d"),
                "days": days
            },
            "totals": {
                "requests": total_requests,
                "tokens": total_tokens,
                "cost": total_cost
            },
            "averages": {
                "requests_per_day": avg_requests_per_day,
                "tokens_per_request": avg_tokens_per_request,
                "cost_per_request": avg_cost_per_request
            },
            "daily_stats": daily_stats
        }
    
    def plot_usage_trends(self, days: int = 30):
        """Plot usage trends over time"""
        report = self.generate_report(days)
        
        if not report["daily_stats"]:
            print("No data available for plotting")
            return
        
        df = pd.DataFrame(report["daily_stats"])
        df["date"] = pd.to_datetime(df["date"])
        
        fig, axes = plt.subplots(2, 2, figsize=(15, 10))
        
        # Requests over time
        axes[0, 0].plot(df["date"], df["total_requests"], marker='o')
        axes[0, 0].set_title("Daily Requests")
        axes[0, 0].set_ylabel("Requests")
        axes[0, 0].tick_params(axis='x', rotation=45)
        
        # Tokens over time
        axes[0, 1].plot(df["date"], df["total_tokens"], marker='o', color='orange')
        axes[0, 1].set_title("Daily Tokens")
        axes[0, 1].set_ylabel("Tokens")
        axes[0, 1].tick_params(axis='x', rotation=45)
        
        # Cost over time
        axes[1, 0].plot(df["date"], df["total_cost"], marker='o', color='green')
        axes[1, 0].set_title("Daily Cost")
        axes[1, 0].set_ylabel("Cost (Credits)")
        axes[1, 0].tick_params(axis='x', rotation=45)
        
        # Success rate over time
        axes[1, 1].plot(df["date"], df["success_rate"], marker='o', color='red')
        axes[1, 1].set_title("Success Rate")
        axes[1, 1].set_ylabel("Success Rate")
        axes[1, 1].set_ylim(0, 1)
        axes[1, 1].tick_params(axis='x', rotation=45)
        
        plt.tight_layout()
        plt.savefig("formation_usage_trends.png", dpi=300, bbox_inches='tight')
        plt.show()

# Example usage with monitoring
class MonitoredFormationClient(RobustFormationClient):
    def __init__(self, private_key, redis_client):
        super().__init__(private_key)
        self.analytics = FormationAnalytics(redis_client)
    
    def chat_completion_with_monitoring(self, user_id: str, model_id: str, messages: List[Dict[str, str]], **kwargs):
        """Chat completion with monitoring and analytics"""
        start_time = time.time()
        success = False
        tokens = 0
        cost = 0.0
        
        try:
            result = self.chat_completion_with_retry(model_id, messages, **kwargs)
            
            success = True
            usage = result["usage"]
            tokens = usage["total_tokens"]
            cost = usage.get("cost_credits", 0)
            
            return result
            
        except Exception as e:
            logger.error(f"Request failed for user {user_id}: {e}")
            raise
            
        finally:
            duration = time.time() - start_time
            
            # Log the request for analytics
            self.analytics.log_request(
                user_id=user_id,
                model_id=model_id,
                tokens=tokens,
                cost=cost,
                duration=duration,
                success=success
            )

# Example usage
def monitoring_example():
    """Example with monitoring and analytics"""
    import redis
    
    redis_client = redis.Redis(host='localhost', port=6379, db=0)
    client = MonitoredFormationClient("0x1234567890abcdef...", redis_client)
    
    # Simulate some requests
    for i in range(10):
        try:
            result = client.chat_completion_with_monitoring(
                user_id=f"user_{i % 3}",  # 3 different users
                model_id="llama2-7b-chat",
                messages=[
                    {"role": "system", "content": "You are a helpful assistant."},
                    {"role": "user", "content": f"Tell me about topic {i}"}
                ]
            )
            print(f"✅ Request {i}: {result['usage']['total_tokens']} tokens")
            
        except Exception as e:
            print(f"❌ Request {i} failed: {e}")
        
        time.sleep(1)  # Rate limiting
    
    # Generate analytics report
    analytics = FormationAnalytics(redis_client)
    report = analytics.generate_report(days=1)
    
    print("\n📊 Analytics Report:")
    print(f"Total requests: {report['totals']['requests']}")
    print(f"Total tokens: {report['totals']['tokens']}")
    print(f"Total cost: {report['totals']['cost']} credits")
    print(f"Average tokens per request: {report['averages']['tokens_per_request']:.1f}")
    print(f"Average cost per request: {report['averages']['cost_per_request']:.3f} credits")

if __name__ == "__main__":
    monitoring_example()
```

---

## Next Steps

- **[Inference Guide](inference-guide.md)** - Getting started with model inference
- **[API Reference](api-reference.md)** - Complete API documentation
- **[Agent Examples](../agents/examples.md)** - Learn about using AI agents

---

**Ready to integrate Formation models into your applications?** Start with the basic examples and gradually add more sophisticated features like error handling, monitoring, and production-ready patterns! 🚀 