use clap::Args;
use colored::*;
use form_p2p::queue::{QueueRequest, QueueResponse, QUEUE_PORT};
use form_types::{StopVmRequest, VmmResponse};
use k256::ecdsa::{RecoveryId, Signature, SigningKey, VerifyingKey};
use tiny_keccak::{Hasher, Sha3};
use alloy_core::primitives::Address;
use alloy_signer_local::{coins_bip39::English, MnemonicBuilder};
use crate::Keystore;

#[derive(Clone, Debug, Args)]
pub struct StopCommand {
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

fn print_stop_queue_response(resp: QueueResponse, vm_id: &str) {
    match resp {
        QueueResponse::OpSuccess => {
            println!(r#"
Your {} stop request was accepted successfully.

Instance {} is now being stopped.

To check the status of your instance, you can run: 

```
form manage config --id {}
```
"#,
            "instance".bold().bright_cyan(),
            vm_id.bold().bright_yellow(),
            vm_id.bright_yellow(),
            );
        }
        QueueResponse::Failure { reason } => {
            if let Some(reason) = reason {
                println!(r#"
Unfortunately your stop request {} for the following reason:

{}

If the reason is missing, or unclear, please consider going to our project
discord at {} and going to the {} channel, submitting an {} on our project github at {}, 
or sending us a direct message on X at {}, and someone from our core team will gladly
help you out.
"#,
                "FAILED".white().on_bright_red(),
                reason.bright_red().on_black(),
                "discord.gg/formation".blue(),
                "chewing-glass".blue(),
                "issue".bright_yellow(),
                "http://github.com/formthefog/formation.git".blue(),
                "@formthefog".blue(),
                );
            }
        }
        _ => {
            println!(r#"
Something went {} wrong. The response received was {:?} which is an invalid response 
to the `{}` command.

Please consider doing one of the following: 

    1. Join our discord at {} and go to the {} channel and paste this response
    2. Submitting an {} on our project github at {} 
    3. Sending us a direct message on X at {}

Someone from our core team will gladly help you out.
"#,
            "terribly".bright_red().on_blue(),
            resp,
            "form manage stop".bright_yellow(),
            "discord.gg/formation".blue(),
            "chewing-glass".blue(),
            "issue".bright_yellow(),
            "http://github.com/formthefog/formation.git".blue(),
            "@formthefog".blue(),
            );
        }
    }
}

impl StopCommand {
    pub async fn handle(&self, provider: &str, vmm_port: u16) -> Result<VmmResponse, Box<dyn std::error::Error>> {
        // Validate inputs - need at least id or name
        let id = self.id.clone().ok_or("Instance ID is required when name is not provided")?;
        let name = self.name.clone().unwrap_or_else(|| id.clone());
        
        let request = StopVmRequest {
            id,
            name,
            signature: None,
            recovery_id: 0
        };
        
        Ok(reqwest::Client::new() 
            .post(&format!("http://{provider}:{vmm_port}/vm/stop"))
            .json(&request)
            .send()
            .await?
            .json::<VmmResponse>()
            .await?
        )
    }

    pub async fn handle_queue(&self, provider: &str, keystore: Option<Keystore>) -> Result<(), Box<dyn std::error::Error>> {
        // Validate inputs - need at least id or name
        let id = match (&self.id, &self.name) {
            (Some(id), _) => id.clone(),
            (None, Some(name)) => name.clone(),
            _ => return Err("Either instance ID or name must be provided".into())
        };
        
        let queue_request = self.prepare_stop_request_queue(&id, keystore).await?; 

        let resp = reqwest::Client::new() 
            .post(&format!("http://{provider}:{}/queue/write_local", QUEUE_PORT))
            .json(&queue_request)
            .send()
            .await?
            .json::<QueueResponse>()
            .await?;

        print_stop_queue_response(resp, &id);

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
        let signing_key = self.get_signing_key(keystore)?;
        
        let mut hasher = Sha3::v256();
        let mut message_hash = [0u8; 32];
        hasher.update(id.as_bytes());
        hasher.finalize(&mut message_hash);
        
        // Use k256::ecdsa for signing like other commands
        let (sig, rec) = signing_key.sign_recoverable(&message_hash).map_err(|e| e.to_string())?;
        
        Ok((hex::encode(&sig.to_vec()), rec, message_hash))
    }

    pub async fn prepare_stop_request_queue(&self, id: &str, keystore: Option<Keystore>) -> Result<QueueRequest, Box<dyn std::error::Error>> {
        let (signature, recovery_id, hash) = self.sign_request(id, keystore.clone())?;
        
        let stop_vm_request = StopVmRequest {
            id: id.to_string(),
            name: self.name.clone().unwrap_or_else(|| id.to_string()),
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
        let mut message_code = vec![3]; // Code 3 for stop operation (as seen in handle_message in API)
        message_code.extend(serde_json::to_vec(&stop_vm_request)?);

        let queue_request = QueueRequest::Write {
            content: message_code,
            topic: hex::encode(topic_hash)
        };

        Ok(queue_request)
    }
}
