mod routes;
mod auth;

use axum::Router;
use std::net::SocketAddr;
use std::path::PathBuf;
use form_config::OperatorConfig;
use clap::Parser;
use auth::auth_middleware;
use tokio::net::TcpListener;
use std::sync::Arc;

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
            eprintln!("Failed to load operator configuration: {e}");
            std::process::exit(1);
        }
    };

    // Extract the secret key for route handlers
    let secret_key = match config.secret_key {
        Some(key) => key,
        None => {
            eprintln!("No secret key found in operator configuration");
            std::process::exit(1);
        }
    };

    // Build our application with routes and state
    let app = Router::new()
        .merge(routes::api_router())
        .with_state(Arc::new(secret_key));

    // Run the app on the configured port
    let addr = SocketAddr::from(([0, 0, 0, 0], config.form_inference_api_port));
    println!("Form Inference API listening on 0.0.0.0:{}", config.form_inference_api_port);

    let listener = TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
