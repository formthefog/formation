use std::collections::{BTreeMap, BTreeSet};
use serde::{Serialize, Deserialize};
use k256::ecdsa::SigningKey;
use crdts::{Map, BFTReg, map::Op, bft_reg::Update, CmRDT};
use chrono::Utc;

use crate::billing::{SubscriptionInfo, UsageTracker};
use crate::api_keys::ApiKey;
use crate::Actor;

pub type AccountOp = Op<String, BFTReg<Account, Actor>, Actor>;

/// Represents a user account with ownership and authorization information
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Account {
    /// Ethereum-style address derived from the user's public key
    pub address: String,
    /// Optional human-readable name for the account
    pub name: Option<String>,
    /// Set of instance IDs owned by this account
    #[serde(default)]
    pub owned_instances: BTreeSet<String>,
    /// Set of agent IDs owned by this account
    #[serde(default)]
    pub owned_agents: BTreeSet<String>,
    /// Set of model IDs owned by this account
    #[serde(default)]
    pub owned_models: BTreeSet<String>,
    /// Map of instance IDs to authorization level for instances where this account has access
    #[serde(default)]
    pub authorized_instances: BTreeMap<String, AuthorizationLevel>,
    /// Subscription information
    #[serde(default)]
    pub subscription: Option<SubscriptionInfo>,
    /// Usage tracking information
    #[serde(default)]
    pub usage: Option<UsageTracker>,
    /// Available credits for pay-as-you-go usage
    #[serde(default)]
    pub credits: u64,
    /// Set of agent IDs that are currently hired by this account
    #[serde(default)]
    pub hired_agents: BTreeSet<String>,
    /// Collection of API keys associated with this account
    #[serde(default)]
    pub api_keys: BTreeMap<String, ApiKey>,
    /// Creation timestamp
    #[serde(default)]
    pub created_at: i64,
    /// Last update timestamp
    #[serde(default)]
    pub updated_at: i64,
}

/// Defines the level of authorization an account has for an instance
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AuthorizationLevel {
    /// Full access - can perform all operations including ownership transfer
    Owner,
    /// Can perform operations except ownership transfer
    Manager,
    /// Can view and start/stop but not modify
    Operator,
    /// Can only view status
    ReadOnly,
}

// Implement AsRef<[u8]> for Account to satisfy Sha3Hash trait requirements
impl AsRef<[u8]> for Account {
    fn as_ref(&self) -> &[u8] {
        // This implementation is a bit of a hack but is sufficient for our purposes
        // The proper implementation would serialize the whole struct
        self.address.as_bytes()
    }
}

impl Account {
    /// Create a new account with the given address
    pub fn new(address: String) -> Self {
        let now = Utc::now().timestamp();
        
        // Initialize with free tier credits (enough for basic usage)
        let initial_credits = 100; // $100 worth of credits
        
        Self {
            address,
            name: None,
            owned_instances: BTreeSet::new(),
            owned_agents: BTreeSet::new(),
            owned_models: BTreeSet::new(),
            authorized_instances: BTreeMap::new(),
            subscription: None,
            usage: Some(UsageTracker::new()), // Initialize with default usage tracker
            credits: initial_credits,
            hired_agents: BTreeSet::new(),
            api_keys: BTreeMap::new(),
            created_at: now,
            updated_at: now,
        }
    }

    /// Add an instance to the owned instances
    pub fn add_owned_instance(&mut self, instance_id: String) {
        self.owned_instances.insert(instance_id);
        self.updated_at = Utc::now().timestamp();
    }

    /// Add an agent to the owned agents
    pub fn add_owned_agent(&mut self, agent_id: String) {
        self.owned_agents.insert(agent_id);
        self.updated_at = Utc::now().timestamp();
    }

    /// Remove an agent from the owned agents
    pub fn remove_owned_agent(&mut self, agent_id: &str) -> bool {
        let removed = self.owned_agents.remove(agent_id);
        if removed {
            self.updated_at = Utc::now().timestamp();
        }
        removed
    }

