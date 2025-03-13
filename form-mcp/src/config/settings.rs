// Settings module for the MCP server
//
// This module contains the settings and configuration for the MCP server.

use std::path::Path;
use anyhow::Result;
use serde::{Serialize, Deserialize};
use crate::defaults;

/// Server configuration settings
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
            host: defaults::SERVER_HOST.to_string(),
            port: defaults::SERVER_PORT,
            workers: defaults::WORKERS,
            cors_enabled: true,
            cors_origins: vec!["http://localhost:3000".to_string()],
            request_timeout: defaults::REQUEST_TIMEOUT_SECS,
        }
    }
}

/// Authentication configuration settings
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
            token_expiration: 3600,
        }
    }
}

/// Database configuration settings
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
            connection_string: "postgres://postgres:postgres@localhost:5432/formation".to_string(),
            max_connections: 5,
        }
    }
}

/// Main settings structure
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
    // If the file exists, load it
    if Path::new(path.as_ref()).exists() {
        let content = std::fs::read_to_string(path)?;
        let settings: Settings = toml::from_str(&content)?;
        return Ok(settings);
    }
    
    // Otherwise, return default settings
    Ok(Settings::default())
}

/// Save settings to a file
pub fn save(settings: &Settings, path: impl AsRef<Path>) -> Result<()> {
    let content = toml::to_string(settings)?;
    std::fs::write(path, content)?;
    Ok(())
}

/// Generate a random secret for JWT
fn generate_random_secret() -> String {
    use rand::{thread_rng, Rng};
    const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ\
                            abcdefghijklmnopqrstuvwxyz\
                            0123456789";
    let mut rng = thread_rng();
    let secret: String = (0..64)
        .map(|_| {
            let idx = rng.gen_range(0..CHARSET.len());
            CHARSET[idx] as char
        })
        .collect();
    
    secret
} 