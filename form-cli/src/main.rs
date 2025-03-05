use std::path::PathBuf;
use form_types::state::{Response, Success};
use clap::{Parser, Subcommand};
use dialoguer::{theme::ColorfulTheme, Confirm};
use colored::*;
use form_cli::{
    decrypt_file, default_config_dir, default_data_dir, default_keystore_dir, join_formnet, operator_config, Config, DnsCommand, Init, Keystore, KitCommand, manage::ManageCommand, Operator, PackCommand, WalletCommand
};
use form_p2p::queue::QUEUE_PORT;
use formnet::{leave, uninstall};
use reqwest::Client;
use serde_json::Value;

/// The official developer CLI for building, deploying and managing 
/// verifiable confidential VPS instances in the Formation network
#[derive(Debug, Parser)]
pub struct Form {
    #[clap(default_value_os_t=default_config_dir())]
    config_dir: PathBuf,
    #[clap(default_value_os_t=default_data_dir())]
    data_dir: PathBuf,
    #[clap(default_value_os_t=default_keystore_dir())]
    keystore_dir: PathBuf,
    /// The ip or domain name of the API provider 
    /// (currently a http api, will switch to gRPC for testnet)
    /// Default is local, however, the best way to get set up 
    /// with a valid provider is to run `form kit init`.
    /// This will set you up with a full developer kit
    /// will allow you to pick a provider, or get a database
    /// of providers and randomly select/rotate providers
    /// on subsequent calls, among other features.
    #[clap(default_value="127.0.0.1")]
    provider: String, 
    /// The port where form pack build gets sent to for the provider
    /// in the future, all request (build, ship, etc.) related to 
    /// building, deploying and managing instances will be handled
    /// via a single gRPC endpoint on the provider, and therefore
    /// this will be phased out. We highly suggest you use the defaults
    /// unless you have a provider that you know is reliable that is using
    /// a different port, in the case of domain based provider, ports may 
    /// not be necessary at all.
    #[clap(default_value="3003")]
    formpack_port: u16, 
    /// The port where form pack ship gets sent to for the provider
    /// Same caveats apply here
    #[clap(default_value="3002")]
    vmm_port: u16,
    /// The port where the providers formnet api listens
    #[clap(default_value="3001")]
    formnet_port: u16,
    #[clap(short='q', default_value_t=true)]
    queue: bool,
    /// The passsword you provided to encrypt your keystore
    /// when you ran `form kit init`
    #[clap(short='P', long="password")]
    keystore_password: Option<String>,
    #[clap(short='D', long="debug", default_value_t=false)]
    debug: bool,
    /// The subcommand that will be called 
    #[clap(subcommand)]
    pub command: FormCommand
}