    /// Add a model to the owned models
    pub fn add_owned_model(&mut self, model_id: String) {
        self.owned_models.insert(model_id);
        self.updated_at = Utc::now().timestamp();
    }

    /// Remove a model from the owned models
    pub fn remove_owned_model(&mut self, model_id: &str) -> bool {
        let removed = self.owned_models.remove(model_id);
        if removed {
            self.updated_at = Utc::now().timestamp();
        }
        removed
    }

    /// Remove an instance from the owned instances
    pub fn remove_owned_instance(&mut self, instance_id: &str) -> bool {
        let removed = self.owned_instances.remove(instance_id);
        if removed {
            self.updated_at = Utc::now().timestamp();
        }
        removed
    }

    /// Add or update an authorization for an instance
    pub fn add_authorization(&mut self, instance_id: String, level: AuthorizationLevel) {
        self.authorized_instances.insert(instance_id, level);
        self.updated_at = Utc::now().timestamp();
    }

    /// Remove an authorization for an instance
    pub fn remove_authorization(&mut self, instance_id: &str) -> Option<AuthorizationLevel> {
        let removed = self.authorized_instances.remove(instance_id);
        if removed.is_some() {
            self.updated_at = Utc::now().timestamp();
        }
        removed
    }
    
    /// Get the authorization level for an instance
    pub fn get_authorization_level(&self, instance_id: &str) -> Option<&AuthorizationLevel> {
        // If the instance is owned, return Owner level
        if self.owned_instances.contains(instance_id) {
            return Some(&AuthorizationLevel::Owner);
        }
        // Otherwise, check the authorized_instances map
        self.authorized_instances.get(instance_id)
    }

    /// Get available credits (either from subscription or pay-as-you-go)
    pub fn available_credits(&self) -> u64 {
        // Get credits from subscription if available
        let subscription_credits = if let Some(sub) = &self.subscription {
            use crate::billing::SubscriptionStatus;
            match sub.status {
                SubscriptionStatus::Active | 
                SubscriptionStatus::Trial | 
                SubscriptionStatus::PastDue => sub.inference_credits_per_period,
                _ => 0,
            }
        } else {
            0
        };
        
        // Pay-as-you-go credits
        let payg_credits = self.credits;
        
        // Use subscription credits first, then pay-as-you-go
        if subscription_credits > 0 {
            subscription_credits
        } else {
            payg_credits
        }
    }
    
    /// Count the number of hired agents
    pub fn hired_agent_count(&self) -> usize {
        self.hired_agents.len()
    }
    
    /// Add a hired agent
    pub fn hire_agent(&mut self, agent_id: String) {
        self.hired_agents.insert(agent_id);
        self.updated_at = Utc::now().timestamp();
    }
    
    /// Remove a hired agent
    pub fn fire_agent(&mut self, agent_id: &str) -> bool {
        let removed = self.hired_agents.remove(agent_id);
        if removed {
            self.updated_at = Utc::now().timestamp();
        }
        removed
    }
    
    /// Add credits to the account
    pub fn add_credits(&mut self, amount: u64) {
        self.credits = self.credits.saturating_add(amount);
        self.updated_at = Utc::now().timestamp();
    }
    
    /// Deduct credits from the account
    pub fn deduct_credits(&mut self, amount: u64) -> bool {
        if self.credits >= amount {
            self.credits -= amount;
            self.updated_at = Utc::now().timestamp();
            true
        } else {
            false
        }
    }

    /// Get the usage tracker, initializing if needed
    pub fn usage_tracker(&mut self) -> &mut UsageTracker {
        if self.usage.is_none() {
            self.usage = Some(UsageTracker::new());
        }
        self.usage.as_mut().unwrap()
    }
    
