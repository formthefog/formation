use chrono::{DateTime, Utc};
use serde::{Serialize, Deserialize};
use std::sync::Arc;
use std::collections::VecDeque;
use crate::datastore::DataStore;
use axum::http::Method;
use tokio::sync::Mutex;

/// Maximum number of events to keep in memory per API key
const MAX_EVENTS_PER_KEY: usize = 1000;

/// Types of API key events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ApiKeyEventType {
    /// API key was created
    Created,
    /// API key was used for authentication
    Used {
        /// Path of the API endpoint that was accessed
        path: String,
        /// HTTP method used
        method: String,
        /// Status code of the response
        status_code: u16,
        /// Whether the request was rate limited
        rate_limited: bool,
    },
    /// API key was revoked
    Revoked {
        /// Reason for revocation, if provided
        reason: Option<String>,
    },
    /// API key permissions were updated
    PermissionsUpdated {
        /// Previous scope
        previous_scope: String,
        /// New scope
        new_scope: String,
    },
    /// API key expiration date was updated
    ExpirationUpdated {
        /// Previous expiration date
        previous_expiration: Option<DateTime<Utc>>,
        /// New expiration date
        new_expiration: Option<DateTime<Utc>>,
    },
}

/// Represents a single API key event for audit logging
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKeyEvent {
    /// ID of the API key
    pub api_key_id: String,
    /// Account ID that owns the key
    pub account_id: String,
    /// Timestamp when the event occurred
    pub timestamp: DateTime<Utc>,
    /// IP address that initiated the event, if available
    pub ip_address: Option<String>,
    /// Type of event
    pub event_type: ApiKeyEventType,
    /// Optional user agent string
    pub user_agent: Option<String>,
}

impl ApiKeyEvent {
    /// Create a new API key usage event
    pub fn new_usage(
        api_key_id: String,
        account_id: String,
        path: String,
        method: Method,
        status_code: u16,
        ip_address: Option<String>,
        user_agent: Option<String>,
        rate_limited: bool,
    ) -> Self {
        Self {
            api_key_id,
            account_id,
            timestamp: Utc::now(),
            ip_address,
            event_type: ApiKeyEventType::Used {
                path,
                method: method.to_string(),
                status_code,
                rate_limited,
            },
            user_agent,
        }
    }
    
    /// Create a new API key creation event
    pub fn new_creation(
        api_key_id: String,
        account_id: String,
        ip_address: Option<String>,
        user_agent: Option<String>,
    ) -> Self {
        Self {
            api_key_id,
            account_id,
            timestamp: Utc::now(),
            ip_address,
            event_type: ApiKeyEventType::Created,
            user_agent,
        }
    }
    
    /// Create a new API key revocation event
    pub fn new_revocation(
        api_key_id: String,
        account_id: String,
        reason: Option<String>,
        ip_address: Option<String>,
        user_agent: Option<String>,
    ) -> Self {
        Self {
            api_key_id,
            account_id,
            timestamp: Utc::now(),
            ip_address,
            event_type: ApiKeyEventType::Revoked { reason },
            user_agent,
        }
    }
    
    /// Create a new API key permissions update event
    pub fn new_permissions_update(
        api_key_id: String,
        account_id: String,
        previous_scope: String,
        new_scope: String,
        ip_address: Option<String>,
        user_agent: Option<String>,
    ) -> Self {
        Self {
            api_key_id,
            account_id,
            timestamp: Utc::now(),
            ip_address,
            event_type: ApiKeyEventType::PermissionsUpdated { 
                previous_scope, 
                new_scope,
            },
            user_agent,
        }
    }
    
    /// Create a new API key expiration update event
    pub fn new_expiration_update(
        api_key_id: String,
        account_id: String,
        previous_expiration: Option<DateTime<Utc>>,
        new_expiration: Option<DateTime<Utc>>,
        ip_address: Option<String>,
        user_agent: Option<String>,
    ) -> Self {
        Self {
            api_key_id,
            account_id,
            timestamp: Utc::now(),
            ip_address,
            event_type: ApiKeyEventType::ExpirationUpdated { 
                previous_expiration, 
                new_expiration,
            },
            user_agent,
        }
    }
}

