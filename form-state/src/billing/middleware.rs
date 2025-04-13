//! Middleware for enforcing subscription and usage limits
//! 
//! This module provides middleware functions for checking eligibility before 
//! allowing agent usage or token consumption.

use axum::{
    extract::{State, Path, Json},
    http::{Request, StatusCode, header},
    middleware::Next,
    response::{Response, IntoResponse},
    body::Body,
    Json as JsonResponse,
};
use std::sync::Arc;
use tokio::sync::Mutex;
use serde_json::json;
use thiserror::Error;

use crate::datastore::DataStore;
use crate::auth::{DynamicClaims, JwtClaims};
use crate::billing::BillingConfig;

/// Error types for eligibility checks
#[derive(Debug, thiserror::Error)]
pub enum EligibilityError {
    #[error("Account not found: {0}")]
    AccountNotFound(String),
    
    #[error("Insufficient credits: need {required}, have {available}")]
    InsufficientCredits { required: u64, available: u64 },
    
    #[error("Agent limit reached: {current} of {maximum}")]
    AgentLimitReached { current: u32, maximum: u32 },
    
    #[error("Agent already hired: {0}")]
    AgentAlreadyHired(String),
    
    #[error("Premium agent not available on current tier")]
    PremiumAgentNotAvailable,
    
    #[error("Token limit exceeded: daily limit of {limit} tokens")]
    DailyTokenLimitExceeded { limit: u64 },
    
    #[error("Model tier not available on current subscription")]
    ModelTierNotAvailable,
    
    #[error("Premium model limit reached: {current} of {maximum}")]
    PremiumModelLimitReached { current: u32, maximum: u32 },
    
    #[error("Inactive subscription")]
    InactiveSubscription,
    
    #[error("Operation not allowed: {0}")]
    OperationNotAllowed(String),
    
    #[error("Database error: {0}")]
    DatabaseError(String),
}

impl IntoResponse for EligibilityError {
    fn into_response(self) -> Response {
        let (status, json_body) = match &self {
            Self::InsufficientCredits { required, available } => {
                (StatusCode::PAYMENT_REQUIRED, json!({
                    "error": "insufficient_credits",
                    "message": "Insufficient credits to perform this operation",
                    "details": {
                        "required_credits": required,
                        "available_credits": available,
                        "additional_needed": required.saturating_sub(*available)
                    }
                }))
            },
            Self::AgentLimitReached { current, maximum } => {
                (StatusCode::PAYMENT_REQUIRED, json!({
                    "error": "agent_limit_reached",
                    "message": "Account has reached the maximum number of allowed agents",
                    "details": {
                        "current_agents": current,
                        "maximum_agents": maximum
                    }
                }))
            },
            Self::InactiveSubscription => {
                (StatusCode::PAYMENT_REQUIRED, json!({
                    "error": "inactive_subscription",
                    "message": "Subscription is inactive or expired"
                }))
            },
            Self::AgentAlreadyHired(agent_id) => {
                (StatusCode::CONFLICT, json!({
                    "error": "agent_already_hired",
                    "message": "The requested agent is already hired",
                    "details": {
                        "agent_id": agent_id
                    }
                }))
            },
            Self::PremiumAgentNotAvailable => {
                (StatusCode::FORBIDDEN, json!({
                    "error": "premium_agent_not_available",
                    "message": "Premium agent not available on current tier"
                }))
            },
            Self::DailyTokenLimitExceeded { limit } => {
                (StatusCode::PAYMENT_REQUIRED, json!({
                    "error": "daily_token_limit_exceeded",
                    "message": "Token limit exceeded: daily limit of {limit} tokens",
                    "details": {
                        "daily_token_limit": limit
                    }
                }))
            },
            Self::ModelTierNotAvailable => {
                (StatusCode::FORBIDDEN, json!({
                    "error": "model_tier_not_available",
                    "message": "Model tier not available on current subscription"
                }))
            },
            Self::PremiumModelLimitReached { current, maximum } => {
                (StatusCode::PAYMENT_REQUIRED, json!({
                    "error": "premium_model_limit_reached",
                    "message": "Premium model limit reached: {current} of {maximum}",
                    "details": {
                        "current_premium_models": current,
                        "maximum_premium_models": maximum
                    }
                }))
            },
            Self::AccountNotFound(user_id) => {
                (StatusCode::NOT_FOUND, json!({
                    "error": "account_not_found",
                    "message": "Account not found",
                    "details": {
                        "account_id": user_id
                    }
                }))
            },
            Self::OperationNotAllowed(msg) => {
                (StatusCode::FORBIDDEN, json!({
                    "error": "operation_not_allowed",
                    "message": msg
                }))
            },
            Self::DatabaseError(msg) => {
                (StatusCode::INTERNAL_SERVER_ERROR, json!({
                    "error": "database_error",
                    "message": msg
                }))
            },
        };

        (status, JsonResponse(json_body)).into_response()
    }
}

