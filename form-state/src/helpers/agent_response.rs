use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::time::{SystemTime, UNIX_EPOCH};
use axum::{response::{IntoResponse, Response as AxumResponse}, http::StatusCode};
use axum::Json;

/// Response format for non-streaming task response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunTaskResponse {
    /// Unique ID for the task
    pub task_id: String,
    
    /// ID of the agent that processed the task
    pub agent_id: String,
    
    /// The final completion/response text from the agent
    pub completion: Option<String>,
    
    /// Error message if the task failed
    pub error: Option<String>,
    
    /// Current status of the task 
    pub status: TaskStatus,
    
    /// Structured result (may be present alongside text completion)
    pub result: Option<serde_json::Value>,
    
    /// Metadata about the task execution
    pub metadata: Option<HashMap<String, serde_json::Value>>,
    
    /// Usage information for billing and metrics
    pub usage: Option<UsageInfo>,
    
    /// Timestamp of when the response was generated (seconds since epoch)
    pub timestamp: u64,
    
    /// How long the task took to complete in milliseconds
    pub duration_ms: u64,
}

/// The current status of a task
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TaskStatus {
    /// Task has been received but not yet started
    Pending,
    
    /// Task is currently being processed
    Running,
    
    /// Task has been completed successfully
    Completed,
    
    /// Task encountered an error
    Failed,
    
    /// Task was cancelled by the user or system
    Cancelled,
    
    /// Task timed out before completion
    TimedOut,
}

impl fmt::Display for TaskStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TaskStatus::Pending => write!(f, "pending"),
            TaskStatus::Running => write!(f, "running"),
            TaskStatus::Completed => write!(f, "completed"),
            TaskStatus::Failed => write!(f, "failed"),
            TaskStatus::Cancelled => write!(f, "cancelled"),
            TaskStatus::TimedOut => write!(f, "timed_out"),
        }
    }
}

/// Usage information for a task, used for billing and metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageInfo {
    /// Number of tokens in the input/prompt
    pub prompt_tokens: u32,
    
    /// Number of tokens in the output/completion
    pub completion_tokens: u32,
    
    /// Total tokens used (prompt + completion)
    pub total_tokens: u32,
    
    /// Time taken to process the request in milliseconds by the agent
    pub duration_ms: u64,
    
    /// Billable time for the request in milliseconds (may differ from duration, if applicable)
    pub billable_duration_ms: u64,
    
    /// LLM API cost if an external provider was used by the agent
    pub provider_cost: Option<f64>,
    
    /// Currency for the provider cost (e.g., "USD")
    pub cost_currency: Option<String>,
    
    /// Computational resources used (CPU, memory, etc.) - reported by agent
    pub resources: Option<ResourceUsage>,
}

/// Resource usage metrics for a task
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceUsage {
    /// CPU time in milliseconds
    pub cpu_ms: Option<u64>,
    
    /// Peak memory usage in megabytes
    pub memory_mb: Option<f64>,
    
    /// GPU time in milliseconds (if GPU was used)
    pub gpu_ms: Option<u64>,
    
    /// Network egress in kilobytes
    pub network_egress_kb: Option<u64>,
    
    /// Network ingress in kilobytes
    pub network_ingress_kb: Option<u64>,
}

/// Chunk of a streaming response from an agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskStreamChunk {
    /// ID of the task this chunk belongs to
    pub task_id: String,
    
    /// ID of the agent generating the chunk
    pub agent_id: String,
    
    /// Unique ID for this specific chunk
    pub chunk_id: String,
    
    /// Content/text of this chunk
    pub content: String,
    
    /// Whether this is the final chunk in the stream
    pub is_final: bool,
    
    /// Error message if there was a problem (can be present in a non-final chunk)
    pub error: Option<String>,
    
    /// Metadata about this chunk or the overall task
    pub metadata: Option<HashMap<String, serde_json::Value>>,
    
    /// Usage information (typically only in the final chunk)
    pub usage: Option<UsageInfo>,
    
    /// Creation timestamp for this chunk (ms since epoch)
    pub timestamp: u64,
}

