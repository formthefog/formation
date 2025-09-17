use axum::{
    extract::State,
    response::sse::{Event, Sse},
    response::IntoResponse,
    response::Response,
    routing::{get, post},
    Json, Router,
};
use futures::stream::StreamExt;
use hex;
use k256::ecdsa::SigningKey;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json;
use sha2::{Digest, Sha256};
use tokio_stream::wrappers::ReceiverStream;
use uuid::Uuid;

use crate::auth::auth_middleware;
use crate::AppState;
use crate::cache::{StreamingChunk, cached_streaming_to_events, cached_non_streaming_to_json};

/// Modify the request to prepend a prompt as a system message if it exists
/// This allows users to provide a prompt that gets added as a system message
fn modify_request_with_prompt(mut request: serde_json::Value) -> serde_json::Value {
    // Extract the prompt value first to avoid borrow conflicts
    let prompt_opt = request.get("prompt").and_then(|p| p.as_str()).map(|s| s.to_string());

    // Check if there's a "prompt" field in the request
    if let Some(prompt) = prompt_opt {
        // Get the current messages array
        if let Some(messages) = request.get_mut("messages") {
            if let Some(messages_array) = messages.as_array_mut() {
                // Create a new system message with the prompt content
                let system_message = serde_json::json!({
                    "role": "system",
                    "content": prompt
                });

                // Insert the system message at the beginning of the messages array
                messages_array.insert(0, system_message);
            }
        }

        // Remove the "prompt" field from the request since it's now a system message
        request.as_object_mut().and_then(|obj| obj.remove("prompt"));
    }

    request
}

