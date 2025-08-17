use axum::{
    routing::{post},
    Json, Router, response::IntoResponse, extract::Path, extract::State,
    response::sse::{Event, Sse},
    response::Response,
};
use serde::{Deserialize, Serialize};
use reqwest::Client;
use uuid::Uuid;
use serde_json;
use tokio_stream::wrappers::ReceiverStream;
use futures::stream::StreamExt;
use k256::ecdsa::{SigningKey, signature::Signer};
use sha2::{Digest, Sha256};
use hex;
use std::sync::Arc;

use crate::auth::auth_middleware;

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
#[derive(Debug, Serialize, Deserialize)]
struct StreamingChatCompletion {
    id: String,
    model: String,
    choices: Vec<StreamingChatChoice>,
    created: u64,
    object: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct StreamingChatChoice {
    index: usize,
    delta: StreamingDelta,
    finish_reason: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
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

    let auth_header = format!("Signature {}.{}.{}", signature_hex, recovery_id_byte, message_hex);

    Ok(auth_header)
}

pub fn router() -> Router<Arc<String>> {
    Router::new()
        // .route("/chat/completions", get(list_chat_completions))
        .route("/chat/completions", post(create_chat_completion))
        .route("/forwarded/chat/completions", post(handle_forwarded_chat_completion))
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
    Json(vec![
        ChatCompletion {
            id: "chatcmpl_abc123".to_string(),
            model: "gpt-4".to_string(),
            choices: vec![
                ChatChoice {
                    index: 0,
                    message: ChatMessage {
                        role: "assistant".to_string(),
                        content: "Hello! How can I help you today?".to_string(),
                    },
                }
            ],
        }
    ])
}

async fn create_chat_completion(
    State(secret_key): State<Arc<String>>,
    Json(request): Json<serde_json::Value>
) -> Response {
    // Extract the model ID from the request
    let model_id = request.get("model").and_then(|v| v.as_str()).unwrap_or("").to_string();

    // Get the last user message as the text for inference
    let text = request.get("messages").and_then(|v| {
        v.as_array()
            .and_then(|arr| {
                arr.iter()
                    .rev() // Reverse to find last user message
                    .find(|msg| msg.get("role").and_then(|r| r.as_str()) == Some("user"))
                    .and_then(|msg| msg.get("content").and_then(|c| c.as_str()))
            })
    }).map_or("".to_string(), |s| s.to_string());

    // Get sampling parameters
    let max_new_tokens = request.get("max_tokens").and_then(|v| v.as_u64()).unwrap_or(100) as u32;
    let temperature = request.get("temperature").and_then(|v| v.as_f64()).unwrap_or(0.7) as f32;
    let stream = request.get("stream").and_then(|v| v.as_bool()).unwrap_or(true);

    // Check if this is a streaming request
    if stream {
        handle_streaming_chat_completion(request, None, secret_key).await
    } else {
        handle_non_streaming_chat_completion(request, None, secret_key).await
    }
}

async fn handle_forwarded_chat_completion(
    State(secret_key): State<Arc<String>>,
    Json(request): Json<ForwardedChatCompletionRequest>
) -> Response {
    // Extract the original request from the forwarded request
    let original_request: serde_json::Value = request.original_request;

    // Check if this is a streaming request
    let is_streaming = original_request.get("stream").and_then(|v| v.as_bool()).unwrap_or(true);

    if is_streaming {
        handle_streaming_chat_completion(original_request, request.forwarded_from, secret_key).await
    } else {
        handle_non_streaming_chat_completion(original_request, request.forwarded_from, secret_key).await
    }
}

async fn handle_streaming_chat_completion(
    request: serde_json::Value,
    forwarded_from: Option<String>,
    secret_key: Arc<String>
) -> Response {

    // Extract the model ID from the request
    let model_id = request.get("model").and_then(|v| v.as_str()).unwrap_or("").to_string();

    // Get the last user message as the text for inference
    let text = request.get("messages").and_then(|v| {
        v.as_array()
            .and_then(|arr| {
                arr.iter()
                    .rev() // Reverse to find last user message
                    .find(|msg| msg.get("role").and_then(|r| r.as_str()) == Some("user"))
                    .and_then(|msg| msg.get("content").and_then(|c| c.as_str()))
            })
    }).map_or("".to_string(), |s| s.to_string());

    // Get sampling parameters
    let max_new_tokens = request.get("max_tokens").and_then(|v| v.as_u64()).unwrap_or(100) as u32;
    let temperature = request.get("temperature").and_then(|v| v.as_f64()).unwrap_or(0.7) as f32;
    let stream = request.get("stream").and_then(|v| v.as_bool()).unwrap_or(true);

    // Call form-state to get the node for this model
    match CLIENT
        .get(format!("http://localhost:3004/v1/models/{model_id}/get_node"))
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
                    }))
                ).into_response();
            }

            match response.json::<FormStateNodeResponse>().await {
                Ok(node_response) => {
                    if !node_response.success {
                        return (
                            axum::http::StatusCode::SERVICE_UNAVAILABLE,
                            Json(serde_json::json!({
                                "success": false,
                                "error": "No nodes available for this model"
                            }))
                        ).into_response();
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
                                }))
                            ).into_response();
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
                        return forward_streaming_request_to_node(request, &node_address, secret_key).await;
                    }

                    // Build the inference URL for localhost
                    let inference_url: String = format!("http://127.0.0.1:{}/v1/chat/completions", node_response.port);

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
                                                                    let event = Event::default().json_data(model_data).unwrap();
                                                                    let _ = tx.send(Ok(event)).await;
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

                                                                        let event = Event::default().json_data(content_event).unwrap();
                                                                        let _ = tx.send(Ok(event)).await;
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

                                                            let event = Event::default().json_data(final_event).unwrap();
                                                            let _ = tx.send(Ok(event)).await;
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
                Err(e) => {
                    (
                        axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                        Json(serde_json::json!({
                            "success": false,
                            "error": format!("Failed to parse form-state response: {}", e)
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
                    "error": format!("Failed to fetch model node from form-state: {}", e)
                }))
            ).into_response()
        }
    }
}

