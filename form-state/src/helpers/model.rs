use crate::datastore::{DataStore, ModelRequest};
use crate::billing::UsageTracker;
use std::sync::Arc;
use tokio::sync::Mutex;
use axum::{extract::{State, Path}, Json};
use serde::{Serialize, Deserialize};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use chrono::Utc;
use serde_json::json;
use crate::auth::RecoveredAddress;

// Helper function to determine if an address belongs to an admin
// This would ideally be replaced with a proper role-based system
fn is_admin_address(address: &str) -> bool {
    // For now, we'll use a simple check for a specific address pattern
    // In a real system, this would query against a database or use JWT claims
    address.to_lowercase() == "0xadmin" || address.starts_with("0x000admin")
}

pub async fn create_model(
    State(state): State<Arc<Mutex<DataStore>>>,
    recovered: RecoveredAddress,
    Json(model_data): Json<ModelRequest>,
) -> impl IntoResponse {
    log::info!("Account {} is attempting to create a new model", recovered.as_hex());
    
    // Validate the model data
    let model_id = match &model_data {
        ModelRequest::Create(model) => {
            if model.name.is_empty() {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(json!({
                        "success": false,
                        "error": "Model name cannot be empty"
                    }))
                );
            }
            model.model_id.clone()
        },
        _ => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({
                    "success": false,
                    "error": "Invalid request type for model creation"
                }))
            );
        }
    };
    
    // Check for duplicate model ID/name
    let mut datastore = state.lock().await;
    if let Some(existing) = datastore.model_state.get_model(&model_id) {
        return (
            StatusCode::CONFLICT,
            Json(json!({
                "success": false,
                "error": format!("Model with ID {} already exists", model_id),
                "existing_model": existing
            }))
        );
    }
    
    // Get or create account for the user
    let account_address = recovered.as_hex();
    let mut account = match datastore.account_state.get_account(&account_address) {
        Some(acc) => acc,
        None => {
            // Create a new account if it doesn't exist
            let new_account = crate::accounts::Account::new(account_address.clone());
            let op = datastore.account_state.update_account_local(new_account.clone());
            if let Err(_) = datastore.handle_account_op(op).await {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({
                        "success": false,
                        "error": "Failed to create new account"
                    }))
                );
            }
            new_account
        }
    };
    
    // Set the owner to the current account
    let model = match model_data {
        ModelRequest::Create(mut model) => {
            model.owner_id = account_address.clone();
            model.created_at = Utc::now().timestamp();
            model.updated_at = Utc::now().timestamp();
            model
        },
        _ => unreachable!(), // We already checked this above
    };
    
    // Create the model in the datastore
    let op = datastore.model_state.update_model_local(model.clone());
    
    // Apply the operation
    if let Err(e) = datastore.handle_model_op(op.clone()).await {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({
                "success": false,
                "error": format!("Failed to create model: {}", e)
            }))
        );
    }
    
    // Add model to account's owned models
    account.add_owned_model(model.model_id.clone());
    let account_op = datastore.account_state.update_account_local(account);
    if let Err(e) = datastore.handle_account_op(account_op).await {
        log::error!("Failed to update account with owned model: {}", e);
    }
    
    // Return success with the created model
    (
        StatusCode::CREATED,
        Json(json!({
            "success": true,
            "message": format!("Model {} created successfully", model.model_id),
            "model": model
        }))
    )
}

