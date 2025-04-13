//! Billing module for subscription and usage-based billing
//! 
//! This module provides functionality for:
//! 1. Subscription management via Stripe
//! 2. Usage tracking and credit management
//! 3. Eligibility checking for operations

use chrono::{DateTime, Utc, NaiveDate, Datelike, TimeZone, Timelike};
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

impl SubscriptionTier {
    /// Get the quota settings for this subscription tier
    pub fn quota(&self) -> SubscriptionQuota {
        match self {
            Self::Free => SubscriptionQuota {
                max_agents: 1,
                inference_credits: 100,
                daily_token_limit: Some(100_000), // 100K tokens per day
                max_api_keys: 2,
                model_access: vec!["basic".to_string()],
                additional_agent_discount: 0, // No discount on additional agents
                max_premium_models: 0,        // No premium models allowed
                premium_agent_access: false,  // No premium agents
            },
            Self::Pro => SubscriptionQuota {
                max_agents: 3,
                inference_credits: 500,
                daily_token_limit: Some(500_000), // 500K tokens per day
                max_api_keys: 5,
                model_access: vec!["basic".to_string(), "standard".to_string()],
                additional_agent_discount: 10, // 10% discount on additional agents
                max_premium_models: 1,        // 1 premium model allowed
                premium_agent_access: true,   // Premium agents allowed
            },
            Self::ProPlus => SubscriptionQuota {
                max_agents: 5,
                inference_credits: 1000,
                daily_token_limit: None, // No daily limit
                max_api_keys: 10,
                model_access: vec!["basic".to_string(), "standard".to_string(), "advanced".to_string()],
                additional_agent_discount: 15, // 15% discount on additional agents
                max_premium_models: 3,        // 3 premium models allowed
                premium_agent_access: true,   // Premium agents allowed
            },
            Self::Power => SubscriptionQuota {
                max_agents: 10,
                inference_credits: 5000,
                daily_token_limit: None, // No daily limit
                max_api_keys: 20,
                model_access: vec!["basic".to_string(), "standard".to_string(), "advanced".to_string(), "enterprise".to_string()],
                additional_agent_discount: 20, // 20% discount on additional agents
                max_premium_models: 10,       // 10 premium models allowed
                premium_agent_access: true,   // Premium agents allowed
            },
            Self::PowerPlus => SubscriptionQuota {
                max_agents: 25,
                inference_credits: 10000,
                daily_token_limit: None, // No daily limit
                max_api_keys: 50,
                model_access: vec!["basic".to_string(), "standard".to_string(), "advanced".to_string(), "enterprise".to_string(), "expert".to_string()],
                additional_agent_discount: 25, // 25% discount on additional agents
                max_premium_models: 25,       // 25 premium models allowed (unlimited)
                premium_agent_access: true,   // Premium agents allowed
            },
        }
    }
}

