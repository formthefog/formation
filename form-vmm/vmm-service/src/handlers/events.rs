use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use tokio::io::AsyncReadExt;
use crate::VmmError;
use form_types::{FormnetMessage, FormnetTopic, GenericPublisher, PeerType};
use shared::interface_config::InterfaceConfig;
use tokio::net::TcpListener;
use conductor::publisher::PubStream;

#[allow(unused)]
async fn request_formnet_invite_for_vm(name: String) -> Result<InterfaceConfig, VmmError> {
    // Request a innernet invitation from local innernet peer
    let mut publisher = GenericPublisher::new("127.0.0.1:5555").await.map_err(|e| {
        VmmError::NetworkError(format!("Unable to publish message to setup networking: {e}"))
    })?;

    let callback = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 5855);

    let listener = TcpListener::bind(callback.clone()).await.map_err(|e| {
        VmmError::NetworkError(
            format!("Unable to bind listener to callback socket to receive formnet invite: {e}")
        )
    })?;

    publisher.publish(
        Box::new(FormnetTopic),
        Box::new(FormnetMessage::AddPeer { 
            peer_id: name.clone(),
            peer_type: PeerType::Instance,
            callback
        })
    ).await.map_err(|e| {
        VmmError::NetworkError(
            format!("Error sending message to broker to request formnet invite: {e}")
        )
    })?;

    tokio::select! {
        Ok((mut stream, _)) = listener.accept() => {
            let mut buf: Vec<u8> = vec![];
            if let Ok(n) = stream.read_to_end(&mut buf).await {
                let invite: shared::interface_config::InterfaceConfig = serde_json::from_slice(&buf[..n]).map_err(|e| {
                    VmmError::NetworkError(
                        format!("Error converting response into InterfaceConfig: {e}")
                    )
                })?;
                return Ok(invite);
            }

            return Err(VmmError::NetworkError(format!("Unable to read response on TcpStream: Error awaiting response to formnet invite request")));
        }
        _ = tokio::time::sleep(std::time::Duration::from_secs(10)) => {
            log::error!("Timed out awaiting invitation response from formnet");
            return Err(VmmError::NetworkError(format!("Timed out awaiting invite from formnet for VM {}", name)));
        }
    }
}
