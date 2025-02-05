use alloy_core::primitives::Address;
use alloy_signer_local::{coins_bip39::English, MnemonicBuilder};
use clap::Args;
use form_p2p::queue::{QueueRequest, QueueResponse};
use k256::ecdsa::{RecoveryId, SigningKey};
use tiny_keccak::{Hasher, Sha3};
use std::path::PathBuf;
use reqwest::{Client, multipart::Form};
use form_pack::{
    formfile::{BuildInstruction, Formfile, FormfileParser}, 
    manager::{PackRequest, PackResponse}
};
use form_pack::pack::Pack;
use crate::{decrypt_file, default_context, default_formfile, Keystore};


/// Create a new instance
#[derive(Debug, Clone, Args)]
pub struct BuildCommand {
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

impl BuildCommand {
    pub async fn handle_queue(mut self, provider: &str, queue_port: u16, password: Option<&str>) -> Result<QueueResponse, Box<dyn std::error::Error>> {
        let request = self.pack_build_request_queue(password).await?;
        let resp: QueueResponse = Client::new()
            .post(format!("http://{provider}:{queue_port}/queue/write_local"))
            .json(&request)
            .send()
            .await?
            .json()
            .await?;
        return Ok(resp)
    }
    pub async fn handle(mut self, provider: &str, formpack_port: u16) -> Result<PackResponse, Box<dyn std::error::Error>> {
        let form = self.pack_build_request().await?;
        println!("Successfully built multipart Form, sending to server");
        let resp: PackResponse = Client::new()
            .post(format!("http://{provider}:{formpack_port}/build"))
            .multipart(form)
            .send()
            .await?
            .json()
            .await?;

        Ok(resp)
    }

    pub async fn pack_build_request_queue(&mut self, password: Option<&str>) -> Result<QueueRequest, Box<dyn std::error::Error>> {
        let artifacts_path = self.build_pack()?;
        let artifact_bytes = std::fs::read(artifacts_path)?;
        let (signature, recovery_id) = self.sign_payload(password)?;
        let pack_request = PackRequest {
            name: hex::encode(self.derive_name(&self.get_signing_key(password)?)?), 
            formfile: self.parse_formfile()?,
            artifacts: artifact_bytes, 
            signature,
            recovery_id: recovery_id.to_byte()
        };

        let mut hasher = Sha3::v256();
        let mut topic_hash = [0u8; 32];
        hasher.update(b"pack");
        hasher.finalize(&mut topic_hash);
        let mut message_code = vec![0];
        message_code.extend(serde_json::to_vec(&pack_request)?);

        let queue_request = QueueRequest::Write {
            content: message_code,
            topic: topic_hash
        };

        Ok(queue_request)
    }

    pub async fn pack_build_request(&mut self) -> Result<Form, String> {
        println!("Building metadata for FormPack Build Request...");
        let metadata = serde_json::to_string(
            &self.parse_formfile()?
        ).map_err(|e| e.to_string())?;

        let artifacts_path = self.build_pack()?;
        println!("Returing multipart form...");
        Ok(Form::new()
            .text("metadata", metadata)
            .file("artifacts", artifacts_path).await.map_err(|e| e.to_string())?
        )
    }

    pub fn parse_formfile(&mut self) -> Result<Formfile, String> {
        let content = std::fs::read_to_string(
            self.formfile.clone()
        ).map_err(|e| e.to_string())?;
        let mut parser = FormfileParser::new();
        Ok(parser.parse(&content).map_err(|e| e.to_string())?)

    }

    pub fn build_pack(&mut self) -> Result<PathBuf, String> {
        println!("Parsing Formfile...");
        let pack = Pack::new(self.context_dir.clone()).map_err(|e| e.to_string())?;
        println!("Gathering Copy Instructions...");
        let copy_instructions = self.parse_formfile()?.build_instructions.iter().filter_map(|inst| {
            match inst {
                BuildInstruction::Copy(to, from) => Some((to.clone(), from.clone())),
                _ => None
            }
        }).collect::<Vec<(PathBuf, PathBuf)>>();
        println!("Preparing artifacts...");
        pack.prepare_artifacts(&copy_instructions).map_err(|e| e.to_string())
    } 
}

impl BuildCommand {
    pub fn get_signing_key(&self, password: Option<&str>) -> Result<SigningKey, String> {
        if let Some(pk) = &self.private_key {
            Ok(SigningKey::from_slice(
                    &hex::decode(pk)
                        .map_err(|e| e.to_string())?
                ).map_err(|e| e.to_string())?
            )
        } else if let Some(kf) = &self.keyfile {
            let kp = {
                let keypair: Keystore = serde_json::from_str(
                    &std::fs::read_to_string(kf).map_err(|e| e.to_string())?
                ).map_err(|e| e.to_string())?;
                let decrypted_pk = if let Some(pw) = password {
                    let bytes = decrypt_file(
                        &hex::decode(keypair.secret_key).map_err(|e| e.to_string())?, 
                        pw
                    ).map_err(|e| e.to_string())?;
                    bytes
                } else {
                    hex::decode(keypair.secret_key).map_err(|e| e.to_string())?
                };
                decrypted_pk
            };
            Ok(SigningKey::from_slice(
                    &hex::decode(kp)
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
    fn sign_payload(&mut self, password: Option<&str>) -> Result<(String, RecoveryId), String> {
        let signing_key = self.get_signing_key(password)?;
        let data = self.build_payload(&signing_key)?;
        let (sig, rec) = signing_key.sign_recoverable(&data).map_err(|e| e.to_string())?;
        Ok((hex::encode(&sig.to_vec()), rec))
    }

    fn derive_name(&mut self, signing_key: &SigningKey) -> Result<[u8; 32], String> {
        let address = Address::from_private_key(signing_key); 
        let mut hasher = Sha3::v256();
        let formfile = self.parse_formfile()?;
        let mut name_hash = [0u8; 32];
        hasher.update(address.as_ref()); 
        hasher.update(formfile.name.as_bytes());
        hasher.finalize(&mut name_hash);
        Ok(name_hash)
    }

    fn build_payload(&mut self, signing_key: &SigningKey) -> Result<Vec<u8>, String> {
        let name_hash = self.derive_name(signing_key)?;
        let mut hasher = Sha3::v256();
        let mut payload_hash = [0u8; 32];
        // Name is always Some(String) at this point
        hasher.update(&name_hash);
        hasher.update(self.parse_formfile()?.to_json().as_bytes());
        hasher.finalize(&mut payload_hash);
        Ok(payload_hash.to_vec())
    }
}
