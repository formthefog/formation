//! A service to create and run formnet, a wireguard based p2p VPN tunnel, behind the scenes
use clap::Parser;
use formnet::{init::init, serve::serve};
use formnet::{create_router, ensure_crdt_datastore, redeem, JoinRequest, JoinResponse, OperatorJoinRequest, NETWORK_NAME};
use reqwest::Client;
use shared::interface_config::InterfaceConfig;

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
        let invitation = request_to_join(parser.bootstraps.clone(), parser.address).await?;
        ensure_crdt_datastore().await?;
        redeem(invitation)?;
    } else {
        init(parser.address).await?;
    }

    let (shutdown, mut receiver) = tokio::sync::broadcast::channel::<()>(2);
    let mut formnet_receiver = shutdown.subscribe();
    let formnet_server_handle = tokio::spawn(async move {
        tokio::select! {
            res = serve(NETWORK_NAME) => {
                if let Err(e) = res {
                    eprintln!("Error trying to serve formnet server: {e}");
                }
            }
            _ = formnet_receiver.recv() => {
                eprintln!("Formnet Server: Received shutdown signal");
            }
        }
    });

    let join_server_handle = tokio::spawn(async move {
        tokio::select! {
            res = start_join_server() => {
                if let Err(e) = res {
                    eprintln!("Error trying to serve join server: {e}");
                }
            }
            _ = receiver.recv() => {
                eprintln!("Join Server: Received shutdown signal");
            }
        }
    });

    tokio::signal::ctrl_c().await?;
    shutdown.send(())?;

    join_server_handle.await?;
    formnet_server_handle.await?;

    Ok(())
}

async fn request_to_join(bootstrap: Vec<String>, address: String) -> Result<InterfaceConfig, Box<dyn std::error::Error>> {
    let request = JoinRequest::OperatorJoinRequest(
        OperatorJoinRequest {
            operator_id: address,
        }
    );

    while let Some(dial) = bootstrap.iter().next() {
        match Client::new()
        .post(&format!("http://{dial}/join"))
        .json(&request)
        .send()
        .await {
            Ok(response) => match response.json::<JoinResponse>().await {
                Ok(JoinResponse::Success { invitation }) => return Ok(invitation),
                _ => {}
            }
            _ => {}
        }
    }
    return Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, "Did not receive a valid invitation")));
}

async fn start_join_server() -> Result<(), Box<dyn std::error::Error>> {
    let router = create_router();
    let listener = tokio::net::TcpListener::bind(
        "0.0.0.0:3001"
    ).await?;

    axum::serve(listener, router).await?;

    return Ok(())
}
