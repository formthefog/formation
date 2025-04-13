use crate::datastore::{DataStore, DB_HANDLE, AccountRequest, ModelRequest};
use crate::db::write_datastore;
use crate::agent::*;
use crate::model::*;
use crate::auth::{JwtClaims, can_view_models, extract_user_info};
use crate::api_keys::ApiKeyAuth;
use crate::billing::{UsageTracker, PeriodUsage};
use std::sync::Arc;
use tokio::sync::Mutex;
use axum::{extract::{State, Path}, Json};
use form_types::state::{Response, Success};
use std::collections::{BTreeMap, HashMap};
use serde::{Serialize, Deserialize};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use chrono::{Utc, DateTime};
use serde_json::json;

pub async fn create_model(
    State(datatore): State<Arc<Mutex<DataStore>>>
) {}

pub async fn update_model(
    State(datatore): State<Arc<Mutex<DataStore>>>
) {}

pub async fn delete_model(
    State(datatore): State<Arc<Mutex<DataStore>>>
) {}

/// Get information about a specific AI model
pub async fn get_model(
    State(state): State<Arc<Mutex<DataStore>>>,
    auth: ApiKeyAuth,
    Path(model_id): Path<String>,
) -> impl IntoResponse {
    log::info!("User {} is requesting model {}", auth.account.address, model_id);
    
    // Check operation permission (in a real implementation, we'd check the API key scope)
    if !auth.api_key.can_perform_operation("models.get") {
        return (
            StatusCode::FORBIDDEN,
            Json(json!({
                "success": false,
                "error": "API key does not have permission to view models"
            }))
        );
    }
    
    // Get the model from datastore
    let datastore = state.lock().await;
    match datastore.model_state.get_model(&model_id) {
        Some(model) => {
            // Log access for auditing
            log::info!("Model access: {} by account {}", model_id, auth.account.address);
            
            // Return the model with 200 OK
            (
                StatusCode::OK, 
                Json(json!({
                    "success": true,
                    "model": model
                }))
            )
        },
        None => {
            // Model not found
            (
                StatusCode::NOT_FOUND,
                Json(json!({
                    "success": false,
                    "error": format!("Model {} not found", model_id)
                }))
            )
        }
    }
}

/// List all available models
pub async fn list_model(
    State(state): State<Arc<Mutex<DataStore>>>,
    auth: ApiKeyAuth,
) -> impl IntoResponse {
    log::info!("Account {} is requesting list of all models", auth.account.address);
    
    // Check operation permission
    if !auth.api_key.can_perform_operation("models.list") {
        return (
            StatusCode::FORBIDDEN,
            Json(json!({
                "success": false,
                "error": "API key does not have permission to list models"
            }))
        );
    }
    
    // Get all models from datastore
    let datastore = state.lock().await;
    let all_models = datastore.model_state.list_models();
    
    // Return the models with 200 OK
    (
        StatusCode::OK, 
        Json(json!({
            "success": true,
            "models": all_models,
            "total": all_models.len()
        }))
    )
}

/// Handler for model inference
pub async fn model_inference(
    State(state): State<Arc<Mutex<DataStore>>>,
    auth: ApiKeyAuth,
    Path(model_id): Path<String>,
    Json(payload): Json<ModelInferenceRequest>,
) -> impl IntoResponse {
    log::info!("Account {} is requesting inference from model {}", auth.account.address, model_id);
    
    // Check operation permission
    if !auth.api_key.can_perform_operation("models.inference") {
        return (
            StatusCode::FORBIDDEN,
            Json(json!({
                "success": false,
                "error": "API key does not have permission to use model inference"
            }))
        );
    }
    
    let mut datastore = state.lock().await;
    
    // Check if the model exists
    if let Some(model) = datastore.model_state.get_model(&model_id) {
        // Get the user's account
        let mut account = auth.account.clone();
        
        // Calculate token usage
        let input_tokens = payload.input_tokens.unwrap_or(0);
        let output_tokens = payload.output_tokens.unwrap_or(0);
        let total_tokens = input_tokens + output_tokens;
        
        // Check if the account has enough credits for this operation
        use crate::billing::middleware::{check_operation_credits, OperationType};
        let operation = OperationType::TokenConsumption {
            model_id: model_id.clone(),
            input_tokens,
            output_tokens,
        };
        
        // Validate eligibility
        if let Err(err) = check_operation_credits(&account, operation) {
            let error_message = format!("{}", err);
            log::warn!("Inference rejected: {}", error_message);
            
            return (
                StatusCode::PAYMENT_REQUIRED,
                Json(json!({
                    "success": false,
                    "error": error_message,
                    "available_credits": account.available_credits()
                }))
            );
        }
        
        // Clone the account for usage tracking
        let mut account_clone = account.clone();
        
        // Initialize usage tracker if needed
        if account_clone.usage.is_none() {
            account_clone.usage = Some(UsageTracker::new());
        }
        
        // Now we know usage exists, we can use expect safely
        let cost = account_clone.usage.as_mut().expect("Usage tracker exists")
            .record_token_usage(&model_id, input_tokens, output_tokens);
        
        log::info!("Recorded {} tokens ({}+{}) for model {}, cost: {} credits", 
            total_tokens, input_tokens, output_tokens, model_id, cost);
        
        // Update the account
        let op = datastore.account_state.update_account_local(account_clone);
        if let Err(err) = datastore.handle_account_op(op).await {
            log::error!("Failed to update account usage tracking: {}", err);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "success": false,
                    "error": "Failed to update usage tracking"
                }))
            );
        }
        
        // Return success with mock inference result
        // In a real implementation, this would call the actual model inference service
        return (
            StatusCode::OK,
            Json(json!({
                "success": true,
                "model": model_id,
                "tokens_used": total_tokens,
                "result": payload.prompt.unwrap_or_else(|| "No prompt provided".to_string()),
                "remaining_credits": account.available_credits()
            }))
        );
    } else {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({
                "success": false,
                "error": format!("Model {} not found", model_id)
            }))
        );
    }
}

/// Request for model inference
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInferenceRequest {
    /// The prompt to generate from
    pub prompt: Option<String>,
    
    /// Number of input tokens
    pub input_tokens: Option<u64>,
    
    /// Number of output tokens
    pub output_tokens: Option<u64>,
    
    /// Additional model parameters
    pub parameters: Option<serde_json::Value>,
}
