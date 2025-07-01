use axum::{
    routing::{get, post, delete},
    Json, Router, http::StatusCode, response::IntoResponse, extract::Path,
};
use serde::{Deserialize, Serialize};
use reqwest::Client;
use uuid::Uuid;
use serde_json;

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
    address: String,
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

async fn create_chat_completion(Json(request): Json<ChatCompletionRequest>) -> impl IntoResponse {
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
                );
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
                        );
                    }

                    // Check if the address is localhost
                    if node_response.address == "localhost" {
                        // Make inference request to the model
                        let inference_url = format!("http://127.0.0.1:{}/generate", node_response.port);

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
                                            "error": format!("Model inference failed with status: {}", inference_response.status())
                                        }))
                                    );
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

                                        (StatusCode::OK, Json(serde_json::json!(chat_completion)))
                                    }
                                    Err(e) => {
                                        (
                                            StatusCode::INTERNAL_SERVER_ERROR,
                                            Json(serde_json::json!({
                                                "success": false,
                                                "error": format!("Failed to parse model inference response: {}", e)
                                            }))
                                        )
                                    }
                                }
                            }
                            Err(e) => {
                                (
                                    StatusCode::INTERNAL_SERVER_ERROR,
                                    Json(serde_json::json!({
                                        "success": false,
                                        "error": format!("Failed to make inference request: {}", e)
                                    }))
                                )
                            }
                        }
                    } else {
                        // For non-localhost addresses, return an error or implement remote inference
                        (
                            StatusCode::SERVICE_UNAVAILABLE,
                            Json(serde_json::json!({
                                "success": false,
                                "error": format!("Model is not available on localhost (address: {})", node_response.address)
                            }))
                        )
                    }
                }
                Err(e) => {
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(serde_json::json!({
                            "success": false,
                            "error": format!("Failed to parse form-state response: {}", e)
                        }))
                    )
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
            )
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