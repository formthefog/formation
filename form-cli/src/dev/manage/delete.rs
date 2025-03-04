use clap::Args;
use colored::*;
use form_p2p::queue::{QueueRequest, QueueResponse, QUEUE_PORT};
use form_types::{DeleteVmRequest, VmmResponse};
use k256::ecdsa::{RecoveryId, SigningKey, VerifyingKey, Signature};
use tiny_keccak::{Hasher, Sha3};
use alloy_core::primitives::Address;
use alloy_signer_local::{coins_bip39::English, MnemonicBuilder};
use crate::Keystore;

#[derive(Clone, Debug, Args)]
pub struct DeleteCommand {
    /// The ID of the instance being started
    #[clap(long, short)]
    pub id: Option<String>,
    /// The name of the instance being started, an alternative to ID
    #[clap(long, short)]
    pub name: Option<String>,
    /// A hexadecimal or base64 representation of a valid private key for 
    /// signing the request. Given this is the create command, this will
    /// be how the network derives ownership of the instance. Authorization
    /// to other public key/wallet addresses can be granted by the owner
    /// after creation, however, this key will be the initial owner until
    /// revoked or changed by a request made with the same signing key
    #[clap(long, short)]
    pub private_key: Option<String>,
    /// An altenrative to private key or mnemonic. If you have a keyfile
    /// stored locally, you can use the keyfile to read in your private key
    //TODO: Add support for HSM and other Enclave based key storage
    #[clap(long, short)]
    pub keyfile: Option<String>,
    /// An alternative to private key or keyfile. If you have a 12 or 24 word 
    /// BIP39 compliant mnemonic phrase, you can use it to derive the signing
    /// key for this request
    //TODO: Add support for HSM and other Enclave based key storage
    #[clap(long, short)]
    pub mnemonic: Option<String>,
}

fn print_delete_queue_response(resp: QueueResponse, vm_id: &str) {
    match resp {
        QueueResponse::OpSuccess => {
            println!("{} DELETE request has been added to the queue for VM {}.", "✅".green(), vm_id.yellow());
            println!("You can check the status of your VM using the `form manage status` command.");
        }
        QueueResponse::Failure { reason } => {
            println!("{} Failed to add DELETE request to the queue for VM {}.", "❌".red(), vm_id.yellow());
            if let Some(message) = reason {
                println!("Error from queue: {}", message);
            }
            println!("Please try again or contact support if the issue persists.");
        }
        _ => {
            println!("{} DELETE request was processed for VM {}.", "ℹ️".blue(), vm_id.yellow());
            println!("You can check the status of your VM using the `form manage status` command.");
        }
    }
}

impl DeleteCommand {
    pub async fn handle(&self, provider: &str, vmm_port: u16) -> Result<VmmResponse, Box<dyn std::error::Error>> {
        let vm_id = match (&self.id, &self.name) {
            (Some(id), _) => id.clone(),
            (_, Some(name)) => name.clone(),
            _ => return Err("Either id or name must be provided".into()),
        };

        let client = reqwest::Client::new();
        let url = format!("http://{}:{}/api/v1/delete", provider, vmm_port);
        
        // Create the request with optional signature
        let request = DeleteVmRequest {
            id: vm_id.clone(),
            name: vm_id.clone(),
            signature: None,
            recovery_id: 0,
        };

        // Send the request
        let response = client.post(&url)
            .json(&request)
            .send()
            .await?
            .json::<VmmResponse>()
            .await?;

        Ok(response)
    }

    pub async fn handle_queue(&self, provider: &str, keystore: Option<Keystore>) -> Result<(), Box<dyn std::error::Error>> {
        let vm_id = match (&self.id, &self.name) {
            (Some(id), _) => id.clone(),
            (_, Some(name)) => name.clone(),
            _ => return Err("Either id or name must be provided".into()),
        };

        // Prepare the queue request
        let queue_request = self.prepare_delete_request_queue(&vm_id, keystore).await?;

        // Send the queue request
        let client = reqwest::Client::new();
        let url = format!("http://{}:{}/request", provider, QUEUE_PORT);
        
        let response = client.post(&url)
            .json(&queue_request)
            .send()
            .await?
            .json::<QueueResponse>()
            .await?;

        // Print response to user
        print_delete_queue_response(response, &vm_id);

        Ok(())
    }

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

    pub fn sign_request(&self, id: &str, keystore: Option<Keystore>) -> Result<(String, RecoveryId, [u8; 32]), String> {
        let key = self.get_signing_key(keystore)?;
        
        // Create a message for signing
        let mut hasher = Sha3::v256();
        let message = format!("DeleteVmRequest:{}", id);
        let mut hash = [0u8; 32];
        hasher.update(message.as_bytes());
        hasher.finalize(&mut hash);
        
        // Sign the message
        let (signature, recovery_id) = key.sign_prehash_recoverable(&hash)
            .map_err(|e| e.to_string())?;
        
        // Encode the signature
        let signature_hex = hex::encode(signature.to_bytes());
        
        Ok((signature_hex, recovery_id, hash))
    }

    pub async fn prepare_delete_request_queue(&self, id: &str, keystore: Option<Keystore>) -> Result<QueueRequest, Box<dyn std::error::Error>> {
        let (signature, recovery_id, hash) = self.sign_request(id, keystore.clone())?;
        
        let delete_vm_request = DeleteVmRequest {
            id: id.to_string(),
            name: id.to_string(),
            signature: Some(signature.clone()),
            recovery_id: recovery_id.to_byte() as u32,
        };
        
        let recovered_address = Address::from_public_key(
            &VerifyingKey::recover_from_msg(
                &hash, 
                &Signature::from_slice(&hex::decode(&signature)?)?,
                recovery_id
            )?
        );

        println!("Request will be signed by address: {recovered_address:x}");

        let mut hasher = Sha3::v256();
        let mut topic_hash = [0u8; 32];
        hasher.update(b"vmm");
        hasher.finalize(&mut topic_hash);
        let mut message_code = vec![2]; // Code 2 for delete operation (as seen in handle_message in API)
        message_code.extend(serde_json::to_vec(&delete_vm_request)?);

        let queue_request = QueueRequest::Write {
            content: message_code,
            topic: hex::encode(topic_hash)
        };
        
        Ok(queue_request)
    }
}
