use std::path::{Path, PathBuf};
use aes_gcm::{aead::Aead, Aes256Gcm, KeyInit, Nonce};
use alloy::signers::local::coins_bip39::{English, Mnemonic};
use dialoguer::{theme::ColorfulTheme, Confirm, Input, Password, Select};
use colored::*;
use keys::{derive_signing_key, KeySet};
use rand::{rngs::OsRng, RngCore};
use serde::{Serialize, Deserialize};
use anyhow::{anyhow, Result};
use clap::Args;

#[derive(Debug, Clone, Serialize, Deserialize, Args)]
pub struct OperatorConfig {
    #[clap(long, short='n', default_value="1")]
    pub network_id: u16,
    #[clap(long, short='K', default_value_os_t=default_keyfile_path())]
    pub keyfile: PathBuf,
    #[clap(long="secret-key", short='S', alias="private-key")]
    pub secret_key: Option<String>,
    #[clap(long="mnemonic-phrase", short='M', aliases=["phrase", "mnemonic"])]
    pub mnemonic: Option<Vec<String>>,
    #[clap(long, short='P')]
    pub public_key: Option<String>,
    #[clap(long, short)]
    pub address: Option<String>,
    #[clap(long="bootstrap-nodes", short='b', alias="to-dial")]
    pub bootstrap_nodes: Vec<String>,
    #[clap(long="bootstrap-domain", short='B', aliases=["domain"])]
    pub bootstrap_domain: Option<String>,
    #[clap(long="is-bootstrap", short='I', help="Whether this node should serve as a bootstrap node")]
    pub is_bootstrap_node: bool,
    #[clap(long="region", short='R', help="Geographic region of this node (e.g., us-east, eu-west)")]
    pub region: Option<String>,
    #[clap(long="datastore-port", short='d', default_value="3004")]
    pub datastore_port: u16,
    #[clap(long="formnet-join-port", short='j', default_value="3001")]
    pub formnet_join_server_port: u16,
    #[clap(long="formnet-service-port", short='f', default_value="51820")]
    pub formnet_service_port: u16,
    #[clap(long="vmm-service-port", short='v', default_value="3002")]
    pub vmm_service_port: u16,
    #[clap(long="pack-manager-port", short='p', default_value="3003")]
    pub pack_manager_port: u16,
    #[clap(long="event-queue-port", short='e', aliases=["mempool-port", "event-pool-port", "mempool", "events"])]
    pub event_queue_port: u16,
    #[clap(long="contract", short='c', aliases=["staking-contract", "avs-contract"])]
    pub contract_address: Option<String>
}

impl OperatorConfig {
    pub fn from_file(path: impl AsRef<Path>, encrypted: bool, password: Option<&str>) -> Result<Self> {
        println!("Attempting to read config from {}", path.as_ref().display());
        let mut plain_config: OperatorConfig = serde_json::from_slice(&std::fs::read(path)?)?;
        if let (None, None) = (&plain_config.mnemonic, &plain_config.secret_key) {
            return Err(anyhow!("Either a mnemonic or secret key is required"));
        }
        if encrypted {
            plain_config = plain_config.decrypt_key(
                password.ok_or(
                    anyhow!(
                        "If config file is encrypted, password is required"
                    )
                )?
            )?; 
        }

        return Ok(plain_config)
    }

    fn decrypt_key(mut self, password: &str) -> Result<Self> {
        (self.mnemonic, self.secret_key) = if let Some(mnemonic) = &self.mnemonic {
            let plain_phrase = mnemonic.iter().filter_map(|ew| {
                match hex::decode(ew) {
                    Ok(ewb) => match decrypt_file(&ewb, password) {
                        Ok(plaintext) => match String::from_utf8(plaintext) {
                            Ok(string) => Some(string),
                            _ => None
                        }
                        _ => None
                    }
                    _ => None
                }
            }).collect::<Vec<String>>();
            if plain_phrase.len() != mnemonic.len() {
                return Err(anyhow!("Error decrypting mnemonic phrase, length doesn't match"));
            }

            let plain_sk = if let Some(key) = &self.secret_key {
                hex::encode(
                    &decrypt_file(
                        &hex::decode(key).map_err(|e| anyhow!("{e}"))?, password
                    ).map_err(|e| anyhow!("{e}"))?
                )
            } else {
                hex::encode(
                    derive_signing_key(
                        &Mnemonic::<English>::new_from_phrase(&mnemonic.join(" "))?, 
                        Some(password)
                    )?.to_bytes()
                )
            };

            (Some(plain_phrase), Some(plain_sk))
        } else {
            let plain_sk = hex::encode(
                &decrypt_file(
                    &hex::decode(
                        self.secret_key.unwrap()
                    ).map_err(|e| anyhow!("{e}"))?, password
                ).map_err(|e| anyhow!("{e}"))?
            );
            (None, Some(plain_sk))
        };         

        Ok(self)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Args)]
