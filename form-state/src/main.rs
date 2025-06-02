use std::path::PathBuf;

use alloy_primitives::Address;
use form_state::datastore::{request_full_state, DataStore};
use form_config::OperatorConfig;
use clap::Parser;
use k256::ecdsa::SigningKey;
use std::sync::Arc;
use tokio::sync::Mutex;
use form_state::api::run;
use std::env;

/// Configuration for admin keys
#[derive(Debug, Clone)]
pub struct AdminConfig {
    /// Whether admin authentication is enabled
    pub enabled: bool,
    /// Set of admin addresses that have global access
    pub admin_addresses: std::collections::HashSet<Address>,
    /// Optional descriptions for audit logging
    pub admin_descriptions: Option<Vec<String>>,
}

impl AdminConfig {
    /// Load admin configuration from environment variables
    pub fn from_environment() -> Result<Self, Box<dyn std::error::Error>> {
        // Check if admin authentication should be enabled
        let enabled = env::var("FORMATION_ADMIN_ENABLED")
            .unwrap_or_else(|_| "true".to_string())
            .parse::<bool>()
            .unwrap_or(true);
        
        if !enabled {
            log::info!("Admin authentication disabled via FORMATION_ADMIN_ENABLED");
            return Ok(AdminConfig {
                enabled: false,
                admin_addresses: std::collections::HashSet::new(),
                admin_descriptions: None,
            });
        }
        
        // Try single admin key first
        let admin_addresses = if let Ok(single_key) = env::var("FORMATION_ADMIN_KEY") {
            vec![single_key]
        } else if let Ok(multiple_keys) = env::var("FORMATION_ADMIN_KEYS") {
            multiple_keys.split(',').map(|s| s.trim().to_string()).collect()
        } else {
            log::info!("No admin keys configured (FORMATION_ADMIN_KEY or FORMATION_ADMIN_KEYS not set)");
            return Ok(AdminConfig {
                enabled: false,
                admin_addresses: std::collections::HashSet::new(),
                admin_descriptions: None,
            });
        };
        
        // Parse addresses
        let mut parsed_addresses = std::collections::HashSet::new();
        for addr_str in admin_addresses {
            let cleaned = addr_str.trim();
            
            // Handle both with and without 0x prefix
            let addr_str = if cleaned.starts_with("0x") {
                cleaned
            } else {
                &format!("0x{}", cleaned)
            };
            
            let address = addr_str.parse::<Address>()
                .map_err(|_| format!("Invalid admin address format: {}", addr_str))?;
                
            if !parsed_addresses.insert(address) {
                return Err(format!("Duplicate admin address: {}", addr_str).into());
            }
        }
        
        // Parse optional descriptions
        let admin_descriptions = env::var("FORMATION_ADMIN_KEY_DESCRIPTIONS")
            .ok()
            .map(|desc_str| {
                desc_str.split(',').map(|s| s.trim().to_string()).collect::<Vec<_>>()
            });
        
        let config = AdminConfig {
            admin_addresses: parsed_addresses,
            admin_descriptions,
            enabled: true,
        };
        
        log::info!("Admin configuration loaded: {} admin keys configured", config.admin_addresses.len());
        
        Ok(config)
    }
    
    /// Create default configuration (no admin keys)
    pub fn default() -> Self {
        AdminConfig {
            admin_addresses: std::collections::HashSet::new(),
            admin_descriptions: None,
            enabled: false,
        }
    }
}

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
    password: Option<String>,
    #[clap(long)]
    jwt_audience: Option<String>,
    #[clap(long)]
    jwt_issuer: Option<String>,
    #[clap(long)]
    jwks_url: Option<String>,
    #[clap(long, default_value="60")]
    jwt_leeway: Option<String>,
    #[clap(long)]
    env_file: Option<PathBuf>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    simple_logger::init_with_level(log::Level::Info)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;

    log::info!("Parsing CLI...");
    let parser = Cli::parse();

    // Load from .env file if specified
    if let Some(env_path) = &parser.env_file {
        log::info!("Loading environment from file: {:?}", env_path);
        match dotenv::from_path(env_path) {
            Ok(_) => log::info!("Successfully loaded environment from file"),
            Err(e) => log::warn!("Failed to load environment from file: {}", e),
        }
    } else {
        // Try to load from default .env file if it exists
        match dotenv::dotenv() {
            Ok(_) => log::info!("Loaded environment from .env file"),
            Err(_) => log::debug!("No .env file found or failed to load it"),
        }
    }

    // Initialize admin configuration from environment
    log::info!("Initializing admin configuration...");
    let admin_config = AdminConfig::from_environment().unwrap_or_else(|e| {
        log::warn!("Failed to load admin config: {}. Using default (no admin keys)", e);
        AdminConfig::default()
    });
    
    if admin_config.enabled {
        log::info!("Admin authentication enabled with {} admin keys", admin_config.admin_addresses.len());
        for (i, addr) in admin_config.admin_addresses.iter().enumerate() {
            let description = admin_config.admin_descriptions
                .as_ref()
                .and_then(|descs| descs.get(i))
                .map(|s| s.as_str())
                .unwrap_or("unnamed");
                
            log::info!("  Admin key {}: 0x{} ({})", i + 1, hex::encode(addr.as_slice()), description);
        }
    } else {
        log::info!("Admin authentication disabled");
    }

    // Configure JWT authentication environment variables
    // CLI arguments take precedence over environment variables
    if let Some(audience) = &parser.jwt_audience {
        log::info!("Setting JWT audience to: {}", audience);
        env::set_var("DYNAMIC_JWT_AUDIENCE", audience);
    }
    
    if let Some(issuer) = &parser.jwt_issuer {
        log::info!("Setting JWT issuer to: {}", issuer);
        env::set_var("DYNAMIC_JWT_ISSUER", issuer);
    }
    
    if let Some(jwks_url) = &parser.jwks_url {
        log::info!("Setting JWKS URL to: {}", jwks_url);
        env::set_var("DYNAMIC_JWKS_URL", jwks_url);
    }
    
    if let Some(leeway) = &parser.jwt_leeway {
        log::info!("Setting JWT leeway to: {}", leeway);
        env::set_var("DYNAMIC_JWT_LEEWAY", leeway);
    }

    // Log the final JWT configuration
    log::info!("JWT Configuration:");
    log::info!("  Audience: {:?}", env::var("DYNAMIC_JWT_AUDIENCE").ok());
    log::info!("  Issuer: {:?}", env::var("DYNAMIC_JWT_ISSUER").ok());
    log::info!("  JWKS URL: {:?}", env::var("DYNAMIC_JWKS_URL").ok());
    log::info!("  Leeway: {:?}", env::var("DYNAMIC_JWT_LEEWAY").ok());

    // Log compilation mode
    #[cfg(feature = "devnet")]
    log::info!("Running in DEVNET mode (queue operations disabled)");
    
    #[cfg(not(feature = "devnet"))]
    log::info!("Running in PRODUCTION mode (queue operations enabled)");
    
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
                log::info!("Attempting to dial {dial}");
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
                log::info!("Attempting to dial {dial}");
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
    
    let (tx, _rx) = tokio::sync::broadcast::channel(1024);
    
    // Always run in full mode, devnet feature controls queue behavior
    let handle = tokio::spawn(async move {
        if let Err(e) = form_state::api::run_api(Arc::new(Mutex::new(datastore.unwrap()))).await {
            eprintln!("Error running datastore: {e}");
        }
    });

    tokio::signal::ctrl_c().await?;
    tx.send(())?;

    handle.await?;

    Ok(())
}
