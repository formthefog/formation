# AI Marketplace Registry Implementation Plan

## Focus: Registering Agents and Models in form-state

This document outlines the implementation plan for adding support for AI models and agents to the form-state datastore. This is a critical foundation for the AI Agent and Model Marketplace, enabling the registration, discovery, and management of AI assets in the network.

## Overview

We will extend the form-state CRDT datastore to include new data types for AI models and agents while following existing patterns. The implementation will include:

1. Data structure definitions
2. CRDT operations integration
3. API endpoints for registration and querying
4. Basic validation rules

## Data Structures

### 1. Model Structure

```rust
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ModelType {
    LLM,
    Embedding,
    Multimodal,
    AudioProcessing,
    ImageGeneration,
    Other(String),
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ModelFramework {
    PyTorch,
    TensorFlow,
    ONNX,
    JAX,
    CoreML,
    Other(String),
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum QuantizationType {
    FP32,
    FP16,
    BF16,
    INT8,
    INT4,
    GPTQ,
    GGUF,
    AWQ,
    Other(String),
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AIModel {
    pub model_id: String,
    pub name: String,
    pub owner_id: String,
    pub version: String,
    pub description: String,
    pub license: String,
    pub model_type: ModelType,
    pub framework: ModelFramework,
    pub parameters: u64,
    pub quantization: Option<QuantizationType>,
    pub context_length: Option<u32>,
    pub input_tokens_per_second: Option<u32>,
    pub output_tokens_per_second: Option<u32>,
    pub tags: Vec<String>,
    pub created_at: i64,
    pub updated_at: i64,
    pub formfile_template: String,
    pub resource_requirements: ModelResourceRequirements,
    pub capabilities: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ModelResourceRequirements {
    pub min_vcpus: u8,
    pub recommended_vcpus: u8,
    pub min_memory_mb: u64,
    pub recommended_memory_mb: u64,
    pub min_disk_gb: u64,
    pub recommended_disk_gb: u64,
    pub requires_gpu: bool,
    pub min_vram_gb: Option<u64>,
    pub cuda_cores: Option<u32>,
    pub tensor_cores: Option<u32>,
}

impl Sha3Hash for AIModel {
    fn hash(&self, hasher: &mut tiny_keccak::Sha3) {
        hasher.update(&bincode::serialize(self).unwrap());
    }
}

impl Default for AIModel {
    fn default() -> Self {
        let null_hash = [0u8; 32];
        let null_hex = hex::encode(null_hash);
        Self {
            model_id: null_hex.clone(),
            name: String::new(),
            owner_id: null_hex.clone(),
            version: "0.1.0".to_string(),
            description: String::new(),
            license: "MIT".to_string(),
            model_type: ModelType::LLM,
            framework: ModelFramework::PyTorch,
            parameters: 0,
            quantization: None,
            context_length: None,
            input_tokens_per_second: None,
            output_tokens_per_second: None,
            tags: Vec::new(),
            created_at: 0,
            updated_at: 0,
            formfile_template: String::new(),
            resource_requirements: ModelResourceRequirements::default(),
            capabilities: Vec::new(),
        }
    }
}

impl Default for ModelResourceRequirements {
    fn default() -> Self {
        Self {
            min_vcpus: 1,
            recommended_vcpus: 2,
            min_memory_mb: 1024,
            recommended_memory_mb: 4096,
            min_disk_gb: 10,
            recommended_disk_gb: 20,
            requires_gpu: false,
            min_vram_gb: None,
            cuda_cores: None,
            tensor_cores: None,
        }
    }
}
```

### 2. Agent Structure

```rust
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AgentFramework {
    LangChain,
    AutoGPT,
    CrewAI,
    LlamaIndex,
    BabyAGI,
    CustomRust,
    CustomPython,
    CustomJS,
    Other(String),
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AgentType {
    Chatbot,
    Assistant,
    DataAnalyst,
    CodeGenerator,
    ContentCreator,
    Researcher,
    Other(String),
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AIAgent {
    pub agent_id: String,
    pub name: String,
    pub owner_id: String,
    pub version: String,
    pub description: String,
    pub license: String,
    pub agent_type: AgentType,
    pub framework: AgentFramework,
    pub compatible_model_types: Vec<ModelType>,
    pub preferred_models: Vec<String>, // model_ids
    pub tags: Vec<String>,
    pub created_at: i64,
    pub updated_at: i64,
    pub formfile_template: String,
    pub resource_requirements: AgentResourceRequirements,
    pub capabilities: Vec<String>,
    pub tools: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AgentResourceRequirements {
    pub min_vcpus: u8,
    pub recommended_vcpus: u8,
    pub min_memory_mb: u64,
    pub recommended_memory_mb: u64,
    pub min_disk_gb: u64,
    pub recommended_disk_gb: u64,
}

impl Sha3Hash for AIAgent {
    fn hash(&self, hasher: &mut tiny_keccak::Sha3) {
        hasher.update(&bincode::serialize(self).unwrap());
    }
}

impl Default for AIAgent {
    fn default() -> Self {
        let null_hash = [0u8; 32];
        let null_hex = hex::encode(null_hash);
        Self {
            agent_id: null_hex.clone(),
            name: String::new(),
            owner_id: null_hex.clone(),
            version: "0.1.0".to_string(),
            description: String::new(),
            license: "MIT".to_string(),
            agent_type: AgentType::Assistant,
            framework: AgentFramework::LangChain,
            compatible_model_types: vec![ModelType::LLM],
            preferred_models: Vec::new(),
            tags: Vec::new(),
            created_at: 0,
            updated_at: 0,
            formfile_template: String::new(),
            resource_requirements: AgentResourceRequirements::default(),
            capabilities: Vec::new(),
            tools: Vec::new(),
        }
    }
}

impl Default for AgentResourceRequirements {
    fn default() -> Self {
        Self {
            min_vcpus: 1,
            recommended_vcpus: 1,
            min_memory_mb: 512,
            recommended_memory_mb: 1024,
            min_disk_gb: 5,
            recommended_disk_gb: 10,
        }
    }
}
```

