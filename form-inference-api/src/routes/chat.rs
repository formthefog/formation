use axum::{
    routing::{get, post, delete},
    Json, Router, http::StatusCode, response::IntoResponse, extract::Path,
    response::sse::{Event, Sse},
    response::Response,
};
use serde::{Deserialize, Serialize};
use reqwest::Client;
use uuid::Uuid;
use serde_json;
use tokio_stream::wrappers::ReceiverStream;

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
#[derive(Debug, Serialize, Deserialize)]
struct ChatCompletionRequest {
    model: String,
    messages: Vec<ChatMessage>,
    #[serde(default)]
    max_tokens: Option<u32>,
    #[serde(default)]
    temperature: Option<f32>,
}

// Forwarded request type (same as ChatCompletionRequest but with additional metadata)
#[derive(Debug, Serialize, Deserialize)]
struct ForwardedChatCompletionRequest {
    model: String,
    messages: Vec<ChatMessage>,
    #[serde(default)]
    max_tokens: Option<u32>,
    #[serde(default)]
    temperature: Option<f32>,
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

pub fn router() -> Router {
    Router::new()
        .route("/chat/completions", get(list_chat_completions))
        .route("/chat/completions", post(create_chat_completion))
        .route("/forwarded/chat/completions", post(handle_forwarded_chat_completion))
        .route("/chat/completions/:completion_id", get(get_chat_completion))
        .route("/chat/completions/:completion_id", post(update_chat_completion))
        .route("/chat/completions/:completion_id", delete(delete_chat_completion))
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

async fn create_chat_completion(Json(request): Json<ChatCompletionRequest>) -> Response {
    handle_streaming_chat_completion(request, None).await
}

async fn handle_forwarded_chat_completion(Json(request): Json<ForwardedChatCompletionRequest>) -> Response {
    // Convert to regular ChatCompletionRequest
    let chat_request = ChatCompletionRequest {
        model: request.model,
        messages: request.messages,
        max_tokens: request.max_tokens,
        temperature: request.temperature,
    };
    handle_streaming_chat_completion(chat_request, request.forwarded_from).await
}

async fn handle_streaming_chat_completion(request: ChatCompletionRequest, forwarded_from: Option<String>) -> Response {
    // Extract the model ID from the request
    let model_id = request.model.clone();

    // Get the last user message as the text for inference
    let text = request.messages
        .iter()
        .filter(|msg| msg.role == "user")
        .last()
        .map(|msg| msg.content.clone())
        .unwrap_or_else(|| "Hello".to_string());

    // Get sampling parameters
    let max_new_tokens = request.max_tokens.unwrap_or(100);
    let temperature = request.temperature.unwrap_or(0.7);

    // Call form-state to get the node for this model
    match CLIENT
        .get(&format!("http://localhost:3004/v1/models/{}/get_node", model_id))
        .send()
        .await
    {
        Ok(response) => {
            if !response.status().is_success() {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
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
                            StatusCode::SERVICE_UNAVAILABLE,
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
                                StatusCode::INTERNAL_SERVER_ERROR,
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
                                StatusCode::SERVICE_UNAVAILABLE,
                                Json(serde_json::json!({
                                    "success": false,
                                    "error": format!("Model not available on this node (forwarded from {})", from)
                                }))
                            ).into_response();
                        }
                    }

                    // If the address is not localhost, forward to that node
                    if node_address != "localhost" {
                        return forward_streaming_request_to_node(request, &node_address).await;
                    }

                    // Build the inference URL for localhost
                    let inference_url = format!("http://127.0.0.1:{}/generate", node_response.port);

                    let inference_request = ModelInferenceRequest {
                        text: text.clone(),
                        sampling_params: SamplingParams {
                            max_new_tokens,
                            temperature,
                        },
                        stream: true,
                        stop_at_limit: false,
                    };

                    // For streaming, we'll make a regular request and simulate streaming
                    // In a real implementation, you'd want to check if the model service supports streaming
                    match CLIENT
                        .post(&inference_url)
                        .header("Content-Type", "application/json")
                        .json(&inference_request)
                        .send()
                        .await
                    {
                        Ok(inference_response) => {
                            if !inference_response.status().is_success() {
                                return (
                                    StatusCode::INTERNAL_SERVER_ERROR,
                                    Json(serde_json::json!({
                                        "success": false,
                                        "error": format!("Model inference failed with status: {} (URL: {})", inference_response.status(), inference_url)
                                    }))
                                ).into_response();
                            }

                            match inference_response.json::<ModelInferenceResponse>().await {
                                Ok(inference_result) => {
                                    // Create streaming response
                                    let completion_id = format!("chatcmpl_{}", Uuid::new_v4().simple());
                                    let created = std::time::SystemTime::now()
                                        .duration_since(std::time::UNIX_EPOCH)
                                        .unwrap()
                                        .as_secs();

                                    // Split the response text into chunks for streaming
                                    let text_chunks: Vec<String> = inference_result.text
                                        .chars()
                                        .collect::<Vec<char>>()
                                        .chunks(10) // Stream in chunks of 10 characters
                                        .map(|chunk| chunk.iter().collect())
                                        .collect();

                                    let (tx, rx) = tokio::sync::mpsc::channel::<Result<Event, std::io::Error>>(100);

                                    // Spawn task to send streaming events
                                    tokio::spawn(async move {
                                        // Send initial event with role
                                        let initial_event = StreamingChatCompletion {
                                            id: completion_id.clone(),
                                            model: model_id.clone(),
                                            choices: vec![
                                                StreamingChatChoice {
                                                    index: 0,
                                                    delta: StreamingDelta {
                                                        role: Some("assistant".to_string()),
                                                        content: None,
                                                    },
                                                    finish_reason: None,
                                                }
                                            ],
                                            created,
                                            object: "chat.completion.chunk".to_string(),
                                        };

                                        if let Ok(()) = tx.send(Ok(Event::default().json_data(initial_event).unwrap())).await {
                                            // Send content chunks
                                            for chunk in text_chunks {
                                                let content_event = StreamingChatCompletion {
                                                    id: completion_id.clone(),
                                                    model: model_id.clone(),
                                                    choices: vec![
                                                        StreamingChatChoice {
                                                            index: 0,
                                                            delta: StreamingDelta {
                                                                role: None,
                                                                content: Some(chunk),
                                                            },
                                                            finish_reason: None,
                                                        }
                                                    ],
                                                    created,
                                                    object: "chat.completion.chunk".to_string(),
                                                };

                                                if let Ok(()) = tx.send(Ok(Event::default().json_data(content_event).unwrap())).await {
                                                    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
                                                } else {
                                                    break;
                                                }
                                            }

                                            // Send final event
                                            let final_event = StreamingChatCompletion {
                                                id: completion_id,
                                                model: model_id,
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

                                            let _ = tx.send(Ok(Event::default().json_data(final_event).unwrap())).await;
                                        }
                                    });

                                    Sse::new(ReceiverStream::new(rx)).into_response()
                                }
                                Err(e) => {
                                    (
                                        StatusCode::INTERNAL_SERVER_ERROR,
                                        Json(serde_json::json!({
                                            "success": false,
                                            "error": format!("Failed to parse model inference response: {}", e)
                                        }))
                                    ).into_response()
                                }
                            }
                        }
                        Err(e) => {
                            (
                                StatusCode::INTERNAL_SERVER_ERROR,
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
                        StatusCode::INTERNAL_SERVER_ERROR,
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
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "success": false,
                    "error": format!("Failed to fetch model node from form-state: {}", e)
                }))
            ).into_response()
        }
    }
}

async fn forward_streaming_request_to_node(request: ChatCompletionRequest, target_node: &str) -> Response {
    let current_node_ip = get_current_node_ip().await;

    // Create forwarded request
    let forwarded_request = ForwardedChatCompletionRequest {
        model: request.model,
        messages: request.messages,
        max_tokens: request.max_tokens,
        temperature: request.temperature,
        forwarded_from: Some(current_node_ip),
    };

    // Forward to the target node's form-inference-api
    let forward_url = format!("http://{}:3000/forwarded/chat/completions", target_node);

    match CLIENT
        .post(&forward_url)
        .header("Content-Type", "application/json")
        .json(&forwarded_request)
        .send()
        .await
    {
        Ok(response) => {
            // For streaming responses, we need to handle them differently
            // For now, we'll return an error indicating streaming forwarding is not fully implemented
            (
                StatusCode::NOT_IMPLEMENTED,
                Json(serde_json::json!({
                    "success": false,
                    "error": "Streaming forwarding between nodes is not yet implemented"
                }))
            ).into_response()
        }
        Err(e) => {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "success": false,
                    "error": format!("Failed to forward streaming request to {}: {}", target_node, e)
                }))
            ).into_response()
        }
    }
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