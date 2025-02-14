use std::{io::Write, path::PathBuf, str::FromStr};
use k256::elliptic_curve::PublicKey;
use alloy_signer_local::coins_bip39::{English, Mnemonic};
use alloy_core::primitives::Address;
use colored::*;
use clap::Args;
use formnet::user_join_formnet;
use k256::{ecdsa::SigningKey, elliptic_curve::SecretKey};
use rand::thread_rng;
use serde::{Serialize, Deserialize};
use dialoguer::{theme::ColorfulTheme, Confirm, Input, Select, Password};
use crate::{encrypt_file, save_config, Config};

#[derive(Clone, Serialize, Deserialize)]
pub struct Keystore {
    pub mnemonic: Option<String>,
    pub secret_key: String,
    pub public_key: String,
    pub address: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, Args)]
pub struct Init {
    #[clap(long, default_value_t=true)]
    pub wizard: bool,
    #[clap(long, short)]
    pub signing_key: Option<String>,
    #[clap(long, short)]
    pub mnemonic: Option<String>,
    #[clap(long, short)]
    pub keystore: Option<PathBuf>,
    #[clap(long, short)]
    pub config_dir: Option<PathBuf>,
    #[clap(long, short)]
    pub data_dir: Option<PathBuf>,
    #[clap(long, short = 'H')]
    pub hosts: Option<Vec<String>>,
    #[clap(long, short)]
    pub pack_manager_port: Option<u16>,
    #[clap(long, short)]
    pub vmm_port: Option<u16>,
    #[clap(long, short)]
    pub formnet_port: Option<u16>,
    #[clap(long, short)]
    pub join_formnet: Option<bool>,
}

impl Default for Init {
    fn default() -> Self {
        Self {
            wizard: true,
            signing_key: None,
            mnemonic: None,
            keystore: None,
            config_dir: None,
            data_dir: None,
            hosts: None,
            pack_manager_port: None,
            vmm_port: None,
            formnet_port: None,
            join_formnet: None
        }
    }
}

impl Init {
    pub async fn handle(&mut self) -> Result<(Config, Keystore), Box<dyn std::error::Error>> {
        self.run_wizard().await
    }

