use crdts::{map::Op, merkle_reg::Sha3Hash, BFTReg, Map, bft_reg::Update, CmRDT};
use crate::Actor;
use serde::{Serialize, Deserialize};
use k256::ecdsa::SigningKey;
use tiny_keccak::Hasher;
use std::collections::{BTreeMap, HashMap};

pub type ModelOp = Op<String, BFTReg<AIModel, Actor>, Actor>; 
pub type ModelMap = Map<String, BFTReg<AIModel, String>, String>;

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
#[allow(non_camel_case_types)]
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
    pub parameters: u64,
    
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
    pub average_rating: Option<u32>,
    
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
    pub price_per_1m_tokens: Option<u64>,
    
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
            parameters: 0,
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
            price_per_1m_tokens: None,
            usage_tracking: ModelUsageTracking::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelState {
    pub map: ModelMap,
    pub pk: String,
    pub node_id: String,
}

impl ModelState {
    pub fn new(pk: String, node_id: String) -> Self {
        Self {
            map: Map::new(),
            pk,
            node_id
        }
    }

    pub fn map(&self) -> &ModelMap {
        &self.map
    }

    /// Update an mode locally and return the operation
    pub fn update_model_local(&mut self, model: AIModel) -> ModelOp {
        let add_ctx = self.map.read_ctx().derive_add_ctx(self.node_id.clone());
        let signing_key = SigningKey::from_slice(
            &hex::decode(self.pk.clone())
                .expect("PANIC: Invalid SigningKey Cannot Decode from Hex"))
                .expect("PANIC: Invalid SigningKey cannot recover from Bytes");
                
        self.map.update(model.model_id.clone(), add_ctx, |reg, _ctx| {
            reg.update(model, self.node_id.clone(), signing_key)
                .expect("PANIC: Unable to sign updates")
        })
    }
    
    pub fn model_op(&mut self, op: ModelOp) -> Option<(String, String)> {
        log::info!("Applying model op");
        self.map.apply(op.clone());
        match op {
            Op::Up { dot, key, op: _ } => Some((dot.actor, key)),
            Op::Rm { .. } => None
        }
    }

    pub fn model_op_success(&self, key: String, update: Update<AIModel, String>) -> (bool, AIModel) {
        if let Some(reg) = self.map.get(&key).val {
            if let Some(v) = reg.val() {
                // If the in the updated register equals the value in the Op it
                // succeeded
                if v.value() == update.op().value {
                    return (true, v.value()) 
                // Otherwise, it could be that it's a concurrent update and was added
                // to the DAG as a head
                } else if reg.dag_contains(&update.hash()) && reg.is_head(&update.hash()) {
                    return (true, v.value()) 
                // Otherwise, we could be missing a child, and this particular update
                // is orphaned, if so we should requst the child we are missing from
                // the actor who shared this update
                } else if reg.is_orphaned(&update.hash()) {
                    return (true, v.value())
                // Otherwise it was a no-op for some reason
                } else {
                    return (false, v.value()) 
                }
            } else {
                return (false, update.op().value) 
            }
        } else {
            return (false, update.op().value);
        }
    }

    pub fn remove_model_local(&mut self, model_id: String) -> ModelOp {
        log::info!("Acquiring remove context for model {}...", model_id);
        let rm_ctx = self.map.read_ctx().derive_rm_ctx();
        log::info!("Building Rm Op for account deletion...");
        self.map.rm(model_id, rm_ctx)
    }

    pub fn get_model(&self, model_id: &String) -> Option<AIModel> {
        if let Some(reg) = self.map.get(model_id).val {
            match reg.val() {
                Some(node) => return Some(node.value()),
                None => return None
            }
        }

        None
    }

    pub fn list_models(&self) -> HashMap<String, AIModel> {
        self.map.iter().filter_map(|ctx| {
            let (id, reg) = ctx.val;
            match reg.val() {
                Some(node) => Some((id.clone(), node.value())),
                None => None
            }
        }).collect()
    }
}
