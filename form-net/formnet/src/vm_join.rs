use std::time::Duration;

use form_types::{BootCompleteRequest, VmmResponse};
use formnet::{redeem, up, JoinRequest, JoinResponse, VmJoinRequest};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    simple_logger::SimpleLogger::new().init().unwrap();
    let host_public_ip = reqwest::blocking::get(
        "https://api.ipify.org"
    )?.text()?;
    // Get name
    let name = std::fs::read_to_string("/etc/vm_name")?;
    let build_id = std::fs::read_to_string("/etc/build_id")?;
    log::info!("Requesting formnet invite for vm {}", name);
    log::info!("Building VmJoinRequest");
    let join_request = VmJoinRequest { vm_id: name.clone() };
    log::info!("Wrapping VmJoinRequest in a JoinRequest");
    let join_request = JoinRequest::InstanceJoinRequest(join_request);
    log::info!("Getting a new client");
    let client = reqwest::Client::new();
    log::info!("Posting request to endpoint using client, awaiting response...");
    // We should be able to access formnet, and the VMM over the bridge gateway
    let resp = client.post(&format!("http://{host_public_ip}:3001/join"))
        .json(&join_request)
        .send().await.map_err(|e| {
            other_err(&e.to_string())
        })?.json::<JoinResponse>().await.map_err(|e| {
            other_err(&e.to_string())
        })?;

    log::info!("Response text: {resp:?}");

    match resp {
        JoinResponse::Success { invitation } => {
            log::info!("Received invitation");
            let invite = invitation;
            let formnet_ip = invite.interface.address.addr().to_string();
            log::info!("extracted formnet IP for {name}");
            log::info!("Attempting to redeem invite");
            if let Err(e) = redeem(invite).map_err(|e| {
                other_err(&e.to_string())
            }) {
                log::error!("Error attempting to redeem invite: {e}");
            }

            log::info!("Successfully redeemed invite");

            log::info!("Spawning thread to bring formnet up");
            let handle = tokio::spawn(async move {
                if let Err(e) = up(
                    Some(Duration::from_secs(60)),
                    None,
                ) {
                    log::error!("Error bringing formnet up: {e}");
                }
            });

            log::info!("Building request to inform VMM service that the boot process has completed for {name}");
            // Send message to VMM api.
            let request = BootCompleteRequest {
                name: name.clone(),
                build_id,
                formnet_ip
            };
            log::info!("Sending BootCompleteRequest {request:?} to http://{host_public_ip}:3002/{name}/boot_complete endpoint");
            let resp = client.post(&format!("http://{host_public_ip}:3002/{}/boot_complete", name))
                .json(&request)
                .send()
                .await?
                .json::<VmmResponse>().await;

            log::info!("BootCompleteRequest Response: {resp:?}");

            log::info!("Formnet up, awaiting kill signal");
            handle.await?;

            Ok(())
        },
        JoinResponse::Error(reason) => return Err(other_err(&reason.to_string()))
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
