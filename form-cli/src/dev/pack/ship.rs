use std::path::PathBuf;
use alloy_core::primitives::Address;
use alloy_signer_local::{coins_bip39::English, MnemonicBuilder};
use clap::Args;
use colored::Colorize;
use form_p2p::queue::{QueueRequest, QueueResponse, QUEUE_PORT};
use form_pack::formfile::{Formfile, FormfileParser};
use form_types::{CreateVmRequest, VmmResponse};
use k256::ecdsa::{RecoveryId, Signature, SigningKey, VerifyingKey};
use tiny_keccak::{Hasher, Sha3};
use crate::{default_context, default_formfile, Keystore};


#[derive(Debug, Clone,  Args)]
pub struct ShipCommand {
    /// Path to the context directory (e.g., . for current directory)
    /// This should be the directory containing the Formfile and other artifacts
    /// however, you can provide a path to the Formfile.
    #[clap(default_value_os_t = default_context())]
    pub context_dir: PathBuf,
    /// The directory where the form pack artifacts can be found
    #[clap(long, short, default_value_os_t = default_formfile(default_context()))]
    pub formfile: PathBuf,
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

pub fn print_ship_queue_response(resp: QueueResponse) {
    match resp {
        QueueResponse::OpSuccess => {
            println!(r#"
Your {} is being processed, and was accepted successfully.

To check the status of your deployment, you can run: 

```
{}
```

This process typically takes a couple of minutes. 

Once your ip addresses return, you can `{}` into it ({}):

"#,
"deployment".bold().bright_cyan(),
"form [OPTIONS] manage get-ips <build-id>".bright_yellow(),
"ssh <username>@<formnet-ip>".bold().bright_green(),
"assuming you provided your ssh public key".bold().bright_yellow(),
);
        }
        QueueResponse::Failure { reason } => {
            if let Some(reason) = reason {
            println!(r#"
Unforutnately your build request {} for the following reason:

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
"form [OPTIONS] pack ship".bright_yellow(),
"discord.gg/formation".blue(),
"chewing-glass".blue(),
"issue".bright_yellow(),
"http://github.com/formthefog/formation.git".blue(),
"@formthefog".blue(),
);
        }
    }
}



impl ShipCommand {
    pub async fn handle(&mut self, provider: &str, vmm_port: u16, keystore: Option<Keystore>) -> Result<VmmResponse, Box<dyn std::error::Error>> {
        // Parse the formfile
        let mut parser = FormfileParser::new();
        let contents = std::fs::read_to_string(&self.formfile)?;
        let formfile = parser.parse(&contents)?;
        let formfile_string = serde_json::to_string(&formfile)?;
        
        // Generate signature for the request
        let (signature, recovery_id, hash) = self.sign_payload(keystore.clone())?;
        
        // Use the name derived from the signing key for consistency with the queue-based method
        let name = hex::encode(self.derive_name(&self.get_signing_key(keystore.clone())?)?);
        println!("Instance name: {name}");
        
        // Create the request with signature
        let request = CreateVmRequest {
            name,
            formfile: formfile_string,
            signature: Some(signature.clone()),
            recovery_id: recovery_id.to_byte() as u32
        };
        
        // Show user which address is signing the request
        let recovered_address = Address::from_public_key(
            &VerifyingKey::recover_from_msg(
                &hash, 
                &Signature::from_slice(&hex::decode(&signature)?)?,
                recovery_id
            )?
        );
        println!("Request will be signed by address: {recovered_address:x}");
        
        // Send the request
        Ok(reqwest::Client::new() 
            .post(&format!("http://{provider}:{vmm_port}/vm/create"))
            .json(&request)
            .send()
            .await?
            .json::<VmmResponse>()
            .await?
        )
    }

    pub async fn handle_queue(&mut self, provider: &str, keystore: Option<Keystore>) -> Result<(), Box<dyn std::error::Error>> {
        let queue_request = self.pack_ship_request_queue(keystore).await?; 

        let resp = reqwest::Client::new() 
            .post(&format!("http://{provider}:{}/queue/write_local", QUEUE_PORT))
            .json(&queue_request)
            .send()
            .await?
            .json::<QueueResponse>()
            .await?;

        print_ship_queue_response(resp);

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

    pub fn sign_payload(&mut self, keystore: Option<Keystore>) -> Result<(String, RecoveryId, [u8; 32]), String> {
        let signing_key = self.get_signing_key(keystore)?;
        let data = self.build_payload(&signing_key)?;
        let (sig, rec) = signing_key.sign_recoverable(&data).map_err(|e| e.to_string())?;
        Ok((hex::encode(&sig.to_vec()), rec, data))
    }

    pub fn derive_name(&mut self, signing_key: &SigningKey) -> Result<[u8; 32], String> {
        let address = Address::from_private_key(signing_key); 
        println!("signer address: {address:x}");
        let mut hasher = Sha3::v256();
        let formfile = self.parse_formfile()?;
        let mut name_hash = [0u8; 32];
        hasher.update(address.as_ref()); 
        hasher.update(formfile.name.as_bytes());
        hasher.finalize(&mut name_hash);
        Ok(name_hash)
    }

    pub fn build_payload(&mut self, signing_key: &SigningKey) -> Result<[u8; 32], String> {
        let name_hash = self.derive_name(signing_key)?;
        let mut hasher = Sha3::v256();
        let mut payload_hash = [0u8; 32];
        // Name is always Some(String) at this point
        hasher.update(&name_hash);
        hasher.update(self.parse_formfile()?.to_json().as_bytes());
        hasher.finalize(&mut payload_hash);
        Ok(payload_hash)
    }

    pub fn parse_formfile(&mut self) -> Result<Formfile, String> {
        let content = std::fs::read_to_string(
            self.formfile.clone()
        ).map_err(|e| e.to_string())?;
        let mut parser = FormfileParser::new();
        Ok(parser.parse(&content).map_err(|e| e.to_string())?)

    }

    pub async fn pack_ship_request_queue(&mut self, keystore: Option<Keystore>) -> Result<QueueRequest, Box<dyn std::error::Error>> {
        let (signature, recovery_id, hash) = self.sign_payload(keystore.clone())?;
        let name = hex::encode(self.derive_name(&self.get_signing_key(keystore)?)?);
        println!("Instance name: {name}");
        let create_vm_request = CreateVmRequest {
            name, 
            formfile: serde_json::to_string(&self.parse_formfile()?)?,
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

        println!("recovered address: {recovered_address:x}");

        let mut hasher = Sha3::v256();
        let mut topic_hash = [0u8; 32];
        hasher.update(b"vmm");
        hasher.finalize(&mut topic_hash);
        let mut message_code = vec![0];
        message_code.extend(serde_json::to_vec(&create_vm_request)?);

        let queue_request = QueueRequest::Write {
            content: message_code,
            topic: hex::encode(topic_hash)
        };

        Ok(queue_request)
    }
}
