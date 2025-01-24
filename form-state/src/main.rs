use form_state::datastore::DataStore;
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
    let datastore = DataStore::new(parser.address.clone(), parser.private_key.clone());

    // datastore.run().await?;
    Ok(())
}
