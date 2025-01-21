use serde::{Serialize, Deserialize};
use std::{fs::File, io::Write, path::{Path, PathBuf}};
use dialoguer::{Input, theme::ColorfulTheme};
use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce
};
use rand::{rngs::OsRng, RngCore};
use sha2::{Sha256, Digest};
use sha3::Keccak256;
use uuid::Uuid;
use crate::Init;

#[derive(Serialize, Deserialize)]
pub struct Config {
    pub config_dir: PathBuf,
    pub data_dir: PathBuf,
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
            hosts: init.hosts.clone().unwrap_or_else(|| vec!["127.0.0.1".to_string()]),
            pack_manager_port: init.pack_manager_port.unwrap_or(3003),
            vmm_port: init.vmm_port.unwrap_or(3002),
            formnet_port: init.formnet_port.unwrap_or(3001),
            join_formnet: init.join_formnet.unwrap_or(true),
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct KeystoreEntry {
    address: String,
    crypto: CryptoData,
    id: String,
    version: u32,
}

#[derive(Serialize, Deserialize)]
struct CryptoData {
    cipher: String,
    cipherparams: CipherParams,
    ciphertext: String,
    kdf: String,
    kdfparams: KdfParams,
    mac: String,
}

#[derive(Serialize, Deserialize)]
struct CipherParams {
    iv: String,
}

#[derive(Serialize, Deserialize)]
struct KdfParams {
    dklen: u32,
    n: u32,
    p: u32,
    r: u32,
    salt: String,
}


pub fn save_to_keystore(signing_key: &str, keystore_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    // Get password from user
    let password: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("Enter password to encrypt keystore")
        .interact()?;

    // Generate random salt and IV
    let mut salt = [0u8; 32];
    let mut iv = [0u8; 12];
    OsRng.fill_bytes(&mut salt);
    OsRng.fill_bytes(&mut iv);

    // Scrypt parameters
    let scrypt_params = KdfParams {
        dklen: 32,
        n: 262144, // 2^18
        p: 1,
        r: 8,
        salt: hex::encode(salt),
    };

    // Derive key using scrypt
    let mut derived_key = [0u8; 32];
    scrypt::scrypt(
        password.as_bytes(),
        &salt,
        &scrypt::Params::new(
            scrypt_params.n.trailing_zeros() as u8,
            scrypt_params.r,
            scrypt_params.p,
            scrypt_params.dklen.try_into()?,
        )?,
        &mut derived_key,
    )?;

    // Encrypt the private key using AES-256-GCM
    let cipher = Aes256Gcm::new_from_slice(&derived_key)?;
    let nonce = Nonce::from_slice(&iv);
    let signing_key_bytes = hex::decode(signing_key)?;
    let ciphertext = cipher
        .encrypt(nonce, signing_key_bytes.as_ref())
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;

    // Calculate MAC
    let mut mac_data = Vec::new();
    mac_data.extend_from_slice(&derived_key[16..32]);
    mac_data.extend_from_slice(&ciphertext);
    let mac = hex::encode(Sha256::digest(&mac_data));

    // Generate Ethereum-style address from public key
    let signing_key_bytes = hex::decode(signing_key)?;
    let public_key = k256::ecdsa::SigningKey::from_slice(&signing_key_bytes)?
        .verifying_key()
        .to_encoded_point(false)
        .to_bytes();
    let address = hex::encode(&Keccak256::digest(&public_key[1..])[12..]);

    // Create keystore entry
    let entry = KeystoreEntry {
        address: format!("0x{}", address),
        crypto: CryptoData {
            cipher: "aes-256-gcm".to_string(),
            cipherparams: CipherParams {
                iv: hex::encode(iv),
            },
            ciphertext: hex::encode(ciphertext),
            kdf: "scrypt".to_string(),
            kdfparams: scrypt_params,
            mac,
        },
        id: Uuid::new_v4().to_string(),
        version: 3,
    };

    // Save to file
    let keystore_file = keystore_path.join(format!("UTC--{}--{}", 
        chrono::Utc::now().format("%Y-%m-%dT%H-%M-%S.%fZ"),
        address));
    let entry_str = serde_json::to_string_pretty(&entry)?;
    let mut file = File::create(keystore_file)?;
    file.write_all(entry_str.as_bytes())?;

    Ok(())
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

// Function to read from keystore (for future use)
pub fn read_from_keystore(keystore_path: &Path, address: &str) -> Result<KeystoreEntry, Box<dyn std::error::Error>> {
    let keystore_file = keystore_path.join(format!("key-{}.json", address));
    let entry_str = std::fs::read_to_string(keystore_file)?;
    let entry: KeystoreEntry = serde_json::from_str(&entry_str)?;
    Ok(entry)
}
