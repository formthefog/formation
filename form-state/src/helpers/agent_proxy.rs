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
use crate::helpers::agent_request::RunTaskRequest;
use crate::helpers::agent_response::{TaskStreamChunk, RunTaskResponse};
use crate::helpers::dns;

// Handler for running an agent task
pub async fn run_agent_task(
    Path(agent_id): Path<String>,
    State(state): State<Arc<Mutex<DataStore>>>,
    recovered: EcdsaRecovered,
    Json(mut request): Json<RunTaskRequest>,
) -> Result<Response, (StatusCode, Json<serde_json::Value>)> {
    // Set agent ID from path parameter
    request.agent_id = agent_id.clone();
    
    // Get authentication token from request or use the authenticated user's identity
    let user_address = recovered.as_hex();
    let auth_token = request.formation_auth_token.clone();
    
    info!("Handling run task request for agent: {}", agent_id);
    
    // Check if the agent exists in our database
    let agent_exists = {
        let datastore = state.lock().unwrap();
        datastore.agent_state.get_agent(&agent_id).is_some()
    };
    
    if !agent_exists {
        return Err((
            StatusCode::NOT_FOUND,
            Json(json!({
                "error": format!("Agent with ID {} not found", agent_id)
            })),
        ));
    }
    
    // If streaming is requested, use a different approach
    if request.streaming() {
        handle_streaming_request(agent_id, request, auth_token.as_deref(), user_address).await
    } else {
        handle_non_streaming_request(agent_id, request, auth_token.as_deref(), user_address).await
    }
}

async fn handle_streaming_request(
    agent_id: String,
    request: RunTaskRequest,
    auth_token: Option<&str>,
    user_address: String,
) -> Result<Response, (StatusCode, Json<serde_json::Value>)> {
    // Create channel for streaming response
    let (tx, rx) = mpsc::channel::<Result<String, std::io::Error>>(100);
    
    // Clone needed values for the async task
    let agent_id_clone = agent_id.clone();
    
    // Spawn a task to handle the forward request and stream back responses
    tokio::spawn(async move {
        let result = dns::forward_agent_request(
            &agent_id_clone, 
            &serde_json::to_value(&request).unwrap_or_default(),
            auth_token
        ).await;
        
        match result {
            Ok(response) => {
                let status = response.status();
                if !status.is_success() {
                    let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
                    // Send error as a final message
                    let _ = tx.send(Ok(serde_json::to_string(&TaskStreamChunk {
                        task_id: request.task_id(),
                        agent_id: agent_id_clone,
                        chunk_id: format!("error-{}", uuid::Uuid::new_v4()),
                        content: String::new(),
                        is_final: true,
                        error: Some(error_text),
                        metadata: None,
                        usage: None,
                        timestamp: std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_millis() as u64,
                    }).unwrap())).await;
                    return;
                }
                
                // Process streaming response using response.bytes_stream()
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
                            let _ = tx.send(Ok(serde_json::to_string(&TaskStreamChunk {
                                task_id: request.task_id(),
                                agent_id: agent_id_clone.clone(),
                                chunk_id: format!("error-{}", uuid::Uuid::new_v4()),
                                content: String::new(),
                                is_final: true,
                                error: Some(format!("Stream error: {}", e)),
                                metadata: None,
                                usage: None,
                                timestamp: std::time::SystemTime::now()
                                    .duration_since(std::time::UNIX_EPOCH)
                                    .unwrap_or_default()
                                    .as_millis() as u64,
                            }).unwrap())).await;
                            break;
                        }
                    }
                }
            }
            Err(e) => {
                error!("Failed to forward request: {}", e);
                // Send error as a final message
                let _ = tx.send(Ok(serde_json::to_string(&TaskStreamChunk {
                    task_id: request.task_id(),
                    agent_id: agent_id_clone,
                    chunk_id: format!("error-{}", uuid::Uuid::new_v4()),
                    content: String::new(),
                    is_final: true,
                    error: Some(format!("Failed to forward request: {}", e)),
                    metadata: None,
                    usage: None,
                    timestamp: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_millis() as u64,
                }).unwrap())).await;
            }
        }
    });
    
    // Create a stream from the channel receiver
    let stream = ReceiverStream::new(rx);
    let body = StreamBody::new(stream);
    
    // Create response with appropriate headers
    let mut headers = HeaderMap::new();
    headers.insert("Content-Type", HeaderValue::from_static("text/event-stream"));
    headers.insert("Cache-Control", HeaderValue::from_static("no-cache"));
    headers.insert("Connection", HeaderValue::from_static("keep-alive"));
    
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

async fn handle_non_streaming_request(
    agent_id: String,
    request: RunTaskRequest,
    auth_token: Option<&str>,
    user_address: String,
) -> Result<Response, (StatusCode, Json<serde_json::Value>)> {
    // Forward the request to the agent service
    let result = dns::forward_agent_request(
        &agent_id, 
        &serde_json::to_value(&request).unwrap_or_default(),
        auth_token
    ).await;
    
    match result {
        Ok(response) => {
            let status = response.status();
            // Convert response to our format
            if status.is_success() {
                // Try to parse as RunTaskResponse
                match response.json::<RunTaskResponse>().await {
                    Ok(task_response) => {
                        // Return the response directly
                        Response::builder()
                            .status(StatusCode::OK)
                            .header("Content-Type", "application/json")
                            .body(axum::body::Body::from(serde_json::to_string(&task_response).unwrap()))
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
                        // Failed to parse the response
                        Err((
                            StatusCode::INTERNAL_SERVER_ERROR,
                            Json(json!({
                                "error": format!("Failed to parse response: {}", e)
                            })),
                        ))
                    }
                }
            } else {
                // Return error from the agent service
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

// Handler for agent hiring endpoint
pub async fn hire_agent(
    Path(agent_id): Path<String>,
    State(state): State<Arc<Mutex<DataStore>>>,
    recovered: EcdsaRecovered,
    Json(request): Json<serde_json::Value>,
) -> (StatusCode, Json<serde_json::Value>) {
    // Verify agent exists
    let agent_exists = {
        let datastore = state.lock().unwrap();
        datastore.agent_state.get_agent(&agent_id).is_some()
    };
    
    if !agent_exists {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({
                "error": format!("Agent with ID {} not found", agent_id)
            })),
        );
    }
    
    // Get user address from authentication
    let user_address = recovered.as_hex();
    
    // Forward to the agent service
    match dns::forward_agent_request(
        &agent_id, 
        &request,
        None  // No auth token for hire request
    ).await {
        Ok(response) => {
            let status = response.status();
            match response.text().await {
                Ok(text) => {
                    if status.is_success() {
                        (
                            StatusCode::OK,
                            Json(json!({
                                "status": "success",
                                "response": serde_json::from_str::<serde_json::Value>(&text).unwrap_or_default()
                            })),
                        )
                    } else {
                        (
                            status,
                            Json(json!({
                                "error": text
                            })),
                        )
                    }
                }
                Err(e) => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({
                        "error": format!("Failed to read response: {}", e)
                    })),
                ),
            }
        }
        Err(e) => (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({
                "error": format!("Failed to forward request: {}", e)
            })),
        ),
    }
} 