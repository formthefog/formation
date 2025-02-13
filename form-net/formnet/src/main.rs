//! A service to create and run formnet, a wireguard based p2p VPN tunnel, behind the scenes
use std::path::PathBuf;
use std::time::Duration;
use alloy_core::primitives::Address;
use k256::ecdsa::SigningKey;
use clap::{Parser, Subcommand, Args};
use form_config::OperatorConfig;
use form_types::PeerType;
use formnet::{init::init, serve::serve};
use formnet::{ensure_crdt_datastore, leave, request_to_join, revert_formnet_resolver, set_formnet_resolver, uninstall, user_join_formnet, vm_join_formnet, NETWORK_NAME};

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
    /// A 20 byte hex string that represents an ethereum address
    #[arg(short, long="signing-key", aliases=["private-key", "secret-key"])]
    signing_key: Option<String>,
    #[arg(short, long, default_value="true")]
    encrypted: bool,
    #[arg(short, long)]
    password: Option<String>,
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
    /// A 20 byte hex string that represents an ethereum address
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
                    let operator_config = OperatorConfig::from_file(
                        parser.config_path,
                        parser.encrypted,
                        parser.password.as_deref(),
                    ).ok();
                    let signing_key = if parser.signing_key.is_none() {
                        let config = operator_config.clone().expect("If signing key is not provided, a valid operator config file must be provided");
                        config.secret_key.expect("Config is guaranteed to have a secret key at this point, if not something went terribly wrong")
                    } else {
                        parser.signing_key.unwrap()
                    };
                    let address = hex::encode(Address::from_private_key(&SigningKey::from_slice(&hex::decode(&signing_key)?)?));
                    let my_ip = if !parser.bootstraps.is_empty() {
                        log::info!("Found bootstrap in parser...");
                        let my_ip = request_to_join(
                            parser.bootstraps.clone(),
                            address.clone(),
                            PeerType::Operator
                        ).await?;
                        ensure_crdt_datastore().await?;
                        my_ip
                    } else if !operator_config.clone().unwrap().bootstrap_nodes.is_empty() {
                        log::info!("Found bootstrap in config...");
                        let my_ip = request_to_join(
                            operator_config.unwrap().bootstrap_nodes.clone(),
                            address.clone(),
                            PeerType::Operator
                        ).await?;
                        ensure_crdt_datastore().await?;
                        my_ip
                    } else {
                        init(address.clone()).await?
                    };

                    let (shutdown, _) = tokio::sync::broadcast::channel::<()>(2);
                    let mut formnet_receiver = shutdown.subscribe();
                    let inner_address = address.clone();
                    let formnet_server_handle = tokio::spawn(async move {
                        tokio::select! {
                            res = serve(NETWORK_NAME, inner_address) => {
                                if let Err(e) = res {
                                    eprintln!("Error trying to serve formnet server: {e}");
                                }
                            }
                            _ = formnet_receiver.recv() => {
                                eprintln!("Formnet Server: Received shutdown signal");
                            }
                        }
                    });

                    tokio::time::sleep(Duration::from_secs(5)).await;
                    log::info!("reverting existing resolver for formnet interface");
                    #[cfg(target_os = "linux")]
                    if let Ok(()) = revert_formnet_resolver().await {
                        #[cfg(target_os = "linux")]
                        set_formnet_resolver(&my_ip.to_string(), "~fog").await?;
                        log::info!("Setting up dns resolver");
                    }

                    tokio::signal::ctrl_c().await?;
                    shutdown.send(())?;
                    formnet_server_handle.await?;
                }
                OperatorOpts::Leave(parser) => {
                    let operator_config = OperatorConfig::from_file(
                        parser.config_path,
                        parser.encrypted,
                        parser.password.as_deref(),
                    ).ok();
                    let signing_key = if parser.signing_key.is_none() {
                        let config = operator_config.clone().expect("If signing key is not provided, a valid operator config file must be provided");
                        config.secret_key.expect("Config is guaranteed to have a secret key at this point, if not something went terribly wrong")
                    } else {
                        parser.signing_key.unwrap()
                    };
                    leave(parser.bootstraps, signing_key).await?; 
                    uninstall()?;
                }
            }
        }
        Membership::User(opts) => {
            let address = hex::encode(Address::from_private_key(&SigningKey::from_slice(&hex::decode(&opts.secret_key)?)?));
            user_join_formnet(address, opts.provider).await?;
        } 
        Membership::Instance => {
            vm_join_formnet().await?;
        }
    }

    Ok(())
}
