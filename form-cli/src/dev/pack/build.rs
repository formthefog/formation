use clap::Args;
use std::path::PathBuf;
use reqwest::{Client, multipart::Form};
use form_pack::{
    formfile::{BuildInstruction, Formfile, FormfileParser}, 
    manager::PackResponse
};
use form_pack::pack::Pack;
use crate::{default_context, default_formfile};


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
    #[cfg(any(feature = "testnet", feature = "mainnet"))]
    #[clap(long, short)]
    pub private_key: Option<String>,
    /// An altenrative to private key or mnemonic. If you have a keyfile
    /// stored locally, you can use the keyfile to read in your private key
    //TODO: Add support for HSM and other Enclave based key storage
    #[cfg(any(feature = "testnet", feature = "mainnet"))]
    #[clap(long, short)]
    pub keyfile: Option<String>,
    /// An alternative to private key or keyfile. If you have a 12 or 24 word 
    /// BIP39 compliant mnemonic phrase, you can use it to derive the signing
    /// key for this request
    //TODO: Add support for HSM and other Enclave based key storage
    #[cfg(any(feature = "mainnet", feature = "testnet"))]
    #[clap(long, short)]
    pub mnemonic: Option<String>,
}

impl BuildCommand {
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

#[cfg(any(feature = "mainnet", feature = "testnet"))]
impl BuildCommand {
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