    /// Get the usage tracker as a reference without modifying
    pub fn get_usage(&self) -> Option<&UsageTracker> {
        self.usage.as_ref()
    }
    
    /// Get maximum allowed agents based on subscription or free tier
    pub fn max_allowed_agents(&self) -> u32 {
        if let Some(sub) = &self.subscription {
            sub.max_agents
        } else {
            1 // Free tier: 1 agent
        }
    }
    
    /// Check if the account can hire an additional agent
    pub fn can_hire_additional_agent(&self) -> bool {
        let current = self.hired_agent_count() as u32;
        let max_allowed = self.max_allowed_agents();
        
        current < max_allowed || self.credits >= 10 // Cost per additional agent
    }
    
    /// Get total token usage for all time
    pub fn total_token_usage(&self) -> u64 {
        match &self.usage {
            Some(usage) => usage.total_tokens_consumed(),
            None => 0,
        }
    }
    
    /// Get token usage for the current billing period
    pub fn current_period_token_usage(&self) -> u64 {
        match &self.usage {
            Some(usage) => usage.current_period_tokens(),
            None => 0,
        }
    }
    
    /// Get today's token usage
    pub fn today_token_usage(&self) -> u64 {
        match &self.usage {
            Some(usage) => usage.today_usage().total_tokens,
            None => 0,
        }
    }
    
    /// Check if account has sufficient credits for token consumption
    pub fn has_sufficient_credits_for_tokens(&self, token_count: u64, token_cost_per_1k: f64) -> bool {
        let cost = (token_count as f64 / 1000.0) * token_cost_per_1k;
        let cost_in_credits = cost.ceil() as u64;
        
        self.available_credits() >= cost_in_credits
    }
    
    /// Check if the account can use the specified tokens for the given model
    /// 
    /// This checks both if:
    /// 1. The account has sufficient credits for the operation
    /// 2. The account hasn't exceeded any token quotas or rate limits
    /// 
    /// Returns true if the tokens can be used, false otherwise
    pub fn can_use_tokens(&self, model_id: &str, input_tokens: u64, output_tokens: u64) -> bool {
        // First check if we have a usage tracker
        let usage = match &self.usage {
            Some(tracker) => tracker,
            None => return self.available_credits() > 0, // If no tracker, just check if we have any credits
        };
        
        // Calculate the cost in credits
        let required_credits = usage.estimate_token_cost(model_id, input_tokens, output_tokens);
        
        // Check against the user's credit balance
        if self.available_credits() < required_credits {
            return false;
        }
        
        // Check rate limits and subscription tier restrictions
        if let Some(subscription) = &self.subscription {
            // Get the quota for this subscription tier
            let quota = subscription.quota();
            
            // Check model access restrictions
            let model_tier = model_id.split("_").next().unwrap_or("basic");
            if !quota.model_access.iter().any(|tier| tier == model_tier) {
                return false; // Model tier not allowed for this subscription
            }
            
            // Check if this is a premium model
            let is_premium = model_id.contains("premium") || model_tier == "enterprise" || model_tier == "expert";
            
            // Count existing premium models
            let premium_count = self.get_usage()
                .map(|u| u.model_usage.keys()
                    .filter(|m| m.contains("premium") || 
                           m.split("_").next().unwrap_or("") == "enterprise" || 
                           m.split("_").next().unwrap_or("") == "expert")
                    .count() as u32)
                .unwrap_or(0);
            
            // Check premium model limit if this is a premium model
            if is_premium && premium_count >= quota.max_premium_models {
                return false;
            }
            
            // Check daily token limits if applicable
            if let Some(daily_limit) = quota.daily_token_limit {
                let today_usage = usage.today_usage().total_tokens;
                
                if today_usage + input_tokens + output_tokens > daily_limit {
                    return false;
                }
            }
        }
        
        // If we reach here, the account can use the tokens
        true
    }
    
