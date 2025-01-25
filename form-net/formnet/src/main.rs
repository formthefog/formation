//! A service to create and run innernet behind the scenes
use clap::Parser;
use formnet::{init::init, serve::serve};
use formnet::NETWORK_NAME;

#[derive(Clone, Debug, Parser)]
struct Opts {
    /// 1 or more bootstrap nodes that are known
    /// and already active in the Network
    /// Will eventually be replaced with a discovery service
    #[arg(short, long, alias="bootstrap")]
    bootstraps: Vec<String>,
    /// A 20 byte hex string that represents an ethereum address
    #[arg(short, long, alias="name")]
    address: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    simple_logger::SimpleLogger::new().init().unwrap();

    let parser = Opts::parse();
    log::info!("{parser:?}");

    if !parser.bootstraps.is_empty() {

    } else {
        init(parser.address).await?;
        serve(NETWORK_NAME).await?;
    }

    Ok(())
}

/*
async fn run<D: DatastoreType + FormnetNode>(
    mut subscriber: impl SubStream<Message = Vec<FormnetMessage>>,
    mut shutdown: Receiver<()>
) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    log::info!("Starting main formnet handler loop");
    loop {
        tokio::select! {
            Ok(msg) = subscriber.receive() => {
                for m in msg {
                    if let Err(e) = handle_message::<D>(&m).await {
                        log::error!("Error handling message {m:?}: {e}");
                    }
                }
            }
            _ = tokio::time::sleep(Duration::from_secs(30)) => {
                log::info!("Heartbeat...");
            }
            _ = shutdown.recv() => {
                log::error!("Received shutdown signal for Formnet");
                break;
            }
        }
    }

    Ok(())
}

async fn handle_message<D: DatastoreType + FormnetNode>(
    message: &FormnetMessage
) -> Result<(), Box<dyn std::error::Error>> {
    use form_types::FormnetMessage::*;
    log::info!("Received message: {message:?}");
    match message {
        AddPeer { peer_type, peer_id, callback } => {
            if is_server() {
                log::info!("Receiving node is Server, adding peer from server...");
                let server_config = ServerConfig { 
                    config_dir: PathBuf::from(SERVER_CONFIG_DIR), 
                    data_dir: PathBuf::from(SERVER_DATA_DIR)
                };
                log::info!("Built Server Config...");
                let inet = InterfaceName::from_str(FormnetMessage::INTERFACE_NAME)?;
                log::info!("Acquired interface name...");
                if let Ok(invitation) = server_add_peer::<D>(
                    &inet,
                    &server_config,
                    &peer_type.into(),
                    peer_id,
                ).await {
                    return server_respond_with_peer_invitation(
                        invitation,
                        *callback
                    ).await;
                }
            }

            let InterfaceConfig { server, ..} = InterfaceConfig::from_interface(
                PathBuf::from(CONFIG_DIR).as_path(), 
                &InterfaceName::from_str(
                    FormnetMessage::INTERFACE_NAME
                )?
            )?;
            let api = Api::new(&server);
            log::info!("Fetching CIDRs...");
            let cidrs: Vec<Cidr> = api.http("GET", "/admin/cidrs")?;
            log::info!("Fetching Peers...");
            let peers: Vec<Peer> = api.http("GET", "/admin/peers")?;
            log::info!("Creating CIDR Tree...");
            let cidr_tree = CidrTree::new(&cidrs[..]);

            if let Ok((content, keypair)) = add_peer(
                &peers, &cidr_tree, &peer_type.into(), peer_id
            ).await {
                log::info!("Creating peer...");
                let peer: Peer = api.http_form("POST", "/admin/peers", content)?;
                respond_with_peer_invitation(
                    &peer,
                    server.clone(), 
                    &cidr_tree, 
                    keypair, 
                    *callback
                ).await?;
            }
        },
        DisablePeer => {},
        EnablePeer => {},
        SetListenPort => {},
        OverrideEndpoint => {},
    }
}
*/

// Create formnet from CLI, Config or Wizard 
// If done via wizard save to file
// Listen for messages on topic "Network" from broker
// Handle messages
//
// Formnet service can:
//  1. Add peers
//  2. Remove peers
//  3. Add CIDRs
//  4. Remove CIDRs
//  5. Rename Peers
//  6. Rename CIDRs
//  7. Enable Peers
//  8. Disable Peers
//  9. Manage Associations
//  10. Manage Endpoints
//
// When a new peer joins the network, a join token will be sent to them
// which they will then "install" via their formnet network service.
//
// In the formnet there are 3 types of peers:
//  1. Operators - All operators are admins and can add CIDRs, Peers, Associations, etc.
//                 All operators run a "server" replica.
//
//  2. Users - Users run a simple client, they are added as a peer, and in future version
//             will have more strictly managed associations to ensure they only have
//             access to the resources they own. In the first version, they have access
//             to the entire network, but instances and resources use internal auth mechanisms
//             such as public/private key auth to provide security.
//
//  3. Instances - Instances are user owned resources, such as Virtual Machines, containers,
//                 etc. Instances are only manageable by their owner. Once they are up and
//                 running the rest of the network just knows they are there. Operators that
//                 are responsible for a given instance can be financially penalized for not
//                 maintaining the instance in the correct state/status.
// 

// So what do we need this to do
// 1. Listen on `topic` for relevant messages from the MessageBroker
// 2. When a message is received, match that message on an action
// 3. Handle the action (by using the API).
