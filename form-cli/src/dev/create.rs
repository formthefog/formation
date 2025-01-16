use std::{fs::OpenOptions, io::Read, path::{Path, PathBuf}};
use alloy::signers::k256::ecdsa::{RecoveryId, SigningKey}; 
use alloy_signer_local::{coins_bip39::English, MnemonicBuilder};
use clap::{Args, Subcommand};
use form_pack::formfile::{Formfile, FormfileParser};
use form_types::{CreateVmRequest, VmResponse};
use random_word::Lang;
use reqwest::Client;
use serde::{Serialize, Deserialize};
use sha3::{Sha3_256, Digest};
use vmm_service::util::default_formfile;

#[derive(Debug, Serialize, Deserialize)]
pub struct Keypair {
    signing_key: String,
    verifying_key: String,
}

#[derive(Debug, Subcommand)]
pub enum PackCommand {
    Build(BuildCommand),
    Validate(ValidateCommand),
    Ship(ShipCommand),
}

/// Create a new instance
#[derive(Debug, Clone, Args)]
pub struct BuildCommand {
    #[clap(long, short, default_value_os_t = default_formfile())]
    pub formfile: PathBuf,
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

impl BuildCommand {
    pub async fn handle(mut self, provider: &str) -> Result<VmResponse, String> {
        //TODO: Replace with gRPC call
        let resp = Client::new().post(provider).json(
            &self.to_request()?
        ).send().await.map_err(|e| e.to_string())?;
        Ok(resp.json().await.map_err(|e| e.to_string())?)
    }

    pub fn to_request(&mut self) -> Result<CreateVmRequest, String> {
        let signing_key: SigningKey = self.get_signing_key()?;
        self.name = Some(self.name.take().ok_or_else(|| {
            format!("{}_{}", random_word::gen(random_word::Lang::En), random_word::gen(random_word::Lang::En))
        })?);

        let (sig, rec) = self.sign_payload(signing_key)?;
        let formfile = self.parse_formfile()?;
        let name = self.name.clone().unwrap_or_else(|| {
            format!("{}_{}", random_word::gen(Lang::En), random_word::gen(Lang::En))
        });

        Ok(CreateVmRequest {
            formfile,
            name,
            signature: Some(sig),
            recovery_id: rec.to_byte() as u32
        })
    }

    pub fn parse_formfile(&mut self) -> Result<Formfile, String> {
        let content = std::fs::read_to_string(
            self.formfile.clone()
        ).map_err(|e| e.to_string())?;
        let mut parser = FormfileParser::new();
        Ok(parser.parse(&content).map_err(|e| e.to_string())?)

    }

    pub fn get_signing_key(&self) -> Result<SigningKey, String> {
        if let Some(pk) = &self.private_key {
            Ok(SigningKey::from_slice(
                    &hex::decode(pk)
                        .map_err(|e| e.to_string())?
                ).map_err(|e| e.to_string())?
            )
        } else if let Some(kf) = &self.keyfile {
            let kp = Keypair::from_file(kf).map_err(|e| e.to_string())?;
            Ok(SigningKey::from_slice(
                    &hex::decode(kp.signing_key)
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

    fn sign_payload(&mut self, signing_key: SigningKey) -> Result<(String, RecoveryId), String> {
        let data = self.build_payload()?;
        let (sig, rec) = signing_key.sign_recoverable(&data).map_err(|e| e.to_string())?;
        Ok((hex::encode(&sig.to_vec()), rec))
    }

    fn build_payload(&mut self) -> Result<Vec<u8>, String> {
        let mut hasher = Sha3_256::new();
        // Name is always Some(String) at this point
        hasher.update(self.name.take().unwrap());
        hasher.update(self.parse_formfile()?.to_json());
        Ok(hasher.finalize().to_vec())
    }
}

#[derive(Debug, Args)]
pub struct ValidateCommand;
#[derive(Debug, Args)]
pub struct ShipCommand;

impl Keypair {
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self, Box<dyn std::error::Error>> {
        let mut buf = Vec::new();
        let mut file = OpenOptions::new()
            .read(true)
            .write(false)
            .open(path.as_ref())?;

        file.read_to_end(&mut buf)?;

        Ok(serde_json::from_slice(&buf)?)
    }
}