// Dummy types
#[derive(Debug, Serialize, Deserialize)]
struct ChatCompletion {
    id: String,
    model: String,
    choices: Vec<ChatChoice>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ChatChoice {
    index: usize,
    message: ChatMessage,
}

#[derive(Debug, Serialize, Deserialize)]
struct ChatMessage {
    role: String,
    content: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct DeleteResponse {
    id: String,
    deleted: bool,
}

// Request types for chat completions (streaming only)
// Removed ChatCompletionRequest - using serde_json::Value to preserve all fields

// Forwarded request type (same as original request but with additional metadata)
#[derive(Debug, Serialize, Deserialize)]
struct ForwardedChatCompletionRequest {
    #[serde(flatten)]
    original_request: serde_json::Value,
    #[serde(default)]
    forwarded_from: Option<String>, // Track forwarding chain
}

// Response types from form-state
#[derive(Debug, Serialize, Deserialize)]
struct FormStateNodeResponse {
    success: bool,
    model: String,
    node_id: String,
    port: u16,
    address: serde_json::Value, // Can be string "localhost" or object {"Ipv4": "67.77.107.55"}
}

// Request types for model inference
#[derive(Debug, Serialize, Deserialize)]
struct ModelInferenceRequest {
    text: String,
    sampling_params: SamplingParams,
    stream: bool,
    stop_at_limit: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct SamplingParams {
    max_new_tokens: u32,
    temperature: f32,
}

// Response types from model inference
#[derive(Debug, Serialize, Deserialize)]
struct ModelInferenceResponse {
    text: String,
    // Add other fields as needed based on the actual response format
}

// Streaming response types
#[derive(Debug, Clone, Serialize, Deserialize)]
struct StreamingChatCompletion {
    id: String,
    model: String,
    choices: Vec<StreamingChatChoice>,
    created: u64,
    object: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StreamingChatChoice {
    index: usize,
    delta: StreamingDelta,
    finish_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StreamingDelta {
    role: Option<String>,
    content: Option<String>,
}

// Create a lazy static client
lazy_static::lazy_static! {
    static ref CLIENT: Client = Client::new();
}

/// Create an ECDSA signature for a forwarded request
/// This ensures that forwarded requests between nodes are properly authenticated
fn create_forwarded_request_signature(
    secret_key: &str,
    message: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    // Decode the secret key from hex
    let secret_key_bytes = hex::decode(secret_key)?;

    // Create signing key
    let signing_key = SigningKey::from_slice(&secret_key_bytes)?;

    // Hash the message with SHA-256
    let mut hasher = Sha256::new();
    hasher.update(message.as_bytes());
    let message_hash = hasher.finalize();

    // Sign the message hash
    let (signature, recovery_id) = signing_key.sign_recoverable(message_hash.as_slice())?;

    // Create the authorization header value in the format: Signature <signature_hex>.<recovery_id>.<message_hex>
    let signature_hex = hex::encode(signature.to_bytes());
    let recovery_id_byte = recovery_id.to_byte();
    let message_hex = hex::encode(message.as_bytes());

    let auth_header = format!(
        "Signature {}.{}.{}",
        signature_hex, recovery_id_byte, message_hex
    );

    Ok(auth_header)
}

pub fn router() -> Router<AppState> {
    Router::new()
        // .route("/chat/completions", get(list_chat_completions))
        .route("/chat/completions", post(create_chat_completion))
        .route(
            "/forwarded/chat/completions",
            post(handle_forwarded_chat_completion),
        )
        .route("/cache/stats", get(get_cache_stats))
        .layer(axum::middleware::from_fn(auth_middleware))
    // .route("/chat/completions/:completion_id", get(get_chat_completion))
    // .route("/chat/completions/:completion_id", post(update_chat_completion))
    // .route("/chat/completions/:completion_id", delete(delete_chat_completion))
}

// Helper function to extract address from the response
fn extract_node_address(address: &serde_json::Value) -> Option<String> {
    match address {
        serde_json::Value::String(s) => {
            // Handle string addresses (like "localhost")
            Some(s.clone())
        }
        serde_json::Value::Object(obj) => {
            // Handle object addresses (like {"Ipv4": "67.77.107.55"})
            if let Some(ipv4) = obj.get("Ipv4") {
                if let Some(ip_str) = ipv4.as_str() {
                    return Some(ip_str.to_string());
                }
            }
            None
        }
        _ => None,
    }
}

// Helper function to get current node's IP address
async fn get_current_node_ip() -> String {
    // This is a simplified implementation - in production you'd want to get the actual IP
    // For now, we'll use a placeholder that should be replaced with actual IP detection
    "127.0.0.1".to_string()
}

// Handler functions with minimal implementations
async fn list_chat_completions() -> Json<Vec<ChatCompletion>> {
    Json(vec![ChatCompletion {
        id: "chatcmpl_abc123".to_string(),
        model: "gpt-4".to_string(),
        choices: vec![ChatChoice {
            index: 0,
            message: ChatMessage {
                role: "assistant".to_string(),
                content: "Hello! How can I help you today?".to_string(),
            },
        }],
    }])
}

async fn create_chat_completion(
    State(app_state): State<AppState>,
    Json(request): Json<serde_json::Value>,
) -> Response {
    // Modify the request to prepend any prompt as a system message
    let modified_request = modify_request_with_prompt(request);

    let stream = modified_request
        .get("stream")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);

    // Check if this is a streaming request
    if stream {
        handle_streaming_chat_completion(modified_request, None, app_state).await
    } else {
        handle_non_streaming_chat_completion(modified_request, None, app_state).await
    }
}

async fn handle_forwarded_chat_completion(
    State(app_state): State<AppState>,
    Json(request): Json<ForwardedChatCompletionRequest>,
) -> Response {
    // Extract the original request from the forwarded request
    let original_request: serde_json::Value = request.original_request;

    // Modify the request to prepend any prompt as a system message
    let modified_request = modify_request_with_prompt(original_request);

    // Check if this is a streaming request
    let is_streaming = modified_request
        .get("stream")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);