/// Context for eligibility checking
pub struct EligibilityContext {
    /// Billing configuration
    pub billing_config: BillingConfig,
    /// Cost of additional agents (in credits)
    pub agent_cost: u64,
}

/// Middleware for checking if an account can use an agent
pub async fn check_agent_eligibility(
    State(state): State<Arc<Mutex<DataStore>>>,
    JwtClaims(claims): JwtClaims,
    Path(agent_id): Path<String>,
    mut request: Request<Body>,
    next: Next,
) -> Result<Response, EligibilityError> {
    // Get user ID from claims
    let user_id = claims.sub.clone();
    
    // Get account information
    let datastore = state.lock().await;
    let account = datastore.account_state.get_account(&user_id)
        .ok_or(EligibilityError::AccountNotFound(user_id.clone()))?;
    
    // Check if the agent exists
    if datastore.agent_state.get_agent(&agent_id).is_none() {
        return Err(EligibilityError::AccountNotFound(agent_id));
    }
    
    // Check subscription status if present
    if let Some(subscription) = &account.subscription {
        use crate::billing::SubscriptionStatus;
        match subscription.status {
            SubscriptionStatus::Active | SubscriptionStatus::Trial => {
                // Subscription is active, continue
            },
            SubscriptionStatus::PastDue => {
                // Past due but still active, continue with warning
                log::warn!("Account {} has past due subscription", user_id);
            },
            _ => {
                // Inactive subscription
                return Err(EligibilityError::InactiveSubscription);
            }
        }
        
        // Check if account can hire another agent
        let current_agents = account.hired_agent_count() as u32;
        let max_agents = subscription.max_agents;
        
        if current_agents >= max_agents {
            // Over limit, check if they have credits for pay-as-you-go
            // A better implementation would use a cost from config
            let required_credits = 10; // Cost per additional agent
            let available_credits = account.available_credits();
            
            if available_credits < required_credits {
                return Err(EligibilityError::AgentLimitReached {
                    current: current_agents,
                    maximum: max_agents,
                });
            }
            
            // They have enough credits to pay as they go
            // We don't deduct credits here - that happens when they actually hire the agent
        }
    } else {
        // No subscription, check if it's a free agent or they have credits
        // For simplicity, we'll assume they can access it
        // Check for credits
        let available_credits = account.available_credits();
        if available_credits < 10 {
            // Frontend should trigger a modal to tell them they need to add credits
            return Err(EligibilityError::InsufficientCredits {
                required: 10,
                available: available_credits,
            });
        }
    }
    
    // Add the account to request extensions for use in the handler
    request.extensions_mut().insert(account);
    
    // Proceed to the handler
    Ok(next.run(request).await)
}

/// Middleware for checking if tokens can be consumed
pub async fn check_token_eligibility(
    State(state): State<Arc<Mutex<DataStore>>>,
    JwtClaims(claims): JwtClaims,
    Json(payload): Json<serde_json::Value>,
    mut request: Request<Body>,
    next: Next,
) -> Result<Response, EligibilityError> {
    // Get user ID from claims
    let user_id = claims.sub.clone();
    
    // Get account information
    let datastore = state.lock().await;
    let account = datastore.account_state.get_account(&user_id)
        .ok_or(EligibilityError::AccountNotFound(user_id.clone()))?;
    
    // Extract token count from payload (simplified - actual would depend on API structure)
    let token_count = payload.get("max_tokens")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    
    // Check if they have enough credits/allocation
    if let Some(subscription) = &account.subscription {
        // Check usage in current period
        if let Some(usage) = &account.usage {
            let current_usage = usage.current_usage();
            
            // If they have a subscription, check if they're within limits
            if current_usage.tokens_consumed + token_count <= subscription.inference_credits_per_period {
                // Within subscription limits, proceed
            } else {
                // Over limits, check pay-as-you-go credits
                let required_credits = token_count / 10_000; // Simplified: 1 credit per 10K tokens
                if required_credits > 0 && account.available_credits() < required_credits {
                    return Err(EligibilityError::InsufficientCredits {
                        required: required_credits,
                        available: account.available_credits(),
                    });
                }
                // They have enough credits to pay as they go
            }
        }
    } else {
        // No subscription, check if they have enough credits
        let required_credits = token_count / 10_000; // Simplified: 1 credit per 10K tokens
        if required_credits > 0 && account.available_credits() < required_credits {
            return Err(EligibilityError::InsufficientCredits {
                required: required_credits,
                available: account.available_credits(),
            });
        }
    }
    
    // Add the account to request extensions for use in the handler
    request.extensions_mut().insert(account);
    
    // Proceed to the handler
    Ok(next.run(request).await)
}