pub async fn update_model(
    State(state): State<Arc<Mutex<DataStore>>>,
    recovered: RecoveredAddress,
    Json(model_data): Json<ModelRequest>,
) -> impl IntoResponse {
    log::info!("Account {} is attempting to update a model", recovered.as_hex());
    
    // Ensure we have a valid update request
    let model_id = match &model_data {
        ModelRequest::Update(model) => model.model_id.clone(),
        _ => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({
                    "success": false,
                    "error": "Invalid request type for model update"
                }))
            );
        }
    };
    
    // Get the existing model
    let mut datastore = state.lock().await;
    let existing_model = match datastore.model_state.get_model(&model_id) {
        Some(model) => model,
        None => {
            return (
                StatusCode::NOT_FOUND,
                Json(json!({
                    "success": false,
                    "error": format!("Model with ID {} not found", model_id)
                }))
            );
        }
    };
    
    // Get the account address
    let account_address = recovered.as_hex();
    
    // Verify ownership - only the owner can update
    if existing_model.owner_id != account_address {
        return (
            StatusCode::FORBIDDEN,
            Json(json!({
                "success": false,
                "error": "You do not have permission to update this model"
            }))
        );
    }
    
    // Update the model
    let updated_model = match model_data {
        ModelRequest::Update(mut model) => {
            // Preserve owner and creation timestamp
            model.owner_id = existing_model.owner_id.clone();
            model.created_at = existing_model.created_at;
            // Update the timestamp
            model.updated_at = Utc::now().timestamp();
            model
        },
        _ => unreachable!(), // We already checked this above
    };
    
    // Create the model update operation
    let op = datastore.model_state.update_model_local(updated_model.clone());
    
    // Apply the operation
    if let Err(e) = datastore.handle_model_op(op.clone()).await {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({
                "success": false,
                "error": format!("Failed to update model: {}", e)
            }))
        );
    }
    
    // Return success with the updated model
    (
        StatusCode::OK,
        Json(json!({
            "success": true,
            "message": format!("Model {} updated successfully", model_id),
            "model": updated_model
        }))
    )
}

pub async fn delete_model(
    State(state): State<Arc<Mutex<DataStore>>>,
    recovered: RecoveredAddress,
    Json(model_data): Json<ModelRequest>,
) -> impl IntoResponse {
    log::info!("Account {} is attempting to delete a model", recovered.as_hex());
    
    // Ensure we have a valid delete request
    let model_id = match &model_data {
        ModelRequest::Delete(id) => id.clone(),
        _ => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({
                    "success": false,
                    "error": "Invalid request type for model deletion"
                }))
            );
        }
    };
    
    // Get the existing model to verify ownership
    let mut datastore = state.lock().await;
    let existing_model = match datastore.model_state.get_model(&model_id) {
        Some(model) => model,
        None => {
            return (
                StatusCode::NOT_FOUND,
                Json(json!({
                    "success": false,
                    "error": format!("Model with ID {} not found", model_id)
                }))
            );
        }
    };
    
    // Get the account address
    let account_address = recovered.as_hex();
    
    // Verify ownership - only the owner can delete
    if existing_model.owner_id != account_address {
        return (
            StatusCode::FORBIDDEN,
            Json(json!({
                "success": false,
                "error": "You do not have permission to delete this model"
            }))
        );
    }
    
    // Create the model deletion operation
    let op = datastore.model_state.remove_model_local(model_id.clone());
    
    // Apply the operation
    if let Err(e) = datastore.handle_model_op(op.clone()).await {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({
                "success": false,
                "error": format!("Failed to delete model: {}", e)
            }))
        );
    }
    
    // Get the account to remove the model from owned models
    if let Some(mut account) = datastore.account_state.get_account(&account_address) {
        account.remove_owned_model(&model_id);
        let account_op = datastore.account_state.update_account_local(account);
        if let Err(e) = datastore.handle_account_op(account_op).await {
            log::error!("Failed to update account after model deletion: {}", e);
        }
    }
    
    // Return success
    (
        StatusCode::OK,
        Json(json!({
            "success": true,
            "message": format!("Model {} deleted successfully", model_id)
        }))
    )
}