/// In-memory storage for API key events
#[derive(Debug, Default)]
pub struct ApiKeyAuditLog {
    /// Events indexed by API key ID
    events: Arc<Mutex<std::collections::HashMap<String, VecDeque<ApiKeyEvent>>>>,
}

impl ApiKeyAuditLog {
    /// Create a new API key audit log
    pub fn new() -> Self {
        Self {
            events: Arc::new(Mutex::new(std::collections::HashMap::new())),
        }
    }
    
    /// Record a new API key event
    pub async fn record(&self, event: ApiKeyEvent) {
        // Store the API key ID before moving the event
        let api_key_id = event.api_key_id.clone();
        
        let mut events = self.events.lock().await;
        
        // Get or create the event queue for this API key
        let queue = events
            .entry(api_key_id.clone())
            .or_insert_with(VecDeque::new);
            
        // Add the event to the queue
        queue.push_back(event);
        
        // Trim the queue if it's too long
        if queue.len() > MAX_EVENTS_PER_KEY {
            queue.pop_front();
        }
        
        // Log that an event was recorded
        log::info!("Recorded API key event for key {}", api_key_id);
    }
    
    /// Get all events for a specific API key
    pub async fn get_events_for_key(&self, api_key_id: &str) -> Vec<ApiKeyEvent> {
        let events = self.events.lock().await;
        events
            .get(api_key_id)
            .map(|queue| queue.iter().cloned().collect())
            .unwrap_or_default()
    }
    
    /// Get all events for a specific account
    pub async fn get_events_for_account(&self, account_id: &str) -> Vec<ApiKeyEvent> {
        let events = self.events.lock().await;
        let mut result = Vec::new();
        
        for queue in events.values() {
            for event in queue.iter() {
                if event.account_id == account_id {
                    result.push(event.clone());
                }
            }
        }
        
        // Sort by timestamp (newest first)
        result.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        result
    }
    
    /// Get all events for all API keys, with optional filtering and pagination
    pub async fn get_all_events(
        &self,
        limit: Option<usize>,
        offset: Option<usize>,
        account_id: Option<&str>,
    ) -> Vec<ApiKeyEvent> {
        let events = self.events.lock().await;
        let mut result = Vec::new();
        
        // Collect all events
        for queue in events.values() {
            for event in queue.iter() {
                if let Some(account) = account_id {
                    if event.account_id == account {
                        result.push(event.clone());
                    }
                } else {
                    result.push(event.clone());
                }
            }
        }
        
        // Sort by timestamp (newest first)
        result.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        
        // Apply pagination
        let offset = offset.unwrap_or(0);
        let limit = limit.unwrap_or(result.len());
        
        if offset < result.len() {
            let end = std::cmp::min(offset + limit, result.len());
            result[offset..end].to_vec()
        } else {
            Vec::new()
        }
    }
    
    /// Get the number of usage events for a specific API key
    pub async fn get_usage_count(&self, api_key_id: &str) -> usize {
        let events = self.events.lock().await;
        events
            .get(api_key_id)
            .map(|queue| {
                queue
                    .iter()
                    .filter(|event| matches!(event.event_type, ApiKeyEventType::Used { .. }))
                    .count()
            })
            .unwrap_or(0)
    }
    
    /// Record an API key event to the database
    pub async fn persist_event(event: ApiKeyEvent, datastore: Arc<tokio::sync::Mutex<DataStore>>) {
        // In a real implementation, this would store events in a database
        // For now, we just log it
        log::info!(
            "API key event: key={} account={} type={:?} time={}",
            event.api_key_id,
            event.account_id,
            event.event_type,
            event.timestamp
        );
        
        // In the future, implement actual persistence to the datastore
        // For example:
        // let mut ds = datastore.lock().await;
        // ds.record_api_key_event(event).await;
    }
}

// Create a global audit log instance
use once_cell::sync::Lazy;
pub static API_KEY_AUDIT_LOG: Lazy<ApiKeyAuditLog> = Lazy::new(ApiKeyAuditLog::new); 