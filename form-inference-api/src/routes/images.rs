use axum::{
    extract::State,
    response::IntoResponse,
    response::Response,
    routing::post,
    Json, Router,
};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json;

use crate::auth::auth_middleware;
use crate::AppState;

// Image generation request types
#[derive(Debug, Serialize, Deserialize)]
struct ImageGenerationRequest {
    prompt: String,
    n: Option<u32>,
    size: Option<String>,
    response_format: Option<String>,
    user: Option<String>,
}

// Image generation response types (defined inline in responses)

// Image editing request types
#[derive(Debug, Serialize, Deserialize)]
struct ImageEditRequest {
    image: String, // base64 encoded image
    prompt: String,
    mask: Option<String>, // base64 encoded mask
    n: Option<u32>,
    size: Option<String>,
    response_format: Option<String>,
    user: Option<String>,
}

// Image variations request types
#[derive(Debug, Serialize, Deserialize)]
struct ImageVariationsRequest {
    image: String, // base64 encoded image
    n: Option<u32>,
    size: Option<String>,
    response_format: Option<String>,
    user: Option<String>,
}

// Forwarded request types
#[derive(Debug, Serialize, Deserialize)]
struct ForwardedImageRequest {
    #[serde(flatten)]
    original_request: serde_json::Value,
    #[serde(default)]
    forwarded_from: Option<String>,
}

// Response types from form-state
#[derive(Debug, Serialize, Deserialize)]
struct FormStateNodeResponse {
    success: bool,
    model: String,
    node_id: String,
    port: u16,
    address: serde_json::Value,
}

// Create a lazy static client
lazy_static::lazy_static! {
    static ref CLIENT: Client = Client::new();
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/images/generations", post(create_image_generation))
        .route("/images/edits", post(create_image_edit))
        .route("/images/variations", post(create_image_variations))
        .route(
            "/forwarded/images/generations",
            post(handle_forwarded_image_generation),
        )
        .route(
            "/forwarded/images/edits",
            post(handle_forwarded_image_edit),
        )
        .route(
            "/forwarded/images/variations",
            post(handle_forwarded_image_variations),
        )
        .layer(axum::middleware::from_fn(auth_middleware))
}