    if is_streaming {
        handle_streaming_chat_completion(modified_request, request.forwarded_from, app_state).await
    } else {
        handle_non_streaming_chat_completion(modified_request, request.forwarded_from, app_state)
            .await
    }
}

async fn handle_streaming_chat_completion(
    request: serde_json::Value,
    forwarded_from: Option<String>,
    app_state: AppState,
) -> Response {
    // Extract the model ID from the request
    let model_id = request
        .get("model")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    // Check cache first
    if let Some(cached_entry) = app_state.cache.get_cached_response(&request) {
        if cached_entry.is_streaming {
            // Return cached streaming response
            let completion_id = format!("chatcmpl_{}", Uuid::new_v4().simple());
            match cached_streaming_to_events(&cached_entry, &completion_id) {
                Ok(events) => {
                    let (tx, rx) = tokio::sync::mpsc::channel::<Result<Event, std::io::Error>>(100);

                    // Send cached events
                    tokio::spawn(async move {
                        for event in events {
                            let _ = tx.send(Ok(event)).await;
                        }
                    });

                    return Sse::new(ReceiverStream::new(rx)).into_response();
                }
                Err(e) => {
                    eprintln!("Error converting cached streaming response: {}", e);
                    // Fall through to normal processing
                }
            }
        }
    }

    // Call form-state to get the node for this model
    match CLIENT
        .get(format!(
            "http://localhost:3004/v1/models/{model_id}/get_node"
        ))
        .send()
        .await
    {
        Ok(response) => {
            if !response.status().is_success() {
                return (
                    axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({
                        "success": false,
                        "error": format!("Form-state API returned status: {}", response.status())
                    })),
                )
                    .into_response();
            }

            match response.json::<FormStateNodeResponse>().await {
                Ok(node_response) => {
                    if !node_response.success {
                        return (
                            axum::http::StatusCode::SERVICE_UNAVAILABLE,
                            Json(serde_json::json!({
                                "success": false,
                                "error": "No nodes available for this model"
                            })),
                        )
                            .into_response();
                    }

                    // Extract the node address
                    let node_address = match extract_node_address(&node_response.address) {
                        Some(addr) => addr,
                        None => {
                            return (
                                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                                Json(serde_json::json!({
                                    "success": false,
                                    "error": "Invalid node address format"
                                })),
                            )
                                .into_response();
                        }
                    };

                    // Check if this is a forwarded request and we're still not on localhost
                    if let Some(from) = forwarded_from {
                        if node_address != "localhost" {
                            return (
                                axum::http::StatusCode::SERVICE_UNAVAILABLE,
                                Json(serde_json::json!({
                                    "success": false,
                                    "error": format!("Model not available on this node (forwarded from {})", from)
                                }))
                            ).into_response();
                        }
                    }

                    // If the address is not localhost, forward to that node
                    if node_address != "localhost" {
                        return forward_streaming_request_to_node(
                            request,
                            &node_address,
                            app_state,
                        )
                        .await;
                    }

                    // Build the inference URL for localhost
                    let inference_url: String = format!(
                        "http://127.0.0.1:{}/v1/chat/completions",
                        node_response.port
                    );

                    // Pass through the entire original request to the model's /chat/completions endpoint
                    // This preserves all parameters and follows the standard OpenAI-compatible format
                    match CLIENT
                        .post(&inference_url)
                        .header("Content-Type", "application/json")
                        .json(&request) // Send the entire original request
                        .send()
                        .await
                    {
                        Ok(inference_response) => {
                            if !inference_response.status().is_success() {
                                return (
                                    axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                                    Json(serde_json::json!({
                                        "success": false,
                                        "error": format!("Model inference failed with status: {} (URL: {})", inference_response.status(), inference_url)
                                    }))
                                ).into_response();
                            }

                            // Handle streaming response from the model
                            // The model should return standard chat completion SSE events
                            // We'll forward them directly if they're properly formatted, or convert them if needed
                            let completion_id = format!("chatcmpl_{}", Uuid::new_v4().simple());
                            let created = std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap()
                                .as_secs();

                            let (tx, rx) = tokio::sync::mpsc::channel::<Result<Event, std::io::Error>>(100);

                            // Collect chunks for caching
                            let mut cached_chunks = Vec::new();
                            let cache = app_state.cache.clone();
                            let request_for_cache = request.clone();

                            // Spawn task to handle streaming from model
                            tokio::spawn(async move {
                                // Stream the response from the model
                                let mut stream = inference_response.bytes_stream();

                                while let Some(chunk_result) = stream.next().await {
                                    match chunk_result {
                                        Ok(chunk) => {
                                            // Parse the chunk as SSE data
                                            if let Ok(chunk_str) = String::from_utf8(chunk.to_vec()) {
                                                // Handle SSE format: "data: {...}\n\n"
                                                for line in chunk_str.lines() {
                                                    if let Some(data) = line.strip_prefix("data: ") {
                                                        // Remove "data: " prefix
                                                        if data != "[DONE]" {
                                                            // Parse the JSON data from the model
                                                            if let Ok(model_data) = serde_json::from_str::<serde_json::Value>(data) {
                                                                // Check if this is already a proper chat completion chunk
                                                                if model_data.get("choices").is_some() && model_data.get("object").is_some() {
                                                                    // Model returned a proper chat completion chunk, forward it directly
                                                                    let event = Event::default().json_data(model_data.clone()).unwrap();
                                                                    let _ = tx.send(Ok(event)).await;

                                                                    // Cache this chunk
                                                                    cached_chunks.push(StreamingChunk {
                                                                        data: serde_json::to_string(&model_data).unwrap_or_default(),
                                                                        is_final: false,
                                                                    });
                                                                } else {
                                                                    // Model returned a different format, try to convert it
                                                                    if let Some(text) = model_data.get("text").and_then(|t| t.as_str()) {
                                                                        let content_event = StreamingChatCompletion {
                                                                            id: completion_id.clone(),
                                                                            model: model_id.clone(),
                                                                            choices: vec![
                                                                                StreamingChatChoice {
                                                                                    index: 0,
                                                                                    delta: StreamingDelta {
                                                                                        role: None,
                                                                                        content: Some(text.to_string()),
                                                                                    },
                                                                                    finish_reason: None,
                                                                                }
                                                                            ],
                                                                            created,
                                                                            object: "chat.completion.chunk".to_string(),
                                                                        };

                                                                        let event = Event::default().json_data(content_event.clone()).unwrap();
                                                                        let _ = tx.send(Ok(event)).await;

                                                                        // Cache this chunk
                                                                        cached_chunks.push(StreamingChunk {
                                                                            data: serde_json::to_string(&content_event).unwrap_or_default(),
                                                                            is_final: false,
                                                                        });
                                                                    }
                                                                }
                                                            }
                                                        } else {
                                                            // Model finished
                                                            let final_event = StreamingChatCompletion {
                                                                id: completion_id.clone(),
                                                                model: model_id.clone(),
                                                                choices: vec![
                                                                    StreamingChatChoice {
                                                                        index: 0,
                                                                        delta: StreamingDelta {
                                                                            role: None,
                                                                            content: None,
                                                                        },
                                                                        finish_reason: Some("stop".to_string()),
                                                                    }
                                                                ],
                                                                created,
                                                                object: "chat.completion.chunk".to_string(),
                                                            };

                                                            let event = Event::default().json_data(final_event.clone()).unwrap();
                                                            let _ = tx.send(Ok(event)).await;

                                                            // Cache final chunk
                                                            cached_chunks.push(StreamingChunk {
                                                                data: serde_json::to_string(&final_event).unwrap_or_default(),
                                                                is_final: true,
                                                            });

                                                            // Store the complete response in cache (5 minutes TTL)
                                                            let _ = cache.store_streaming_response(
                                                                &request_for_cache,
                                                                &model_id,
                                                                cached_chunks.clone(),
                                                                5, // 5 minutes TTL
                                                            ).await;

                                                            break;
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                        Err(e) => {
                                            eprintln!("Error reading stream chunk: {e}");
                                            break;
                                        }
                                    }
                                }
                            });

                            Sse::new(ReceiverStream::new(rx)).into_response()
                        }
                        Err(e) => {
                            (
                                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                                Json(serde_json::json!({
                                    "success": false,
                                    "error": format!("Failed to make inference request to {}: {}", inference_url, e)
                                }))
                            ).into_response()
                        }
                    }
                }
                Err(e) => (
                    axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({
                        "success": false,
                        "error": format!("Failed to parse form-state response: {}", e)
                    })),
                )
                    .into_response(),
            }
        }
        Err(e) => (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "success": false,
                "error": format!("Failed to fetch model node from form-state: {}", e)
            })),
        )
            .into_response(),
    }
}

async fn handle_non_streaming_chat_completion(
    request: serde_json::Value,
    forwarded_from: Option<String>,
    app_state: AppState,
) -> Response {
    // Extract the model ID from the request
    let model_id = request
        .get("model")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    // Check cache first
    if let Some(cached_entry) = app_state.cache.get_cached_response(&request) {
        if !cached_entry.is_streaming {
            // Return cached non-streaming response
            match cached_non_streaming_to_json(&cached_entry) {
                Ok(response) => {
                    return Json(response).into_response();
                }
                Err(e) => {
                    eprintln!("Error converting cached non-streaming response: {}", e);
                    // Fall through to normal processing
                }
            }
        }
    }

    // Call form-state to get the node for this model
    match CLIENT
        .get(format!(
            "http://localhost:3004/v1/models/{model_id}/get_node"
        ))
        .send()
        .await
    {
        Ok(response) => {
            if !response.status().is_success() {
                return (
                    axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({
                        "success": false,
                        "error": format!("Form-state API returned status: {}", response.status())
                    })),
                )
                    .into_response();
            }

            match response.json::<FormStateNodeResponse>().await {
                Ok(node_response) => {
                    if !node_response.success {
                        return (
                            axum::http::StatusCode::SERVICE_UNAVAILABLE,
                            Json(serde_json::json!({
                                "success": false,
                                "error": "No nodes available for this model"
                            })),
                        )
                            .into_response();
                    }

                    // Extract the node address
                    let node_address = match extract_node_address(&node_response.address) {
                        Some(addr) => addr,
                        None => {
                            return (
                                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                                Json(serde_json::json!({
                                    "success": false,
                                    "error": "Invalid node address format"
                                })),
                            )
                                .into_response();
                        }
                    };

                    // Check if this is a forwarded request and we're still not on localhost
                    if let Some(from) = forwarded_from {
                        if node_address != "localhost" {
                            return (
                                axum::http::StatusCode::SERVICE_UNAVAILABLE,
                                Json(serde_json::json!({
                                    "success": false,
                                    "error": format!("Model not available on this node (forwarded from {})", from)
                                }))
                            ).into_response();
                        }
                    }

                    // If the address is not localhost, forward to that node
                    if node_address != "localhost" {
                        return forward_streaming_request_to_node(
                            request,
                            &node_address,
                            app_state,
                        )
                        .await;
                    }

                    // Build the inference URL for localhost
                    let inference_url = format!(
                        "http://127.0.0.1:{}/v1/chat/completions",
                        node_response.port
                    );

                    // Pass through the entire original request to the model's /chat/completions endpoint
                    // This preserves all parameters and follows the standard OpenAI-compatible format
                    match CLIENT
                        .post(&inference_url)
                        .header("Content-Type", "application/json")
                        .json(&request) // Send the entire original request
                        .send()
                        .await
                    {
                        Ok(inference_response) => {
                            if !inference_response.status().is_success() {
                                return (
                                    axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                                    Json(serde_json::json!({
                                        "success": false,
                                        "error": format!("Model inference failed with status: {} (URL: {})", inference_response.status(), inference_url)
                                    }))
                                ).into_response();
                            }

                            // Handle non-streaming response from the model
                            // The model should return a standard chat completion response
                            match inference_response.json::<serde_json::Value>().await {
                                Ok(model_response) => {
                                    // Check if the model returned a proper chat completion response
                                    if let Some(choices) = model_response.get("choices").and_then(|c| c.as_array()) {
                                        if !choices.is_empty() {
                                            // Model returned a proper chat completion response, pass it through
                                            // Cache the response (5 minutes TTL)
                                            let _ = app_state.cache.store_non_streaming_response(
                                                &request,
                                                &model_id,
                                                model_response.clone(),
                                                5, // 5 minutes TTL
                                            ).await;

                                            Json(model_response).into_response()
                                        } else {
                                            // No choices in response
                                            (
                                                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                                                Json(serde_json::json!({
                                                    "success": false,
                                                    "error": "Model returned empty choices array"
                                                }))
                                            ).into_response()
                                        }
                                    } else {
                                        // Model didn't return a proper chat completion response
                                        // Try to extract text from the response and create a proper format
                                        if let Some(text) = model_response.get("text").and_then(|t| t.as_str()) {
                                            let completion_id = format!("chatcmpl_{}", Uuid::new_v4().simple());
                                            let _created = std::time::SystemTime::now()
                                                .duration_since(std::time::UNIX_EPOCH)
                                                .unwrap()
                                                .as_secs();

                                            // Create a proper non-streaming chat completion response
                                            let completion = ChatCompletion {
                                                id: completion_id.clone(),
                                                model: model_id.clone(),
                                                choices: vec![
                                                    ChatChoice {
                                                        index: 0,
                                                        message: ChatMessage {
                                                            role: "assistant".to_string(),
                                                            content: text.to_string(),
                                                        },
                                                    }
                                                ],
                                            };

                                            // Cache the response (5 minutes TTL)
                                            let completion_json = serde_json::to_value(&completion).unwrap_or_default();
                                            let _ = app_state.cache.store_non_streaming_response(
                                                &request,
                                                &model_id,
                                                completion_json.clone(),
                                                5, // 5 minutes TTL
                                            ).await;

                                            Json(completion).into_response()
                                        } else {
                                            // Fallback: return the raw model response
                                            Json(model_response).into_response()
                                        }
                                    }
                                }
                                Err(e) => {
                                    (
                                        axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                                        Json(serde_json::json!({
                                            "success": false,
                                            "error": format!("Failed to parse model response: {}", e)
                                        }))
                                    ).into_response()
                                }
                            }
                        }
                        Err(e) => {
                            (
                                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                                Json(serde_json::json!({
                                    "success": false,
                                    "error": format!("Failed to make inference request to {}: {}", inference_url, e)
                                }))
                            ).into_response()
                        }
                    }
                }
                Err(e) => (
                    axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({
                        "success": false,
                        "error": format!("Failed to parse form-state response: {}", e)
                    })),
                )
                    .into_response(),
            }
        }
        Err(e) => (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "success": false,
                "error": format!("Failed to fetch model node from form-state: {}", e)
            })),
        )
            .into_response(),
    }
}

