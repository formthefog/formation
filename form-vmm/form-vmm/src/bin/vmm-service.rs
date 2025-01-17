use clap::Parser;
use log::info;
use vmm_service::{CliArgs, CliCommand, VmManager}; 
use vmm_service::{config::wizard::run_config_wizard, ServiceConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Setup the logger
    simple_logger::init_with_level(log::Level::Info)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;

    // Parse command line args
    let args = CliArgs::parse();

    match args.command {
        CliCommand::Run { config, wizard, sub_addr, pub_addr } => {
            let config = if wizard {
                info!("Running configuration wizard");
                run_config_wizard()?
            } else if let Some(config_path) = config {
                info!("Loading configuration from {}", config_path.display());
                ServiceConfig::from_file(&config_path.to_string_lossy())?
            } else {
                info!("Using default configuration");
                ServiceConfig::default()
            };

            let (shutdown_tx, shutdown_rx) = tokio::sync::broadcast::channel(1024);
            let handle = tokio::task::spawn(async move {
                if let Err(e) = run_vm_manager(
                    config,
                    shutdown_rx,
                    sub_addr.as_deref(), 
                    pub_addr
                ).await {
                    log::error!("{e}");
                }
            });

            let _ = tokio::signal::ctrl_c().await;
            shutdown_tx.send(())?;
            handle.await?;

        }
        CliCommand::Configure {
            output,
            non_interactive,
            start,
            ..
        } => {
            // Create configuration
            let config = if non_interactive {
                ServiceConfig::default()
            } else {
                run_config_wizard()?
            };

            // Save config if requested
            if let Some(path) = output {
                info!("Saving configuration to {}", path.display());
                config.save_to_file(&path.to_string_lossy())?;
            }

            // Start service if requested
            if start {
                info!("Starting service with new configuration");
            }
        }

        CliCommand::Status => {
            info!("Checking service status");
            // TODO: implement status check
        }

    }

    Ok(())
}

async fn run_vm_manager(
    config: ServiceConfig,
    shutdown_rx: tokio::sync::broadcast::Receiver<()>,
    subscriber_uri: Option<&str>,
    publisher_uri: Option<String>
) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    let (event_sender, event_receiver) = tokio::sync::mpsc::channel(1024);
    let api_addr = "0.0.0.0:3002".parse()?;
    let formnet_endpoint = "http://127.0.0.1:3001/join".to_string();
    let vm_manager = VmManager::new(
        event_sender,
        api_addr,
        config,
        formnet_endpoint,
        subscriber_uri,
        publisher_uri,
    ).await?;

    vm_manager.run(shutdown_rx, event_receiver).await 
}
