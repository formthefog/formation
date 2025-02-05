use std::path::PathBuf;

use alloy_primitives::Address;
use form_state::datastore::{request_full_state, DataStore};
use form_config::OperatorConfig;
use clap::Parser;
use k256::ecdsa::SigningKey;

#[derive(Clone, Debug, Parser)]
pub struct Cli {
    #[clap(long="config-path", short='C', alias="config", default_value_os_t=PathBuf::from("/etc/formation/operator-config.json"))]
    config_path: PathBuf,
    #[clap(long, short, alias="bootstrap")]
    to_dial: Vec<String>,
    #[clap(long, short)]
    secret_key: Option<String>,
    #[clap(long, short, default_value="true")]
    encrypted: bool, 
    #[clap(long, short)]
    password: Option<String>

}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    simple_logger::init_with_level(log::Level::Info)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;

    log::info!("Parsing CLI...");
    let parser = Cli::parse();

    let config = OperatorConfig::from_file(parser.config_path, parser.encrypted, parser.password.as_deref()).ok(); 
    let private_key = if let Some(pk) = &parser.secret_key {
        pk.clone()
    } else {
        config.clone().unwrap().secret_key.ok_or(Box::new(std::io::Error::new(std::io::ErrorKind::Other, "Secret Key required")))?
    };

    log::info!("Acquired private key...");

    let address = hex::encode(Address::from_private_key(&SigningKey::from_slice(&hex::decode(&private_key)?)?)); 
    let mut datastore = if parser.to_dial.is_empty() {
        if config.is_none() {
            let datastore = DataStore::new(address.clone(), private_key.clone());
            Some(datastore)
        } else if config.clone().unwrap().bootstrap_nodes.is_empty() {
            let datastore = DataStore::new(address.clone(), private_key.clone());
            Some(datastore)
        } else { 
            None
        }
    } else {
        None
    };

    if datastore.is_none() {
        if !&parser.to_dial.is_empty() {
            let mut iter = parser.to_dial.iter();
            let mut state = None;
            while let Some(dial) = iter.next() {
                match request_full_state(dial).await {
                    Ok(s) => {
                        state = Some(s);
                        break;
                    } 
                    Err(_) => {}
                }
            };
            let state = state.expect("Unable to acquire state from bootstrap nodes");
            let ds = DataStore::new_from_state(
                address.clone(), private_key.clone(), state
            );
            datastore = Some(ds)
        } else if !config.is_none() && !config.clone().unwrap().bootstrap_nodes.is_empty() {
            let unwrapped_config = config.clone().unwrap();
            let mut iter = unwrapped_config.bootstrap_nodes.iter();
            let mut state = None;
            while let Some(dial) = iter.next() {
                match request_full_state(dial).await {
                    Ok(s) => {
                        state = Some(s);
                        break;
                    } 
                    Err(_) => {}
                }
            };
            let state = state.expect("Unable to acquire staate from bootstrap nodes");
            let ds = DataStore::new_from_state(
                address.clone(), private_key.clone(), state
            );
            datastore = Some(ds)
        } else {
            panic!("Something went terribly wrong trying to instantiate the Datastore");
        }
    };

    log::info!("Built data store, running...");
    
    let (tx, rx) = tokio::sync::broadcast::channel(1024); 

    let handle = tokio::spawn(async move {
        if let Err(e) = datastore.unwrap().run(rx).await {
            eprintln!("Error running datastore: {e}");
        }
    });

    tokio::signal::ctrl_c().await?;
    tx.send(())?;

    handle.await?;

    Ok(())
}