    /// Check if the account can hire a specific agent
    /// 
    /// This checks:
    /// 1. If the agent is already hired (prevents duplicate hiring)
    /// 2. If the account has reached its agent limit
    /// 3. If the account has sufficient credits to hire the agent
    /// 4. If the agent type is allowed for the account's subscription tier
    ///
    /// Returns true if the agent can be hired, false otherwise
    pub fn can_hire_agent(&self, agent_id: &str) -> bool {
        // Check if agent is already hired
        if self.hired_agents.contains(agent_id) {
            return false; // Already hired this agent
        }
        
        // Get subscription quota if available
        let quota = self.subscription.as_ref().map(|sub| sub.quota());
        
        // Get current and max agent counts
        let current_agent_count = self.hired_agent_count() as u32;
        let max_allowed = self.max_allowed_agents();
        
        // If we're under the limit, check agent type eligibility
        if current_agent_count < max_allowed {
            // Check if this is a premium agent
            let is_premium = agent_id.contains("premium_") || agent_id.contains("expert_");
            
            // If it's a premium agent, check subscription permissions
            if is_premium {
                // Free tier and tiers without premium_agent_access cannot use premium agents
                if let Some(quota) = &quota {
                    if !quota.premium_agent_access {
                        return false;
                    }
                } else {
                    return false; // No subscription means no premium agents
                }
            }
            
            return true; // Non-premium agents can be hired if under limit
        }
        
        // If we're at or over the limit, check if we have credits for an additional agent
        // Base cost is 10 credits per additional agent beyond the subscription limit
        let base_cost: u64 = 10;
        
        // Apply any discount from the subscription
        let additional_agent_cost = if let Some(quota) = quota.clone() {
            if quota.additional_agent_discount > 0 {
                let discount = (base_cost as f64 * (quota.additional_agent_discount as f64 / 100.0)).ceil() as u64;
                base_cost.saturating_sub(discount)
            } else {
                base_cost
            }
        } else {
            base_cost
        };
        
        // Check credit balance
        if self.available_credits() < additional_agent_cost {
            return false;
        }
        
        // Check subscription tier restrictions
        if let Some(quota) = quota.clone() {
            // Check if this is a premium agent
            let is_premium = agent_id.contains("premium_") || agent_id.contains("expert_");
            
            // Premium agents require premium_agent_access
            if is_premium && !quota.premium_agent_access {
                return false;
            }
            
            // Hard caps on total agents by tier
            // Free tier users are limited to a maximum of 2 agents total (1 included + 1 extra)
            if self.subscription.as_ref().map_or(false, |s| s.tier == crate::billing::SubscriptionTier::Free) 
                && current_agent_count >= 2 {
                return false;
            }
            
            // Pro tier users can hire up to 5 agents total (included + extra)
            if self.subscription.as_ref().map_or(false, |s| s.tier == crate::billing::SubscriptionTier::Pro) 
                && current_agent_count >= 5 {
                return false;
            }
            
            // ProPlus users can hire up to 10 agents total
            if self.subscription.as_ref().map_or(false, |s| s.tier == crate::billing::SubscriptionTier::ProPlus) 
                && current_agent_count >= 10 {
                return false;
            }
            
            // Power users can hire up to 20 agents total
            if self.subscription.as_ref().map_or(false, |s| s.tier == crate::billing::SubscriptionTier::Power) 
                && current_agent_count >= 20 {
                return false;
            }
            
            // PowerPlus users can hire up to 50 agents total
            if self.subscription.as_ref().map_or(false, |s| s.tier == crate::billing::SubscriptionTier::PowerPlus) 
                && current_agent_count >= 50 {
                return false;
            }
        }
        
        // If we've passed all checks, the user can hire this agent
        true
    }

