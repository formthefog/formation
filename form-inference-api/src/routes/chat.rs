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
use futures::stream::TryStreamExt;

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

// Request types for chat completions
#[derive(Debug, Serialize, Deserialize)]
struct ChatCompletionRequest {
    model: String,
    messages: Vec<ChatMessage>,
    #[serde(default)]
    max_tokens: Option<u32>,
    #[serde(default)]
    temperature: Option<f32>,
    #[serde(default)]
    stream: Option<bool>,
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
    // Check if streaming is requested
    let should_stream = request.stream.unwrap_or(false);

    if should_stream {
        // Handle streaming response
        handle_streaming_chat_completion(request).await
    } else {
        // Handle non-streaming response
        handle_non_streaming_chat_completion(request).await
    }
}

async fn handle_non_streaming_chat_completion(request: ChatCompletionRequest) -> Response {
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

                    // Build the inference URL based on the address type
                    let inference_url = if node_address == "localhost" {
                        format!("http://127.0.0.1:{}/generate", node_response.port)
                    } else {
                        // For remote nodes, use port 3006 as specified
                        format!("http://{}:3006/generate", node_address)
                    };

                    let inference_request = ModelInferenceRequest {
                        text: text.clone(),
                        sampling_params: SamplingParams {
                            max_new_tokens,
                            temperature,
                        },
                    };

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
                                    // Create a chat completion response
                                    let chat_completion = ChatCompletion {
                                        id: format!("chatcmpl_{}", Uuid::new_v4().simple()),
                                        model: model_id,
                                        choices: vec![
                                            ChatChoice {
                                                index: 0,
                                                message: ChatMessage {
                                                    role: "assistant".to_string(),
                                                    content: inference_result.text,
                                                },
                                            }
                                        ],
                                    };

                                    (StatusCode::OK, Json(serde_json::json!(chat_completion))).into_response()
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

async fn handle_streaming_chat_completion(request: ChatCompletionRequest) -> Response {
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

                    // Build the inference URL based on the address type
                    let inference_url = if node_address == "localhost" {
                        format!("http://127.0.0.1:{}/generate", node_response.port)
                    } else {
                        // For remote nodes, use port 3006 as specified
                        format!("http://{}:3006/generate", node_address)
                    };

                    let inference_request = ModelInferenceRequest {
                        text: text.clone(),
                        sampling_params: SamplingParams {
                            max_new_tokens,
                            temperature,
                        },
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