/// Enum for different types of operations that require credit checking
#[derive(Debug, Clone)]
pub enum OperationType {
    /// Token consumption with a specific model
    TokenConsumption {
        model_id: String,
        input_tokens: u64,
        output_tokens: u64,
    },
    /// Agent hiring
    AgentHire {
        agent_id: String,
    },
    /// Custom operation with a specified credit cost
    Custom {
        name: String,
        credit_cost: u64,
    },
}

/// Check if an account has sufficient credits for a specific operation
/// 
/// This is a centralized function for checking credit eligibility across different
/// operation types. It uses the new account methods can_use_tokens and can_hire_agent
/// to perform the checks.
/// 
/// Returns Ok(()) if the operation can proceed, or an EligibilityError if not
pub fn check_operation_credits(
    account: &crate::accounts::Account,
    operation: OperationType
) -> Result<(), EligibilityError> {
    match operation {
        OperationType::TokenConsumption { model_id, input_tokens, output_tokens } => {
            // Use the can_use_tokens method we implemented previously
            if !account.can_use_tokens(&model_id, input_tokens, output_tokens) {
                // Get estimated cost from the usage tracker
                let required_credits = if let Some(usage) = &account.usage {
                    usage.estimate_token_cost(&model_id, input_tokens, output_tokens)
                } else {
                    // Simplified fallback estimation if no usage tracker
                    ((input_tokens + output_tokens) as f64 / 1000.0).ceil() as u64
                };
                
                return Err(EligibilityError::InsufficientCredits {
                    required: required_credits,
                    available: account.available_credits(),
                });
            }
        },
        
        OperationType::AgentHire { agent_id } => {
            // Use the can_hire_agent method we implemented previously
            if !account.can_hire_agent(&agent_id) {
                // Check why it failed
                if account.hired_agents.contains(&agent_id) {
                    return Err(EligibilityError::AgentAlreadyHired(agent_id));
                }
                
                // Check if it's a limit issue
                let current_agents = account.hired_agent_count() as u32;
                let max_allowed = account.max_allowed_agents();
                
                if current_agents >= max_allowed {
                    // Credit issue
                    return Err(EligibilityError::AgentLimitReached {
                        current: current_agents,
                        maximum: max_allowed,
                    });
                }
                
                // Otherwise it's likely a tier restriction
                return Err(EligibilityError::OperationNotAllowed(
                    "Your subscription tier does not allow hiring this type of agent".to_string()
                ));
            }
        },
        
        OperationType::Custom { name, credit_cost } => {
            // Simple check for custom operations with a fixed credit cost
            if account.available_credits() < credit_cost {
                return Err(EligibilityError::InsufficientCredits {
                    required: credit_cost,
                    available: account.available_credits(),
                });
            }
        },
    }
    
    // If we reach here, the operation is allowed
    Ok(())
}