pub struct KeyFile {
    #[clap(default_value_os_t=default_keyfile_path())]
    filepath: PathBuf,
    #[clap(long, short)]
    secret_key: Option<String>,
    #[clap(long, short)]
    mnemoinc: Option<String>,
    #[clap(long, short)]
    public_key: Option<String>,
    #[clap(long, short)]
    address: Option<String>
}

fn default_keyfile_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".config").join(".keystore")
}

mod keys {
    use super::*;
    use alloy::{primitives::Address, signers::{
        k256::ecdsa::{SigningKey, VerifyingKey}, local::coins_bip39::{English, Mnemonic}
    }};
    use k256::{elliptic_curve::SecretKey, Secp256k1};
    use rand::thread_rng;

    #[derive(Clone, Debug, Serialize, Deserialize)]
    pub struct KeySet {
        pub mnemonic: Vec<String>,
        pub secret_key: String,
        pub public_key: String,
        pub address: String,
    }

    impl KeySet {
        pub fn from_mnemonic(mnemonic: Mnemonic<English>, password: Option<&str>) -> Self {
            let signing_key = derive_signing_key(&mnemonic, password).expect("Unable to derive signing key from mnemonic");
            let public_key = derive_public_key(&signing_key);
            let address = derive_address(&public_key);
            let phrase = mnemonic.to_phrase().split_whitespace().into_iter().map(|s| s.to_string()).collect();
            KeySet {
                mnemonic: phrase,
                secret_key: hex::encode(signing_key.to_bytes()),
                public_key: hex::encode(public_key.to_sec1_bytes()),
                address: hex::encode(address)
            }
        }

        pub fn random(password: Option<&str>, count: usize) -> Self {
            let mnemonic = generate_mnemonic(count);
            Self::from_mnemonic(mnemonic, password)
        }
        
        pub fn from_private_key(key: SigningKey) -> Self {
            let public_key = derive_public_key(&key);
            let address = derive_address(&public_key);
            Self {
                mnemonic: vec![],
                secret_key: hex::encode(key.to_bytes()),
                public_key: hex::encode(public_key.to_sec1_bytes()),
                address: hex::encode(address)
            }
        }
    }

    pub fn generate_mnemonic(count: usize) -> Mnemonic<English> {
        let mut rng = thread_rng();
        Mnemonic::<English>::new_with_count(&mut rng, count).expect("Unable to generate mnemonic")
    }

    pub fn derive_signing_key(
        mnemonic: &Mnemonic<English>,
        password: Option<&str>
    ) -> Result<SigningKey> {
        let path = "m/44'/60'/0'/0'/0";
        let master_key = mnemonic.derive_key(path, password)?;
        let signing_key: &SigningKey = master_key.as_ref();
        let secret_key: SecretKey<Secp256k1> = signing_key.into();
        return Ok(SigningKey::from(secret_key));
    }

    pub fn derive_public_key(signing_key: &SigningKey) -> VerifyingKey {
        signing_key.verifying_key().clone()
    }

    pub fn derive_address(public_key: &VerifyingKey) -> Address {
        Address::from_public_key(public_key)
    }
}

mod prompts {
    use alloy::signers::k256::ecdsa::SigningKey;

    use super::*;

    pub fn network_id(theme: &ColorfulTheme) -> Result<u16> {
        println!("\n{}", "Network Configuration".bold().green());
        println!("The network ID is used to distinguish between different networks.");
        println!("Common values:");
        println!("  1 = Mainnet");
        println!("  2 = Jericho");
        println!("  3 = Uruk");
        println!("  >50000 = Local");
        
        let network_id: u16 = Input::with_theme(theme)
            .with_prompt("Enter network ID")
            .default(5)
            .interact_text()?;
        
        Ok(network_id)
    }