/// Quota settings for each subscription tier
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SubscriptionQuota {
    /// Maximum number of agents included in the subscription
    pub max_agents: u32,
    
    /// Inference credits allocated per billing period
    pub inference_credits: u64,
    
    /// Optional daily token limit (None means unlimited)
    pub daily_token_limit: Option<u64>,
    
    /// Maximum number of API keys that can be created
    pub max_api_keys: u32,
    
    /// Model access tiers available to this subscription
    pub model_access: Vec<String>,
    
    /// Discount percentage on additional agents (beyond the max included)
    pub additional_agent_discount: u8,
    
    /// Maximum number of premium models that can be used
    pub max_premium_models: u32,
    
    /// Whether this tier has access to premium agents
    pub premium_agent_access: bool,
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

impl SubscriptionInfo {
    /// Get the quota for this subscription
    pub fn quota(&self) -> SubscriptionQuota {
        self.tier.quota()
    }
    
    /// Create a new subscription with the specified tier
    pub fn new(tier: SubscriptionTier) -> Self {
        let now = Utc::now();
        let thirty_days = chrono::Duration::days(30);
        let quota = tier.quota();
        
        Self {
            stripe_customer_id: None,
            stripe_subscription_id: None,
            tier,
            status: SubscriptionStatus::Trial,
            created_at: now,
            current_period_start: now,
            current_period_end: now + thirty_days,
            auto_renew: false,
            max_agents: quota.max_agents,
            inference_credits_per_period: quota.inference_credits,
        }
    }
}

impl Default for SubscriptionInfo {
    fn default() -> Self {
        let now = Utc::now();
        let thirty_days = chrono::Duration::days(30);
        let free_quota = SubscriptionTier::Free.quota();
        
        Self {
            stripe_customer_id: None,
            stripe_subscription_id: None,
            tier: SubscriptionTier::Free,
            status: SubscriptionStatus::Trial,
            created_at: now,
            current_period_start: now,
            current_period_end: now + thirty_days,
            auto_renew: false,
            max_agents: free_quota.max_agents,
            inference_credits_per_period: free_quota.inference_credits,
        }
    }
}

/// Usage metrics for a specific model
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ModelUsage {
    /// Input tokens consumed
    pub input_tokens: u64,
    
    /// Output tokens generated
    pub output_tokens: u64,
    
    /// Number of requests made to this model
    pub request_count: u64,
    
    /// Last time this model was used
    pub last_used: DateTime<Utc>,
}

impl Default for ModelUsage {
    fn default() -> Self {
        Self {
            input_tokens: 0,
            output_tokens: 0,
            request_count: 0,
            last_used: Utc::now(),
        }
    }
}

/// Usage metrics for a specific time period
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PeriodUsage {
    /// Total tokens consumed (input + output)
    pub tokens_consumed: u64,
    
    /// Input tokens only
    pub input_tokens: u64,
    
    /// Output tokens only
    pub output_tokens: u64,
    
    /// Total agent requests
    pub agent_requests: u64,
    
    /// Breakdown of usage by model
    pub model_breakdown: BTreeMap<String, ModelUsage>,
    
    /// Timestamp of last activity
    pub last_activity: DateTime<Utc>,
}

impl Default for PeriodUsage {
    fn default() -> Self {
        Self {
            tokens_consumed: 0,
            input_tokens: 0,
            output_tokens: 0,
            agent_requests: 0,
            model_breakdown: BTreeMap::new(),
            last_activity: Utc::now(),
        }
    }
}

/// Daily usage record for time-series analysis
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DailyUsage {
    /// The date of this usage record
    pub date: NaiveDate,
    
    /// Total tokens consumed on this day
    pub total_tokens: u64,
    
    /// Token consumption by hour (0-23)
    pub hourly_breakdown: [u64; 24],
    
    /// Agent requests made on this day
    pub agent_requests: u64,
}

impl Default for DailyUsage {
    fn default() -> Self {
        Self {
            date: Utc::now().date_naive(),
            total_tokens: 0,
            hourly_breakdown: [0; 24],
            agent_requests: 0,
        }
    }
}

/// Usage tracking for billing purposes
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct UsageTracker {
    /// Token usage by month (YYYY-MM format)
    pub token_usage: BTreeMap<String, PeriodUsage>,
    
    /// Agent requests by month (YYYY-MM format)
    pub agent_requests: BTreeMap<String, PeriodUsage>,
    
    /// Token usage by model
    pub model_usage: BTreeMap<String, ModelUsage>,
    
    /// Daily usage records (last 90 days for time-series analysis)
    pub daily_usage: BTreeMap<String, DailyUsage>,
    
    /// Weekly aggregated usage (last 12 weeks)
    pub weekly_usage: BTreeMap<String, PeriodUsage>,
    
    /// Current period start time
    pub current_period_start: DateTime<Utc>,
    
    /// Credits consumed in current billing period
    pub current_period_credits_used: u64,
    
    /// Timestamp of last token usage
    pub last_token_usage: DateTime<Utc>,
    
    /// Timestamp of last agent usage
    pub last_agent_usage: DateTime<Utc>,
    
    /// Map of agent IDs to their usage statistics
    #[serde(default)]
    pub agent_usage: BTreeMap<String, AgentUsageStats>,
    
    /// Agent usage by month (YYYY-MM format)
    #[serde(default)]
    pub agent_usage_periods: BTreeMap<String, AgentPeriodUsage>,
}

/// Statistics on agent usage
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct AgentUsageStats {
    /// Total hours the agent has been used (in milliseconds)
    pub total_hours_ms: u64,
    
    /// Number of times the agent has been hired
    pub hire_count: u64,
    
    /// Last time the agent was used
    pub last_used: DateTime<Utc>,
}

/// Agent usage metrics for a specific period
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct AgentPeriodUsage {
    /// Total hours the agents have been used (in milliseconds)
    pub total_hours_ms: u64,
    
    /// Total number of agent hires
    pub hire_count: u64,
    
    /// Breakdown of usage by agent
    pub agent_breakdown: BTreeMap<String, AgentUsageStats>,
    
    /// Timestamp of last activity
    pub last_activity: DateTime<Utc>,
}

impl Default for UsageTracker {
    fn default() -> Self {
        let now = Utc::now();
        Self {
            token_usage: BTreeMap::new(),
            agent_requests: BTreeMap::new(),
            model_usage: BTreeMap::new(),
            daily_usage: BTreeMap::new(),
            weekly_usage: BTreeMap::new(),
            current_period_start: now,
            current_period_credits_used: 0,
            last_token_usage: now,
            last_agent_usage: now,
            agent_usage: BTreeMap::new(),
            agent_usage_periods: BTreeMap::new(),
        }
    }
}

impl UsageTracker {
    /// Create a new usage tracker with default values and free tier allocation
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Get usage for the current period
    pub fn current_usage(&self) -> PeriodUsage {
        // Use the current period key, or return default if not found
        self.token_usage
            .get(&format!("{}", self.current_period_start.format("%Y-%m")))
            .cloned()
            .unwrap_or_default()
    }
    
    /// Get the current day's usage
    pub fn today_usage(&self) -> DailyUsage {
        let today = Utc::now().date_naive().format("%Y-%m-%d").to_string();
        self.daily_usage
            .get(&today)
            .cloned()
            .unwrap_or_default()
    }
    
    /// Get total tokens consumed (both input and output)
    pub fn total_tokens_consumed(&self) -> u64 {
        self.model_usage.values().map(|usage| usage.input_tokens + usage.output_tokens).sum()
    }
    
    /// Get tokens consumed in the current period
    pub fn current_period_tokens(&self) -> u64 {
        self.current_usage().tokens_consumed
    }
    
    /// Get usage for a specific model
    pub fn model_usage(&self, model_id: &str) -> ModelUsage {
        self.model_usage
            .get(model_id)
            .cloned()
            .unwrap_or_default()
    }
    
    /// Record token usage for a specific model
    pub fn record_token_usage(&mut self, model_id: &str, input_tokens: u64, output_tokens: u64) -> u64 {
        let now = Utc::now();
        self.last_token_usage = now;
        
        // Update model-specific usage
        let model_usage = self.model_usage.entry(model_id.to_string()).or_default();
        model_usage.input_tokens += input_tokens;
        model_usage.output_tokens += output_tokens;
        model_usage.request_count += 1;
        model_usage.last_used = now;
        
        // Update current month usage
        let month_key = now.format("%Y-%m").to_string();
        let period_usage = self.token_usage.entry(month_key.clone()).or_default();
        period_usage.tokens_consumed += input_tokens + output_tokens;
        period_usage.input_tokens += input_tokens;
        period_usage.output_tokens += output_tokens;
        period_usage.last_activity = now;
        
        // Update model breakdown in period usage
        let period_model_usage = period_usage.model_breakdown.entry(model_id.to_string()).or_default();
        period_model_usage.input_tokens += input_tokens;
        period_model_usage.output_tokens += output_tokens;
        period_model_usage.request_count += 1;
        period_model_usage.last_used = now;
        
        // Update daily usage
        let day_key = now.date_naive().format("%Y-%m-%d").to_string();
        let daily_usage = self.daily_usage.entry(day_key).or_default();
        daily_usage.total_tokens += input_tokens + output_tokens;
        daily_usage.hourly_breakdown[now.hour() as usize] += input_tokens + output_tokens;
        
        // Update weekly usage
        let iso_week = now.iso_week();
        let week_key = format!("{}-W{:02}", now.year(), iso_week.week());
        let weekly_usage = self.weekly_usage.entry(week_key).or_default();
        weekly_usage.tokens_consumed += input_tokens + output_tokens;
        weekly_usage.input_tokens += input_tokens;
        weekly_usage.output_tokens += output_tokens;
        weekly_usage.last_activity = now;
        
        // Update current period credits
        // First determine how many credits this usage costs
        let total_tokens = input_tokens + output_tokens;
        let token_cost = self.calculate_token_cost(model_id, input_tokens, output_tokens);
        self.current_period_credits_used += token_cost;
        
        token_cost
    }
    
    /// Calculate the cost in credits for token usage
    fn calculate_token_cost(&self, model_id: &str, input_tokens: u64, output_tokens: u64) -> u64 {
        // This is a simplified implementation that should be refined based on actual billing rules
        // In a real implementation, you would likely factor in different rates for input vs output tokens
        let total_tokens = input_tokens + output_tokens;
        
        // Convert to credits - this is a simplified calculation
        // In real implementation, this would use the BillingConfig price data
        let tokens_in_thousands = (total_tokens as f64) / 1000.0;
        tokens_in_thousands.ceil() as u64
    }
    
    /// Get remaining credits for the current billing period
    pub fn remaining_credits(&self, total_credits_per_period: u64) -> u64 {
        if self.current_period_credits_used >= total_credits_per_period {
            0
        } else {
            total_credits_per_period - self.current_period_credits_used
        }
    }
    
    /// Check if the account has sufficient credits for a token operation
    pub fn has_sufficient_credits(&self, total_credits_per_period: u64, required_credits: u64) -> bool {
        self.remaining_credits(total_credits_per_period) >= required_credits
    }
    
    /// Estimate token cost for a planned operation
    pub fn estimate_token_cost(&self, model_id: &str, estimated_input_tokens: u64, estimated_output_tokens: u64) -> u64 {
        self.calculate_token_cost(model_id, estimated_input_tokens, estimated_output_tokens)
    }

    /// Record agent usage for hiring
    pub fn record_agent_usage(&mut self, agent_id: &str, duration_hours: f64) -> u64 {
        let now = Utc::now();
        self.last_agent_usage = now;
        
        // Update agent-specific usage
        let agent_usage = self.agent_usage.entry(agent_id.to_string()).or_default();
        agent_usage.total_hours_ms += (duration_hours * 1000.0) as u64;
        agent_usage.hire_count += 1;
        agent_usage.last_used = now;
        
        // Update monthly agent usage
        let month_key = now.format("%Y-%m").to_string();
        let period_usage = self.agent_usage_periods.entry(month_key.clone()).or_default();
        period_usage.total_hours_ms += (duration_hours * 1000.0) as u64;
        period_usage.hire_count += 1;
        period_usage.last_activity = now;
        
        // Update agent breakdown in period usage
        let period_agent_usage = period_usage.agent_breakdown.entry(agent_id.to_string()).or_default();
        period_agent_usage.total_hours_ms += (duration_hours * 1000.0) as u64;
        period_agent_usage.hire_count += 1;
        period_agent_usage.last_used = now;
        
        // Update daily usage
        let day_key = now.date_naive().format("%Y-%m-%d").to_string();
        let daily_usage = self.daily_usage.entry(day_key).or_default();
        daily_usage.agent_requests += 1;
        
        // Calculate agent cost
        let agent_cost = self.calculate_agent_cost(agent_id, duration_hours);
        self.current_period_credits_used += agent_cost;
        
        agent_cost
    }
    
    /// Calculate the cost in credits for agent usage
    fn calculate_agent_cost(&self, agent_id: &str, duration_hours: f64) -> u64 {
        // This is a simplified implementation that should be refined based on actual billing rules
        // In a real implementation, you would consider the agent type, tier, etc.
        
        // Convert to credits - simplified calculation
        // Assume 1 credit per hour of agent usage
        duration_hours.ceil() as u64
    }
    
    /// Get total agent hours used in the current period
    pub fn total_agent_hours(&self, period: Option<String>) -> f64 {
        match period {
            Some(period_key) => {
                self.agent_usage_periods
                    .get(&period_key)
                    .map(|p| p.total_hours_ms as f64 / 1000.0)
                    .unwrap_or(0.0)
            },
            None => {
                // If no period specified, get current month
                let current_month = Utc::now().format("%Y-%m").to_string();
                self.agent_usage_periods
                    .get(&current_month)
                    .map(|p| p.total_hours_ms as f64 / 1000.0)
                    .unwrap_or(0.0)
            }
        }
    }
    
    /// Get agent usage statistics for a specific agent
    pub fn agent_usage_stats(&self, agent_id: &str) -> Option<&AgentUsageStats> {
        self.agent_usage.get(agent_id)
    }
    
    /// Reset usage for a new billing period
    pub fn reset_period_usage(&mut self) {
        self.current_period_credits_used = 0;
        // Note: We don't reset historical usage data, only the current period credit counter
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