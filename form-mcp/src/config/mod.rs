// Configuration module for the MCP server
//
// This module handles loading and managing configuration settings
// for the MCP server.

mod settings;

pub use settings::Settings;

use std::path::Path;
use std::sync::Arc;
use crate::errors::ServerError;

/// Load configuration from a file
pub fn load_config(path: Option<&str>) -> Result<Arc<Settings>, ServerError> {
    let config_path = path.unwrap_or("config/default.toml");
    settings::load(config_path)
        .map(Arc::new)
        .map_err(|e| ServerError::Config(format!("Failed to load config: {}", e)))
}

/// Save configuration to a file
pub fn save_config(config: &Settings, path: &str) -> Result<(), ServerError> {
    settings::save(config, path)
        .map_err(|e| ServerError::Config(format!("Failed to save config: {}", e)))
} 