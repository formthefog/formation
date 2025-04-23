//! A service to create and run formnet, a wireguard based p2p VPN tunnel, behind the scenes
use std::path::PathBuf;
use std::time::Duration;
use alloy_core::primitives::Address;
use k256::ecdsa::SigningKey;
use clap::{Parser, Subcommand, Args};
use form_config::OperatorConfig;
use form_types::PeerType;
use formnet::{init, serve};
use formnet::{leave, uninstall, user_join_formnet, vm_join_formnet, NETWORK_NAME};
#[cfg(target_os = "linux")]
use formnet::{revert_formnet_resolver, set_formnet_resolver};
use reqwest::Client;
use serde_json::Value; 
use colored::Colorize;
use formnet::bootstrap;
use formnet::api;

// Import the Shutdown struct from the correct location
use std::net::Shutdown;

#[derive(Clone, Debug, Parser)]
struct Cli {
    #[clap(subcommand)]
    opts: Membership,
}

#[derive(Clone, Debug, Subcommand)]
enum Membership {
    #[command(alias="node", subcommand)]
    Operator(OperatorOpts),
    #[command(alias="dev")]
    User(UserOpts),
    #[command(alias="vm")]
    Instance
}

#[derive(Clone, Debug, Subcommand)]
enum OperatorOpts {
    #[command(alias="install")]
    Join(OperatorJoinOpts),
    #[command(alias="uninstall")]
    Leave(OperatorLeaveOpts)
}

#[derive(Clone, Debug, Args)]
struct OperatorJoinOpts {
    /// The path to the operator config file 
    #[arg(long="config-path", short='C', aliases=["config", "config-file"], default_value_os_t=PathBuf::from(".operator-config.json"))]
    config_path: PathBuf,
    /// 1 or more bootstrap nodes that are known
    /// and already active in the Network
    /// Will eventually be replaced with a discovery service
    #[arg(short, long, alias="bootstrap")]
    bootstraps: Vec<String>,
    /// A domain that resolves to one or more bootstrap nodes
    /// This will be used instead of or in addition to the bootstrap nodes
    #[arg(long="bootstrap-domain", alias="domain")]
    bootstrap_domain: Option<String>,
    #[arg(short, long="signing-key", aliases=["private-key", "secret-key"])]
    signing_key: Option<String>,
    #[arg(short, long, default_value="true")]
    encrypted: bool,
    #[arg(short, long)]
    password: Option<String>,
    #[arg(long="public-ip", short='i')]
    public_ip: Option<String>,
}

#[derive(Clone, Debug, Args)]
struct OperatorLeaveOpts {
    /// The path to the operator config file 
    #[arg(long="config-path", short='C', aliases=["config", "config-file"], default_value_os_t=PathBuf::from(".operator-config.json"))]
    config_path: PathBuf,
    /// 1 or more bootstrap nodes that are known
    /// and already active in the Network
    /// Will eventually be replaced with a discovery service
    #[arg(short, long, alias="bootstrap")]
    bootstraps: Vec<String>,
    /// A domain that resolves to one or more bootstrap nodes
    /// This will be used instead of or in addition to the bootstrap nodes
    #[arg(long="bootstrap-domain", alias="domain")]
    bootstrap_domain: Option<String>,
    #[arg(short, long="signing-key", aliases=["private-key", "secret-key"])]
    signing_key: Option<String>,
    #[arg(short, long, default_value="true")]
    encrypted: bool,
    #[arg(short, long)]
    password: Option<String>,
}

#[derive(Clone, Debug, Args)]
struct UserOpts {
    #[arg(alias="endpoint")]
    provider: String, 
    #[arg(alias="endpoint-port")]
    port: u16,
    #[arg(long, short)]
    secret_key: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    simple_logger::init_with_level(log::Level::Info).unwrap();

    let cli = Cli::parse();
    log::info!("{cli:?}");
    
