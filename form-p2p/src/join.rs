use std::net::SocketAddr;
use form_traits::{Node, NodeInfo};
use form_types::{Event, NetworkEvent, NetworkTopic};

/// Builds a `NetworkEvent::Join` message and publishes it to the Broker.
pub async fn join_network<'a, N, I>(
    node: &'a N,
    bootstrap_nodes: Vec<SocketAddr>,
    forwarded: bool,
) -> Result<(), Box<dyn std::error::Error>> 
    where
        N: Node<Info = I>,
        I: NodeInfo,
        <N as Node>::Error: 'static,
{
    let timestamp = chrono::Utc::now().timestamp();
    let (sig, recovery_id) = node.sign_join(&node.info(), &timestamp)?;
    node.publish(
        Box::new(NetworkTopic), 
        Box::new(
            Event::NetworkEvent(NetworkEvent::Join { 
                node_id: node.id(),
                node_address: node.ip_address(),
                sig: sig.to_string(),
                recovery_id: recovery_id.to_byte() as u32,
                to_dial: bootstrap_nodes,
                forwarded
            })
        )
    ).await?;

    Ok(())
}
