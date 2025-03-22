use crdts::{bft_reg::Update, map::Op, merkle_reg::Sha3Hash, BFTReg, CmRDT, Map};
use crate::Actor;
use serde::{Serialize, Deserialize};
use tiny_keccak::{Hasher, Sha3};
use std::collections::BTreeMap;
use crate::model::{ModelType, ModelLicense};

pub type AgentOp = Op<String, BFTReg<AIAgent, Actor>, Actor>; 

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
    pub average_rating: Option<u32>,
    
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
    pub price_per_request: Option<u64>,
    
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
