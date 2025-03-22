# AI Marketplace Type Definitions

This document provides detailed specifications for the core data types needed in the AI Agent and Model Marketplace. These types will be integrated into the form-state CRDT datastore.

## AIModel Structure

The AIModel struct represents an AI model registered in the marketplace, with comprehensive metadata to support discovery, deployment, and usage.

```rust
/// Represents the type/category of AI model
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ModelType {
    /// Large Language Models (text generation)
    LLM,
    /// Text embedding models
    Embedding,
    /// Models handling multiple modalities (text, image, audio)
    Multimodal,
    /// Audio processing models (speech-to-text, text-to-speech)
    AudioProcessing,
    /// Image generation models
    ImageGeneration,
    /// Image understanding/vision models
    ComputerVision,
    /// Diffusion models
    Diffusion,
    /// Any other model type not covered above
    Other(String),
}

/// Represents the ML framework used by the model
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ModelFramework {
    PyTorch,
    TensorFlow,
    ONNX,
    JAX,
    CoreML,
    TensorRT,
    Other(String),
}

/// Represents the quantization approach used (if any)
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
    GGML,
    Other(String),
}

/// Represents licensing options for models
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ModelLicense {
    MIT,
    Apache2,
    GPL3,
    BSD,
    CC_BY,
    CC_BY_SA,
    CC_BY_NC,
    CC_BY_NC_SA,
    Proprietary,
    Custom(String),
}

/// Represents various input/output modes supported by a model
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ModelIOMode {
    TextToText,
    TextToImage,
    ImageToText,
    TextToAudio,
    AudioToText,
    ImageToImage,
    Other(String),
}

/// Main AIModel struct representing a registered model in the marketplace
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AIModel {
    /// Unique identifier for the model
    pub model_id: String,
    
    /// Human-readable name
    pub name: String,
    
    /// Owner's account ID
    pub owner_id: String,
    
    /// Semantic version (e.g., "1.2.3")
    pub version: String,
    
    /// Concise description of the model
    pub description: String,
    
    /// Detailed markdown documentation
    pub documentation: Option<String>,
    
    /// License type
    pub license: ModelLicense,
    
    /// Primary model type
    pub model_type: ModelType,
    
    /// Framework used
    pub framework: ModelFramework,
    
    /// Input/output modes supported
    pub io_modes: Vec<ModelIOMode>,
    
    /// Parameter count (in billions)
    pub parameters: f64,
    
    /// Quantization used (if any)
    pub quantization: Option<QuantizationType>,
    
    /// Maximum token context length
    pub context_length: Option<u32>,
    
    /// Average input tokens processed per second (benchmark)
    pub input_tokens_per_second: Option<u32>,
    
    /// Average output tokens generated per second (benchmark)
    pub output_tokens_per_second: Option<u32>,
    
    /// Searchable tags
    pub tags: Vec<String>,
    
    /// Creation timestamp
    pub created_at: i64,
    
    /// Last update timestamp
    pub updated_at: i64,
    
    /// Base64 encoded Formfile template
    pub formfile_template: String,
    
    /// URL to model weights or registry location
    pub weights_url: Option<String>,
    
    /// SHA-256 checksum of weights file
    pub weights_checksum: Option<String>,
    
    /// Size of weights file in bytes
    pub weights_size_bytes: Option<u64>,
    
    /// Resource requirements for deployment
    pub resource_requirements: ModelResourceRequirements,
    
    /// List of specific capabilities this model offers
    pub capabilities: Vec<String>,
    
    /// Average rating (1-5)
    pub average_rating: Option<f32>,
    
    /// Number of deployments
    pub deployment_count: u64,
    
    /// Download/usage count
    pub usage_count: u64,
    
    /// Whether this model is featured/verified
    pub is_featured: bool,
    
    /// Whether this is a private model (only visible to owner and authorized users)
    pub is_private: bool,
    
    /// Key-value store for arbitrary metadata
    pub metadata: BTreeMap<String, String>,
    
    /// Repository URL (if open source)
    pub repository_url: Option<String>,
    
    /// Demo URL (if available)
    pub demo_url: Option<String>,
    
    /// Paper URL (if academic)
    pub paper_url: Option<String>,
    
    /// Base price per 1000 tokens (if commercial)
    pub price_per_1k_tokens: Option<f64>,
    
    /// Usage tracking settings
    pub usage_tracking: ModelUsageTracking,
}

/// Specifies the computing resources required to run a model
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ModelResourceRequirements {
    /// Minimum CPU cores required
    pub min_vcpus: u8,
    
    /// Recommended CPU cores for optimal performance
    pub recommended_vcpus: u8,
    
    /// Minimum RAM required (MB)
    pub min_memory_mb: u64,
    
    /// Recommended RAM for optimal performance (MB)
    pub recommended_memory_mb: u64,
    
    /// Minimum disk space required (GB)
    pub min_disk_gb: u64,
    
    /// Recommended disk space (GB)
    pub recommended_disk_gb: u64,
    
    /// Whether GPU is required
    pub requires_gpu: bool,
    
    /// Minimum VRAM required if GPU is used (GB)
    pub min_vram_gb: Option<u64>,
    
    /// Recommended VRAM for optimal performance (GB)
    pub recommended_vram_gb: Option<u64>,
    
    /// Required CUDA cores (if applicable)
    pub cuda_cores: Option<u32>,
    
    /// Required Tensor cores (if applicable)
    pub tensor_cores: Option<u32>,
    
    /// Required CPU extensions (AVX, AVX2, etc.)
    pub required_cpu_extensions: Vec<String>,
    
    /// Required CUDA version (if applicable)
    pub required_cuda_version: Option<String>,
}

/// Specifies how usage is tracked for a model
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ModelUsageTracking {
    /// Whether to track token usage
    pub track_tokens: bool,
    
    /// Whether to track inference requests
    pub track_requests: bool,
    
    /// Whether to compute royalties
    pub enable_royalties: bool,
    
    /// Percentage of revenue that goes to the creator (0-100)
    pub royalty_percentage: u8,
    
    /// Additional usage metrics to track
    pub custom_metrics: Vec<String>,
}

impl Sha3Hash for AIModel {
    fn hash(&self, hasher: &mut tiny_keccak::Sha3) {
        hasher.update(&bincode::serialize(self).unwrap());
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
            recommended_vram_gb: None,
            cuda_cores: None,
            tensor_cores: None,
            required_cpu_extensions: Vec::new(),
            required_cuda_version: None,
        }
    }
}

impl Default for ModelUsageTracking {
    fn default() -> Self {
        Self {
            track_tokens: true,
            track_requests: true,
            enable_royalties: false,
            royalty_percentage: 0,
            custom_metrics: Vec::new(),
        }
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
            documentation: None,
            license: ModelLicense::MIT,
            model_type: ModelType::LLM,
            framework: ModelFramework::PyTorch,
            io_modes: vec![ModelIOMode::TextToText],
            parameters: 0.0,
            quantization: None,
            context_length: None,
            input_tokens_per_second: None,
            output_tokens_per_second: None,
            tags: Vec::new(),
            created_at: 0,
            updated_at: 0,
            formfile_template: String::new(),
            weights_url: None,
            weights_checksum: None,
            weights_size_bytes: None,
            resource_requirements: ModelResourceRequirements::default(),
            capabilities: Vec::new(),
            average_rating: None,
            deployment_count: 0,
            usage_count: 0,
            is_featured: false,
            is_private: false,
            metadata: BTreeMap::new(),
            repository_url: None,
            demo_url: None,
            paper_url: None,
            price_per_1k_tokens: None,
            usage_tracking: ModelUsageTracking::default(),
        }
    }
}
```