/// Check if an account can hire a specific agent
pub async fn validate_agent_eligibility(
    user_id: String,
    agent_id: String,
    state: Arc<Mutex<DataStore>>,
) -> Result<(), EligibilityError> {
    // Get account from datastore
    let datastore = state.lock().await;
    let account = datastore.account_state.get_account(&user_id)
        .ok_or_else(|| EligibilityError::AccountNotFound(user_id.clone()))?;
    
    // Check if agent is already hired
    if account.hired_agents.contains(&agent_id) {
        return Err(EligibilityError::AgentAlreadyHired(agent_id));
    }
    
    // Get subscription quota if available
    let quota = account.subscription.as_ref().map(|sub| sub.quota());
    
    // Get current agent count and maximum allowed
    let current_agents = account.hired_agent_count() as u32;
    let max_agents = account.max_allowed_agents();
    
    // Check subscription status if one exists
    if let Some(subscription) = &account.subscription {
        use crate::billing::SubscriptionStatus;
        match subscription.status {
            SubscriptionStatus::Active | SubscriptionStatus::Trial => {
                // Subscription is active, continue
            },
            SubscriptionStatus::PastDue => {
                // Past due but still active, continue with warning
                log::warn!("Account {} has past due subscription", user_id);
            },
            _ => {
                // Inactive subscription
                return Err(EligibilityError::InactiveSubscription);
            }
        }
    }
    
    // Check if agent is premium
    let is_premium = agent_id.contains("premium_") || agent_id.contains("expert_");
    
    if is_premium {
        // Check premium agent access
        if let Some(quota) = &quota {
            if !quota.premium_agent_access {
                return Err(EligibilityError::PremiumAgentNotAvailable);
            }
        } else {
            // No subscription means no premium agents
            return Err(EligibilityError::PremiumAgentNotAvailable);
        }
    }
    
    // If under max allowed agents, then we're good
    if current_agents < max_agents {
        return Ok(());
    }
    
    // Over max allowed, check if they can pay for additional agents
    let base_cost: u64 = 10; // Cost per additional agent
    
    // Apply any discount from the subscription
    let required_credits = if let Some(quota) = quota {
        if quota.additional_agent_discount > 0 {
            let discount = (base_cost as f64 * (quota.additional_agent_discount as f64 / 100.0)).ceil() as u64;
            base_cost.saturating_sub(discount)
        } else {
            base_cost
        }
    } else {
        base_cost
    };
    
    let available_credits = account.available_credits();
    
    if available_credits < required_credits {
        return Err(EligibilityError::InsufficientCredits {
            required: required_credits,
            available: available_credits,
        });
    }
    
    // Check tier-specific hard limits on maximum agents
    match account.subscription.as_ref().map(|s| s.tier) {
        Some(crate::billing::SubscriptionTier::Free) if current_agents >= 2 => {
            return Err(EligibilityError::OperationNotAllowed(
                "Free tier is limited to 2 agents maximum".to_string()
            ));
        },
        Some(crate::billing::SubscriptionTier::Pro) if current_agents >= 5 => {
            return Err(EligibilityError::OperationNotAllowed(
                "Pro tier is limited to 5 agents maximum".to_string()
            ));
        },
        Some(crate::billing::SubscriptionTier::ProPlus) if current_agents >= 10 => {
            return Err(EligibilityError::OperationNotAllowed(
                "ProPlus tier is limited to 10 agents maximum".to_string()
            ));
        },
        Some(crate::billing::SubscriptionTier::Power) if current_agents >= 20 => {
            return Err(EligibilityError::OperationNotAllowed(
                "Power tier is limited to 20 agents maximum".to_string()
            ));
        },
        Some(crate::billing::SubscriptionTier::PowerPlus) if current_agents >= 50 => {
            return Err(EligibilityError::OperationNotAllowed(
                "PowerPlus tier is limited to 50 agents maximum".to_string()
            ));
        },
        _ => {}
    }
    
    // If we reach here, the account has enough credits to hire an additional agent
    Ok(())
}

/// Check if an account can use tokens for a given model
pub async fn validate_token_eligibility(
    user_id: String,
    model_id: String,
    input_tokens: u64,
    output_tokens: u64,
    state: Arc<Mutex<DataStore>>,
) -> Result<(), EligibilityError> {
    // Get account from datastore
    let datastore = state.lock().await;
    let account = datastore.account_state.get_account(&user_id)
        .ok_or_else(|| EligibilityError::AccountNotFound(user_id.clone()))?;
    
    // First check if we have a usage tracker
    let usage = match &account.usage {
        Some(tracker) => tracker,
        None => {
            // If no tracker, just check if we have any credits
            if account.available_credits() == 0 {
                return Err(EligibilityError::InsufficientCredits {
                    required: 1,
                    available: 0,
                });
            }
            return Ok(());
        }
    };
    
    // Calculate the cost in credits
    let required_credits = usage.estimate_token_cost(&model_id, input_tokens, output_tokens);
    let available_credits = account.available_credits();
    
    // Check against the user's credit balance
    if available_credits < required_credits {
        return Err(EligibilityError::InsufficientCredits {
            required: required_credits,
            available: available_credits,
        });
    }
    
    // Check subscription restrictions
    if let Some(subscription) = &account.subscription {
        // Get the quota for this subscription tier
        let quota = subscription.quota();
        
        // Check model access restrictions
        let model_tier = model_id.split("_").next().unwrap_or("basic");
        if !quota.model_access.iter().any(|tier| tier == model_tier) {
            return Err(EligibilityError::ModelTierNotAvailable);
        }
        
        // Check if this is a premium model
        let is_premium = model_id.contains("premium") || model_tier == "enterprise" || model_tier == "expert";
        
        if is_premium {
            // Count existing premium models
            let premium_count = account.get_usage()
                .map(|u| u.model_usage.keys()
                    .filter(|m| m.contains("premium") || 
                           m.split("_").next().unwrap_or("") == "enterprise" || 
                           m.split("_").next().unwrap_or("") == "expert")
                    .count() as u32)
                .unwrap_or(0);
            
            // Check premium model limit
            if premium_count >= quota.max_premium_models {
                return Err(EligibilityError::PremiumModelLimitReached {
                    current: premium_count,
                    maximum: quota.max_premium_models,
                });
            }
        }
        
        // Check daily token limits if applicable
        if let Some(daily_limit) = quota.daily_token_limit {
            let today_usage = usage.today_usage().total_tokens;
            let total_requested = input_tokens + output_tokens;
            
            if today_usage + total_requested > daily_limit {
                return Err(EligibilityError::DailyTokenLimitExceeded {
                    limit: daily_limit,
                });
            }
        }
    }
    
    // If we reach here, the account can use the tokens
    Ok(())
} 