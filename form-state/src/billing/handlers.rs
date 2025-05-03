//! API handlers for billing-related operations
//! 
//! This module provides handlers for:
//! 1. Checking subscription status
//! 2. Managing credits
//! 3. Viewing usage statistics

use axum::{
    extract::{State, Json},
    http::StatusCode,
    response::IntoResponse,
};
use std::sync::Arc;
use tokio::sync::Mutex;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::datastore::DataStore;
use crate::billing::{SubscriptionInfo, SubscriptionStatus, SubscriptionTier};
use crate::signature_auth::SignatureAuth;

/// Response for usage statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageResponse {
    /// Total agent requests
    pub total_agent_requests: u64,
    
    /// Total tokens consumed
    pub total_tokens_consumed: u64,
    
    /// Number of agents hired
    pub agents_hired: u32,
    
    /// Usage breakdown by agent
    pub agent_usage: Vec<AgentUsage>,
    
    /// Usage breakdown by model
    pub model_usage: Vec<ModelUsage>,
    
    /// Available credits
    pub available_credits: u64,
    
    /// Credits used in current period
    pub credits_used: u64,
    
    /// Subscription details (if any)
    pub subscription: Option<SubscriptionInfo>,
}

/// Usage statistics for a specific agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentUsage {
    /// Agent ID
    pub agent_id: String,
    
    /// Number of requests
    pub requests: u64,
    
    /// Total tokens consumed (if applicable)
    pub tokens: Option<u64>,
    
    /// Whether the agent is currently hired
    pub is_hired: bool,
}

/// Usage statistics for a specific model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelUsage {
    /// Model ID
    pub model_id: String,
    
    /// Total tokens consumed
    pub tokens: u64,
}

/// Request for adding credits
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddCreditsRequest {
    /// Number of credits to add
    pub amount: u64,
    
    /// Stripe payment intent ID (if available)
    pub payment_intent_id: Option<String>,
}

/// Response for subscription information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscriptionResponse {
    /// Subscription tier
    pub tier: SubscriptionTier,
    
    /// Subscription status
    pub status: SubscriptionStatus,
    
    /// Maximum agents allowed
    pub max_agents: u32,
    
    /// Currently hired agents
    pub current_agents: u32,
    
    /// Inference credits per period
    pub inference_credits: u64,
    
    /// Current period token usage
    pub current_period_tokens: u64,
    
    /// When the current period started
    pub current_period_start: String,
    
    /// When the current period ends
    pub current_period_end: String,
    
    /// Whether the subscription will auto-renew
    pub auto_renew: bool,
}

/// Request for processing a Stripe checkout session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiProcessStripeCheckoutSession {
    /// Stripe checkout session ID
    pub session_id: String,
    
    /// Account ID
    pub account_id: String,
    
    /// Subscription info (optional)
    pub subscription_info: Option<SubscriptionInfo>,
    
    /// Credits to add (optional)
    pub credits_added: Option<u64>,
}

/// Request for verifying subscription
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiVerifySubscription {
    /// Account ID
    pub account_id: String,
}

/// Handler for getting subscription status
pub async fn get_subscription_status(
    State(_state): State<Arc<Mutex<DataStore>>>,
    auth: SignatureAuth,
) -> Result<Json<SubscriptionResponse>, StatusCode> {
    // Get account directly from SignatureAuth
    let account = auth.account;
    
    // Get subscription information
    let subscription = account.subscription.clone()
        .unwrap_or_else(|| {
            // Default to free tier if no subscription exists
            SubscriptionInfo::default()
        });
    
    // Get usage information
    let current_tokens = if let Some(usage) = &account.usage {
        usage.current_usage().tokens_consumed
    } else {
        0
    };
    
    // Convert to response format
    let response = SubscriptionResponse {
        tier: subscription.tier,
        status: subscription.status,
        max_agents: subscription.max_agents,
        current_agents: account.hired_agent_count() as u32,
        inference_credits: subscription.inference_credits_per_period,
        current_period_tokens: current_tokens,
        current_period_start: subscription.current_period_start.to_rfc3339(),
        current_period_end: subscription.current_period_end.to_rfc3339(),
        auto_renew: subscription.auto_renew,
    };
    
    Ok(Json(response))
}