## AIAgent Structure

The AIAgent struct represents an AI agent that can be deployed and potentially use models from the marketplace.

```rust
/// Represents the framework/platform used to build the agent
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AgentFramework {
    LangChain,
    AutoGPT,
    CrewAI,
    LlamaIndex,
    BabyAGI,
    AgentGPT,
    FormationAgent,
    CustomRust,
    CustomPython,
    CustomJS,
    Other(String),
}

/// Represents the category/type of agent
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AgentType {
    Chatbot,
    Assistant,
    DataAnalyst,
    CodeGenerator,
    ContentCreator,
    Researcher,
    AutomationAgent,
    MultiAgent,
    Copilot,
    Other(String),
}

/// Represents the runtime environment for the agent
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AgentRuntime {
    Python,
    NodeJS,
    Rust,
    Go,
    Java,
    Docker,
    WebAssembly,
    Other(String),
}

/// Represents a tool that an agent can use
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AgentTool {
    /// Tool identifier
    pub id: String,
    
    /// Human-readable name
    pub name: String,
    
    /// Description of what the tool does
    pub description: String,
    
    /// Tool category/type
    pub tool_type: String,
    
    /// Authentication requirements (if any)
    pub auth_required: bool,
    
    /// Configuration template (JSON schema)
    pub config_schema: Option<String>,
}

/// Main AIAgent struct representing a registered agent in the marketplace
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AIAgent {
    /// Unique identifier for the agent
    pub agent_id: String,
    
    /// Human-readable name
    pub name: String,
    
    /// Owner's account ID
    pub owner_id: String,
    
    /// Semantic version (e.g., "1.2.3")
    pub version: String,
    
    /// Concise description of the agent
    pub description: String,
    
    /// Detailed markdown documentation
    pub documentation: Option<String>,
    
    /// License type
    pub license: ModelLicense,
    
    /// Agent category/type
    pub agent_type: AgentType,
    
    /// Framework/platform used
    pub framework: AgentFramework,
    
    /// Runtime environment
    pub runtime: AgentRuntime,
    
    /// Model types this agent is compatible with
    pub compatible_model_types: Vec<ModelType>,
    
    /// Specific models this agent works best with (model_ids)
    pub preferred_models: Vec<String>,
    
    /// Whether this agent requires a specific model
    pub requires_specific_model: bool,
    
    /// Specific model ID required (if applicable)
    pub required_model_id: Option<String>,
    
    /// Searchable tags
    pub tags: Vec<String>,
    
    /// Creation timestamp
    pub created_at: i64,
    
    /// Last update timestamp
    pub updated_at: i64,
    
    /// Base64 encoded Formfile template
    pub formfile_template: String,
    
    /// Resource requirements for deployment
    pub resource_requirements: AgentResourceRequirements,
    
    /// List of specific capabilities this agent offers
    pub capabilities: Vec<String>,
    
    /// Tools this agent can use
    pub tools: Vec<AgentTool>,
    
    /// Whether the agent has persistent memory/state
    pub has_memory: bool,
    
    /// Whether the agent can access external APIs
    pub has_external_api_access: bool,
    
    /// Whether the agent can access the Internet
    pub has_internet_access: bool,
    
    /// Whether the agent has filesystem access
    pub has_filesystem_access: bool,
    
    /// Average rating (1-5)
    pub average_rating: Option<f32>,
    
    /// Number of deployments
    pub deployment_count: u64,
    
    /// Usage count
    pub usage_count: u64,
    
    /// Whether this agent is featured/verified
    pub is_featured: bool,
    
    /// Whether this is a private agent (only visible to owner and authorized users)
    pub is_private: bool,
    
    /// Key-value store for arbitrary metadata
    pub metadata: BTreeMap<String, String>,
    
    /// Repository URL (if open source)
    pub repository_url: Option<String>,
    
    /// Demo URL (if available)
    pub demo_url: Option<String>,
    
    /// Base price per request (if commercial)
    pub price_per_request: Option<f64>,
    
    /// Usage tracking settings
    pub usage_tracking: AgentUsageTracking,
    
    /// Configuration schema for customization (JSON schema)
    pub config_schema: Option<String>,
}

/// Specifies the computing resources required to run an agent
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AgentResourceRequirements {
    /// Minimum CPU cores required
    pub min_vcpus: u8,
    
    /// Recommended CPU cores for optimal performance
    pub recommended_vcpus: u8,
    
    /// Minimum RAM required (MB)
    pub min_memory_mb: u64,
    
    /// Recommended RAM for optimal performance (MB)
    pub recommended_memory_mb: u64,
    
    /// Minimum disk space required (GB)
    pub min_disk_gb: u64,
    
    /// Recommended disk space (GB)
    pub recommended_disk_gb: u64,
    
    /// Whether GPU is required (excluding model requirements)
    pub requires_gpu: bool,
}

/// Specifies how usage is tracked for an agent
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AgentUsageTracking {
    /// Whether to track requests
    pub track_requests: bool,
    
    /// Whether to compute royalties
    pub enable_royalties: bool,
    
    /// Percentage of revenue that goes to the creator (0-100)
    pub royalty_percentage: u8,
    
    /// Additional usage metrics to track
    pub custom_metrics: Vec<String>,
}

impl Sha3Hash for AIAgent {
    fn hash(&self, hasher: &mut tiny_keccak::Sha3) {
        hasher.update(&bincode::serialize(self).unwrap());
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
            requires_gpu: false,
        }
    }
}

impl Default for AgentUsageTracking {
    fn default() -> Self {
        Self {
            track_requests: true,
            enable_royalties: false,
            royalty_percentage: 0,
            custom_metrics: Vec::new(),
        }
    }
}

impl Default for AgentTool {
    fn default() -> Self {
        Self {
            id: String::new(),
            name: String::new(),
            description: String::new(),
            tool_type: "generic".to_string(),
            auth_required: false,
            config_schema: None,
        }
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
            documentation: None,
            license: ModelLicense::MIT,
            agent_type: AgentType::Assistant,
            framework: AgentFramework::LangChain,
            runtime: AgentRuntime::Python,
            compatible_model_types: vec![ModelType::LLM],
            preferred_models: Vec::new(),
            requires_specific_model: false,
            required_model_id: None,
            tags: Vec::new(),
            created_at: 0,
            updated_at: 0,
            formfile_template: String::new(),
            resource_requirements: AgentResourceRequirements::default(),
            capabilities: Vec::new(),
            tools: Vec::new(),
            has_memory: false,
            has_external_api_access: false,
            has_internet_access: false,
            has_filesystem_access: false,
            average_rating: None,
            deployment_count: 0,
            usage_count: 0,
            is_featured: false,
            is_private: false,
            metadata: BTreeMap::new(),
            repository_url: None,
            demo_url: None,
            price_per_request: None,
            usage_tracking: AgentUsageTracking::default(),
            config_schema: None,
        }
    }
}
```

## ModelOp and AgentOp Types

These Operation types follow the CRDT pattern used in the existing codebase:

```rust
pub type ModelOp = Op<String, BFTReg<AIModel, Actor>, Actor>;
pub type AgentOp = Op<String, BFTReg<AIAgent, Actor>, Actor>;

// Request Types
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ModelRequest {
    Op(ModelOp),
    Create(AIModel),
    Update(AIModel),
    Delete(String),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum AgentRequest {
    Op(AgentOp),
    Create(AIAgent),
    Update(AIAgent),
    Delete(String),
}

// Response Types
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ModelResponse {
    pub success: bool,
    pub model: Option<AIModel>,
    pub message: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AgentResponse {
    pub success: bool,
    pub agent: Option<AIAgent>,
    pub message: Option<String>,
}
``` 