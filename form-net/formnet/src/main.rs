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
