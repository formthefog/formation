use clap::Parser;
use vmm_service::{CliArgs, CliCommand, VmManager}; 
use form_config::OperatorConfig;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Setup the logger
    simple_logger::init_with_level(log::Level::Info)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;

    // Parse command line args
    let args = CliArgs::parse();
    let config = OperatorConfig::from_file(args.config, args.encrypted, args.password.as_deref()).ok();
    match args.command {
        CliCommand::Run { signing_key, sub_addr, pub_addr } => {
            let signing_key = if signing_key.is_none() {
                let config = config.unwrap();
                config.secret_key.unwrap()
            } else {
                signing_key.unwrap()
            };
            let (shutdown_tx, shutdown_rx) = tokio::sync::broadcast::channel(1024);
            let manager_shutdown = shutdown_tx.subscribe();
            let handle = tokio::task::spawn(async move {
                if let Err(e) = run_vm_manager(
                    signing_key,
                    shutdown_rx,
                    manager_shutdown,
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
        _ => {}
    }

    Ok(())
}

async fn run_vm_manager(
    signing_key: String,
    shutdown_rx: tokio::sync::broadcast::Receiver<()>,
    manager_shutdown: tokio::sync::broadcast::Receiver<()>,
    subscriber_uri: Option<&str>,
    publisher_uri: Option<String>
) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    let (event_sender, event_receiver) = tokio::sync::mpsc::channel(1024);
    let api_addr = "0.0.0.0:3002".parse()?;
    let formnet_endpoint = "127.0.0.1:3001".to_string();
    let vm_manager = VmManager::new(
        event_sender,
        api_addr,
        formnet_endpoint,
        signing_key,
        subscriber_uri,
        publisher_uri,
        manager_shutdown
    ).await?;

    vm_manager.run(shutdown_rx, event_receiver).await 
}