/// Handler for getting usage statistics
pub async fn get_usage_stats(
    State(_state): State<Arc<Mutex<DataStore>>>,
    auth: SignatureAuth,
) -> Result<Json<UsageResponse>, StatusCode> {
    // Get account directly from SignatureAuth
    let account = auth.account;
    
    // Initialize response with default values
    let mut response = UsageResponse {
        total_agent_requests: 0,
        total_tokens_consumed: 0,
        agents_hired: account.hired_agents.len() as u32,
        agent_usage: Vec::new(),
        model_usage: Vec::new(),
        available_credits: account.available_credits(),
        credits_used: 0,
        subscription: account.subscription.clone(),
    };
    
    // Fill in usage statistics if available
    if let Some(usage) = &account.usage {
        let current = usage.current_usage();
        
        response.total_agent_requests = current.agent_requests;
        response.total_tokens_consumed = current.tokens_consumed;
        
        // Add agent usage - this would be more detailed in a real implementation
        for agent_id in &account.hired_agents {
            response.agent_usage.push(AgentUsage {
                agent_id: agent_id.clone(),
                requests: 0, // In a real implementation, we would track this
                tokens: None,
                is_hired: true,
            });
        }
        
        // Add model usage
        for (model_id, tokens) in &usage.model_usage {
            response.model_usage.push(ModelUsage {
                model_id: model_id.clone(),
                tokens: tokens.input_tokens + tokens.output_tokens,
            });
        }
        
        // Calculate credits used based on token consumption
        // This is a simplified calculation - real implementation would use price tiers
        response.credits_used = current.tokens_consumed / 100_000;
    }
    
    Ok(Json(response))
}

/// Handler for adding credits
pub async fn add_credits(
    State(state): State<Arc<Mutex<DataStore>>>,
    auth: SignatureAuth,
    Json(request): Json<AddCreditsRequest>,
) -> impl IntoResponse {
    // Get account from SignatureAuth
    let mut account = auth.account;
    
    // Add credits to the account
    let mut datastore = state.lock().await;
    
    // Add credits
    account.add_credits(request.amount);
    
    // Update account in datastore
    let op = datastore.account_state.update_account_local(account.clone());
    if let Err(err) = datastore.handle_account_op(op).await {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({
                "success": false,
                "error": format!("Failed to update account: {}", err)
            }))
        );
    }
    
    // Return success response
    (
        StatusCode::OK, 
        Json(json!({
            "success": true,
            "credits_added": request.amount,
            "total_credits": account.available_credits()
        }))
    )
}

/// Handler for verifying subscription
pub async fn verify_subscription(
    State(_state): State<Arc<Mutex<DataStore>>>,
    auth: SignatureAuth,
) -> impl IntoResponse {
    // Get account directly from SignatureAuth
    let account = auth.account;
    
    // Return the current subscription status
    (
        StatusCode::OK,
        Json(json!({
            "success": true,
            "subscription": account.subscription,
            "tier": account.subscription.as_ref().map(|s| s.tier).unwrap_or_default(),
            "credits": account.available_credits()
        }))
    )
}

/// Handler for processing a Stripe checkout session
pub async fn process_stripe_checkout_session(
    State(state): State<Arc<Mutex<DataStore>>>,
    Json(request): Json<ApiProcessStripeCheckoutSession>,
) -> impl IntoResponse {
    log::info!("Processing checkout session data for account {}", request.account_id);
    
    // Get the account from the datastore
    let mut datastore = state.lock().await;
    let mut account = match datastore.account_state.get_account(&request.account_id) {
        Some(account) => account,
        None => {
            log::error!("Account not found: {}", request.account_id);
            return (
                StatusCode::NOT_FOUND,
                Json(json!({
                    "success": false,
                    "error": "Account not found"
                }))
            );
        }
    };
    
    // In the new architecture, checkout processing happens in the frontend
    // This endpoint just receives the processed data
    
    // Example update for subscription
    if let Some(subscription_info) = request.subscription_info {
        account.subscription = Some(subscription_info);
        
        let op = datastore.account_state.update_account_local(account.clone());
        if let Err(err) = datastore.handle_account_op(op).await {
            log::error!("Failed to update account: {}", err);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "success": false,
                    "error": "Failed to update account"
                }))
            );
        }
        
        return (
            StatusCode::OK,
            Json(json!({
                "success": true,
                "message": "Subscription updated"
            }))
        );
    }
    
    // Example update for credits
    if let Some(credits) = request.credits_added {
        account.add_credits(credits);
        
        let op = datastore.account_state.update_account_local(account.clone());
        if let Err(err) = datastore.handle_account_op(op).await {
            log::error!("Failed to update account: {}", err);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "success": false,
                    "error": "Failed to update account"
                }))
            );
        }
        
        return (
            StatusCode::OK,
            Json(json!({
                "success": true,
                "credits_added": credits,
                "total_credits": account.available_credits()
            }))
        );
    }
    
    // If neither subscription nor credits were provided
    (
        StatusCode::BAD_REQUEST,
        Json(json!({
            "success": false,
            "error": "No subscription or credits data provided"
        }))
    )
} 