// Settings module for configuration
//
// This module defines the settings structure and loading/saving functions
// for the MCP server configuration.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use anyhow::Result;

/// Server settings for the MCP server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerSettings {
    /// Host address to bind to
    pub host: String,
    /// Port to listen on
    pub port: u16,
    /// Number of worker threads
    pub workers: usize,
    /// Enable CORS
    pub cors_enabled: bool,
    /// CORS allowed origins
    pub cors_origins: Vec<String>,
    /// Request timeout in seconds
    pub request_timeout: u64,
}

impl Default for ServerSettings {
    fn default() -> Self {
        Self {
            host: crate::defaults::SERVER_HOST.to_string(),
            port: crate::defaults::SERVER_PORT,
            workers: num_cpus::get(),
            cors_enabled: false,
            cors_origins: vec!["*".to_string()],
            request_timeout: crate::defaults::REQUEST_TIMEOUT_SECS,
        }
    }
}

/// Authentication settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthSettings {
    /// Enable authentication
    pub enabled: bool,
    /// JWT secret for token generation/validation
    pub jwt_secret: String,
    /// Token expiration time in seconds
    pub token_expiration: u64,
}

impl Default for AuthSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            jwt_secret: generate_random_secret(),
            token_expiration: 86400, // 24 hours
        }
    }
}

/// Database settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseSettings {
    /// Database type (e.g., postgres, sqlite)
    pub db_type: String,
    /// Connection string
    pub connection_string: String,
    /// Maximum connections
    pub max_connections: u32,
}

impl Default for DatabaseSettings {
    fn default() -> Self {
        Self {
            db_type: "postgres".to_string(),
            connection_string: "postgres://postgres:postgres@localhost:5432/form_mcp".to_string(),
            max_connections: 5,
        }
    }
}

/// Complete settings for the MCP server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    /// Environment (development, staging, production)
    pub environment: String,
    /// Server settings
    pub server: ServerSettings,
    /// Authentication settings
    pub auth: AuthSettings,
    /// Database settings
    pub database: DatabaseSettings,
    /// Log level
    pub log_level: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            environment: "development".to_string(),
            server: ServerSettings::default(),
            auth: AuthSettings::default(),
            database: DatabaseSettings::default(),
            log_level: "info".to_string(),
        }
    }
}

/// Load settings from a file
pub fn load(path: impl AsRef<Path>) -> Result<Settings> {
    let config_str = match fs::read_to_string(&path) {
        Ok(config_str) => config_str,
        Err(_) => {
            // If the file doesn't exist, create default settings
            let default_settings = Settings::default();
            save(&default_settings, path)?;
            return Ok(default_settings);
        }
    };
    
    let settings: Settings = toml::from_str(&config_str)?;
    Ok(settings)
}

/// Save settings to a file
pub fn save(settings: &Settings, path: impl AsRef<Path>) -> Result<()> {
    let config_str = toml::to_string_pretty(settings)?;
    
    // Create parent directories if they don't exist
    if let Some(parent) = path.as_ref().parent() {
        fs::create_dir_all(parent)?;
    }
    
    fs::write(path, config_str)?;
    Ok(())
}

/// Generate a random secret for JWT
fn generate_random_secret() -> String {
    use rand::{thread_rng, Rng};
    const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
    let mut rng = thread_rng();
    let secret: String = (0..32)
        .map(|_| {
            let idx = rng.gen_range(0..CHARSET.len());
            CHARSET[idx] as char
        })
        .collect();
    secret
} 