    /// Add a new API key to the account
    pub fn add_api_key(&mut self, api_key: ApiKey) -> Result<(), String> {
        // Check if we're at the limit for API keys
        let max_allowed = self.max_allowed_api_keys();
        let current_count = self.api_keys.len() as u32;
        
        if current_count >= max_allowed {
            return Err(format!("API key limit reached ({}/{})", current_count, max_allowed));
        }
        
        // Add the key to the account
        self.api_keys.insert(api_key.id.clone(), api_key);
        self.updated_at = Utc::now().timestamp();
        
        Ok(())
    }
    
    /// Remove an API key from the account
    pub fn remove_api_key(&mut self, key_id: &str) -> bool {
        let removed = self.api_keys.remove(key_id).is_some();
        if removed {
            self.updated_at = Utc::now().timestamp();
        }
        return removed;
    }
    
    /// Get an API key by ID
    pub fn get_api_key(&self, key_id: &str) -> Option<&ApiKey> {
        self.api_keys.get(key_id)
    }
    
    /// Get an API key by (key prefix + secret)
    pub fn get_api_key_by_secret(&self, full_key: &str) -> Option<&ApiKey> {
        // Extract the prefix and secret
        let parts: Vec<&str> = full_key.splitn(2, '_').collect();
        if parts.len() < 2 {
            return None;
        }
        
        // Check each key to find a matching one
        for key in self.api_keys.values() {
            if key.verify_secret(full_key) {
                return Some(key);
            }
        }
        
        None
    }
    
    /// List all API keys (active only)
    pub fn list_active_api_keys(&self) -> Vec<&ApiKey> {
        self.api_keys.values()
            .filter(|key| key.is_valid())
            .collect()
    }
    
    /// List all API keys (including revoked/expired)
    pub fn list_all_api_keys(&self) -> Vec<&ApiKey> {
        self.api_keys.values().collect()
    }
    
    /// Revoke an API key
    pub fn revoke_api_key(&mut self, key_id: &str) -> bool {
        if let Some(key) = self.api_keys.get_mut(key_id) {
            key.revoke();
            self.updated_at = Utc::now().timestamp();
            return true;
        }
        false
    }
    
    /// Get the maximum number of API keys allowed for this account
    pub fn max_allowed_api_keys(&self) -> u32 {
        if let Some(subscription) = &self.subscription {
            // Get the quota from the subscription tier
            subscription.tier.quota().max_api_keys
        } else {
            // Default for accounts without a subscription
            5 // Free tier gets 5 API keys
        }
    }

    /// Check if an agent is owned by this account
    pub fn owns_agent(&self, agent_id: &str) -> bool {
        self.owned_agents.contains(agent_id)
    }

    /// Get the count of owned agents
    pub fn owned_agent_count(&self) -> usize {
        self.owned_agents.len()
    }
}

/// State container for accounts
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AccountState {
    node_id: String,
    pk: String,
    pub map: Map<String, BFTReg<Account, Actor>, Actor>
}

impl AccountState {
    /// Create a new AccountState
    pub fn new(node_id: String, pk: String) -> Self {
        Self {
            node_id,
            pk,
            map: Map::new()
        }
    }

    /// Get the map of accounts
    pub fn map(&self) -> Map<String, BFTReg<Account, Actor>, Actor> {
        self.map.clone()
    }
    
    /// Update an account locally and return the operation
    pub fn update_account_local(&mut self, account: Account) -> AccountOp {
        let add_ctx = self.map.read_ctx().derive_add_ctx(self.node_id.clone());
        let signing_key = SigningKey::from_slice(
            &hex::decode(self.pk.clone())
                .expect("PANIC: Invalid SigningKey Cannot Decode from Hex"))
                .expect("PANIC: Invalid SigningKey cannot recover from Bytes");
                
        self.map.update(account.address.clone(), add_ctx, |reg, _ctx| {
            reg.update(account, self.node_id.clone(), signing_key)
                .expect("PANIC: Unable to sign updates")
        })
    }
    
    pub fn account_op(&mut self, op: AccountOp) -> Option<(String, String)> {
        log::info!("Applying peer op");
        self.map.apply(op.clone());
        match op {
            Op::Up { dot, key, op: _ } => Some((dot.actor, key)),
            Op::Rm { .. } => None
        }
    }