async fn handle_non_streaming_chat_completion(
    request: serde_json::Value,
    forwarded_from: Option<String>,
    secret_key: Arc<String>
) -> Response {
    // Extract the model ID from the request
    let model_id = request.get("model").and_then(|v| v.as_str()).unwrap_or("").to_string();

    // Get the last user message as the text for inference
    let text = request.get("messages").and_then(|v| {
        v.as_array()
            .and_then(|arr| {
                arr.iter()
                    .rev() // Reverse to find last user message
                    .find(|msg| msg.get("role").and_then(|r| r.as_str()) == Some("user"))
                    .and_then(|msg| msg.get("content").and_then(|c| c.as_str()))
            })
    }).map_or("".to_string(), |s| s.to_string());

    // Get sampling parameters
    let max_new_tokens = request.get("max_tokens").and_then(|v| v.as_u64()).unwrap_or(100) as u32;
    let temperature = request.get("temperature").and_then(|v| v.as_f64()).unwrap_or(0.7) as f32;
    let stream = request.get("stream").and_then(|v| v.as_bool()).unwrap_or(true);

    // Call form-state to get the node for this model
    match CLIENT
        .get(format!("http://localhost:3004/v1/models/{model_id}/get_node"))
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
                    }))
                ).into_response();
            }

            match response.json::<FormStateNodeResponse>().await {
                Ok(node_response) => {
                    if !node_response.success {
                        return (
                            axum::http::StatusCode::SERVICE_UNAVAILABLE,
                            Json(serde_json::json!({
                                "success": false,
                                "error": "No nodes available for this model"
                            }))
                        ).into_response();
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
                                }))
                            ).into_response();
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
                        return forward_streaming_request_to_node(request, &node_address, secret_key).await;
                    }

                    // Build the inference URL for localhost
                    let inference_url = format!("http://127.0.0.1:{}/v1/chat/completions", node_response.port);

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
                                            let created = std::time::SystemTime::now()
                                                .duration_since(std::time::UNIX_EPOCH)
                                                .unwrap()
                                                .as_secs();

                                            // Create a proper non-streaming chat completion response
                                            let completion = ChatCompletion {
                                                id: completion_id,
                                                model: model_id,
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
                Err(e) => {
                    (
                        axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                        Json(serde_json::json!({
                            "success": false,
                            "error": format!("Failed to parse form-state response: {}", e)
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
                    "error": format!("Failed to fetch model node from form-state: {}", e)
                }))
            ).into_response()
        }
    }
}

async fn forward_streaming_request_to_node(request: serde_json::Value, target_node: &str, secret_key: Arc<String>) -> Response {
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
        secret_key.as_str(),
    ) {
        Ok(header) => header,
        Err(e) => {
            eprintln!("Failed to create signature for forwarded request: {}", e);
            return (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "success": false,
                    "error": format!("Failed to create authentication signature: {}", e)
                }))
            ).into_response();
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
                let is_streaming = request.get("stream").and_then(|v| v.as_bool()).unwrap_or(true);

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
                        Err(e) => {
                            (
                                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                                Json(serde_json::json!({
                                    "success": false,
                                    "error": format!("Failed to parse forwarded response: {}", e)
                                }))
                            ).into_response()
                        }
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
                                }))
                            ).into_response()
                        }
                    }
                    Err(e) => {
                        (
                            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                            Json(serde_json::json!({
                                "success": false,
                                "error": format!("Failed to read forwarded error response: {}", e)
                            }))
                        ).into_response()
                    }
                }
            }
        }
        Err(e) => {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "success": false,
                    "error": format!("Failed to forward request to {}: {}", target_node, e)
                }))
            ).into_response()
        }
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

async fn get_chat_completion(Path(completion_id): Path<String>) -> Json<ChatCompletion> {
    Json(ChatCompletion {
        id: completion_id,
        model: "gpt-4".to_string(),
        choices: vec![
            ChatChoice {
                index: 0,
                message: ChatMessage {
                    role: "assistant".to_string(),
                    content: "Hello! How can I help you today?".to_string(),
                },
            }
        ],
    })
}

async fn update_chat_completion() -> Json<ChatCompletion> {
    Json(ChatCompletion {
        id: "chatcmpl_abc123".to_string(),
        model: "gpt-4".to_string(),
        choices: vec![
            ChatChoice {
                index: 0,
                message: ChatMessage {
                    role: "assistant".to_string(),
                    content: "Updated chat completion.".to_string(),
                },
            }
        ],
    })
}

async fn delete_chat_completion() -> Json<DeleteResponse> {
    Json(DeleteResponse {
        id: "chatcmpl_abc123".to_string(),
        deleted: true,
    })
}