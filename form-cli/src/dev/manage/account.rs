use clap::Args;
use colored::*;
use k256::ecdsa::{RecoveryId, Signature, SigningKey, VerifyingKey};
use tiny_keccak::{Hasher, Sha3};
use alloy_core::primitives::Address;
use alloy_signer_local::{coins_bip39::English, MnemonicBuilder};
use reqwest::Client;
use serde::{Serialize, Deserialize};
use form_types::state::Response;
use crate::Keystore;

/// Command for transferring ownership of an instance from one account to another
#[derive(Clone, Debug, Args)]
pub struct TransferOwnershipCommand {
    /// The instance ID to transfer ownership of
    #[clap(long, short)]
    pub instance_id: String,
    
    /// The address of the account to transfer ownership to
    #[clap(long, short)]
    pub to_address: String,
    
    /// A hexadecimal representation of a valid private key for 
    /// signing the request. The owner of this key must be the current owner
    /// of the instance.
    #[clap(long, short)]
    pub private_key: Option<String>,
    
    /// An alternative to private key or mnemonic. If you have a keyfile
    /// stored locally, you can use the keyfile to read in your private key
    #[clap(long, short)]
    pub keyfile: Option<String>,
    
    /// An alternative to private key or keyfile. If you have a 12 or 24 word 
    /// BIP39 compliant mnemonic phrase, you can use it to derive the signing
    /// key for this request
    #[clap(long, short)]
    pub mnemonic: Option<String>,
}

/// Account request structure for transferring ownership
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TransferOwnershipRequest {
    pub from_address: String,
    pub to_address: String,
    pub instance_id: String,
    pub signature: String,
    pub recovery_id: u32,
}

impl TransferOwnershipCommand {
    /// Handle the command execution
    pub async fn handle(&self, provider: &str, port: u16, keystore: Option<Keystore>) -> Result<(), Box<dyn std::error::Error>> {
        println!("Transferring ownership of instance {} to {}", self.instance_id.yellow(), self.to_address.green());
        
        // Get the signing key
        let signing_key = self.get_signing_key(keystore.clone())?;
        
        // Get the from_address from the signing key
        let from_address = hex::encode(Address::from_private_key(&signing_key));
        println!("Request will be signed by account: {}", from_address.green());
        
        // Sign the request
        let (signature, recovery_id, _) = self.sign_request(&self.instance_id, keystore.clone())?;
        
        // Create the transfer request
        let request = TransferOwnershipRequest {
            from_address: from_address.clone(),
            to_address: self.to_address.clone(),
            instance_id: self.instance_id.clone(),
            signature,
            recovery_id: recovery_id.to_byte() as u32,
        };
        
        // Send the request to the server
        let client = Client::new();
        let url = format!("http://{}:{}/account/transfer-ownership", provider, port);
        
        let response: Response<serde_json::Value> = client.post(&url)
            .json(&request)
            .send()
            .await?
            .json()
            .await?;
        
        match response {
            Response::Success(_) => {
                println!("✅ {}", "Successfully transferred ownership of instance".green());
                println!("Instance ID: {}", self.instance_id.yellow());
                println!("From: {}", from_address.green());
                println!("To: {}", self.to_address.green());
            },
            Response::Failure { reason } => {
                if let Some(r) = reason {
                    println!("❌ {}: {}", "Failed to transfer ownership".red(), r);
                } else {
                    println!("❌ {}", "Failed to transfer ownership: Unknown error".red());
                }
                return Err("Failed to transfer ownership".into());
            }
        }
        
        Ok(())
    }
    
    /// Get the signing key from various sources (private_key, keystore, mnemonic)
    pub fn get_signing_key(&self, keystore: Option<Keystore>) -> Result<SigningKey, String> {
        if let Some(pk) = &self.private_key {
            Ok(SigningKey::from_slice(
                    &hex::decode(pk)
                        .map_err(|e| e.to_string())?
                ).map_err(|e| e.to_string())?
            )
        } else if let Some(ks) = keystore {
            Ok(SigningKey::from_slice(
                &hex::decode(ks.secret_key)
                    .map_err(|e| e.to_string())?
                ).map_err(|e| e.to_string())?
            )
        } else if let Some(mnemonic) = &self.mnemonic {
            Ok(SigningKey::from_slice(&MnemonicBuilder::<English>::default()
                .phrase(mnemonic)
                .derivation_path("m/44'/60'/0'/0/0").map_err(|e| e.to_string())?
                .build().map_err(|e| e.to_string())?.to_field_bytes().to_vec()
            ).map_err(|e| e.to_string())?)
                
        } else {
            Err("A signing key is required, use either private_key, mnemonic or keyfile CLI arg to provide a valid signing key".to_string())
        }
    }
    
    /// Sign a request with the selected key
    pub fn sign_request(&self, instance_id: &str, keystore: Option<Keystore>) -> Result<(String, RecoveryId, [u8; 32]), String> {
        let signing_key = self.get_signing_key(keystore)?;
        
        let mut hasher = Sha3::v256();
        let mut message_hash = [0u8; 32];
        hasher.update(instance_id.as_bytes());
        hasher.finalize(&mut message_hash);
        
        // Sign the message
        let (sig, rec) = signing_key.sign_recoverable(&message_hash).map_err(|e| e.to_string())?;
        
        Ok((hex::encode(&sig.to_vec()), rec, message_hash))
    }
} 