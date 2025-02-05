use std::path::PathBuf;
use vmm_service::{VmManager, util::default_formfile};
use form_pack::formfile::FormfileParser;
use form_types::CreateVmRequest;
use clap::Parser;

#[derive(Parser)]
struct Cli {
    #[clap(long, short)]
    test_run: usize,
    #[clap(long, short, default_value_t=false)]
    pubsub: bool,
    #[clap(long, short, default_value_os_t=default_formfile(PathBuf::from("./")))]
    formfile: PathBuf,
    #[clap(long, short, default_value_t=String::from("127.0.0.1:51520"))]
    pack_manager: String
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    let parser = Cli::parse();
    // Setup the logger
    simple_logger::init_with_level(log::Level::Info)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;

    // Create the base service configuration
    /*
    log::info!("Building service config");
    let service_config = ServiceConfig::default();
    */

    log::info!("Establishing event and shutdown transactions");
    let (event_tx, event_rx) = tokio::sync::mpsc::channel(1024);
    let (shutdown_tx, shutdown_rx) = tokio::sync::broadcast::channel(1024);
    let formnet_endpoint = "127.0.0.1:3001".to_string();
    let api_addr = "127.0.0.1:3002".parse()?;
    let _pack_manager_endpoint = "127.0.0.1:3003".to_string();
    let _datastore_endpoint = "127.0.0.1:3004".to_string(); 
    let _event_queue_endpoint = "127.0.0.1:3005".to_string(); 

    log::info!("Established endpoints...");
    log::info!("Building VM Manager...");

    let (subscriber_uri, publisher_uri) = if parser.pubsub {
        (Some("127.0.0.1:5556"), Some("127.0.0.1:5555".to_string()))
    } else {
        (None, None)
    };

    let vm_manager = VmManager::new(
        event_tx,
        api_addr,
        formnet_endpoint,
        "test-signing-key".to_string(),
        subscriber_uri,
        publisher_uri,
    ).await?;

    log::info!("Built VM Manager, sleeping for 5 seconds...");
    std::thread::sleep(std::time::Duration::from_secs(5));

    log::info!("Starting VM Manager, then sleeping for 5 seconds...");
    let handle = tokio::task::spawn(async move {
        tokio::select! {
            res = vm_manager.run(shutdown_rx, event_rx) => {
                match res {
                    Ok(()) => log::warn!("VM Manager stopped"),
                    Err(e) => log::error!("VM Manager panicked due to error: {e}")
                }
            }
        }
    });

    std::thread::sleep(std::time::Duration::from_secs(5));

    let formfile = {
        let contents = std::fs::read_to_string(parser.formfile)?;
        FormfileParser::new().parse(&contents).map_err(|e| {
            Box::new(
                std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("Unable to parse Formfile: {e}")
                )
            )
        })?
    };
    log::info!("Building CreateVmRequest...");
    let create_vm_request = CreateVmRequest {
        name: format!("test-vm-{}", parser.test_run),
        recovery_id: 0,
        formfile: serde_json::to_string(&formfile)?,
        signature: Some("test-signature".to_string())
    };

    log::info!("Built CreateVmRequest, converting to JSON string...");
    let client = reqwest::Client::new();
    let resp = client.post("http://127.0.0.1:3002/vm")
        .json(&create_vm_request)
        .send()
        .await?;

    log::info!("Sent to API, received response: {resp:?}...");
    tokio::signal::ctrl_c().await?;
    shutdown_tx.send(())?;
    handle.await?;
    Ok(())
}
