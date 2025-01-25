use form_state::datastore::{request_full_state, DataStore};
use clap::Parser;

#[derive(Clone, Debug, Parser)]
pub struct Cli {
    #[clap()]
    to_dial: Option<String>,
    #[clap(long, short)]
    address: String,
    #[clap(long, short)]
    private_key: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {

    let parser = Cli::parse();
    let datastore = if let Some(to_dial) = parser.to_dial {
        let state = request_full_state(to_dial).await?;
        let datastore = DataStore::new_from_state(
            parser.address.clone(), parser.private_key.clone(), state
        );
        datastore
    } else {
        let datastore = DataStore::new(parser.address.clone(), parser.private_key.clone());
        datastore
    };

    datastore.run().await?;

    Ok(())
}