    pub fn keyfile(theme: &ColorfulTheme) -> Result<PathBuf> {
        println!("\n{}", "Keyfile Configuration".bold().green());
        println!("The keyfile stores your encrypted credentials.");
        
        let default_path = default_keyfile_path();
        let use_default = Confirm::with_theme(theme)
            .with_prompt(format!("Use default keyfile path? ({})", default_path.display()))
            .default(true)
            .interact()?;

        if use_default {
            Ok(default_path)
        } else {
            let path: String = Input::with_theme(theme)
                .with_prompt("Enter custom keyfile path")
                .interact_text()?;
            Ok(PathBuf::from(path))
        }
    }

    pub fn key_setup(theme: &ColorfulTheme) -> Result<(Option<String>, Option<Vec<String>>, Option<String>, Option<String>)> {
        println!("\n{}", "Key Configuration".bold().green());
        println!("Your keys are used to sign transactions and prove your identity.");
        
        let options = vec![
            "Generate new keys from mnemonic",
            "Import from existing mnemonic",
            "Import from private key",
            "Skip for now"
        ];
        
        let selection = Select::with_theme(theme)
            .with_prompt("How would you like to configure your keys?")
            .default(0)
            .items(&options)
            .interact()?;

        match selection {
            0 => {
                let count = match Select::with_theme(theme)
                    .with_prompt("How many words would you like your mnemonic to be?")
                    .default(0)
                    .items(&[12, 24])
                    .interact()? {
                        0 => 12,
                        1 => 24,
                        _ => unreachable!()
                };
                let password = {
                    match Confirm::with_theme(theme)
                        .with_prompt("Would you like to enhance security with a password for your mnemonic phrase?")
                        .default(true)
                        .interact()? {
                            true => {
                                Some(Password::with_theme(theme)
                                    .with_prompt("Provide a secure password to enhance the security of your mnemonic phrase")
                                    .with_confirmation("Confirm your password", "Passwords do not match")
                                    .interact()?
                                )
                            }
                            false => None,
                        }
                    };
                // Generate new keys from mnemonic
                let keyset = KeySet::random(password.as_deref(), count);
                
                println!("\n{}", "Generated New Keys:".bold().yellow());
                print!("Mnemonic Phrase: ");
                keyset.mnemonic.iter().for_each(|s| print!("{}, ", s.blue().bold()));
                print!("\n");
                println!("Address: {}{}", "0x".bright_green(), keyset.address.bright_green());
                println!("\n{}", "⚠️  IMPORTANT ⚠️ ".bold().red());
                println!("Please store your mnemonic phrase safely. It cannot be recovered if lost!");
                
                Confirm::with_theme(theme)
                    .with_prompt("I have safely stored my mnemonic phrase")
                    .interact()?;
                
                Ok((
                    Some(keyset.secret_key),
                    Some(keyset.mnemonic),
                    Some(keyset.public_key),
                    Some(keyset.address)
                ))
            },
            1 => {
                // Import from existing mnemonic
                let phrase: String = Input::with_theme(theme)
                    .with_prompt("Enter your mnemonic phrase")
                    .interact_text()?;
                
                let password = {
                    match Confirm::with_theme(theme)
                        .with_prompt("Did you provide a password for your mnemonic phrase?")
                        .default(true)
                        .interact()? {
                            true => {
                                Some(Password::with_theme(theme)
                                    .with_prompt("Enter your password:")
                                    .interact()?
                                )
                            }
                            false => None,
                        }
                    };
                let keyset = KeySet::from_mnemonic(Mnemonic::new_from_phrase(&phrase)?, password.as_deref());
                println!("Derived address: 0x{}", keyset.address.bright_green());
                
                Ok((
                    Some(keyset.secret_key),
                    Some(keyset.mnemonic),
                    Some(keyset.public_key),
                    Some(keyset.address)
                ))
            },
            2 => {
                // Import from private key
                let key: String = Input::with_theme(theme)
                    .with_prompt("Enter your private key (hex)")
                    .interact_text()?;
                
                let signing_key = SigningKey::from_slice(&hex::decode(key)?)?;
                let keyset = KeySet::from_private_key(signing_key);
                println!("Derived address: 0x{}", keyset.address.bright_green());
                
                Ok((
                    Some(keyset.secret_key),
                    None,
                    Some(keyset.public_key),
                    Some(keyset.address)
                ))
            },
            _ => Ok((None, None, None, None))
        }
    }

    pub fn bootstrap_nodes(theme: &ColorfulTheme) -> Result<Vec<String>> {
        println!("\n{}", "Bootstrap Nodes Configuration".bold().green());
        println!("Bootstrap nodes help you connect to the network initially.");
        
        let mut nodes = Vec::new();
        loop {
            let node: String = Input::with_theme(theme)
                .with_prompt("Enter bootstrap node address (leave empty to finish)")
                .allow_empty(true)
                .interact_text()?;

            if node.is_empty() {
                break;
            }
            nodes.push(node);
        }

        Ok(nodes)
    }

