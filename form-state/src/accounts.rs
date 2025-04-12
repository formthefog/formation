use std::collections::{BTreeMap, BTreeSet};
use serde::{Serialize, Deserialize};
use k256::ecdsa::SigningKey;
use crdts::{Map, BFTReg, map::Op, bft_reg::Update, CmRDT};
use chrono::Utc;

use crate::billing::{SubscriptionInfo, UsageTracker};
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
        Self {
            address,
            name: None,
            owned_instances: BTreeSet::new(),
            authorized_instances: BTreeMap::new(),
            subscription: None,
            usage: None,
            credits: 0,
            hired_agents: BTreeSet::new(),
            created_at: now,
            updated_at: now,
        }
    }

    /// Add an instance to the owned instances
    pub fn add_owned_instance(&mut self, instance_id: String) {
        self.owned_instances.insert(instance_id);
        self.updated_at = Utc::now().timestamp();
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
