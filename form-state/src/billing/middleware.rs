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
};
use std::sync::Arc;
use tokio::sync::Mutex;
use serde_json::json;

use crate::datastore::DataStore;
use crate::auth::{DynamicClaims, JwtClaims};
use crate::billing::BillingConfig;

/// Error response for eligibility middleware
#[derive(Debug)]
pub enum EligibilityError {
    /// Account has insufficient credits for the operation
    InsufficientCredits {
        available: u64,
        required: u64,
    },
    /// Account has reached maximum allowed agents
    AgentLimitReached {
        current: u32,
        maximum: u32,
    },
    /// Account subscription is inactive or expired
    InactiveSubscription,
    /// Agent is not found
    AgentNotFound,
    /// Account is not found
    AccountNotFound,
    /// Authentication error
    AuthError,
    /// Other errors
    Other(String),
}

impl IntoResponse for EligibilityError {
    fn into_response(self) -> Response {
        let (status, json_body) = match &self {
            Self::InsufficientCredits { available, required } => {
                (StatusCode::PAYMENT_REQUIRED, json!({
                    "error": "insufficient_credits",
                    "message": "Insufficient credits to perform this operation",
                    "details": {
                        "available_credits": available,
                        "required_credits": required,
                        "additional_needed": required.saturating_sub(*available)
                    }
                }))
            },
            Self::AgentLimitReached { current, maximum } => {
                (StatusCode::PAYMENT_REQUIRED, json!({
                    "error": "agent_limit_reached",
                    "message": "Maximum allowed agents reached for your subscription tier",
                    "details": {
                        "current_agents": current,
                        "maximum_agents": maximum
                    }
                }))
            },
            Self::InactiveSubscription => {
                (StatusCode::PAYMENT_REQUIRED, json!({
                    "error": "inactive_subscription",
                    "message": "Your subscription is inactive or expired",
                }))
            },
            Self::AgentNotFound => {
                (StatusCode::NOT_FOUND, json!({
                    "error": "agent_not_found",
                    "message": "The requested agent was not found"
                }))
            },
            Self::AccountNotFound => {
                (StatusCode::NOT_FOUND, json!({
                    "error": "account_not_found",
                    "message": "Account not found"
                }))
            },
            Self::AuthError => {
                (StatusCode::UNAUTHORIZED, json!({
                    "error": "authentication_error",
                    "message": "Authentication error"
                }))
            },
            Self::Other(msg) => {
                (StatusCode::INTERNAL_SERVER_ERROR, json!({
                    "error": "eligibility_error",
                    "message": msg
                }))
            }
        };

        let body = Body::from(serde_json::to_string(&json_body).unwrap_or_default());
        let self_string = self.to_string(); // Store the string before moving self
        Response::builder()
            .status(status)
            .header(header::CONTENT_TYPE, "application/json")
            .body(body)
            .unwrap_or_else(|_| (status, self_string).into_response())
    }
}

impl std::fmt::Display for EligibilityError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InsufficientCredits { available, required } => {
                write!(f, "Insufficient credits: available={}, required={}", available, required)
            },
            Self::AgentLimitReached { current, maximum } => {
                write!(f, "Agent limit reached: current={}, maximum={}", current, maximum)
            },
            Self::InactiveSubscription => {
                write!(f, "Inactive subscription")
            },
            Self::AgentNotFound => {
                write!(f, "Agent not found")
            },
            Self::AccountNotFound => {
                write!(f, "Account not found")
            },
            Self::AuthError => {
                write!(f, "Authentication error")
            },
            Self::Other(msg) => {
                write!(f, "Eligibility error: {}", msg)
            }
        }
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
        .ok_or(EligibilityError::AccountNotFound)?;
    
    // Check if the agent exists
    if datastore.agent_state.get_agent(&agent_id).is_none() {
        return Err(EligibilityError::AgentNotFound);
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
                available: available_credits,
                required: 10,
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
        .ok_or(EligibilityError::AccountNotFound)?;
    
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
                        available: account.available_credits(),
                        required: required_credits,
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
                available: account.available_credits(),
                required: required_credits,
            });
        }
    }
    
    // Add the account to request extensions for use in the handler
    request.extensions_mut().insert(account);
    
    // Proceed to the handler
    Ok(next.run(request).await)
} 