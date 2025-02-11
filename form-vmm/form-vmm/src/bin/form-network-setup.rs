use vmm_service::net_setup::NetworkSetupError;


#[tokio::main] 
async fn main() -> Result<(), NetworkSetupError> {
    simple_logger::init_with_level(log::Level::Info)
        .map_err(|e| NetworkSetupError::Critical(anyhow::anyhow!(e)))?;

    vmm_service::net_setup::configure_bridge_network(true, false).await?;

    Ok(())
}
