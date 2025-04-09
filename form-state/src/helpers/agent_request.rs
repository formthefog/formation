use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::str::FromStr;
use uuid::Uuid;

/// Main request structure for the run_task endpoint, compatible with fama-ai
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunTaskRequest {
    /// JWT token for authentication
    pub jwt: String,
    // Core fields
    /// ID of the agent to run (also extracted from URL path)
    pub agent_id: String,
    
    /// Optional task ID for tracking (generated if not provided)
    pub task_id: Option<String>,
    
    /// The prompt/task description to be executed
    pub task: String,
    
    // Model configuration
    /// ID of the model to use (e.g., "gpt-4-turbo", "claude-3-opus")
    pub model_id: String,
    
    /// Provider of the model (e.g., "openai", "anthropic", "formation")
    pub model_provider: String,
    
    /// API key for the model provider (if not using Formation credits)
    pub provider_api_key: Option<String>,
    
    // Knowledge retrieval configuration
    /// Whether to use semantic search for knowledge retrieval
    pub enable_semantic_search: Option<bool>,
    
    /// Type of vector database to use (e.g., "lancedb", "pinecone")
    pub vector_db_type: Option<String>,
    
    /// URLs to use as knowledge sources
    pub knowledge_urls: Option<Vec<String>>,
    
    /// Raw text to use as knowledge source
    pub knowledge_text: Option<String>,
    
    /// Type of knowledge source (e.g., "pdf_url", "web", "text")
    pub knowledge_source_type: Option<String>,
    
    /// Provider for embeddings (e.g., "openai", "cohere")
    pub embedder_provider: Option<String>,
    
    /// Embedding model to use
    pub embedder_model: Option<String>,
    
    /// Dimensions for the embedder (required for local embedders)
    pub embedder_dimensions: Option<i32>,
    
    // Chunking configuration
    /// Chunking strategy for knowledge sources
    pub chunking_strategy: Option<String>,
    
    /// Maximum size of each chunk
    pub chunk_size: Option<i32>,
    
    /// Number of characters to overlap between chunks
    pub chunk_overlap: Option<i32>,
    
    /// Similarity threshold for semantic chunking
    pub similarity_threshold: Option<f32>,
    
    // Storage and memory configuration
    /// Storage type (e.g., "sqlite", "postgres", "mongodb")
    pub storage_type: Option<String>,
    
    /// Connection string or configuration for storage
    pub storage_connection: Option<String>,
    
    /// Session ID for resuming conversations
    pub session_id: Option<String>,
    
    /// User ID for personalization
    pub user_id: Option<String>,
    
    /// Whether to enable chat history memory
    pub enable_chat_history: Option<bool>,
    
    /// Whether to enable user-specific memories
    pub enable_user_memories: Option<bool>,
    
    /// Whether to enable conversation summaries
    pub enable_summaries: Option<bool>,
    
    /// Number of messages to keep in memory
    pub memory_depth: Option<i32>,
    
    // Tool configuration
    /// Whether to enable web search tools
    pub enable_web_search: Option<bool>,
    
    /// Whether to enable file manipulation tools
    pub enable_file_tools: Option<bool>,
    
    /// Whether to enable mathematical tools
    pub enable_math_tools: Option<bool>,
    
    /// Custom tool definitions (fama-ai format)
    pub custom_tools: Option<Vec<HashMap<String, serde_json::Value>>>,
    
    // MCP parameters (Container management)
    /// List of MCP servers to connect to
    pub mcp_servers: Option<Vec<HashMap<String, serde_json::Value>>>,
    
    /// Whether to enable filesystem MCP server
    pub use_filesystem_mcp: Option<bool>,
    
    /// Root path for filesystem MCP server
    pub filesystem_root_path: Option<String>,
    
    // Formation-specific extensions
    /// Whether to stream the response (default: true)
    pub streaming: Option<bool>,
    
    /// Timeout in seconds (default: 300)
    pub timeout_seconds: Option<u32>,
    
    /// Formation API key (for billing and auth)
    pub formation_api_key: Option<String>,
    
    /// Formation authentication token
    pub formation_auth_token: Option<String>,
    
    // Memory configuration specific to Formation
    /// Detailed memory configuration
    pub memory_config: Option<MemoryConfig>,
    
    /// Storage configuration for files/artifacts
    pub storage_config: Option<StorageConfig>,
    
    /// Knowledge base configuration
    pub knowledge_base_config: Option<KnowledgeBaseConfig>,
    
    // Formation agent tools
    /// Enabled tools list (Formation format)
    pub enabled_tools: Option<Vec<Tool>>,
    
    /// Custom tools in Formation format
    pub formation_custom_tools: Option<Vec<CustomTool>>,
    
    // Advanced Formation parameters
    /// Additional custom parameters
    pub custom_parameters: Option<HashMap<String, serde_json::Value>>,
}