/// Get information about a specific AI model
pub async fn get_model(
    State(state): State<Arc<Mutex<DataStore>>>,
    recovered: RecoveredAddress,
    Path(model_id): Path<String>,
) -> impl IntoResponse {
    log::info!("User {} is requesting model {}", recovered.as_hex(), model_id);
    
    // Get the authenticated user's address
    let authenticated_address = recovered.as_hex();
    
    // Get the model from datastore
    let datastore = state.lock().await;
    
    match datastore.model_state.get_model(&model_id) {
        Some(model) => {
            // Check authorization if the model is private
            if model.is_private {
                // Check if user is the owner, has authorization, or is an admin
                let account = datastore.account_state.get_account(&authenticated_address);
                
                // Determine if the user has access to this model
                let is_authorized = match account {
                    Some(account) => {
                        // User is authorized if they own the model or are an admin
                        account.owned_models.contains(&model_id) || 
                        is_admin_address(&authenticated_address) ||
                        model.owner_id == authenticated_address
                    },
                    None => false
                };
                
                if !is_authorized {
                    log::warn!("Unauthorized attempt to access private model: {} by {}", model_id, authenticated_address);
                    return (
                        StatusCode::FORBIDDEN,
                        Json(json!({
                            "success": false,
                            "error": "You don't have permission to access this private model"
                        }))
                    );
                }
            }
            
            // Log access for auditing
            log::info!("Model access: {} by account {}", model_id, recovered.as_hex());
            
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
    recovered: RecoveredAddress,
) -> impl IntoResponse {
    log::info!("Account {} is requesting list of all models", recovered.as_hex());
    
    // Get the authenticated user's address
    let authenticated_address = recovered.as_hex();
    
    // Get all models from datastore
    let datastore = state.lock().await;
    
    // Check if the user is an admin
    let is_admin = is_admin_address(&authenticated_address);
    
    // Get the account
    let account = datastore.account_state.get_account(&authenticated_address);
    
    // Get all models
    let all_models = datastore.model_state.list_models();
    
    // Filter the models based on authorization
    let filtered_models: Vec<_> = all_models
        .into_iter()
        .filter(|(model_id, model)| {
            // If model is not private, everyone can see it
            if !model.is_private {
                return true;
            }
            
            // Admins can see all models
            if is_admin {
                return true;
            }
            
            // Owner can see their own models
            if model.owner_id == authenticated_address {
                return true;
            }
            
            // Check if user has ownership in their account records
            if let Some(acc) = &account {
                if acc.owned_models.contains(model_id) {
                    return true;
                }
            }
            
            // Otherwise, the user can't see this model
            false
        })
        .map(|(_, model)| model)
        .collect();
    
    // Return the filtered models with 200 OK
    (
        StatusCode::OK, 
        Json(json!({
            "success": true,
            "models": filtered_models,
            "total": filtered_models.len()
        }))
    )
}

/// Handler for model inference
pub async fn model_inference(
    State(state): State<Arc<Mutex<DataStore>>>,
    recovered: RecoveredAddress,
    Path(model_id): Path<String>,
    Json(payload): Json<ModelInferenceRequest>,
) -> impl IntoResponse {
    let account_address = recovered.as_hex();
    log::info!("Account {} is requesting inference from model {}", account_address, model_id);
    
    let mut datastore = state.lock().await;
    
    // Check if the model exists
    if let Some(model) = datastore.model_state.get_model(&model_id) {
        // Check authorization if the model is private
        if model.is_private {
            // Check if user is the owner, has authorization, or is an admin
            let account = datastore.account_state.get_account(&account_address);
            
            // Determine if the user has access to this model
            let is_authorized = match &account {
                Some(account) => {
                    // User is authorized if they own the model or are an admin
                    account.owned_models.contains(&model_id) || 
                    is_admin_address(&account_address) ||
                    model.owner_id == account_address
                },
                None => false
            };
            
            if !is_authorized {
                log::warn!("Unauthorized attempt to use private model for inference: {} by {}", model_id, account_address);
                return (
                    StatusCode::FORBIDDEN,
                    Json(json!({
                        "success": false,
                        "error": "You don't have permission to use this private model for inference"
                    }))
                );
            }
        }
        
        // Get or create the user's account
        let mut account = match datastore.account_state.get_account(&account_address) {
            Some(acc) => acc,
            None => {
                // Create a new account if it doesn't exist
                let new_account = crate::accounts::Account::new(account_address.clone());
                let op = datastore.account_state.update_account_local(new_account.clone());
                if let Err(_) = datastore.handle_account_op(op).await {
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(json!({
                            "success": false,
                            "error": "Failed to create new account"
                        }))
                    );
                }
                new_account
            }
        };
        
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
        
        // Initialize usage tracker if needed
        if account.usage.is_none() {
            account.usage = Some(UsageTracker::new());
        }
        
        // Now we know usage exists, we can use expect safely
        let cost = account.usage.as_mut().expect("Usage tracker exists")
            .record_token_usage(&model_id, input_tokens, output_tokens);
        
        log::info!("Recorded {} tokens ({}+{}) for model {}, cost: {} credits", 
            total_tokens, input_tokens, output_tokens, model_id, cost);
        
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
        
        // Return successful response
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
