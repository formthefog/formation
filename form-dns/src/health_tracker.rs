use std::collections::HashMap;
use std::net::IpAddr;
use std::time::Duration;
use tokio::time;
use reqwest::Client;
use log::{debug, error, info, warn};
use serde::{Deserialize, Serialize};
use std::str::FromStr;

use crate::health::SharedIpHealthRepository;

// Default values
pub const DEFAULT_HEARTBEAT_TIMEOUT: Duration = Duration::from_secs(60);
pub const DEFAULT_CHECK_INTERVAL: Duration = Duration::from_secs(10);
pub const DEFAULT_STALE_TIMEOUT: Duration = Duration::from_secs(300); // 5 minutes

/// Simple struct to represent a node from form-state (legacy format)
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Node {
    pub node_id: String,
    pub last_heartbeat: i64,
    pub host: String, // This will be parsed to an IP address
}

/// Response wrapper for form-state API (legacy format)
#[derive(Debug, Clone, Serialize, Deserialize)]
struct StateResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
}

/// Heartbeat data from form-state (new format)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeHeartbeat {
    pub node_id: String,
    pub public_ip: String,
    pub private_ip: String,
    pub timestamp: u64,
    pub region: Option<String>,
    pub status: String,
}

/// Response format for new form-state API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormStateResponse {
    pub nodes: Vec<NodeHeartbeat>,
}

/// Health tracker service that monitors node health and updates the IP health repository
#[allow(unused)]
pub struct HealthTracker {
    /// Form-state API endpoint
    form_state_api: String,
    /// Repository for tracking IP health status
    health_repo: SharedIpHealthRepository,
    /// HTTP client for API requests
    http_client: Client,
    /// Heartbeat timeout
    heartbeat_timeout: Duration,
    /// Check interval
    check_interval: Duration,
    /// How long an IP stays unavailable before being reset
    stale_timeout: Duration,
    /// Node ID to IP mapping cache
    node_ip_cache: HashMap<String, IpAddr>,
}

impl HealthTracker {
    /// Create a new health tracker with the provided repository
    pub fn new(
        form_state_api: String,
        health_repo: SharedIpHealthRepository,
        heartbeat_timeout: Option<Duration>,
        check_interval: Option<Duration>,
        stale_timeout: Option<Duration>,
    ) -> Self {
        Self {
            form_state_api,
            health_repo,
            http_client: Client::new(),
            heartbeat_timeout: heartbeat_timeout.unwrap_or(DEFAULT_HEARTBEAT_TIMEOUT),
            check_interval: check_interval.unwrap_or(DEFAULT_CHECK_INTERVAL),
            stale_timeout: stale_timeout.unwrap_or(DEFAULT_STALE_TIMEOUT),
            node_ip_cache: HashMap::new(),
        }
    }

    /// Start the health tracking service
    pub async fn start_monitoring(&self) {
        info!("Starting health tracker monitoring loop");
        let mut interval = time::interval(self.check_interval);
        
        loop {
            interval.tick().await;
            if let Err(e) = self.check_node_health().await {
                error!("Error checking node health: {}", e);
            }
        }
    }

    /// Check node health and update IP health status
    async fn check_node_health(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        debug!("Checking node health from form-state");
        
        // Fetch nodes from form-state API
        let nodes = self.fetch_nodes().await?;
        
        // Process each node
        for node in nodes {
            self.process_node_heartbeat(node).await?;
        }
        
        // Clean stale entries
        let mut repo = self.health_repo.write().await;
        repo.clear_stale_unavailable(self.stale_timeout);
        drop(repo);
        
        Ok(())
    }
    
    /// Fetch nodes from form-state API
    async fn fetch_nodes(&self) -> Result<Vec<NodeHeartbeat>, Box<dyn std::error::Error + Send + Sync>> {
        let url = format!("{}/api/nodes", self.form_state_api);
        let response = self.http_client
            .get(&url)
            .send()
            .await?
            .json::<FormStateResponse>()
            .await?;
            
        Ok(response.nodes)
    }
    
    /// Process a node heartbeat and update health status
    async fn process_node_heartbeat(&self, node: NodeHeartbeat) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Try to parse the public IP
        if let Ok(ip) = IpAddr::from_str(&node.public_ip) {
            let mut repo = self.health_repo.write().await;
            
            // Determine health status based on node status
            if node.status.to_lowercase() == "active" {
                repo.mark_available(ip);
                debug!("Marked IP {} as available", ip);
            } else {
                let reason = format!("Node {} reported status: {}", node.node_id, node.status);
                repo.mark_unavailable(ip, reason.clone());
                debug!("Marked IP {} as unavailable: {}", ip, reason);
            }
        } else {
            warn!("Failed to parse IP address: {}", node.public_ip);
        }
        
        Ok(())
    }
}

/// Start the health tracker service with the specified configuration
/// 
/// # Arguments
/// 
/// * `form_state_api` - URL to the form-state API endpoint
/// * `heartbeat_timeout` - Optional timeout for heartbeat freshness
/// * `check_interval` - Optional interval between health checks
/// * `stale_timeout` - Optional timeout for marking stale nodes
pub async fn start_health_tracker(
    form_state_api: String,
    heartbeat_timeout: Option<Duration>,
    check_interval: Option<Duration>,
    stale_timeout: Option<Duration>,
) -> SharedIpHealthRepository {
    info!("Starting health tracker with form-state API: {}", form_state_api);
    
    let ht = heartbeat_timeout.unwrap_or(DEFAULT_HEARTBEAT_TIMEOUT);
    
    // Create shared health repository
    let health_repo = crate::health::create_shared_repository(ht);
    
    // Create health tracker instance
    let tracker = HealthTracker::new(
        form_state_api,
        health_repo.clone(),
        heartbeat_timeout,
        check_interval,
        stale_timeout,
    );
    
    // Spawn the monitoring task
    tokio::spawn(async move {
        tracker.start_monitoring().await;
    });
    
    health_repo
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::Ipv4Addr;

    #[tokio::test]
    async fn test_process_node_heartbeat() {
        let health_repo = crate::health::create_shared_repository(Duration::from_secs(60));
        let tracker = HealthTracker::new(
            "http://example.com".to_string(),
            health_repo.clone(),
            None, None, None
        );
        
        // Create an active node
        let active_node = NodeHeartbeat {
            node_id: "node1".to_string(),
            public_ip: "1.2.3.4".to_string(),
            private_ip: "10.0.0.1".to_string(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::SystemTime::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            region: Some("us-east".to_string()),
            status: "active".to_string(),
        };
        
        // Process the heartbeat
        tracker.process_node_heartbeat(active_node).await.unwrap();
        
        // Verify that the node was marked as available
        let ip = IpAddr::V4(Ipv4Addr::new(1, 2, 3, 4));
        let repo = health_repo.read().await;
        assert!(repo.is_available(&ip));
        
        // Now test with an inactive node
        let inactive_node = NodeHeartbeat {
            node_id: "node2".to_string(),
            public_ip: "5.6.7.8".to_string(),
            private_ip: "10.0.0.2".to_string(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::SystemTime::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            region: Some("us-west".to_string()),
            status: "inactive".to_string(),
        };
        
        // Process the heartbeat
        tracker.process_node_heartbeat(inactive_node).await.unwrap();
        
        // Verify that the node was marked as unavailable
        let ip2 = IpAddr::V4(Ipv4Addr::new(5, 6, 7, 8));
        assert!(!repo.is_available(&ip2));
    }
} 
