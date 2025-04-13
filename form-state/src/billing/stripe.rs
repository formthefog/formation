//! Subscription and billing data storage
//! 
//! This module provides data structures for storing subscription information
//! and checking eligibility for operations.
//! 
//! Instead of connecting directly to Stripe, it receives data from the frontend
//! and provides eligibility checks based on stored data.

use crate::billing::{SubscriptionInfo, SubscriptionStatus, SubscriptionTier};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Error types for billing operations
#[derive(Debug, thiserror::Error)]
pub enum BillingError {
    #[error("Subscription not found: {0}")]
    SubscriptionNotFound(String),
    
    #[error("Account not found: {0}")]
    AccountNotFound(String),
    
    #[error("Insufficient credits: {0}")]
    InsufficientCredits(String),
    
    #[error("Other error: {0}")]
    Other(String),
}

/// Type alias for Result with BillingError
pub type BillingResult<T> = Result<T, BillingError>;

/// Billing data storage
#[derive(Clone)]
pub struct BillingStore {
    /// Configuration
    config: Arc<BillingConfig>,
}

/// Configuration for billing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BillingConfig {
    /// Whether to use test mode
    pub test_mode: bool,
    
    /// Default credit cost for pay-as-you-go
    pub credit_cost_per_1k_tokens: f64,
    
    /// Default agent cost (in credits)
    pub agent_hire_credit_cost: u64,
}

impl BillingConfig {
    /// Create a default configuration
    pub fn default() -> Self {
        Self {
            test_mode: false,
            credit_cost_per_1k_tokens: 0.01, // $0.01 per 1k tokens
            agent_hire_credit_cost: 100,      // 100 credits to hire an agent
        }
    }
}

impl BillingStore {
    /// Create a new billing store
    pub fn new(config: BillingConfig) -> Self {
        Self {
            config: Arc::new(config),
        }
    }
    
    /// Create a default billing store
    pub fn default() -> Self {
        Self::new(BillingConfig::default())
    }
    
    /// Get the credit cost for token usage
    pub fn get_token_credit_cost(&self, token_count: u64) -> u64 {
        let cost_per_1k = self.config.credit_cost_per_1k_tokens;
        ((token_count as f64 / 1000.0) * cost_per_1k).ceil() as u64
    }
    
    /// Check if an account has sufficient credits for token usage
    pub fn check_token_eligibility(&self, account_id: &str, token_count: u64, stored_subscription: &Option<SubscriptionInfo>, available_credits: u64) -> BillingResult<bool> {
        // If they have a subscription, check if they're within their limit
        if let Some(subscription) = stored_subscription {
            // Active subscriptions may have included tokens
            if subscription.status == SubscriptionStatus::Active || 
               subscription.status == SubscriptionStatus::Trial {
                // If they have enough included tokens, they're eligible
                // Frontend should track usage and provide current period consumption
                if token_count <= subscription.inference_credits_per_period {
                    return Ok(true);
                }
                
                // Otherwise, check if they have enough pay-as-you-go credits
                let credit_cost = self.get_token_credit_cost(token_count);
                if available_credits >= credit_cost {
                    return Ok(true);
                }
                
                return Err(BillingError::InsufficientCredits(
                    format!("Account has insufficient credits: available={}, required={}", 
                            available_credits, credit_cost)
                ));
            }
            
            // Inactive subscriptions require pay-as-you-go credits
            let credit_cost = self.get_token_credit_cost(token_count);
            if available_credits >= credit_cost {
                return Ok(true);
            }
            
            return Err(BillingError::InsufficientCredits(
                format!("Account has inactive subscription and insufficient credits: available={}, required={}", 
                        available_credits, credit_cost)
            ));
        }
        
        // No subscription, check pay-as-you-go credits
        let credit_cost = self.get_token_credit_cost(token_count);
        if available_credits >= credit_cost {
            return Ok(true);
        }
        
        Err(BillingError::InsufficientCredits(
            format!("Account has no subscription and insufficient credits: available={}, required={}", 
                    available_credits, credit_cost)
        ))
    }
    
    /// Check if an account can hire another agent
    pub fn check_agent_eligibility(&self, agent_id: &str, stored_subscription: &Option<SubscriptionInfo>, current_agents: u32, available_credits: u64) -> BillingResult<bool> {
        // Check if they have an active subscription
        if let Some(subscription) = stored_subscription {
            if subscription.status == SubscriptionStatus::Active || 
               subscription.status == SubscriptionStatus::Trial {
                // If they're under their agent limit, they're eligible
                if current_agents < subscription.max_agents {
                    return Ok(true);
                }
                
                // If over limit, check if they have enough credits for pay-as-you-go
                if available_credits >= self.config.agent_hire_credit_cost {
                    return Ok(true);
                }
                
                return Err(BillingError::InsufficientCredits(
                    format!("Account has reached agent limit and has insufficient credits: current_agents={}, max_agents={}, available_credits={}, required={}", 
                            current_agents, subscription.max_agents, available_credits, self.config.agent_hire_credit_cost)
                ));
            }
            
            // Inactive subscription requires pay-as-you-go credits
            if available_credits >= self.config.agent_hire_credit_cost {
                return Ok(true);
            }
            
            return Err(BillingError::InsufficientCredits(
                format!("Account has inactive subscription and insufficient credits: available_credits={}, required={}", 
                        available_credits, self.config.agent_hire_credit_cost)
            ));
        }
        
        // No subscription, check pay-as-you-go credits
        if available_credits >= self.config.agent_hire_credit_cost {
            return Ok(true);
        }
        
        Err(BillingError::InsufficientCredits(
            format!("Account has no subscription and insufficient credits: available_credits={}, required={}", 
                    available_credits, self.config.agent_hire_credit_cost)
        ))
    }
}

/// Billing transaction record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BillingTransaction {
    /// Transaction ID
    pub id: String,
    
    /// Account ID
    pub account_id: String,
    
    /// Amount 
    pub amount: u64,
    
    /// Description
    pub description: String,
    
    /// Credits added or consumed
    pub credits: u64,
    
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    
    /// Status (completed, failed, etc.)
    pub status: String,
} 