/// `form` is the core command of the CLI, all subcommands fall under the  
/// `form` command
#[derive(Debug, Subcommand)]
pub enum FormCommand {
    /// Commands related to building a developer kit locally,
    /// including the `init` wizard
    #[clap(subcommand)]
    Kit(KitCommand),
    /// Commands related to managing your wallet locally,
    #[clap(subcommand)]
    Wallet(WalletCommand),
    /// Commands related to building your workload in a manner that is
    /// compatible with the formation network
    #[clap(subcommand)]
    Pack(PackCommand),
    /// Commands related to managing the build and deployment of you workload 
    /// including status, join (to join formnet), leave (to leave and uninstall formnet),
    /// `formnet-up` to refresh the peers in formnet and stay up to date on the routes
    /// and endpoints to peers, including your workload instances.
    /// and get_ips to get the formnet ip addresses of the redundant instances
    /// from your build id
    #[clap(subcommand)]
    Manage(ManageCommand),
    /// Commands related to managing domain names associated with your workload 
    /// this includes adding and removing domains from pointing to particular
    /// builds, and it includes the ability to get a vanity domain for developer
    /// access within formnet
    #[clap(subcommand)]
    Dns(DnsCommand),
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut parser = Form::parse();
    // Attempt to load form kit
    // if none provided, prompt to run init
    match parser.command {
        FormCommand::Pack(ref pack_command) => {
            match pack_command {
                PackCommand::Build(build_command) => {
                    println!("Attempting to acquire config and keystore");
                    let (config, keystore) = load_config_and_keystore(&parser).await?;
                    println!("getting provider from config");
                    let provider = config.hosts[0].clone();
                    if parser.queue {
                        let resp = build_command.clone().handle_queue(&provider, QUEUE_PORT, keystore).await;
                        println!("Response: {resp:?}");
                    } else {
                        let resp = build_command.clone().handle(&provider, config.pack_manager_port).await;
                        println!("Response: {resp:?}");
                    }
                }
                PackCommand::DryRun(dry_run_command) => {
                    let resp = dry_run_command.clone().handle().await?;
                    println!("Response: {resp:?}");
                }
                PackCommand::Validate(validate_command) => {
                    let resp = validate_command.handle().await?;
                    for line in resp.lines() {
                        println!("{line}")
                    }
                }
                PackCommand::Ship(ship_command) => {
                    let (config, keystore) = load_config_and_keystore(&parser).await?;
                    let provider = config.hosts[0].clone();
                    if parser.queue {
                        let _ = ship_command.clone().handle_queue(&provider, Some(keystore)).await?;
                    } else {
                        let _ = ship_command.clone().handle(&provider, config.pack_manager_port).await?;
                    }
                }
                PackCommand::Status(status_command) => {
                    let (config, _) = load_config_and_keystore(&parser).await?;
                    let provider = config.hosts[0].clone();
                    status_command.handle_status(provider, 3004).await?;
                }
            }
        }
        FormCommand::Kit(ref mut kit_command) => {
            if parser.debug {
                simple_logger::SimpleLogger::new().init().unwrap();
            }
            match kit_command {
                KitCommand::Init(ref mut init) => {
                    let (config, keystore) = init.handle().await?;
                    let host = config.hosts[0].clone();
                    if let true = config.join_formnet {
                        join_formnet(keystore.address.to_string(), host).await?; 
                    }
                }
                KitCommand::Operator(sub) => {
                    match sub {
                        Operator::Config => {
                            operator_config()?;
                        }
                    }
                }
            }
        }
        FormCommand::Manage(ref manage_command) => {
            match manage_command {
                ManageCommand::Join(join_command) => {
                    if parser.debug {
                        simple_logger::SimpleLogger::new().init().unwrap();
                    }
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
                    let (config, keystore) = load_config_and_keystore(&parser).await?;
                    let provider = config.hosts[0].clone();
                    join_command.handle_join_command(provider, keystore, publicip).await?;
                }
                ManageCommand::Leave(_) => {
                    let (config, keystore) = load_config_and_keystore(&parser).await?;
                    let signing_key = keystore.secret_key;
                    leave(config.hosts, signing_key).await?; 
                    println!(r#"
You have {}, you will {} nodes unless you rejoin."#,
"officially left formnet".yellow(),
"no longer be able to connect to other formnet".bold().bright_red(),
);
                    uninstall().await?;
                    println!(r#"
The {} interface has officially been removed from your machine
"#,
"formnet".bold().yellow(),
);
                }
                ManageCommand::GetIp(get_ip_command) => {
                    let (config, _) = load_config_and_keystore(&parser).await?;
                    let build_id = get_ip_command.build_id.clone();
                    let host = config.hosts[0].clone();
                    let resp = Client::new()
                        .get(format!("http://{host}:3004/instance/list"))
                        .send()
                        .await?.json::<Response<Value>>().await?;

                    let ips = match resp {
                        Response::Success(Success::List(values)) => {
                            values.iter().filter_map(|inst| {
                                if let Some(bid) = inst.get("build_id").and_then(|bid| bid.as_str()) {
                                    if bid == build_id {
                                        if let Some(ip) = inst.get("formnet_ip").and_then(|ip| ip.as_str()) {
                                            Some(ip.to_string())
                                        } else {
                                            None 
                                        }
                                    } else {
                                        None
                                    }
                                } else {
                                    None
                                }
                            }).collect::<Vec<String>>()
                        }
                        _ => vec![],
                    };
                    let ips_string = ips.join(", ");
                    println!(r#"
Your build has {} instances, below are their formnet ip addresses:

Instance IP Addrsses: {}
"#, 
format!("{}", ips.len()).yellow(), 
ips_string.yellow(),
);
                }
                ManageCommand::FormnetUp(formnet_up_command) => {
                if parser.debug {
                    simple_logger::SimpleLogger::new().init().unwrap();
                }
                    formnet_up_command.handle_formnet_up()?;
                }
                ManageCommand::Stop(stop_command) => {
                    let (config, keystore) = load_config_and_keystore(&parser).await?;
                    let provider = config.hosts[0].clone();
                    if parser.queue {
                        stop_command.handle_queue(&provider, Some(keystore)).await?;
                    } else {
                        let resp = stop_command.handle(&provider, config.vmm_port).await?;
                        println!("Response: {:?}", resp);
                    }
                }
                ManageCommand::Start(start_command) => {
                    let (config, keystore) = load_config_and_keystore(&parser).await?;
                    let provider = config.hosts[0].clone();
                    if parser.queue {
                        start_command.handle_queue(&provider, Some(keystore)).await?;
                    } else {
                        let resp = start_command.handle(&provider, config.vmm_port).await?;
                        println!("Response: {:?}", resp);
                    }
                }
                ManageCommand::Delete(delete_command) => {
                    let (config, keystore) = load_config_and_keystore(&parser).await?;
                    let provider = config.hosts[0].clone();
                    if parser.queue {
                        delete_command.handle_queue(&provider, Some(keystore)).await?;
                    } else {
                        let resp = delete_command.handle(&provider, config.vmm_port).await?;
                        println!("Response: {:?}", resp);
                    }
                }
                ManageCommand::Commit(commit_command) => {
                    let (config, keystore) = load_config_and_keystore(&parser).await?;
                    let provider = config.hosts[0].clone();
                    if parser.queue {
                        commit_command.handle_queue(&provider, Some(keystore)).await?;
                    } else {
                        commit_command.handle(&provider, config.vmm_port).await?;
                    }
                }
                _ => {}
            }
        }
        _ => {}
    }