    pub fn bootstrap_domain(theme: &ColorfulTheme) -> Result<Option<String>> {
        println!("\n{}", "Bootstrap Domain Configuration".bold().green());
        println!("A bootstrap domain allows you to connect to the nearest healthy bootstrap node automatically.");
        
        let use_domain = Confirm::with_theme(theme)
            .with_prompt("Would you like to use a bootstrap domain for network discovery?")
            .default(true)
            .interact()?;
            
        if use_domain {
            let domain: String = Input::with_theme(theme)
                .with_prompt("Enter bootstrap domain")
                .default("bootstrap.formation.cloud".to_string())
                .interact_text()?;
                
            if domain.is_empty() {
                Ok(None)
            } else {
                Ok(Some(domain))
            }
        } else {
            Ok(None)
        }
    }

    pub fn bootstrap_role(theme: &ColorfulTheme) -> Result<bool> {
        println!("\n{}", "Bootstrap Node Role".bold().green());
        println!("Bootstrap nodes help other nodes discover and join the network.");
        println!("They need to be reliable and have good connectivity.");
        
        let is_bootstrap = Confirm::with_theme(theme)
            .with_prompt("Should this node serve as a bootstrap node?")
            .default(false)
            .interact()?;
            
        if is_bootstrap {
            println!("{}", "This node will be registered as a bootstrap node in the network.".bold().yellow());
            println!("It will be added to the bootstrap domain and used by new nodes joining the network.");
        }
        
        Ok(is_bootstrap)
    }

    pub fn region(theme: &ColorfulTheme) -> Result<Option<String>> {
        println!("\n{}", "Node Region Configuration".bold().green());
        println!("The geographic region helps optimize network connectivity.");
        
        let regions = vec![
            "none",
            "us-east", 
            "us-west", 
            "us-central",
            "eu-west", 
            "eu-central", 
            "asia-east", 
            "asia-southeast",
            "custom"
        ];
        
        let selection = Select::with_theme(theme)
            .with_prompt("Select the geographic region of this node")
            .default(0)
            .items(&regions)
            .interact()?;
            
        match selection {
            0 => Ok(None),
            n if n == regions.len() - 1 => {
                let custom_region: String = Input::with_theme(theme)
                    .with_prompt("Enter custom region")
                    .interact_text()?;
                Ok(Some(custom_region))
            },
            _ => Ok(Some(regions[selection].to_string()))
        }
    }

    pub fn service_port(theme: &ColorfulTheme, service_name: &str, default_port: u16) -> Result<u16> {
        println!("\n{}", format!("{} Port Configuration", service_name).bold().green());
        
        let port: u16 = Input::with_theme(theme)
            .with_prompt(format!("Enter {} port", service_name))
            .default(default_port)
            .interact_text()?;
        
        Ok(port)
    }

    pub fn contract_address(theme: &ColorfulTheme) -> Result<Option<String>> {
        println!("\n{}", "Contract Configuration".bold().green());
        println!("Enter the staking contract address for your AVS.");
        
        let address: String = Input::with_theme(theme)
            .with_prompt("Enter contract address (leave empty to skip)")
            .allow_empty(true)
            .interact_text()?;

        Ok(if address.is_empty() { None } else { Some(address) })
    }
}

// Main wizard function
pub fn run_config_wizard() -> Result<OperatorConfig> {
    let theme = ColorfulTheme::default();
    
    println!("{}", "\nWelcome to the Operator Configuration Wizard".bold().blue());
    println!("This wizard will help you set up your operator configuration.\n");

    // Collect all configuration values
    let network_id = prompts::network_id(&theme)?;
    let keyfile = prompts::keyfile(&theme)?;
    
    // Handle key generation/import
    let (secret_key, mnemonic, public_key, address) = prompts::key_setup(&theme)?;

    // Network configuration
    let bootstrap_nodes = prompts::bootstrap_nodes(&theme)?;
    let bootstrap_domain = prompts::bootstrap_domain(&theme)?;
    let is_bootstrap_node = prompts::bootstrap_role(&theme)?;
    let region = prompts::region(&theme)?;
    
    // Service ports
    let datastore_port = prompts::service_port(&theme, "Datastore", 3004)?;
    let formnet_join_server_port = prompts::service_port(&theme, "Formnet Join Server", 3001)?;
    let formnet_service_port = prompts::service_port(&theme, "Formnet Service", 51820)?;
    let vmm_service_port = prompts::service_port(&theme, "VMM Service", 3002)?;
    let pack_manager_port = prompts::service_port(&theme, "Pack Manager", 3003)?;
    let event_queue_port = prompts::service_port(&theme, "Event Queue", 3005)?;
    
    let contract_address = prompts::contract_address(&theme)?;

    // Create the config
    let config = OperatorConfig {
        network_id,
        keyfile,
        secret_key,
        mnemonic,
        public_key,
        address,
        bootstrap_nodes,
        bootstrap_domain,
        is_bootstrap_node,
        region,
        datastore_port,
        formnet_join_server_port,
        formnet_service_port,
        vmm_service_port,
        pack_manager_port,
        event_queue_port,
        contract_address,
    };

    Ok(config)
}

