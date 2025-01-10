use form_types::{Event, NetworkEvent, NetworkTopic};
use form_traits::{Node, NodeInfo, Event as EventTrait};


/// Builds a `NetworkEvent::Heartbeat` and publishes it to the `Broker`.
pub async fn heartbeat<'a, N, I> (
    node: &'a N,
    peers: impl IntoIterator<Item = I> + 'a,
) -> Result<(), Box<dyn std::error::Error>> 
    where 
    N: Node<Info = I> + 'a,
    I: NodeInfo + 'a,
    <N as Node>::Error: 'static 

{
    for peer in peers {
        let now = chrono::Utc::now();
        let timestamp = now.timestamp(); 

        let (sig, recovery_id) = node.sign_heartbeat(&peer, &timestamp)?;
        node.publish(
            Box::new(NetworkTopic),
            Box::new(
                Event::NetworkEvent(NetworkEvent::Heartbeat {
                    node_id: node.info().id().clone(),
                    node_address: node.info().ip_address().clone(),
                    dst: peer.ip_address().clone(),
                    timestamp,
                    sig: sig.to_string(),
                    recovery_id: recovery_id.to_byte() as u32
                })
            ) as Box<dyn EventTrait + Send> 
        ).await?;
    }

    Ok(())
}