    Ok(())
}

pub async fn load_config_and_keystore(parser: &Form) -> Result<(Config, Keystore), Box<dyn std::error::Error>> {
    println!("loading config");
    let config = load_config(parser).await?;
    let _host = config.hosts[0].clone();
    println!("loading keystore");
    let keystore = load_keystore(&parser, &config).await?;

    Ok((config, keystore))
}

pub async fn load_keystore(parser: &Form, config: &Config) -> Result<Keystore, Box<dyn std::error::Error>> {
    let keystore: Keystore = {
        if let Some(password) = &parser.keystore_password {
            println!("Password provided, assuming encryption...");
            let path = config.keystore_path.clone().join("form_id");
            let data = std::fs::read(path)?;
            serde_json::from_slice(&decrypt_file(&data, &password)?)?
        } else {
            let password: String = dialoguer::Password::with_theme(&ColorfulTheme::default())
                .with_prompt("Provide your password for Keystore: ")
                .interact()?;

            let path = config.keystore_path.clone().join("form_id");
            let data = std::fs::read(path)?;
            serde_json::from_slice(&decrypt_file(&data, &password)?)?
        }
    };

    Ok(keystore)
}

pub async fn load_config(parser: &Form) -> Result<Config, Box<dyn std::error::Error>> {
    let home = std::env::var("HOME").unwrap_or(".".to_string());
    let formkit_config: Config = {
        let path = std::env::var("FORMKIT").unwrap_or(format!("{home}/.config/form/config.json"));
        let formkit_config_data = std::fs::read_to_string(path);
        match formkit_config_data {
            Ok(data) => serde_json::from_str(&data)?,
            Err(_) => {
                if Confirm::with_theme(&ColorfulTheme::default())
                    .with_prompt("No formkit config found, would you like to set one up?")
                    .default(true)
                    .interact()? {
                        let (config, _keystore) = Init::default().handle().await?;
                        config
                } else {
                    println!("{}", "WARNING!: Using defaults which may not be set up properly, and may lead to errors when building, shipping, and managing your instances".yellow());
                    let config = Config {
                        config_dir: parser.config_dir.clone(),
                        data_dir: parser.data_dir.clone(),
                        keystore_path: parser.keystore_dir.clone(),
                        hosts: vec![parser.provider.clone()],
                        pack_manager_port: parser.formpack_port,
                        vmm_port: parser.vmm_port,
                        formnet_port: parser.formnet_port,
                        join_formnet: true,
                    };
                    config
                }
            }
        }
    };

    Ok(formkit_config)
}
