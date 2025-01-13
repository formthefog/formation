use std::{fs::OpenOptions, io::Read, path::Path};
use alloy::signers::k256::ecdsa::{RecoveryId, SigningKey}; 
use alloy_signer_local::{coins_bip39::English, LocalSigner, MnemonicBuilder};
use clap::Args;
use form_types::{CreateVmRequest, VmResponse};
use reqwest::{Client, Response};
use serde::{Serialize, Deserialize};
use sha3::{Sha3_256, Digest};

#[derive(Debug, Serialize, Deserialize)]
pub struct Keypair {
    signing_key: String,
    verifying_key: String,
}

/// Create a new instance
#[derive(Debug, Clone, Args)]
pub struct CreateCommmand {
    /// The Linux Distro you would like your instance to use 
    #[clap(long, short, default_value="ubuntu")]
    pub distro: String,
    /// The version of the distro of your choice. If it is not a valid version 
    /// it will be rejected
    #[clap(long, short, default_value="22.04")]
    pub version: String,
    /// The amount of memory in megabytes you'd like to have allocated to your
    /// instance.
    #[clap(long, short='b', default_value_t=512)]
    pub memory_mb: u64,
    /// The number of virtual CPUs you would like to have allocated to your
    /// instance
    #[clap(long, short='c', default_value_t=1)]
    pub vcpu_count: u8,
    /// A human readable name you'd like your instance to have, if left
    /// blank, a random name will be assigned
    #[clap(long, short)]
    pub name: Option<String>,
    /// The path to a user-data.yaml file, must be compatible with cloud-init
    /// (see https://cloudinit.readthedocs.io/en/latest/reference/examples.html for examples)
    /// You can use the cloud-init-wizard command to build a valid custom cloud-init file.
    //TODO: Add feature to provide common config files like Dockerfile type formats and 
    // auto convert them to valid cloud-init user data
    #[clap(long, short)]
    pub user_data: Option<String>,
    /// The path to a meta-data.yaml file, must be compatible with cloud-init
    /// (see https://cloudinit.readthedocs.io/en/latest/reference/examples.html) 
    /// You can use the cloud-init-wizard command to build a valid custom cloud-init file.
    //TODO: Add feature to provide common config files like Dockerfile type formats and 
    // auto convert them to valid cloud-init user data
    #[clap(long, short='t')]
    pub meta_data: Option<String>,
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

impl CreateCommmand {
    pub async fn handle(mut self, provider: &str) -> Result<VmResponse, String> {
        //TODO: Replace with gRPC call
        let resp = Client::new().post(provider).json(
            &self.to_request()?
        ).send().await.map_err(|e| e.to_string())?;
        Ok(resp.json()?)
    }

    pub fn to_request(&mut self) -> Result<CreateVmRequest, String> {
        let signing_key: SigningKey = self.get_signing_key()?;
        let user_data = self.get_user_data();
        let meta_data = self.get_metadata();
        self.name = Some(self.name.take().ok_or_else(|| {
            format!("{}_{}", random_word::gen(random_word::Lang::En), random_word::gen(random_word::Lang::En))
        })?);
        let (sig, rec) = self.sign_payload(signing_key)?;
        Ok(CreateVmRequest {
            distro: self.distro.clone(),
            version: self.version.clone(),
            memory_mb: self.memory_mb,
            vcpu_count: self.vcpu_count,
            name: self.name.take().unwrap(),
            user_data,
            meta_data,
            signature: Some(sig),
            recovery_id: rec.to_byte() as u32
        })
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
        hasher.update(&self.distro);
        hasher.update(&self.version);
        // Name is always Some(String) at this point
        hasher.update(self.name.take().unwrap());
        hasher.update(self.memory_mb.to_be_bytes());
        hasher.update(self.vcpu_count.to_be_bytes());
        if let Some(user_data) = self.get_user_data() {
            hasher.update(user_data)
        }
        if let Some(meta_data) = self.get_metadata() {
            hasher.update(meta_data)
        }

        Ok(hasher.finalize().to_vec())
    }

    fn get_user_data(&self) -> Option<String> {
        if let Some(user_data) = &self.user_data {
            let mut buf = String::new();
            let mut file = OpenOptions::new()
                .read(true)
                .write(false)
                .open(user_data).ok()?;

            file.read_to_string(&mut buf).ok()?;
           return Some(buf)
        }

        None
    }

    fn get_metadata(&self) -> Option<String> {
        if let Some(meta_data) = &self.user_data {
            let mut buf = String::new();
            let mut file = OpenOptions::new()
                .read(true)
                .write(false)
                .open(meta_data).ok()?;

            file.read_to_string(&mut buf).ok()?;
           return Some(buf)
        }

        None
    }
}

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
