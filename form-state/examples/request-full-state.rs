use form_state::datastore::{DataStore, MergeableState};
use reqwest::Client;


#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let resp = request_full_state("170.250.22.3").await?;
    println!("{:?}", resp);
    Ok(())
}

pub async fn request_full_state(to_dial: &str) -> Result<MergeableState, Box<dyn std::error::Error>> {
    let resp = Client::new()
        .get(format!("http://{to_dial}:3004/bootstrap/full_state"))
        .send().await?.json().await?;


    Ok(resp)
}
