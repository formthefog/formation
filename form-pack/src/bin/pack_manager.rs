use form_pack::manager::FormPackManager;
use tokio::sync::broadcast::channel;


#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = "0.0.0.0:51520".parse()?;
    let manager = FormPackManager::new(addr);
    let (tx, rx) = channel(1);
    tokio::task::spawn(async move {
        if let Err(e) = manager.run(rx).await {
            eprintln!("Error running FormPackManager: {e}");
        };
    });

    tokio::signal::ctrl_c().await?;

    tx.send(())?;

    Ok(())
}
