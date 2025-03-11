use std::time::Duration;

use form_types::{BootCompleteRequest, VmmResponse};
use formnet::{request_to_join, up};
use reqwest::Client;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    simple_logger::SimpleLogger::new().init().unwrap();
    let host_public_ip = reqwest::blocking::get(
        "https://api.ipify.org"
    )?.text()?;
    // Get name
    let name = std::fs::read_to_string("/etc/vm_name")?;
    let build_id = std::fs::read_to_string("/etc/build_id")?;
    match request_to_join(vec![host_public_ip.clone()], name.clone(), form_types::PeerType::Instance, None, None, None).await {
        Ok(ip)=> {
            log::info!("Received invitation");
            let formnet_ip = ip; 
            log::info!("extracted formnet IP for {name}: {formnet_ip}");
            log::info!("Attempting to redeem invite");
            log::info!("Spawning thread to bring formnet up");
            let handle = tokio::spawn(async move {
                if let Err(e) = up(
                    Some(Duration::from_secs(60)),
                    None,
                ).await {
                    log::error!("Error bringing formnet up: {e}");
                }
            });

            log::info!("Building request to inform VMM service that the boot process has completed for {name}");
            // Send message to VMM api.
            let request = BootCompleteRequest {
                name: name.clone(),
                build_id,
                formnet_ip: formnet_ip.to_string()
            };

            log::info!("Sending BootCompleteRequest {request:?} to http://{host_public_ip}:3002/boot_complete endpoint");

            let resp = Client::new().post(&format!("http://{host_public_ip}:3002/boot_complete"))
                .json(&request)
                .send()
                .await?
                .json::<VmmResponse>().await;

            log::info!("BootCompleteRequest Response: {resp:?}");

            log::info!("Formnet up, awaiting kill signal");
            handle.await?;

            Ok(())
        },
        Err(reason) => {
            log::info!("Error trying to join formnet: {reason}");
            return Err(other_err(&reason.to_string()))
        }
    }
}

pub fn other_err(msg: &str) -> Box<dyn std::error::Error> {
    Box::new(
        std::io::Error::new(
            std::io::ErrorKind::Other,
            msg
        )
    )
}
