use std::net::{IpAddr, Ipv4Addr};
use reqwest::Client;
use serde::{Serialize, Deserialize};
use form_dns::api::{BootstrapNodeRequest, BootstrapNodeResponse};

// Simple CLI-based tool to manage bootstrap nodes
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Parse command line arguments
    let args: Vec<String> = std::env::args().collect();
    
    if args.len() < 2 {
        println!("Usage:");
        println!("  {} add <node_id> <ip_address> [region] [ttl]", args[0]);
        println!("  {} remove <ip_address>", args[0]);
        println!("  {} list", args[0]);
        return Ok(());
    }
    
    let command = &args[1];
    let client = Client::new();
    
    match command.as_str() {
        "add" => {
            if args.len() < 4 {
                println!("Usage: {} add <node_id> <ip_address> [region] [ttl]", args[0]);
                return Ok(());
            }
            
            let node_id = &args[2];
            let ip_address: IpAddr = args[3].parse()?;
            let region = if args.len() > 4 { Some(args[4].clone()) } else { None };
            let ttl = if args.len() > 5 { Some(args[5].parse()?) } else { None };
            
            let request = BootstrapNodeRequest {
                node_id: node_id.to_string(),
                ip_address,
                region,
                ttl,
            };
            
            println!("Adding bootstrap node: {} at {}", node_id, ip_address);
            let response = client.post("http://localhost:3005/bootstrap/add")
                .json(&request)
                .send()
                .await?;
                
            let result: BootstrapNodeResponse = response.json().await?;
            match result {
                BootstrapNodeResponse::Success => {
                    println!("✅ Bootstrap node added successfully!");
                },
                BootstrapNodeResponse::Failure(msg) => {
                    println!("❌ Failed to add bootstrap node: {}", msg);
                },
                _ => println!("Unexpected response from server"),
            }
        },
        "remove" => {
            if args.len() < 3 {
                println!("Usage: {} remove <ip_address>", args[0]);
                return Ok(());
            }
            
            let ip_address: IpAddr = args[2].parse()?;
            
            let request = BootstrapNodeRequest {
                node_id: "".to_string(), // Not used for removal
                ip_address,
                region: None,
                ttl: None,
            };
            
            println!("Removing bootstrap node at {}", ip_address);
            let response = client.post("http://localhost:3005/bootstrap/remove")
                .json(&request)
                .send()
                .await?;
                
            let result: BootstrapNodeResponse = response.json().await?;
            match result {
                BootstrapNodeResponse::Success => {
                    println!("✅ Bootstrap node removed successfully!");
                },
                BootstrapNodeResponse::Failure(msg) => {
                    println!("❌ Failed to remove bootstrap node: {}", msg);
                },
                _ => println!("Unexpected response from server"),
            }
        },
        "list" => {
            println!("Listing all bootstrap nodes:");
            let response = client.get("http://localhost:3005/bootstrap/list")
                .send()
                .await?;
                
            let result: BootstrapNodeResponse = response.json().await?;
            match result {
                BootstrapNodeResponse::NodesList(nodes) => {
                    if nodes.is_empty() {
                        println!("No bootstrap nodes configured.");
                    } else {
                        println!("| Node ID            | IP Address      | Region       | Health Status | TTL  |");
                        println!("|--------------------|-----------------|--------------|--------------:|------|");
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
            println!("Unknown command: {}", command);
            println!("Usage:");
            println!("  {} add <node_id> <ip_address> [region] [ttl]", args[0]);
            println!("  {} remove <ip_address>", args[0]);
            println!("  {} list", args[0]);
        }
    }
    
    Ok(())
} 