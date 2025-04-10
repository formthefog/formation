//! Billing module for subscription and usage-based billing
//! 
//! This module provides functionality for:
//! 1. Subscription management via Stripe
//! 2. Usage tracking and credit management
//! 3. Eligibility checking for operations

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};

// Re-export submodules
pub mod stripe;
pub mod handlers;
pub mod middleware;

/// Subscription tier levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
pub enum SubscriptionTier {
    /// Free tier with limited capabilities
    Free,
    /// Pro tier with higher limits
    Pro,
    /// Professional Plus tier
    ProPlus,
    /// Power tier for larger workloads
    Power,
    /// Power Plus tier for maximum capability
    PowerPlus,
}

impl Default for SubscriptionTier {
    fn default() -> Self {
        Self::Free
    }
}

/// Subscription status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, PartialOrd, Ord, Hash)]
pub enum SubscriptionStatus {
    /// Subscription is active
    Active,
    /// Subscription is in trial period
    Trial,
    /// Payment is past due but subscription still active
    PastDue,
    /// Subscription is canceled
    Canceled,
    /// Subscription has expired
    Expired,
    /// Error with subscription
    Error,
}

impl Default for SubscriptionStatus {
    fn default() -> Self {
        Self::Trial
    }
}

/// Subscription information
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SubscriptionInfo {
    /// Stripe customer ID
    pub stripe_customer_id: Option<String>,
    
    /// Stripe subscription ID
    pub stripe_subscription_id: Option<String>,
    
    /// Subscription tier
    pub tier: SubscriptionTier,
    
    /// Subscription status
    pub status: SubscriptionStatus,
    
    /// When the subscription was created
    pub created_at: DateTime<Utc>,
    
    /// When the current billing period started
    pub current_period_start: DateTime<Utc>,
    
    /// When the current billing period ends
    pub current_period_end: DateTime<Utc>,
    
    /// Whether the subscription will auto-renew
    pub auto_renew: bool,
    
    /// Maximum number of agents allowed
    pub max_agents: u32,
    
    /// Inference credits per billing period
    pub inference_credits_per_period: u64,
}

impl Default for SubscriptionInfo {
    fn default() -> Self {
        let now = Utc::now();
        let thirty_days = chrono::Duration::days(30);
        
        Self {
            stripe_customer_id: None,
            stripe_subscription_id: None,
            tier: SubscriptionTier::Free,
            status: SubscriptionStatus::Trial,
            created_at: now,
            current_period_start: now,
            current_period_end: now + thirty_days,
            auto_renew: false,
            max_agents: 1,
            inference_credits_per_period: 100,
        }
    }
}

/// Usage tracking for billing purposes
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct UsageTracker {
    /// Token usage by time period
    pub token_usage: BTreeMap<String, PeriodUsage>,
    
    /// Agent requests by time period
    pub agent_requests: BTreeMap<String, PeriodUsage>,
    
    /// Token usage by model
    pub model_usage: BTreeMap<String, u64>,
    
    /// Current period start time
    pub current_period_start: DateTime<Utc>,
}

impl UsageTracker {
    /// Get usage for the current period
    pub fn current_usage(&self) -> PeriodUsage {
        // Use the current period key, or return default if not found
        self.token_usage
            .get(&format!("{}", self.current_period_start.format("%Y-%m")))
            .map(|usage| usage.clone()).unwrap_or(PeriodUsage::default())
    }
}

/// Usage statistics for a specific time period
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PeriodUsage {
    /// Total tokens consumed
    pub tokens_consumed: u64,
    
    /// Total agent requests
    pub agent_requests: u64,
}

impl Default for PeriodUsage {
    fn default() -> Self {
        Self {
            tokens_consumed: 0,
            agent_requests: 0,
        }
    }
}

/// Billing configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BillingConfig {
    /// Base price per 1K tokens (in credits)
    pub token_base_price: f64,
    
    /// Price multipliers for different models
    pub model_price_multipliers: HashMap<String, f64>,
    
    /// Credit cost per additional agent (beyond subscription limit)
    pub additional_agent_cost: u64,
    
    /// Whether to enforce strict limits
    pub strict_enforcement: bool,
}

impl Default for BillingConfig {
    fn default() -> Self {
        Self {
            token_base_price: 0.01,
            model_price_multipliers: HashMap::new(),
            additional_agent_cost: 10,
            strict_enforcement: true,
        }
    }
} 