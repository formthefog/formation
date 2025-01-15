use clap::Parser;
use form_pack::image_builder::serve; 

#[derive(Debug, Parser)]
pub struct Cli {
    #[clap(long, short)]
    port: u16
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let parser = Cli::parse();

    let addr = format!("127.0.0.1:{}", parser.port);

    serve(&addr).await?;

    Ok(())
}