impl TaskStreamChunk {
    /// Create a new streaming chunk
    pub fn new(task_id: String, agent_id: String, content: String) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
            
        Self {
            task_id,
            agent_id,
            chunk_id: format!("chunk-{}", uuid::Uuid::new_v4()),
            content,
            is_final: false,
            error: None,
            metadata: None,
            usage: None,
            timestamp: now,
        }
    }
    
    /// Mark this chunk as the final one in the stream
    pub fn finalize(mut self) -> Self {
        self.is_final = true;
        self
    }
    
    /// Add an error to this chunk
    pub fn with_error(mut self, error: String) -> Self {
        self.error = Some(error);
        self
    }
    
    /// Add usage information to this chunk
    pub fn with_usage(mut self, usage: UsageInfo) -> Self {
        self.usage = Some(usage);
        self
    }
    
    /// Add metadata to this chunk
    pub fn with_metadata(mut self, metadata: HashMap<String, serde_json::Value>) -> Self {
        self.metadata = Some(metadata);
        self
    }
}

/// Represents a tool call from an agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    /// ID of the tool being called
    pub tool_id: String,
    
    /// Name of the tool
    pub tool_name: String,
    
    /// Arguments passed to the tool
    pub arguments: serde_json::Value,
    
    /// Task ID that originated this tool call
    pub task_id: String,
    
    /// Agent ID that is making the tool call
    pub agent_id: String,
    
    /// Unique ID for this tool call
    pub call_id: String,
}

/// Result returned from a tool after execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallResult {
    /// ID of the tool that was called
    pub tool_id: String,
    
    /// Tool call ID this result is responding to
    pub call_id: String,
    
    /// Whether the tool call was successful
    pub success: bool,
    
    /// Result data from the tool
    pub result: Option<serde_json::Value>,
    
    /// Error message if the tool call failed
    pub error: Option<String>,
    
    /// Tool-specific metadata about the execution
    pub metadata: Option<HashMap<String, serde_json::Value>>,
}

/// Detailed error response for the API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiError {
    /// Error code for programmatic handling
    pub code: String,
    
    /// Human-readable error message
    pub message: String,
    
    /// Optional detailed error information
    pub details: Option<serde_json::Value>,
    
    /// HTTP status code associated with this error
    pub status_code: u16,
    
    /// Request ID for tracking this error
    pub request_id: String,
    
    /// Path that triggered the error
    pub path: String,
    
    /// Timestamp when the error occurred
    pub timestamp: u64,
}

impl ApiError {
    /// Create a new API error
    pub fn new(code: &str, message: &str, status_code: u16) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
            
        Self {
            code: code.to_string(),
            message: message.to_string(),
            details: None,
            status_code,
            request_id: uuid::Uuid::new_v4().to_string(),
            path: String::new(),
            timestamp: now,
        }
    }
    
    /// Set the request path
    pub fn with_path(mut self, path: &str) -> Self {
        self.path = path.to_string();
        self
    }
    
    /// Add detailed error information
    pub fn with_details(mut self, details: serde_json::Value) -> Self {
        self.details = Some(details);
        self
    }
    
    /// Set the request ID
    pub fn with_request_id(mut self, request_id: &str) -> Self {
        self.request_id = request_id.to_string();
        self
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> AxumResponse {
        let status = StatusCode::from_u16(self.status_code).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
        (status, Json(self)).into_response()
    }
}

/// Extension trait for task response conversion
pub trait IntoTaskResponse {
    /// Convert to a full task response
    fn into_task_response(self, task_id: String, agent_id: String) -> RunTaskResponse;
}

/// Extension trait for stream chunk conversion
pub trait IntoStreamChunk {
    /// Convert to a stream chunk
    fn into_stream_chunk(self, task_id: String, agent_id: String) -> TaskStreamChunk;
} 