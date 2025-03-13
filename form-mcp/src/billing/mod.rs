// Billing module for the MCP server
//
// This module handles billing and payment integration for the MCP server.

/// Represents a billing record for resource usage
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BillingRecord {
    /// User ID
    pub user_id: String,
    /// Resource ID
    pub resource_id: String,
    /// Resource type
    pub resource_type: String,
    /// Usage amount
    pub usage: f64,
    /// Usage unit (e.g., "hour", "GB")
    pub unit: String,
    /// Timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

// The full billing system will be implemented in future sub-tasks 