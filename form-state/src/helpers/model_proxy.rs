use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Response,
    Json,
};
use std::sync::{Arc, Mutex};
use serde_json::json;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use axum::body::StreamBody;
use reqwest::header::{HeaderMap, HeaderValue};
use log::{info, error};

use crate::DataStore;
use crate::auth::EcdsaRecovered;
use crate::helpers::dns;

// Handler for generating text with a model
pub async fn generate_text(
    Path(model_id): Path<String>,
    State(state): State<Arc<Mutex<DataStore>>>,
    recovered: EcdsaRecovered,
    Json(request): Json<serde_json::Value>,
) -> Result<Response, (StatusCode, Json<serde_json::Value>)> {
    // Get user address from authentication
    let user_address = recovered.as_hex();
    
    // Extract auth token if present
    let auth_token = request.get("auth_token")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    
    info!("Handling generation request for model: {}", model_id);
    
    // Check if the model exists in our database
    let model_exists = {
        let datastore = state.lock().unwrap();
        datastore.model_state.get_model(&model_id).is_some()
    };
    
    if !model_exists {
        return Err((
            StatusCode::NOT_FOUND,
            Json(json!({
                "error": format!("Model with ID {} not found", model_id)
            })),
        ));
    }
    
    // Check if streaming is requested
    let is_streaming = request.get("stream")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    
    if is_streaming {
        handle_streaming_model_request(model_id, request, auth_token.as_deref()).await
    } else {
        handle_non_streaming_model_request(model_id, request, auth_token.as_deref()).await
    }
}

async fn handle_streaming_model_request(
    model_id: String,
    request: serde_json::Value,
    auth_token: Option<&str>,
) -> Result<Response, (StatusCode, Json<serde_json::Value>)> {
    // Create channel for streaming response
    let (tx, rx) = mpsc::channel::<Result<String, std::io::Error>>(100);
    
    // Clone needed values for the async task
    let model_id_clone = model_id.clone();
    
    // Spawn a task to handle the forward request and stream back responses
    tokio::spawn(async move {
        let result = dns::forward_model_request(
            &model_id_clone, 
            &request,
            auth_token
        ).await;
        
        match result {
            Ok(response) => {
                let status = response.status();
                if !status.is_success() {
                    let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
                    // Send error as a final message
                    let _ = tx.send(Ok(json!({
                        "error": error_text,
                        "model_id": model_id_clone,
                        "is_final": true
                    }).to_string())).await;
                    return;
                }
                
                // Process streaming response
                let mut stream = response.bytes_stream();
                use futures::StreamExt;
                
                while let Some(item) = stream.next().await {
                    match item {
                        Ok(bytes) => {
                            let chunk_str = String::from_utf8_lossy(&bytes);
                            let _ = tx.send(Ok(chunk_str.to_string())).await;
                        }
                        Err(e) => {
                            error!("Error reading stream: {}", e);
                            // Send error message
                            let _ = tx.send(Ok(json!({
                                "error": format!("Stream error: {}", e),
                                "model_id": model_id_clone,
                                "is_final": true
                            }).to_string())).await;
                            break;
                        }
                    }
                }
            }
            Err(e) => {
                error!("Failed to forward model request: {}", e);
                // Send error as a final message
                let _ = tx.send(Ok(json!({
                    "error": format!("Failed to forward request: {}", e),
                    "model_id": model_id_clone,
                    "is_final": true
                }).to_string())).await;
            }
        }
    });
    
    // Create a stream from the channel receiver
    let stream = ReceiverStream::new(rx);
    let body = StreamBody::new(stream);
    
    // Create and return the streaming response
    let response = Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "text/event-stream")
        .header("Cache-Control", "no-cache")
        .header("Connection", "keep-alive")
        .body(body.into())
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "error": format!("Failed to create streaming response: {}", e)
                })),
            )
        })?;
    
    Ok(response)
}

async fn handle_non_streaming_model_request(
    model_id: String,
    request: serde_json::Value,
    auth_token: Option<&str>,
) -> Result<Response, (StatusCode, Json<serde_json::Value>)> {
    // Forward the request to the model service
    let result = dns::forward_model_request(
        &model_id, 
        &request,
        auth_token
    ).await;
    
    match result {
        Ok(response) => {
            let status = response.status();
            if status.is_success() {
                // Try to parse as JSON
                match response.json::<serde_json::Value>().await {
                    Ok(json_response) => {
                        // Return the response directly
                        Response::builder()
                            .status(StatusCode::OK)
                            .header("Content-Type", "application/json")
                            .body(axum::body::Body::from(serde_json::to_string(&json_response).unwrap()))
                            .map_err(|e| {
                                (
                                    StatusCode::INTERNAL_SERVER_ERROR,
                                    Json(json!({
                                        "error": format!("Failed to create response: {}", e)
                                    })),
                                )
                            })
                    }
                    Err(e) => {
                        // Failed to parse the response as JSON
                        Err((
                            StatusCode::INTERNAL_SERVER_ERROR,
                            Json(json!({
                                "error": format!("Failed to parse response: {}", e)
                            })),
                        ))
                    }
                }
            } else {
                // Return error from the model service
                let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
                Err((
                    status,
                    Json(json!({
                        "error": error_text
                    })),
                ))
            }
        }
        Err(e) => {
            // Return error from DNS lookup or forwarding
            Err((
                StatusCode::SERVICE_UNAVAILABLE,
                Json(json!({
                    "error": format!("Failed to forward request: {}", e)
                })),
            ))
        }
    }
}