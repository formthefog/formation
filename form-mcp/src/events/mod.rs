// Events module for the MCP server
//
// This module implements the event system for workload state changes
// and notifications.

/// Event represents a state change or notification
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Event {
    /// Event type
    pub event_type: String,
    /// Event source
    pub source: String,
    /// Event data
    pub data: serde_json::Value,
    /// Event timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

// The full event system will be implemented in future sub-tasks 