### 3. CRDT Operations

Following the pattern in form-state, we need to add operation types for models and agents:

```rust
pub type ModelOp = Op<String, BFTReg<AIModel, Actor>, Actor>;
pub type AgentOp = Op<String, BFTReg<AIAgent, Actor>, Actor>;

// Add to existing operation enum if one exists
pub enum DatastoreOp {
    // Existing variants...
    Model(ModelOp),
    Agent(AgentOp),
}
```

## Implementation Tasks

### 1. Add Data Structures to form-state (2 days)

1. Create new file `models.rs` in form-state/src/
2. Create new file `agents.rs` in form-state/src/
3. Add data structures defined above
4. Add module declarations to lib.rs
5. Implement Sha3Hash trait for both models

### 2. Extend Datastore for Models and Agents (3 days)

1. Add model and agent maps to the Datastore struct
```rust
pub struct Datastore {
    // Existing fields...
    pub models: Map<String, BFTReg<AIModel, Actor>, Actor>,
    pub agents: Map<String, BFTReg<AIAgent, Actor>, Actor>,
}
```

2. Implement methods for model operations:
```rust
impl Datastore {
    // Existing methods...
    
    pub fn add_model(&mut self, model: AIModel, actor: Actor) -> Option<ModelOp> {
        // Implementation following existing patterns
    }
    
    pub fn update_model(&mut self, model_id: &str, model: AIModel, actor: Actor) -> Option<ModelOp> {
        // Implementation following existing patterns
    }
    
    pub fn get_model(&self, model_id: &str) -> Option<&AIModel> {
        // Implementation following existing patterns
    }
    
    pub fn list_models(&self) -> Vec<&AIModel> {
        // Implementation following existing patterns
    }
    
    pub fn delete_model(&mut self, model_id: &str, actor: Actor) -> Option<ModelOp> {
        // Implementation following existing patterns
    }
}
```

3. Implement parallel methods for agent operations (same pattern)

### 3. Add API Endpoints (2 days)

1. Create API handlers for model operations:
```rust
// In main.rs or appropriate routing file
app.at("/models")
   .post(|req: Request<AppState>| async move {
       // Handle model creation
   });

app.at("/models/:id")
   .get(|req: Request<AppState>| async move {
       // Handle model retrieval  
   })
   .put(|req: Request<AppState>| async move {
       // Handle model update
   })
   .delete(|req: Request<AppState>| async move {
       // Handle model deletion
   });

app.at("/models/search")
   .post(|req: Request<AppState>| async move {
       // Handle model searching
   });
```

2. Create parallel API routes for agent operations (same pattern)

### 4. Implement Validation Logic (2 days)

1. Add validation functions to ensure data integrity:
```rust
impl AIModel {
    pub fn validate(&self) -> Result<(), ValidationError> {
        // Validation rules for model data
    }
}

impl AIAgent {
    pub fn validate(&self) -> Result<(), ValidationError> {
        // Validation rules for agent data
    }
}
```

2. Integrate validation into API handlers and datastore operations

### 5. Write Tests (1 day)

1. Write unit tests for model and agent data structures
2. Write integration tests for API endpoints
3. Test CRDT operations for conflict resolution

## Total Estimated Time: 10 working days

## Implementation Flow

1. 🟢 **Day 1-2:** Define data structures and add to codebase
2. 🟢 **Day 3-5:** Implement datastore extensions and CRDT operations
3. 🟢 **Day 6-7:** Create API endpoints for registration and querying
4. 🟢 **Day 8-9:** Add validation logic and error handling
5. 🟢 **Day 10:** Write tests and documentation

## Next Steps After Completion

After this foundation is in place, we can proceed with:

1. Enhancing the API with more advanced query capabilities
2. Implementing the template system for models and agents
3. Building the build distribution system for AI workloads 