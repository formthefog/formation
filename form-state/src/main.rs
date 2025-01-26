use std::path::PathBuf;

use alloy_primitives::Address;
use form_state::datastore::{request_full_state, DataStore};
use form_config::OperatorConfig;
use clap::Parser;
use k256::ecdsa::SigningKey;

#[derive(Clone, Debug, Parser)]
pub struct Cli {
    #[clap(alias="config", default_value_os_t=PathBuf::from("/etc/formation/operator-config.json"))]
    config_path: PathBuf,
    #[clap(alias="bootstrap")]
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
    let parser = Cli::parse();

    let private_key = if let Some(pk) = &parser.secret_key {
        pk.clone()
    } else {
        let config = OperatorConfig::from_file(parser.config_path, parser.encrypted, parser.password.as_deref())?; 
        config.secret_key.ok_or(Box::new(std::io::Error::new(std::io::ErrorKind::Other, "Secret Key required")))?
    };

    let address = hex::encode(Address::from_private_key(&SigningKey::from_slice(&hex::decode(&private_key)?)?)); 

    let datastore = if !&parser.to_dial.is_empty() {
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
        let state = state.ok_or(Box::new(std::io::Error::new(std::io::ErrorKind::Other, "Unable to acquire state from any bootstrap nodes provided")))?;
        let datastore = DataStore::new_from_state(
            address.clone(), private_key.clone(), state
        );
        datastore
    } else {
        let datastore = DataStore::new(address.clone(), private_key.clone());
        datastore
    };

    datastore.run().await?;

    Ok(())
}