// Function to save the config while redacting sensitive information
pub fn save_config_and_keystore(
    config: &OperatorConfig,
    config_path: &PathBuf,
    encrypt_keys: bool
) -> Result<()> {
    // Create a copy of the config that we'll save
    let mut safe_config = config.clone();
    
    if encrypt_keys {
        if let (Some(secret_key), Some(mnemonic), Some(address)) = 
            (&safe_config.secret_key, &safe_config.mnemonic, &safe_config.address) {
            
            println!("\n{}", "Encrypting Keystore".bold().green());
            println!("Please choose a strong password to encrypt your keys.");
            println!("This password will be required to decrypt your keys later.");
            println!("{}", "⚠️  Encrypting the Keystore may take up to 2 minutes⚠️  ".bold().yellow());
            
            let theme = ColorfulTheme::default();
            let password: String = Password::with_theme(&theme)
                .with_prompt("Enter encryption password")
                .with_confirmation("Confirm your password", "Entered password does not match")
                .interact()?;
            
            // Create and save the encrypted keystore
            let keystore = KeySet::from_mnemonic(
                Mnemonic::<English>::new_from_phrase(&mnemonic.join(" "))?,
                Some(&password),
            );
            
            let keystore_path = safe_config.keyfile.clone();
            if let Some(parent) = keystore_path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            
            let keystore_json = serde_json::to_vec(&keystore)?;
            let encrypted_hex_mnemonic = keystore.mnemonic.iter().filter_map(|s| {
                match encrypt_file(s.as_bytes(), &password) {
                    Ok(eb) => {
                        let hex = hex::encode(&eb);
                        Some(hex)
                    },
                    Err(_) => None
                }
            }).collect::<Vec<String>>();
            if encrypted_hex_mnemonic.len() != keystore.mnemonic.len() {
            return Err(anyhow!("One or more of the words in the mnemonic was not properly encrypted")); }

            let encrypted_secret_key = hex::encode(
                &encrypt_file(
                    &hex::decode(&secret_key)?,
                    &password
                ).map_err(|e| anyhow!("{e}"))?
            );

            safe_config.mnemonic = Some(encrypted_hex_mnemonic);
            safe_config.secret_key = Some(encrypted_secret_key);
            safe_config.address = Some(hex::encode(address));
            let encrypted = encrypt_file(&keystore_json, &password).map_err(|e| anyhow!(format!("Unable to encrypt file {e}")))?;
            std::fs::write(&keystore_path, encrypted)?;
            
            println!("\n{}", "Keystore encrypted and saved successfully!".bold().green());
            println!("Location: {}", keystore_path.display());
        }
    }
    
    // Remove sensitive data from config
    
    // Save the redacted configuration
    if let Some(parent) = config_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    
    let config_str = serde_json::to_string_pretty(&safe_config)?;
    std::fs::write(config_path, config_str)?;
    
    println!("\n{}", "Configuration saved successfully!".bold().green());
    println!("Location: {}", config_path.display());
    
    Ok(())
}

pub fn load_keystore(keyfile: &PathBuf) -> Result<Vec<u8>> {
    let theme = ColorfulTheme::default();
    
    // Read the keystore file
    let encrypted = std::fs::read(keyfile)?;
    
    println!("\n{}", "Decrypting Keystore".bold().green());
    println!("Please enter your password to decrypt the keystore.");
    
    let password: String = Input::with_theme(&theme)
        .with_prompt("Enter password")
        .interact_text()?;
    
    let decrypted = decrypt_file(&encrypted, &password).map_err(|e| anyhow!(format!("Unable to decrypt file: {e}")))?;
    Ok(decrypted)
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