    pub fn account_op_success(&self, key: String, update: Update<Account, String>) -> (bool, Account) {
        if let Some(reg) = self.map.get(&key).val {
            if let Some(v) = reg.val() {
                // If the in the updated register equals the value in the Op it
                // succeeded
                if v.value() == update.op().value {
                    return (true, v.value()) 
                // Otherwise, it could be that it's a concurrent update and was added
                // to the DAG as a head
                } else if reg.dag_contains(&update.hash()) && reg.is_head(&update.hash()) {
                    return (true, v.value()) 
                // Otherwise, we could be missing a child, and this particular update
                // is orphaned, if so we should requst the child we are missing from
                // the actor who shared this update
                } else if reg.is_orphaned(&update.hash()) {
                    return (true, v.value())
                // Otherwise it was a no-op for some reason
                } else {
                    return (false, v.value()) 
                }
            } else {
                return (false, update.op().value) 
            }
        } else {
            return (false, update.op().value);
        }
    }

    pub fn remove_instance_local(&mut self, id: String) -> AccountOp {
        log::info!("Acquiring remove context...");
        let rm_ctx = self.map.read_ctx().derive_rm_ctx();
        log::info!("Building Rm Op...");
        self.map.rm(id, rm_ctx)
    }

    /// Get an account by address
    pub fn get_account(&self, address: &str) -> Option<Account> {
        let read_ctx = self.map.get(&address.to_string());
        if let Some(reg) = read_ctx.val {
            if let Some(v) = reg.val() {
                return Some(v.value());
            }
        }
        None
    }
    
    /// Get all accounts
    pub fn list_accounts(&self) -> Vec<Account> {
        let mut accounts = Vec::new();
        for ctx in self.map.iter() {
            let (_, reg) = ctx.val;
            if let Some(val) = reg.val() {
                accounts.push(val.value());
            }
        }
        accounts
    }
    
    /// Get all accounts that have ownership of an instance
    pub fn get_owners_of_instance(&self, instance_id: &str) -> Vec<Account> {
        let mut accounts = Vec::new();
        for ctx in self.map.iter() {
            let (_, reg) = ctx.val;
            if let Some(val) = reg.val() {
                let account = val.value();
                if account.owned_instances.contains(instance_id) {
                    accounts.push(account);
                }
            }
        }
        accounts
    }
    
    /// Check if an account has appropriate authorization for an instance operation
    pub fn verify_authorization(&self, address: &str, instance_id: &str, required_level: &AuthorizationLevel) -> bool {
        if let Some(account) = self.get_account(address) {
            if let Some(level) = account.get_authorization_level(instance_id) {
                match (level, required_level) {
                    (AuthorizationLevel::Owner, _) => true, // Owner can do anything
                    (AuthorizationLevel::Manager, AuthorizationLevel::Owner) => false, // Manager can't do Owner actions
                    (AuthorizationLevel::Manager, _) => true, // Manager can do anything except Owner actions
                    (AuthorizationLevel::Operator, AuthorizationLevel::Owner | AuthorizationLevel::Manager) => false,
                    (AuthorizationLevel::Operator, _) => true, // Operator can do basic operations
                    (AuthorizationLevel::ReadOnly, AuthorizationLevel::ReadOnly) => true, // ReadOnly can only read
                    (AuthorizationLevel::ReadOnly, _) => false,
                }
            } else {
                false
            }
        } else {
            false
        }
    }

    /// Remove an account locally and return the operation
    pub fn remove_account_local(&mut self, address: String) -> AccountOp {
        log::info!("Acquiring remove context for account {}...", address);
        let rm_ctx = self.map.read_ctx().derive_rm_ctx();
        log::info!("Building Rm Op for account deletion...");
        self.map.rm(address, rm_ctx)
    }
} 
