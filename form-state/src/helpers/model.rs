use crate::datastore::{DataStore, DB_HANDLE, AccountRequest, ModelRequest};
use crate::db::write_datastore;
use crate::agent::*;
use crate::model::*;
use crate::auth::JwtClaims;
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

pub async fn get_model(
    State(datatore): State<Arc<Mutex<DataStore>>>
) {}

pub async fn list_model(
    State(datatore): State<Arc<Mutex<DataStore>>>
) {}

/// Handler for model inference
pub async fn model_inference(
    State(state): State<Arc<Mutex<DataStore>>>,
    JwtClaims(claims): JwtClaims,
    Path(model_id): Path<String>,
    Json(payload): Json<ModelInferenceRequest>,
) -> impl IntoResponse {
    log::info!("User {} is requesting inference from model {}", claims.sub, model_id);
    
    let mut datastore = state.lock().await;
    
    // Check if the model exists
    if let Some(model) = datastore.model_state.get_model(&model_id) {
        // Get the user's account
        if let Some(mut account) = datastore.account_state.get_account(&claims.sub) {
            // Calculate token usage
            let input_tokens = payload.input_tokens.unwrap_or(0);
            let output_tokens = payload.output_tokens.unwrap_or(0);
            let total_tokens = input_tokens + output_tokens;
            
            // Record token usage in the usage tracker
            if let Some(ref mut usage) = account.usage {
                // Record usage using the proper method
                let cost = usage.record_token_usage(&model_id, input_tokens, output_tokens);
                log::info!("Recorded {} tokens ({}+{}) for model {}, cost: {} credits", 
                    total_tokens, input_tokens, output_tokens, model_id, cost);
            } else {
                // Create new usage tracker if none exists
                account.usage = Some(UsageTracker::new());
                if let Some(ref mut usage) = account.usage {
                    let cost = usage.record_token_usage(&model_id, input_tokens, output_tokens);
                    log::info!("Created new usage tracker and recorded {} tokens for model {}, cost: {} credits", 
                        total_tokens, model_id, cost);
                }
            }
            
            // Update the account
            let op = datastore.account_state.update_account_local(account.clone());
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
                    "error": "Account not found"
                }))
            );
        }
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
