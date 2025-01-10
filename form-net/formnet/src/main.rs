//! A service to create and run innernet behind the scenes
use clap::Parser;
use innernet_server::initialize::InitializeOpts;
use reqwest::Client;
use std::path::PathBuf;
use std::str::FromStr;
use std::time::Duration;
use innernet_server::{serve, uninstall, ServerConfig};
use shared::interface_config::InterfaceConfig;
use shared::{Cidr, CidrTree, NetworkOpts, Peer};
use tokio::{net::TcpListener, sync::broadcast::Receiver};
use wireguard_control::{Backend, InterfaceName};
use client::util::Api;
use conductor::subscriber::SubStream;
use form_types::{FormnetMessage, FormnetTopic};
use alloy::signers::k256::ecdsa::{SigningKey, VerifyingKey};
use rand_core::OsRng;
use formnet::*;

#[derive(Debug, Parser)]
struct Opts {
    #[arg(short, long, alias="bootstrap")]
    dial: Option<String>,
    #[arg(short, long)]
    public_key: Option<String>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {

    simple_logger::SimpleLogger::new().init().unwrap();

    let parser = Opts::parse();
    log::info!("{parser:?}");

    let interface_handle = if let Some(to_dial) = parser.dial {
        // A bootstrap node was provided, request that the 
        // new operator (local) be added to the network
        // as a peer.
        let client = Client::new();
        let public_key = if let Some(ref pk) = parser.public_key {
            pk.clone()
        } else {
            let signing_key = SigningKey::random(&mut OsRng);
            let verifying_key = VerifyingKey::from(signing_key);
            hex::encode(&verifying_key.to_encoded_point(false).to_bytes())
        };
        
        log::info!("PublicKey: {public_key}");
        //TODO: issue challenge/response
        let response = client.post(format!("http://{to_dial}:3001/join"))
            .json(
                &JoinRequest::OperatorJoinRequest(
                    OperatorJoinRequest { 
                        operator_id: public_key 
                    }
                )
            ).send().await?;

        log::info!("{:?}", response);

        let status = response.status().clone();

        // Let's print out the error response body if we get a non-success status
        if !response.status().is_success() {
            let error_body = response.text().await?.clone();
            log::info!("Error response body: {}", error_body);
            // Now fail the test
            panic!("Request failed with status {} and error: {}", status, error_body);
        }

        assert!(response.status().is_success());
        let join_response = response.json::<JoinResponse>().await?;
        log::info!("{}", serde_json::to_string_pretty(&join_response)?);
        let handle = tokio::spawn(async move {
            match join_response {
                JoinResponse::Success { invitation } => {
                    let network_opts = NetworkOpts {
                        backend: Backend::Kernel,
                    mtu: None,
                    no_routing: false,
                    };
                    let interface_name = InterfaceName::from_str("formnet")?;
                    let target_conf = PathBuf::from("/etc/formnet").join(interface_name.to_string()).with_extension("conf");
                    redeem_invite(&interface_name, invitation, target_conf, network_opts)?;
                    return Ok(())
                }
                JoinResponse::Error(e) => {
                    return Err(
                        Box::new(
                            std::io::Error::new(
                                std::io::ErrorKind::Other,
                                e
                            )
                        ) as Box<dyn std::error::Error + Send + Sync + 'static>
                    )
                }
            }
        });

        handle
    } else {
        // No bootstrap was provided create and serve formnet as server
        log::info!("Attempting to start formnet server, no bootstrap provided...");
        let handle = tokio::spawn(async move {
            log::info!("Building server config...");
            let conf = ServerConfig { config_dir: SERVER_CONFIG_DIR.into(), data_dir: SERVER_DATA_DIR.into() };
            log::info!("Building init options...");
            let init_opts = InitializeOpts::default(); 
            log::info!("Acquiring interface name...");
            let interface_name = InterfaceName::from_str("formnet")?;
            log::info!("Building network opts...");
            let network_opts = NetworkOpts {
                backend: Backend::Kernel,
                mtu: None,
                no_routing: false,
            };

            log::info!("Initializing the server");
            init(&conf, init_opts)?;
            log::info!("Serving the server...");
            serve(interface_name, &conf, network_opts).await?;
            log::info!("Server shutdown, cleaning up...");
            uninstall(&interface_name, &conf, network_opts, true)?;

            Ok::<(), Box<dyn std::error::Error + Send + Sync + 'static>>(())
        });

        handle
    };

    let (tx, rx) = tokio::sync::broadcast::channel(3);
    let _api_shutdown = tx.subscribe();
    
    log::info!("Spawning API Server for taking join requests...");
    let api_handle = tokio::spawn(async move {
        let api = create_router();
        let listener = TcpListener::bind("0.0.0.0:3001").await?;

        let _ = axum::serve(
            listener,
            api
        ).await?;

        Ok::<(), Box<dyn std::error::Error + Send + Sync + 'static>>(())
    });

    log::info!("Spawning subscriber to broker...");
    let handle = tokio::spawn(async move {
        let sub = FormnetSubscriber::new(
            "127.0.0.1:5556",
            vec![
                FormnetTopic.to_string()
            ]
        ).await?;
        log::info!("Created formnet subscriber...");
        if let Err(e) = run(
            sub,
            rx
        ).await {
            log::error!("Error running formnet handler: {e}");
        }

        Ok::<(), Box<dyn std::error::Error + Send + Sync + 'static>>(())
    });

    tokio::signal::ctrl_c().await?;

    if let Err(e) = tx.send(()) {
        log::info!("Error sending shutdown signal: {e}");
    }
    let _ = handle.await?;
    let _ = api_handle.await?;
    let _ = interface_handle.await?;

    Ok(())
}

async fn run(
    mut subscriber: impl SubStream<Message = Vec<FormnetMessage>>,
    mut shutdown: Receiver<()>
) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    log::info!("Starting main formnet handler loop");
    loop {
        tokio::select! {
            Ok(msg) = subscriber.receive() => {
                for m in msg {
                    if let Err(e) = handle_message(&m).await {
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

async fn handle_message(
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
                if let Ok(invitation) = server_add_peer(
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
    Ok(())
}

// Create innernet from CLI, Config or Wizard 
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
