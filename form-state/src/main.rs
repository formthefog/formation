use ditto::Map;
use form_state::{datastore::{request_associations_state, request_cidr_state, request_peer_state, request_site_id, DataStore}, network::{DnsState, NetworkState}};
use clap::Parser;

#[derive(Clone, Debug, Parser)]
pub struct Cli {
    #[clap()]
    to_dial: Option<String>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {

    let parser = Cli::parse();
    let datastore = if let Some(to_dial) = parser.to_dial {
        let site_id = request_site_id(to_dial.clone()).await?;
        let peer_state = request_peer_state(to_dial.clone()).await?;
        let cidr_state = request_cidr_state(to_dial.clone()).await?;
        let assoc_state = request_associations_state(to_dial.clone()).await?;
        let network_state = NetworkState {
            peers: Map::from_state(peer_state, Some(site_id)).map_err(|e| {
                Box::new(
                    std::io::Error::new(
                        std::io::ErrorKind::Other,
                        format!("Error trying to create Datastore: {e:?}")
                    )
                )
            })?,
            cidrs: Map::from_state(cidr_state, Some(site_id)).map_err(|e| {
                Box::new(
                    std::io::Error::new(
                        std::io::ErrorKind::Other,
                        format!("Error trying to create Datastore: {e:?}")
                    )
                )
            })?,
            associations: Map::from_state(assoc_state, Some(site_id)).map_err(|e| {
                Box::new(
                    std::io::Error::new(
                        std::io::ErrorKind::Other,
                        format!("Error trying to create Datastore: {e:?}")
                    )
                )
            })?,
            dns_state: DnsState::new(),
        };
        DataStore::new_from_state(network_state)
    } else {
        DataStore::new(Some(1)).map_err(|e| {
            Box::new(
                std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("Error trying to create Datastore: {e:?}")
                )
            )
        })?
    };

    datastore.run().await?;
    
    Ok(())
}
