use std::{fmt::Debug, path::PathBuf};
use clap::Args;
use colored::Colorize;
use form_types::state::{Response, Success};
use reqwest::Client;
use serde_json::json;
use dialoguer::{theme::ColorfulTheme, Confirm};

use crate::{default_context, default_formfile};

/// Remove a domain record
#[derive(Debug, Clone, Args)]
pub struct RemoveCommand {
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
    /// The domain name you want to remove
    #[clap(long="domain", short='d')]
    pub domain_name: String,
    /// Skip confirmation prompt
    #[clap(long="yes", short='y', default_value_t=false)]
    pub skip_confirmation: bool,
}

pub fn print_remove_response(domain_name: String) {
    println!(r#"
The domain {} has been successfully removed.

The changes may take a few minutes to fully propagate through the system.
"#,
    domain_name.blue()
    );
}

pub fn print_remove_invalid_response<T: Debug>(r: Success<T>) {
    println!(r#"
Something went {} wrong. Received {} which is not a
valid response format for a domain removal.
"#,
    "terribly".bold().bright_red(),
    format!("{:?}", r).blue()
    );
}

pub fn print_remove_failure(reason: Option<String>) {
    println!(r#"

Sadly, the request to remove the DNS record failed.

Reason: {}

If you're not sure what to do from here, please consider doing one of the following:

    1. Join our discord at {} and go to the {} channel and paste this response
    2. Submitting an {} on our project github at {} 
    3. Sending us a direct message on X at {}

Someone from our core team will gladly help you out.
"#,
    if let Some(r) = reason { r.bold().bright_red() } else { "none".bold().bright_red() },
    "discord.gg/formation".blue(),
    "chewing-glass".blue(),
    "issue".bright_yellow(),
    "http://github.com/formthefog/formation.git".blue(),
    "@formthefog".blue(),
    )
}

impl RemoveCommand {
    pub async fn handle_remove_command(&self, provider: String) -> Result<(), Box<dyn std::error::Error>> {
        let domain = self.domain_name.clone();
        
        // Confirm deletion unless --yes flag is used
        if !self.skip_confirmation {
            let confirm = Confirm::with_theme(&ColorfulTheme::default())
                .with_prompt(format!("Are you sure you want to remove the domain {}? This action cannot be undone.", domain))
                .default(false)
                .interact()?;
            
            if !confirm {
                println!("Operation cancelled.");
                return Ok(());
            }
        }
        
        // Construct the request to the /record/{domain}/delete endpoint
        let endpoint = format!("http://{provider}:3004/record/{domain}/delete");
        
        // Send the request
        let resp = Client::new()
            .delete(&endpoint)
            .send().await?
            .json::<Response<String>>().await?;

        // Handle the response
        match resp {
            Response::Success(Success::None) | Response::Success(Success::Some(_)) => {
                print_remove_response(domain);
            },
            Response::Success(other) => {
                print_remove_invalid_response(other);
            },
            Response::Failure { reason } => {
                print_remove_failure(reason);
            }
        }

        Ok(())
    }
}