// Helper function to extract address from the response
fn extract_node_address(address: &serde_json::Value) -> Option<String> {
    match address {
        serde_json::Value::String(s) => {
            Some(s.clone())
        }
        serde_json::Value::Object(obj) => {
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
    "127.0.0.1".to_string()
}

// Image generation handler
async fn create_image_generation(
    State(app_state): State<AppState>,
    Json(request): Json<ImageGenerationRequest>,
) -> Response {
    // Convert to serde_json::Value for forwarding
    let request_json = serde_json::to_value(&request).unwrap_or_default();

    // Debug: Log the request
    println!("Request: {:?}", request_json);

    let model_id = "stablediffusion";

    println!("Proceeding directly to handle_image_generation (caching disabled)");
    handle_image_generation(request_json, None, app_state, model_id).await
}

// Image edit handler
async fn create_image_edit(
    State(app_state): State<AppState>,
    Json(request): Json<ImageEditRequest>,
) -> Response {
    let request_json = serde_json::to_value(&request).unwrap_or_default();

    let model_id = "stable-diffusion-xl"; // Placeholder model ID
    handle_image_edit(request_json, None, app_state, model_id).await
}

// Image variations handler
async fn create_image_variations(
    State(app_state): State<AppState>,
    Json(request): Json<ImageVariationsRequest>,
) -> Response {
    let request_json = serde_json::to_value(&request).unwrap_or_default();

    let model_id = "stable-diffusion-xl"; // Placeholder model ID
    handle_image_variations(request_json, None, app_state, model_id).await
}

// Forwarded handlers
async fn handle_forwarded_image_generation(
    State(app_state): State<AppState>,
    Json(request): Json<ForwardedImageRequest>,
) -> Response {
    let original_request = request.original_request;
    let model_id = "stable-diffusion-xl"; // Placeholder model ID
    handle_image_generation(original_request, request.forwarded_from, app_state, model_id).await
}

async fn handle_forwarded_image_edit(
    State(app_state): State<AppState>,
    Json(request): Json<ForwardedImageRequest>,
) -> Response {
    let original_request = request.original_request;
    let model_id = "stable-diffusion-xl"; // Placeholder model ID
    handle_image_edit(original_request, request.forwarded_from, app_state, model_id).await
}

async fn handle_forwarded_image_variations(
    State(app_state): State<AppState>,
    Json(request): Json<ForwardedImageRequest>,
) -> Response {
    let original_request = request.original_request;
    let model_id = "stable-diffusion-xl"; // Placeholder model ID
    handle_image_variations(original_request, request.forwarded_from, app_state, model_id).await
}

// Core image generation logic
async fn handle_image_generation(
    request: serde_json::Value,
    forwarded_from: Option<String>,
    app_state: AppState,
    model_id: &str,
) -> Response {
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
                        return forward_image_request_to_node(
                            request,
                            &node_address,
                            app_state,
                            "generations",
                        )
                        .await;
                    }

                    // Build the inference URL for localhost
                    let inference_url = format!(
                        "http://127.0.0.1:{}/v1/images/generations",
                        node_response.port
                    );

                    // Forward the request to the model
                    match CLIENT
                        .post(&inference_url)
                        .header("Content-Type", "application/json")
                        .json(&request)
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

                            match inference_response.json::<serde_json::Value>().await {
                                Ok(model_response) => {
                                    Json(model_response).into_response()
                                }
                                Err(e) => (
                                    axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                                    Json(serde_json::json!({
                                        "success": false,
                                        "error": format!("Failed to parse model response: {}", e)
                                    }))
                                ).into_response()
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

// Core image edit logic
async fn handle_image_edit(
    request: serde_json::Value,
    forwarded_from: Option<String>,
    app_state: AppState,
    model_id: &str,
) -> Response {
    // Similar to handle_image_generation but for edits
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

                    if node_address != "localhost" {
                        return forward_image_request_to_node(
                            request,
                            &node_address,
                            app_state,
                            "edits",
                        )
                        .await;
                    }

                    let inference_url = format!(
                        "http://127.0.0.1:{}/v1/images/edits",
                        node_response.port
                    );

                    match CLIENT
                        .post(&inference_url)
                        .header("Content-Type", "application/json")
                        .json(&request)
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

                            match inference_response.json::<serde_json::Value>().await {
                                Ok(model_response) => {

                                    Json(model_response).into_response()
                                }
                                Err(e) => (
                                    axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                                    Json(serde_json::json!({
                                        "success": false,
                                        "error": format!("Failed to parse model response: {}", e)
                                    }))
                                ).into_response()
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

// Core image variations logic
async fn handle_image_variations(
    request: serde_json::Value,
    forwarded_from: Option<String>,
    app_state: AppState,
    model_id: &str,
) -> Response {
    // Similar to handle_image_generation but for variations
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

                    if node_address != "localhost" {
                        return forward_image_request_to_node(
                            request,
                            &node_address,
                            app_state,
                            "variations",
                        )
                        .await;
                    }

                    let inference_url = format!(
                        "http://127.0.0.1:{}/v1/images/variations",
                        node_response.port
                    );

                    match CLIENT
                        .post(&inference_url)
                        .header("Content-Type", "application/json")
                        .json(&request)
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

                            match inference_response.json::<serde_json::Value>().await {
                                Ok(model_response) => {
                                    Json(model_response).into_response()
                                }
                                Err(e) => (
                                    axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                                    Json(serde_json::json!({
                                        "success": false,
                                        "error": format!("Failed to parse model response: {}", e)
                                    }))
                                ).into_response()
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

// Forward image request to another node
async fn forward_image_request_to_node(
    request: serde_json::Value,
    target_node: &str,
    _app_state: AppState,
    endpoint: &str,
) -> Response {
    let current_node_ip = get_current_node_ip().await;

    let forwarded_request = ForwardedImageRequest {
        original_request: request.clone(),
        forwarded_from: Some(current_node_ip),
    };

    let forward_url = format!("http://{target_node}:3006/forwarded/images/{endpoint}");

    match CLIENT
        .post(&forward_url)
        .header("Content-Type", "application/json")
        .json(&forwarded_request)
        .send()
        .await
    {
        Ok(response) => {
            if response.status().is_success() {
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
            } else {
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