/// Memory type options for agents
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MemoryType {
    /// No persistent memory
    None,
    
    /// Basic conversation memory
    Conversation,
    
    /// Formation's persistent memory
    FormationPersistent,
    
    /// Pinecone vector database
    PineconeVectorDB,
    
    /// ChromaDB vector database
    ChromaDB,
    
    /// LanceDB vector database
    LanceDB,
    
    /// Qdrant vector database
    Qdrant,
    
    /// Custom memory type
    Custom(String),
}

impl fmt::Display for MemoryType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MemoryType::None => write!(f, "none"),
            MemoryType::Conversation => write!(f, "conversation"),
            MemoryType::FormationPersistent => write!(f, "formation_persistent"),
            MemoryType::PineconeVectorDB => write!(f, "pinecone"),
            MemoryType::ChromaDB => write!(f, "chroma"),
            MemoryType::LanceDB => write!(f, "lancedb"),
            MemoryType::Qdrant => write!(f, "qdrant"),
            MemoryType::Custom(s) => write!(f, "custom:{}", s),
        }
    }
}

impl FromStr for MemoryType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "none" => Ok(MemoryType::None),
            "conversation" => Ok(MemoryType::Conversation),
            "formation_persistent" => Ok(MemoryType::FormationPersistent),
            "pinecone" => Ok(MemoryType::PineconeVectorDB),
            "chroma" => Ok(MemoryType::ChromaDB),
            "lancedb" => Ok(MemoryType::LanceDB),
            "qdrant" => Ok(MemoryType::Qdrant),
            s if s.starts_with("custom:") => Ok(MemoryType::Custom(s[7..].to_string())),
            _ => Err(format!("Unknown memory type: {}", s)),
        }
    }
}

/// Configuration for agent memory
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryConfig {
    /// Type of memory to use
    pub memory_type: MemoryType,
    
    /// Endpoint URL for the memory service
    pub endpoint: Option<String>,
    
    /// API key for the memory service
    pub api_key: Option<String>,
    
    /// Collection name in the memory service
    pub collection_name: Option<String>,
    
    /// Conversation ID for this interaction
    pub conversation_id: Option<String>,
}

/// Storage types for files and artifacts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StorageType {
    /// No storage
    None,
    
    /// Formation's built-in storage
    FormationStorage,
    
    /// Amazon S3 compatible storage
    S3,
    
    /// Google Cloud Storage
    GCS,
    
    /// Azure Blob Storage
    Azure,
    
    /// Custom storage provider
    Custom(String),
}

/// Configuration for file storage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    /// Type of storage to use
    pub storage_type: StorageType,
    
    /// Endpoint URL for the storage service
    pub endpoint: Option<String>,
    
    /// API key for the storage service
    pub api_key: Option<String>,
    
    /// Bucket name in the storage service
    pub bucket_name: Option<String>,
    
    /// Folder path within the bucket
    pub folder_path: Option<String>,
}

/// Knowledge base types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum KnowledgeBaseType {
    /// No knowledge base
    None,
    
    /// Formation's RAG system
    FormationRAG,
    
    /// Pinecone knowledge base
    Pinecone,
    
    /// Weaviate knowledge base
    Weaviate,
    
    /// Qdrant knowledge base
    Qdrant,
    
    /// Custom knowledge base
    Custom(String),
}

