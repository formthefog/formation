use axum::response::sse::Event;
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;
use uuid::Uuid;

/// Cache entry for storing inference responses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntry {
    pub id: String,
    pub model: String,
    pub created: u64,
    pub is_streaming: bool,
    pub response_data: ResponseData,
    pub expires_at: u64, // Unix timestamp instead of Instant
}

/// Response data that can be either streaming chunks or a complete response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ResponseData {
    /// For streaming responses, store all the chunks
    Streaming(Vec<StreamingChunk>),
    /// For non-streaming responses, store the complete JSON response
    NonStreaming(serde_json::Value),
}

/// A single streaming chunk
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamingChunk {
    pub data: String,
    pub is_final: bool,
}

/// Cache for storing inference responses with TTL
pub struct InferenceCache {
    /// Map from request hash to cache entry
    entries: DashMap<String, CacheEntry>,
    /// Background cleanup task handle
    cleanup_handle: Arc<RwLock<Option<tokio::task::JoinHandle<()>>>>,
}

impl InferenceCache {
    pub fn new() -> Self {
        let cache = Self {
            entries: DashMap::new(),
            cleanup_handle: Arc::new(RwLock::new(None)),
        };
        cache.start_cleanup_task();
        cache
    }

    /// Generate a cache key from the request
    pub fn generate_cache_key(&self, request: &serde_json::Value) -> String {
        // Create a deterministic hash of the request for caching
        // We exclude certain fields that shouldn't affect caching (like timestamps)
        let mut cacheable_request = request.clone();

        // Remove fields that shouldn't affect caching
        if let Some(obj) = cacheable_request.as_object_mut() {
            obj.remove("id");
            obj.remove("created");
            // Keep model, messages, temperature, max_tokens, etc. as they affect the response
        }

        // Create a hash of the request
        let request_str = serde_json::to_string(&cacheable_request).unwrap_or_default();
        let mut hasher = Sha256::new();
        hasher.update(request_str.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    /// Store a streaming response in the cache
    pub async fn store_streaming_response(
        &self,
        request: &serde_json::Value,
        model: &str,
        chunks: Vec<StreamingChunk>,
        ttl_minutes: u64,
    ) -> String {
        let cache_key = self.generate_cache_key(request);
        let id = format!("chatcmpl_{}", Uuid::new_v4().simple());
        let created = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let entry = CacheEntry {
            id: id.clone(),
            model: model.to_string(),
            created,
            is_streaming: true,
            response_data: ResponseData::Streaming(chunks),
            expires_at: created + (ttl_minutes * 60),
        };

        self.entries.insert(cache_key, entry);
        id
    }

    /// Store a non-streaming response in the cache
    pub async fn store_non_streaming_response(
        &self,
        request: &serde_json::Value,
        model: &str,
        response: serde_json::Value,
        ttl_minutes: u64,
    ) -> String {
        let cache_key = self.generate_cache_key(request);
        let id = format!("chatcmpl_{}", Uuid::new_v4().simple());
        let created = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let entry = CacheEntry {
            id: id.clone(),
            model: model.to_string(),
            created,
            is_streaming: false,
            response_data: ResponseData::NonStreaming(response),
            expires_at: created + (ttl_minutes * 60),
        };

        self.entries.insert(cache_key, entry);
        id
    }

    /// Retrieve a cached response
    pub fn get_cached_response(&self, request: &serde_json::Value) -> Option<CacheEntry> {
        let cache_key = self.generate_cache_key(request);

        if let Some(entry) = self.entries.get(&cache_key) {
            // Check if the entry has expired
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs();
            if entry.expires_at > now {
                Some(entry.clone())
            } else {
                // Entry has expired, remove it
                self.entries.remove(&cache_key);
                None
            }
        } else {
            None
        }
    }

    /// Check if a request is currently being processed (to avoid duplicate requests)
    pub fn is_request_in_progress(&self, request: &serde_json::Value) -> bool {
        let cache_key = self.generate_cache_key(request);
        self.entries.contains_key(&cache_key)
    }

    /// Start the background cleanup task
    fn start_cleanup_task(&self) {
        let entries = self.entries.clone();
        let cleanup_handle = self.cleanup_handle.clone();

        let handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(60)); // Run every minute

            loop {
                interval.tick().await;

                // Remove expired entries
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs();
                entries.retain(|_, entry| entry.expires_at > now);
            }
        });

        // Store the cleanup task handle
        tokio::spawn(async move {
            let mut handle_guard = cleanup_handle.write().await;
            *handle_guard = Some(handle);
        });
    }

    /// Get cache statistics
    pub fn get_stats(&self) -> CacheStats {
        let total_entries = self.entries.len();
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let active_entries = self.entries.iter()
            .filter(|entry| entry.expires_at > now)
            .count();

        CacheStats {
            total_entries,
            active_entries,
            expired_entries: total_entries - active_entries,
        }
    }
}

/// Cache statistics
#[derive(Debug, Clone)]
pub struct CacheStats {
    pub total_entries: usize,
    pub active_entries: usize,
    pub expired_entries: usize,
}

/// Convert a cached streaming response to SSE events
pub fn cached_streaming_to_events(
    entry: &CacheEntry,
    completion_id: &str,
) -> Result<Vec<Event>, Box<dyn std::error::Error>> {
    match &entry.response_data {
        ResponseData::Streaming(chunks) => {
            let mut events = Vec::new();

            for chunk in chunks {
                if chunk.is_final {
                    // Create final event
                    let final_event = serde_json::json!({
                        "id": completion_id,
                        "model": entry.model,
                        "choices": [{
                            "index": 0,
                            "delta": {},
                            "finish_reason": "stop"
                        }],
                        "created": entry.created,
                        "object": "chat.completion.chunk"
                    });
                    events.push(Event::default().json_data(final_event).unwrap());
                } else {
                    // Parse the chunk data as JSON and create SSE event
                    if let Ok(chunk_data) = serde_json::from_str::<serde_json::Value>(&chunk.data) {
                        events.push(Event::default().json_data(chunk_data).unwrap());
                    } else {
                        // If parsing fails, create a simple data event
                        events.push(Event::default().data(&chunk.data));
                    }
                }
            }

            Ok(events)
        }
        ResponseData::NonStreaming(_) => {
            Err("Expected streaming response data".into())
        }
    }
}

/// Convert a cached non-streaming response to JSON
pub fn cached_non_streaming_to_json(
    entry: &CacheEntry,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    match &entry.response_data {
        ResponseData::NonStreaming(response) => Ok(response.clone()),
        ResponseData::Streaming(_) => {
            Err("Expected non-streaming response data".into())
        }
    }
}