    async fn run_wizard(&mut self) -> Result<(Config, Keystore), Box<dyn std::error::Error>> {
        #[cfg(target_os = "windows")]
        let home_dir = std::env::var("APPDATA").unwrap_or(".".to_string());
        let home_dir = std::env::var("HOME").unwrap_or(".".to_string());
        println!("{}", "Welcome to the form kit configuration wizard".blue().bold());
        println!("\n{}", "============================================".blue().bold());

        let options = vec!["Create new wallet", "Import from Private Key", "Import from Mnemonic Phrase"];
        println!("\nWARNING: Currently form kit only supports Ethereum Compatible Wallets");
        let selection = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("How would you like to set up your wallet")
            .default(0)
            .items(&options)
            .interact()?;

        let signing_key: SigningKey = match selection {
            0 => {
                let count = match Select::with_theme(&ColorfulTheme::default())
                    .with_prompt("Would you like a 12 or 24 word mnemonic phrase to derive your new keypair from?")
                    .default(1)
                    .items(&[12, 24])
                    .interact()? {
                        0 => 12,
                        1 => 24,
                        _ => unreachable!()
                    };

                let mut rng = thread_rng(); 
                let mnemonic = Mnemonic::<English>::new_with_count(&mut rng, count)?;
                self.mnemonic = Some(mnemonic.to_phrase());
                let seed = mnemonic.to_seed(None)?;
                SecretKey::from_slice(&seed[..32])?.into()
            },
            1 => {
                let key_str = Input::with_theme(&ColorfulTheme::default())
                    .with_prompt("Enter your Secret Key (hex)")
                    .validate_with(|input: &String| -> Result<(), Box<dyn std::error::Error>> {
                        SigningKey::from_slice(&hex::decode(input)?)?;
                        Ok(())
                    })
                    .interact_text()?;
                SigningKey::from_slice(&hex::decode(key_str)?)?
            },
            2 => {
                let mnemonic_str = Input::with_theme(&ColorfulTheme::default())
                    .with_prompt("Enter your 12 or 24 word mnemonic phrase")
                    .validate_with(|input: &String| -> Result<(), Box<dyn std::error::Error>> {
                        Mnemonic::<English>::from_str(&input)?;
                        Ok(())
                    })
                    .interact_text()?.to_string();
                let mnemonic = Mnemonic::<English>::from_str(&mnemonic_str)?;
                self.mnemonic = Some(mnemonic.to_phrase());
                let seed = mnemonic.to_seed(None)?;
                SecretKey::from_slice(&seed[..32])?.into()
            }
            _ => unimplemented!()
        };

        self.signing_key = Some(hex::encode(signing_key.to_bytes()));

        if self.keystore.is_none() {
            let keystore_dir: String = Input::with_theme(&ColorfulTheme::default())
                .with_prompt("Enter keystore directory path")
                .allow_empty(true)
                .default(format!("{home_dir}/.keystore"))
                .interact()?;

            if !keystore_dir.is_empty() {
                std::fs::create_dir_all(&keystore_dir)?;
                self.keystore = Some(PathBuf::from(keystore_dir));
            }
        }

        if self.config_dir.is_none() {
            let config_dir: String = Input::with_theme(&ColorfulTheme::default())
                .with_prompt("Enter config directory path")
                .allow_empty(true)
                .default(format!("{home_dir}/.config/form"))
                .interact()?;

            if !config_dir.is_empty() {
                std::fs::create_dir_all(config_dir.clone())?;
                self.config_dir = Some(PathBuf::from(config_dir));
            }
        }

        if self.data_dir.is_none() {
            let data_dir: String = Input::with_theme(&ColorfulTheme::default())
                .with_prompt("Enter data directory path")
                .allow_empty(true)
                .default(format!("{home_dir}/.local/share/form"))
                .interact()?;

            if !data_dir.is_empty() {
                std::fs::create_dir_all(data_dir.clone())?;
                self.data_dir = Some(PathBuf::from(data_dir));
            }
        }

        if self.hosts.is_none() {
            let hosts: String = Input::with_theme(&ColorfulTheme::default())
                .with_prompt("Enter hosts (comma separated)")
                .allow_empty(true)
                .default("localhost".into())
                .validate_with(|input: &String| -> Result<(), &str> {
                    for host in input.split(',') {
                        let host = host.trim();
                        if !host.chars().all(|c| c.is_alphanumeric() || c == '-') {
                            return Err("Hosts must be alphanumeric with optional dashes");
                        }
                    }
                    Ok(())
                })
                .interact()?;

            if !hosts.is_empty() {
                self.hosts = Some(hosts.split(',').map(String::from).collect());
            }
        }

        if self.pack_manager_port.is_none() {
            let port: String = Input::with_theme(&ColorfulTheme::default())
                .with_prompt("Enter the port for the providers pack manager API endpoint")
                .allow_empty(true)
                .default("3003".into())
                .interact()?;

            if !port.is_empty() {
                self.pack_manager_port = Some(port.parse()?);
            }
        }


        if self.vmm_port.is_none() {
            let port: String = Input::with_theme(&ColorfulTheme::default())
                .with_prompt("Enter the port for the providers virtual machine manager API endpoint")
                .allow_empty(true)
                .default("3002".into())
                .interact()?;

            if !port.is_empty() {
                self.vmm_port = Some(port.parse()?);
            }
        }

        if self.formnet_port.is_none() {
            let port: String = Input::with_theme(&ColorfulTheme::default())
                .with_prompt("Enter the port for the providers formnet API endpoint")
                .default("3001".into())
                .interact()?;

            self.formnet_port = Some(port.parse()?);
        }

        if self.join_formnet.is_none() {
            let join = Confirm::with_theme(&ColorfulTheme::default())
                .with_prompt("Would you like to joing Formnet? Doing so will make your journey much more convenient and enjoyable.")
                .default(true)
                .interact()?;

            self.join_formnet = Some(join);
        }

        println!("\n{}", "Final Configuration:".blue().bold());

        println!("{}", "==================".blue());
        if let Some(ref key) = self.signing_key {
            println!("Signing Key: {}", format!("{}", key.yellow()));
        }
        if let Some(ref mnemonic) = self.mnemonic {
            println!("Mnemonic:");
            let words = mnemonic.split_whitespace();
            for (idx, word) in words.enumerate() {
                if idx % 3 == 0 {
                    print!("\n");
                }

                print!("{:>2}. {:<15}", (idx + 1).to_string().yellow(), word.blue());
            }
            print!("\n");
        }

        let (secret_key, public_key, mnemonic, address) = if let Some(key) = &self.signing_key {
            let signing_key = SigningKey::from_slice(&hex::decode(key)?)?;
            let public_key = signing_key.verifying_key().clone();
            (signing_key, public_key, None, Address::from_public_key(&public_key))
        } else if let Some(phrase) = &self.mnemonic {
            let mnemonic = Mnemonic::<English>::new_from_phrase(&phrase)?;
            let seed = mnemonic.to_seed(None)?;
            let signing_key: SigningKey = SecretKey::from_slice(&seed[..32])?.into();
            let public_key = signing_key.verifying_key().clone();
            let address = Address::from_public_key(&public_key);
            (signing_key, public_key, Some(phrase), address)
        } else {
            return Err(
                Box::new(
                    std::io::Error::new(
                        std::io::ErrorKind::Other,
                        "No Mnemonic Phrase or Signing Key"
                    )
                )
            )
        };

        let keystore = Keystore {
            mnemonic: mnemonic.cloned(),
            secret_key: hex::encode(SecretKey::from(secret_key).to_bytes()),
            public_key: hex::encode(PublicKey::from(public_key).to_sec1_bytes().as_ref()),
            address: hex::encode(address)
        };

        if Confirm::with_theme(&ColorfulTheme::default())
            .with_prompt("Would you like to save your wallet to a keystore?")
            .default(true)
            .interact()? {

                //TODO: Separate the files for different components
                //No need to encrypt address or pubkey
                let password: String = Password::with_theme(&ColorfulTheme::default())
                    .with_prompt("Provide a password for the keystore")
                    .with_confirmation("Confirm the password", "Passwords do not match")
                    .interact()?;
                let enc_contents = encrypt_file(&serde_json::to_vec(&keystore)?, &password)?;

                let keyfile: String = Input::with_theme(&ColorfulTheme::default())
                    .with_prompt("Provide a name for the keyfile")
                    .default("form_id".into())
                    .interact()?;

                if let Some(ks) = &self.keystore {
                    let keyfile_path = ks.join(&keyfile);
                    let mut file = std::fs::File::create(&keyfile_path)?;
                    file.write_all(&enc_contents)?;
                    // Update the keystore path in self to point to the file
                    self.keystore = Some(keyfile_path);
                }
        }

        println!("Config Directory: {}", self.config_dir.as_ref().map_or("Not set".to_string(), |p| p.display().to_string()));
        println!("Data Directory: {}", self.data_dir.as_ref().map_or("Not set".to_string(), |p| p.display().to_string()));
        println!("Hosts: {}", self.hosts.as_ref().map_or("Not set".to_string(), |h| h.join(", ")));
        println!("Pack Manager Port: {}", self.pack_manager_port.map_or("Not set".to_string(), |p| p.to_string()));
        println!("VMM Port: {}", self.vmm_port.map_or("Not set".to_string(), |p| p.to_string()));
        println!("Formnet Port: {}", self.formnet_port.map_or("Not set".to_string(), |p| p.to_string()));
        println!("Join Formnet: {}", self.join_formnet.map_or("Not set".to_string(), |j| j.to_string()));

        let config = Config::new(self);
        std::fs::create_dir_all(config.config_dir.clone())?;
        let config_path = config.config_dir.join("config.json");
        println!("Saving config to path: {}", config_path.display().to_string());
        save_config(&config, &config_path)?;

        // Display final configuration
        println!("\n{}", "Configuration saved successfully!".green().bold());
        println!("Configuration file: {}", config_path.display());

        let _host = if let Some(hosts) = &self.hosts {
            hosts[0].clone()
        } else {
            return Err(
                Box::new(
                    std::io::Error::new(
                        std::io::ErrorKind::Other, 
                        "No hosts to request formnet invite from"
                    )
                )
            )
        };


        Ok((config, keystore))
    }
}


pub async fn join_formnet(address: String, provider: String) -> Result<(), Box<dyn std::error::Error>> {
    user_join_formnet(address, provider).await?;

    Ok(())
}