/// Configuration for knowledge base access
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeBaseConfig {
    /// Type of knowledge base
    pub kb_type: KnowledgeBaseType,
    
    /// Endpoint URL for the knowledge base
    pub endpoint: Option<String>,
    
    /// API key for the knowledge base
    pub api_key: Option<String>,
    
    /// Index name in the knowledge base
    pub index_name: Option<String>,
}

/// Tool configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tool {
    /// ID of the tool to use
    pub tool_id: String,
    
    /// Parameters to pass to the tool
    pub parameters: Option<HashMap<String, serde_json::Value>>,
}

/// Custom tool definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomTool {
    /// Name of the tool
    pub name: String,
    
    /// Description of what the tool does
    pub description: String,
    
    /// JSON schema for the tool's parameters
    pub parameters_schema: serde_json::Value,
    
    /// Endpoint URL to call when using the tool
    pub endpoint: String,
    
    /// Authentication token for the tool
    pub auth_token: Option<String>,
}

/// Request structure for the hire_agent endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HireAgentRequest {
    /// JWT token for authentication
    pub jwt: String,
    
    /// Optional metadata for the hiring transaction
    pub metadata: Option<HashMap<String, String>>,
}

impl RunTaskRequest {
    /// Get a task ID, generating one if not present
    pub fn task_id(&self) -> String {
        self.task_id.clone().unwrap_or_else(|| Uuid::new_v4().to_string())
    }
    
    /// Check if streaming is enabled (defaults to true)
    pub fn streaming(&self) -> bool {
        self.streaming.unwrap_or(true)
    }
    
    /// Get timeout in seconds (defaults to 300s/5min)
    pub fn timeout_seconds(&self) -> u32 {
        self.timeout_seconds.unwrap_or(300)
    }
    
    /// Convert to agent-specific request format if needed
    pub fn to_agent_specific_format(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap_or_default()
    }
    
    /// Create a new request with minimal required fields
    pub fn new(agent_id: String, model_id: String, model_provider: String, task: String) -> Self {
        Self {
            jwt: "".to_string(),
            agent_id,
            task_id: None,
            task,
            model_id,
            model_provider,
            provider_api_key: None,
            enable_semantic_search: None,
            vector_db_type: None,
            knowledge_urls: None,
            knowledge_text: None,
            knowledge_source_type: None,
            embedder_provider: None,
            embedder_model: None,
            embedder_dimensions: None,
            chunking_strategy: None,
            chunk_size: None,
            chunk_overlap: None,
            similarity_threshold: None,
            storage_type: None,
            storage_connection: None,
            session_id: None,
            user_id: None,
            enable_chat_history: None,
            enable_user_memories: None,
            enable_summaries: None,
            memory_depth: None,
            enable_web_search: None,
            enable_file_tools: None,
            enable_math_tools: None,
            custom_tools: None,
            mcp_servers: None,
            use_filesystem_mcp: None,
            filesystem_root_path: None,
            streaming: Some(true),
            timeout_seconds: None,
            formation_api_key: None,
            formation_auth_token: None,
            memory_config: None,
            storage_config: None,
            knowledge_base_config: None,
            enabled_tools: None,
            formation_custom_tools: None,
            custom_parameters: None,
        }
    }
    
    /// Add an auth token to the request
    pub fn with_auth_token(mut self, token: String) -> Self {
        self.formation_auth_token = Some(token);
        self
    }
    
    /// Add a Formation API key to the request
    pub fn with_api_key(mut self, api_key: String) -> Self {
        self.formation_api_key = Some(api_key);
        self
    }
    
    /// Configure streaming option
    pub fn with_streaming(mut self, streaming: bool) -> Self {
        self.streaming = Some(streaming);
        self
    }
    
    /// Set a custom timeout
    pub fn with_timeout(mut self, timeout_seconds: u32) -> Self {
        self.timeout_seconds = Some(timeout_seconds);
        self
    }
}

impl HireAgentRequest {
    /// Create a new agent hiring request
    pub fn new(jwt: String) -> Self {
        Self {
            jwt,
            metadata: None,
        }
    }
    
    /// Add metadata to the request
    pub fn with_metadata(mut self, metadata: HashMap<String, String>) -> Self {
        self.metadata = Some(metadata);
        self
    }
} 
