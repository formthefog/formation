use std::net::IpAddr;
use serde::{Serialize, Deserialize};
use log::{info, warn, error};
use std::error::Error;
use reqwest::Client;

/// Default DNS API endpoint for bootstrap domain management
const DEFAULT_DNS_API: &str = "http://localhost:3005";

/// Request to register/unregister a bootstrap node
#[derive(Serialize, Deserialize, Debug)]
pub struct BootstrapNodeRequest {
    pub node_id: String,
    pub ip_address: IpAddr,
    pub region: Option<String>,
    pub ttl: Option<u32>,
}

/// Response from bootstrap node management API
#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type")]
pub enum BootstrapNodeResponse {
    #[serde(rename = "success")]
    Success,
    #[serde(rename = "failure")]
    Failure(String),
    #[serde(rename = "nodes_list")]
    NodesList(Vec<BootstrapNodeInfo>),
}

/// Information about a bootstrap node
#[derive(Serialize, Deserialize, Debug)]
pub struct BootstrapNodeInfo {
    pub node_id: String,
    pub ip_address: IpAddr,
    pub region: Option<String>,
    pub ttl: u32,
    pub health_status: String,
}

/// Register a node as a bootstrap node
pub async fn register_bootstrap_node(
    node_id: &str,
    ip_address: IpAddr,
    region: Option<String>,
    ttl: Option<u32>,
    dns_api: Option<&str>
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let api_url = dns_api.unwrap_or(DEFAULT_DNS_API);
    info!("Registering node {} at {} as a bootstrap node", node_id, ip_address);
    
    let client = reqwest::Client::new();
    let request = BootstrapNodeRequest {
        node_id: node_id.to_string(),
        ip_address,
        region,
        ttl: None, // Use the DNS server's default TTL
    };
    
    let response = client
        .post(&format!("{}/bootstrap/add", api_url))
        .json(&request)
        .send()
        .await?;
        
    match response.status() {
        reqwest::StatusCode::OK => {
            let result = response.json::<BootstrapNodeResponse>().await?;
            match result {
                BootstrapNodeResponse::Success => {
                    info!("Successfully registered node as bootstrap node");
                    Ok(())
                },
                BootstrapNodeResponse::Failure(err) => {
                    error!("Failed to register bootstrap node: {}", err);
                    Err(err.into())
                },
                _ => {
                    error!("Unexpected response from bootstrap registration API");
                    Err("Unexpected response from bootstrap registration API".into())
                }
            }
        },
        status => {
            let error_text = response.text().await?;
            error!("Failed to register bootstrap node: {} - {}", status, error_text);
            Err(format!("HTTP error {}: {}", status, error_text).into())
        }
    }
}

/// Unregister a node as a bootstrap node
pub async fn unregister_bootstrap_node(
    node_id: &str,
    ip_address: Option<IpAddr>,
    dns_api: Option<&str>
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let api_url = dns_api.unwrap_or(DEFAULT_DNS_API);
    
    if let Some(ip) = ip_address {
        info!("Unregistering bootstrap node {} at {}", node_id, ip);
    } else {
        info!("Unregistering bootstrap node {}", node_id);
    }
    
    let client = reqwest::Client::new();
    let request = BootstrapNodeRequest {
        node_id: node_id.to_string(),
        ip_address: ip_address.unwrap_or_else(|| "0.0.0.0".parse().unwrap()),
        region: None,
        ttl: None,
    };
    
    let response = client
        .post(&format!("{}/bootstrap/remove", api_url))
        .json(&request)
        .send()
        .await?;
        
    match response.status() {
        reqwest::StatusCode::OK => {
            let result = response.json::<BootstrapNodeResponse>().await?;
            match result {
                BootstrapNodeResponse::Success => {
                    info!("Successfully unregistered bootstrap node");
                    Ok(())
                },
                BootstrapNodeResponse::Failure(err) => {
                    error!("Failed to unregister bootstrap node: {}", err);
                    Err(err.into())
                },
                _ => {
                    error!("Unexpected response from bootstrap unregistration API");
                    Err("Unexpected response from bootstrap unregistration API".into())
                }
            }
        },
        status => {
            let error_text = response.text().await?;
            error!("Failed to unregister bootstrap node: {} - {}", status, error_text);
            Err(format!("HTTP error {}: {}", status, error_text).into())
        }
    }
}

/// Get the list of current bootstrap nodes
pub async fn list_bootstrap_nodes(
    dns_api: Option<&str>
) -> Result<Vec<BootstrapNodeInfo>, Box<dyn std::error::Error + Send + Sync>> {
    let api_url = dns_api.unwrap_or(DEFAULT_DNS_API);
    info!("Listing bootstrap nodes");
    
    let client = reqwest::Client::new();
    let response = client
        .get(&format!("{}/bootstrap/list", api_url))
        .send()
        .await?;
        
    match response.status() {
        reqwest::StatusCode::OK => {
            let result = response.json::<BootstrapNodeResponse>().await?;
            match result {
                BootstrapNodeResponse::NodesList(nodes) => {
                    Ok(nodes)
                },
                _ => {
                    error!("Unexpected response from bootstrap list API");
                    Err("Unexpected response from bootstrap list API".into())
                }
            }
        },
        status => {
            let error_text = response.text().await?;
            error!("Failed to list bootstrap nodes: {} - {}", status, error_text);
            Err(format!("HTTP error {}: {}", status, error_text).into())
        }
    }
} 