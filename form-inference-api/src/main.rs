mod routes;

use axum::{Router, Server};
use std::net::SocketAddr;
use std::path::PathBuf;
use form_config::OperatorConfig;
use clap::Parser;

/// Command-line arguments
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path to the operator config JSON file
    #[clap(long)]
    config: PathBuf,
}

fn load_operator_config(config_path: PathBuf) -> Result<OperatorConfig, Box<dyn std::error::Error>> {
    if config_path.exists() {
        Ok(OperatorConfig::from_file(config_path, false, None)?)
    } else {
        Err(format!("Operator config file not found: {}", config_path.display()).into())
    }
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    // Load operator configuration
    let config = match load_operator_config(args.config) {
        Ok(config) => config,
        Err(e) => {
            eprintln!("Failed to load operator configuration: {}", e);
            std::process::exit(1);
        }
    };

    // Build our application with routes
    let app = Router::new()
        .merge(routes::api_router());

    // Run the app on the configured port
    let addr = SocketAddr::from(([127, 0, 0, 1], config.form_inference_api_port));
    println!("Form Inference API listening on {}", addr);

    Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}
