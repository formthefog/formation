use serde::{Serialize, Deserialize};
use std::{fs::File, io::Write, path::{Path, PathBuf}};
use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce
};
use rand::{rngs::OsRng, RngCore};
use crate::Init;

#[derive(Serialize, Deserialize)]
pub struct Config {
    pub config_dir: PathBuf,
    pub data_dir: PathBuf,
    pub keystore_path: PathBuf,
    pub hosts: Vec<String>,
    pub pack_manager_port: u16,
    pub vmm_port: u16,
    pub formnet_port: u16,
    pub join_formnet: bool,
}

impl Config {
    pub fn new(init: &Init) -> Self {
        Config {
            config_dir: init.config_dir.clone().unwrap_or_else(|| {
                let default_config_dir = if cfg!(target_os = "windows") {
                    let appdata = std::env::var("APPDATA")
                        .unwrap_or_else(|_| ".".to_string());
                    PathBuf::from(appdata)
                } else {
                    let home = std::env::var("HOME")
                        .unwrap_or_else(|_| ".".to_string());
                    PathBuf::from(home).join(".config")
                };
                default_config_dir
            }),
            data_dir: init.data_dir.clone().unwrap_or_else(|| {
                let default_data_dir = if cfg!(target_os = "windows") {
                    let localappdata = std::env::var("LOCALAPPDATA")
                        .unwrap_or_else(|_| ".".to_string());
                    PathBuf::from(localappdata)
                } else {
                    let home = std::env::var("HOME")
                        .unwrap_or_else(|_| ".".to_string());
                    PathBuf::from(home).join(".local").join("share")
                };
                default_data_dir
            }),
            keystore_path: init.keystore.clone().unwrap_or_else(|| {
                let default_keystore = if cfg!(target_os = "windows") {
                    let localappdata = std::env::var("APPDATA")
                        .unwrap_or_else(|_| ".".to_string());
                    PathBuf::from(localappdata).join(".keystore").join("form_id")
                } else {
                    let home = std::env::var("HOME")
                        .unwrap_or_else(|_| ".".to_string());
                    PathBuf::from(home).join(".keystore").join("form_id")
                };
                default_keystore
            }),
            hosts: init.hosts.clone().unwrap_or_else(|| vec!["127.0.0.1".to_string()]),
            pack_manager_port: init.pack_manager_port.unwrap_or(3003),
            vmm_port: init.vmm_port.unwrap_or(3002),
            formnet_port: init.formnet_port.unwrap_or(3001),
            join_formnet: init.join_formnet.unwrap_or(true),
        }
    }
}

pub fn encrypt_file(contents: &[u8], password: &str) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    // Generate a random salt
    let mut salt = [0u8; 32];
    OsRng.fill_bytes(&mut salt);

    // Generate a random nonce
    let mut nonce_bytes = [0u8; 12];
    OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    // Derive key from password using Argon2
    let argon2 = argon2::Argon2::default();
    let mut key = [0u8; 32];
    argon2
        .hash_password_into(password.as_bytes(), &salt, &mut key)
        .map_err(|e| format!("Failed to hash password: {}", e))?;

    // Create cipher instance
    let cipher = Aes256Gcm::new_from_slice(&key)?;

    // Encrypt the contents
    let ciphertext = cipher
        .encrypt(nonce, contents)
        .map_err(|e| format!("Encryption failed: {}", e))?;

    // Combine salt + nonce + ciphertext
    let mut encrypted = Vec::new();
    encrypted.extend_from_slice(&salt);
    encrypted.extend_from_slice(&nonce_bytes);
    encrypted.extend(ciphertext);

    Ok(encrypted)
}

pub fn decrypt_file(encrypted: &[u8], password: &str) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    if encrypted.len() < 44 { // 32 + 12 minimum
        return Err("Invalid encrypted data".into());
    }

    // Split the data back into salt, nonce, and ciphertext
    let salt = &encrypted[..32];
    let nonce = Nonce::from_slice(&encrypted[32..44]);
    let ciphertext = &encrypted[44..];

    // Derive the same key from password
    let argon2 = argon2::Argon2::default();
    let mut key = [0u8; 32];
    argon2
        .hash_password_into(password.as_bytes(), salt, &mut key)
        .map_err(|e| format!("Failed to hash password: {}", e))?;

    // Create cipher instance
    let cipher = Aes256Gcm::new_from_slice(&key)?;

    // Decrypt the contents
    let plaintext = cipher
        .decrypt(nonce, ciphertext)
        .map_err(|e| format!("Decryption failed: {}", e))?;

    Ok(plaintext)
}

pub fn save_config(config: &Config, path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let config_str = serde_json::to_string_pretty(config)?;
    let mut file = File::create(path)?;
    file.write_all(config_str.as_bytes())?;
    Ok(())
}

// Function to read configuration (for future use)
pub fn read_config(config_dir: &Path) -> Result<Config, Box<dyn std::error::Error>> {
    let config_path = config_dir.join("config.json");
    let config_str = std::fs::read_to_string(config_path)?;
    let config: Config = serde_json::from_str(&config_str)?;
    Ok(config)
}

pub fn default_config_dir() -> PathBuf {
    PathBuf::from(std::env::var("HOME").unwrap_or(".".to_string())).join(".config").join("form")
}

pub fn default_data_dir() -> PathBuf {
    PathBuf::from(std::env::var("HOME").unwrap_or(".".to_string())).join(".local").join("share").join("form")
}

pub fn default_keystore_dir() -> PathBuf {
    PathBuf::from(std::env::var("HOME").unwrap_or(".".to_string())).join(".keystore")
}