async fn forward_streaming_request_to_node(
    request: serde_json::Value,
    target_node: &str,
    app_state: AppState,
) -> Response {
    let current_node_ip = get_current_node_ip().await;

    // Create forwarded request with all original parameters
    let forwarded_request = ForwardedChatCompletionRequest {
        original_request: request.clone(),
        forwarded_from: Some(current_node_ip),
    };

    // Forward to the target node's form-inference-api
    let forward_url = format!("http://{target_node}:3006/forwarded/chat/completions");

    // Create a signed authorization header for the forwarded request
    // This ensures that forwarded requests between nodes are properly authenticated
    let auth_header = match create_forwarded_request_signature_for_node(
        &forwarded_request,
        target_node,
        app_state.secret_key.as_str(),
    ) {
        Ok(header) => header,
        Err(e) => {
            eprintln!("Failed to create signature for forwarded request: {}", e);
            return (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "success": false,
                    "error": format!("Failed to create authentication signature: {}", e)
                })),
            )
                .into_response();
        }
    };

    match CLIENT
        .post(&forward_url)
        .header("Content-Type", "application/json")
        .header("Authorization", &auth_header)
        .json(&forwarded_request)
        .send()
        .await
    {
        Ok(response) => {
            if response.status().is_success() {
                // Check if the original request was streaming
                let is_streaming = request
                    .get("stream")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(true);

                if is_streaming {
                    // Forward the streaming response directly
                    let (tx, rx) = tokio::sync::mpsc::channel::<Result<Event, std::io::Error>>(100);

                    tokio::spawn(async move {
                        let mut stream = response.bytes_stream();

                        while let Some(chunk_result) = stream.next().await {
                            match chunk_result {
                                Ok(chunk) => {
                                    if let Ok(chunk_str) = String::from_utf8(chunk.to_vec()) {
                                        // Forward the chunk directly as SSE data
                                        // The remote node is already sending properly formatted SSE events
                                        // We just need to forward them as-is
                                        if !chunk_str.trim().is_empty() {
                                            // Create a simple SSE event with the raw data
                                            let event = Event::default().data(chunk_str);
                                            let _ = tx.send(Ok(event)).await;
                                        }
                                    }
                                }
                                Err(e) => {
                                    eprintln!("Error reading forwarded stream chunk: {e}");
                                    break;
                                }
                            }
                        }
                    });

                    Sse::new(ReceiverStream::new(rx)).into_response()
                } else {
                    // For non-streaming requests, return the JSON response directly
                    match response.json::<serde_json::Value>().await {
                        Ok(json) => Json(json).into_response(),
                        Err(e) => (
                            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                            Json(serde_json::json!({
                                "success": false,
                                "error": format!("Failed to parse forwarded response: {}", e)
                            })),
                        )
                            .into_response(),
                    }
                }
            } else {
                // Forward the error response
                match response.text().await {
                    Ok(error_text) => {
                        match serde_json::from_str::<serde_json::Value>(&error_text) {
                            Ok(json) => Json(json).into_response(),
                            Err(_) => (
                                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                                Json(serde_json::json!({
                                    "success": false,
                                    "error": format!("Forwarded error: {}", error_text)
                                })),
                            )
                                .into_response(),
                        }
                    }
                    Err(e) => (
                        axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                        Json(serde_json::json!({
                            "success": false,
                            "error": format!("Failed to read forwarded error response: {}", e)
                        })),
                    )
                        .into_response(),
                }
            }
        }
        Err(e) => (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "success": false,
                "error": format!("Failed to forward request to {}: {}", target_node, e)
            })),
        )
            .into_response(),
    }
}

/// Create a signature for a forwarded request to another node
/// This function creates an ECDSA signature using the operator's secret key
/// to authenticate inter-node communication
fn create_forwarded_request_signature_for_node(
    forwarded_request: &ForwardedChatCompletionRequest,
    target_node: &str,
    secret_key: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    // Create a message to sign that includes the target node and request details
    let message = format!(
        "forwarded_request:{}:{}",
        target_node,
        serde_json::to_string(forwarded_request)?
    );

    // Sign the message
    create_forwarded_request_signature(secret_key, &message)
}

/// Get cache statistics
async fn get_cache_stats(State(app_state): State<AppState>) -> Json<serde_json::Value> {
    let stats = app_state.cache.get_stats();
    Json(serde_json::json!({
        "total_entries": stats.total_entries,
        "active_entries": stats.active_entries,
        "expired_entries": stats.expired_entries,
        "cache_hit_rate": if stats.total_entries > 0 {
            (stats.active_entries as f64 / stats.total_entries as f64) * 100.0
        } else {
            0.0
        }
    }))
}
