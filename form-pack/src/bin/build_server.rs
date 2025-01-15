use clap::Parser; 

#[derive(Debug, Parser)]
pub struct Cli {
    #[clap(long, short)]
    port: u16
}

#[tokio::main]
async fn main() {
    let parser = Cli::parse();
}
