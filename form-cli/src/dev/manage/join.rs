use alloy_core::primitives::Address;
use alloy_signer_local::{coins_bip39::English, MnemonicBuilder};
use clap::Args;
use colored::Colorize;
use formnet::user_join_formnet;
use k256::ecdsa::SigningKey;
use std::{path::PathBuf, process::Command};
use crate::{default_context, default_formfile, Keystore};


/// Create a new instance
#[derive(Debug, Clone, Args)]
pub struct JoinCommand {
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

impl JoinCommand {
    pub async fn handle_join_command(
        &self,
        provider: String,
        keystore: Keystore,
        publicip: Option<String>
    ) -> Result<(), Box<dyn std::error::Error>> {
        let address = hex::encode(Address::from_private_key(&self.get_signing_key(Some(keystore))?));
        user_join_formnet(address, provider, publicip).await?;
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
}

#[derive(Debug, Clone, Args)]
pub struct FormnetUp;

impl FormnetUp {
    pub fn handle_formnet_up(&self) -> Result<(), Box<dyn std::error::Error>> {
        let _child = Command::new("nohup")
            .arg("formnet-up")
            .stdout(std::fs::File::create(".formnet.log")?)
            .stderr(std::fs::File::create(".formnet-errors.log")?)
            .spawn()?;

        println!(
r#"
{} has been brought up and is being refreshed every {} to find {} and update {} 
"#,
"formnet".bold().bright_yellow(),
"60 seconds".bold().bright_blue(),
"new peers".bold().bright_magenta(),
"existing peers".bold().bright_magenta(),
);

        Ok(())
    }
}
