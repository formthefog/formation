use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    simple_logger::init_with_level(log::Level::Info)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;

    log::info!("Logger set up, attempting to get broker endpoints");
    let (frontend, backend) = load_or_get_broker_endpoints(None).await;
    log::info!("Broker endpoints acquired");
    let broker = form_broker::broker::Broker::new(&frontend, &backend).await?;

    broker.start().await?;

    Ok(())
}

async fn load_or_get_broker_endpoints(_config: Option<PathBuf>) -> (String, String) {
    ("127.0.0.1:5555".to_string(), "127.0.0.1:5556".to_string())
}
