use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, Subcommand};
use dialoguer::{theme::ColorfulTheme, Confirm, Input};
use form_config::*;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Run the interactive operator configuration wizard
    Wizard,
    /// Manage bootstrap nodes in the DNS
    Bootstrap {
        #[command(subcommand)]
        action: BootstrapCommands,
    },
}

#[derive(Subcommand)]
enum BootstrapCommands {
    /// Add a bootstrap node to the DNS
    Add {
        /// Node ID (used for identification)
        #[arg(long, short)]
        node_id: String,
        
        /// IP address of the bootstrap node
        #[arg(long, short)]
        ip: String,
        
        /// Region of the bootstrap node (e.g., us-east, eu-west)
        #[arg(long, short)]
        region: Option<String>,
        
        /// Custom TTL value in seconds
        #[arg(long, short)]
        ttl: Option<u32>,
        
        /// DNS API endpoint (default: http://localhost:3005)
        #[arg(long, short, default_value = "http://localhost:3005")]
        api: String,
    },
    
    /// Remove a bootstrap node from the DNS
    Remove {
        /// IP address of the bootstrap node to remove
        #[arg(long, short)]
        ip: String,
        
        /// DNS API endpoint (default: http://localhost:3005)
        #[arg(long, short, default_value = "http://localhost:3005")]
        api: String,
    },
    
    /// List all bootstrap nodes
    List {
        /// DNS API endpoint (default: http://localhost:3005)
        #[arg(long, short, default_value = "http://localhost:3005")]
        api: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    
    match cli.command {
        Some(Commands::Wizard) => run_wizard(),
        Some(Commands::Bootstrap { action }) => manage_bootstrap_nodes(action).await,
        None => run_wizard(),
    }
}

fn run_wizard() -> Result<()> {
    let config = run_config_wizard()?;
    
    // Ask about configuration save location
    let theme = ColorfulTheme::default();
    let default_config_path = PathBuf::from("./secrets/.operator-config.json");
    
    let use_default_path = Confirm::with_theme(&theme)
        .with_prompt(format!("Save config to {}?", default_config_path.display()))
        .default(true)
        .interact()?;

    let config_path = if use_default_path {
        std::fs::create_dir_all(default_config_path.parent().unwrap())?;
        default_config_path
    } else {
        let path: String = Input::with_theme(&theme)
            .with_prompt("Enter config file path")
            .interact_text()?;
        PathBuf::from(path)
    };

    // Ask about key encryption if keys are present
    let encrypt_keys = if config.secret_key.is_some() || config.mnemonic.is_some() {
        Confirm::with_theme(&theme)
            .with_prompt("Would you like to encrypt your keys in the keystore?")
            .default(true)
            .interact()?
    } else {
        false
    };

    save_config_and_keystore(&config, &config_path, encrypt_keys)?;
    Ok(())
}

/// Manage bootstrap nodes in the DNS
async fn manage_bootstrap_nodes(action: BootstrapCommands) -> Result<()> {
    // Define data structures for API requests/responses
    #[derive(serde::Serialize, serde::Deserialize)]
    struct BootstrapNodeRequest {
        node_id: String,
        ip_address: String,
        region: Option<String>,
        ttl: Option<u32>,
    }

    #[derive(serde::Serialize, serde::Deserialize, Debug)]
    #[serde(tag = "type")]
    enum BootstrapNodeResponse {
        #[serde(rename = "success")]
        Success,
        #[serde(rename = "failure")]
        Failure(String),
        #[serde(rename = "nodes_list")]
        NodesList(Vec<BootstrapNodeInfo>),
    }

    #[derive(serde::Serialize, serde::Deserialize, Debug)]
    struct BootstrapNodeInfo {
        node_id: String,
        ip_address: String,
        region: Option<String>,
        ttl: u32,
        health_status: String,
    }

    let client = reqwest::Client::new();
    
    match action {
        BootstrapCommands::Add { node_id, ip, region, ttl, api } => {
            println!("Adding bootstrap node: {} at {}", node_id, ip);
            
            let request = BootstrapNodeRequest {
                node_id,
                ip_address: ip,
                region,
                ttl,
            };
            
            let response = client.post(&format!("{}/bootstrap/add", api))
                .json(&request)
                .send()
                .await?;
                
            match response.status() {
                reqwest::StatusCode::OK => {
                    println!("✅ Bootstrap node added successfully!");
                },
                _ => {
                    let error_text = response.text().await?;
                    println!("❌ Failed to add bootstrap node: {}", error_text);
                }
            }
        },
        
        BootstrapCommands::Remove { ip, api } => {
            println!("Removing bootstrap node at {}", ip);
            
            let request = BootstrapNodeRequest {
                node_id: "".to_string(), // Not used for removal
                ip_address: ip,
                region: None,
                ttl: None,
            };
            
            let response = client.post(&format!("{}/bootstrap/remove", api))
                .json(&request)
                .send()
                .await?;
                
            match response.status() {
                reqwest::StatusCode::OK => {
                    println!("✅ Bootstrap node removed successfully!");
                },
                _ => {
                    let error_text = response.text().await?;
                    println!("❌ Failed to remove bootstrap node: {}", error_text);
                }
            }
        },
        
        BootstrapCommands::List { api } => {
            println!("Listing all bootstrap nodes:");
            
            let response = client.get(&format!("{}/bootstrap/list", api))
                .send()
                .await?;
                
            match response.status() {
                reqwest::StatusCode::OK => {
                    let result = response.json::<BootstrapNodeResponse>().await?;
                    
                    match result {
                        BootstrapNodeResponse::NodesList(nodes) => {
                            if nodes.is_empty() {
                                println!("No bootstrap nodes configured.");
                            } else {
                                println!("| {:<18} | {:<15} | {:<12} | {:<14} | {:<4} |",
                                    "Node ID", "IP Address", "Region", "Health Status", "TTL");
                                println!("|--------------------|-----------------|--------------|----------------|------|");
                                
                                for node in nodes {
                                    println!("| {:<18} | {:<15} | {:<12} | {:<14} | {:<4} |",
                                        node.node_id,
                                        node.ip_address,
                                        node.region.unwrap_or_else(|| "-".to_string()),
                                        node.health_status,
                                        node.ttl
                                    );
                                }
                            }
                        },
                        _ => println!("Unexpected response from server"),
                    }
                },
                _ => {
                    let error_text = response.text().await?;
                    println!("❌ Failed to list bootstrap nodes: {}", error_text);
                }
            }
        },
    }
    
    Ok(())
}
