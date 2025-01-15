use form_pack::manager::FormPackManager;
use tokio::sync::broadcast::channel;


#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let manager = FormPackManager::new();
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