    match cli.opts {
        Membership::Operator(parser) => {
            match parser {
                OperatorOpts::Join(parser) => {
                    let op_config = match OperatorConfig::from_file(
                        parser.config_path,
                        parser.encrypted,
                        parser.password.as_deref(),
                    ).ok() {
                        Some(c) => c,
                        None => {
                            log::error!("Could not retrieve operator configuration");
                            return Ok(());
                        }
                    };

                    if op_config.secret_key.is_none() {
                        log::error!("Operator config must contain a secret key");
                        return Ok(());
                    }

                    let secret_key_string = op_config.secret_key.unwrap();
                    let sk = SigningKey::from_slice(
                        &hex::decode(&secret_key_string)?
                    )?;

                    let address = hex::encode(Address::from_private_key(&sk));

                    // Build bootstrap list, combining user-provided bootstraps with the bootstrap domain
                    let mut bootstraps = parser.bootstraps.clone();
                    if bootstraps.is_empty() {
                        bootstraps = op_config.bootstrap_nodes.clone();
                        if bootstraps.is_empty() {
                            if let Some(bootstrap_domain) = &op_config.bootstrap_domain {
                                bootstraps = vec![bootstrap_domain.clone()];
                            }
                        }
                    }

                    // If no bootstraps are specified, initialize the node without joining
                    if bootstraps.is_empty() {
                        log::info!("No bootstraps specified, initializing node without joining");
                        if let Err(e) = formnet::init::init(address).await {
                            log::error!("Error in formnet init... {e}");
                        }
                        return Ok(());
                    }

                    // Log and join using bootstrap nodes
                    log::info!("Using bootstrap nodes: {:?}", bootstraps);
                    
                    // Attempt to get our outbound IP
                    let pub_ip = match publicip::get_any(publicip::Preference::Ipv4) {
                        Some(ip) => {
                            log::info!("Detected public IP: {}", ip);
                            Some(ip.to_string())
                        }
                        None => {
                            log::warn!("Failed to detect public IP");
                            parser.public_ip.clone()
                        }
                    };

                    // Join the network, passing the bootstrap node flag and region from the operator config
                    match formnet::join::request_to_join(
                        bootstraps,
                        op_config.address.clone().unwrap_or_default(),
                        PeerType::Operator,
                        pub_ip,
                        Some(op_config.is_bootstrap_node),
                        op_config.region.clone(),
                    ).await {
                        Ok(ip) => {
                            log::info!("Successfully joined with IP {}", ip);
                        }
                        Err(e) => {
                            log::error!("Failed to join: {}", e);
                            return Ok(());
                        }
                    }
                }
                OperatorOpts::Leave(parser) => {
                    let op_config = match OperatorConfig::from_file(
                        parser.config_path,
                        parser.encrypted,
                        parser.password.as_deref(),
                    ).ok() {
                        Some(c) => c,
                        None => {
                            log::error!("Could not retrieve operator configuration");
                            return Ok(());
                        }
                    };

                    if op_config.secret_key.is_none() {
                        log::error!("Operator config must contain a secret key");
                        return Ok(());
                    }

                    // If this node is a bootstrap node, unregister it from the DNS service
                    if op_config.is_bootstrap_node {
                        log::info!("Unregistering from bootstrap domain");
                        
                        // Attempt to unregister from bootstrap service
                        match formnet::bootstrap::unregister_bootstrap_node(
                            &op_config.address.clone().unwrap_or_default(),
                            None, // We don't need to specify IP as we're using the node ID
                            None  // Use default DNS API endpoint
                        ).await {
                            Ok(_) => {
                                log::info!("Successfully unregistered from bootstrap domain");
                            },
                            Err(e) => {
                                log::warn!("Failed to unregister from bootstrap domain: {}", e);
                                // Continue with the leave process even if unregistration fails
                            }
                        }
                    }

                    // Proceed with the leave command
                    let address = op_config.address.clone().unwrap_or_default();
                    
                    // Use the leave function directly instead of Shutdown
                    match leave(vec![], address).await {
                        Ok(_) => {
                            log::info!("Node successfully shutdown");
                            return Ok(());
                        },
                        Err(e) => {
                            log::error!("Failed to shutdown node: {}", e);
                            return Ok(());
                        }
                    }
                }
            }
        }
        Membership::User(opts) => {
            let publicip = {
                let res = Client::new().get("http://api.ipify.org?format=json")
                    .send().await?.json::<Value>().await;
                let ipopt = if let Ok(ip) =  res {
                        let opt = ip.clone().get("ip").and_then(|i| i.as_str()).clone().map(String::from);
                        opt
                } else {
                    None
                };
                ipopt
            };
            if let Some(ref ip) = publicip {
                println!("Found your {}: {}", "public IP".bold().bright_blue(), ip.bold().bright_yellow());
            }

            let address = hex::encode(Address::from_private_key(&SigningKey::from_slice(&hex::decode(&opts.secret_key)?)?));
            user_join_formnet(address, opts.provider, publicip).await?;
        } 
        Membership::Instance => {
            vm_join_formnet().await?;
        }
    }

    Ok(())
}
