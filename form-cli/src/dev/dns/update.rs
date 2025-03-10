use std::{fmt::Debug, path::PathBuf};
use clap::Args;
use colored::Colorize;
use form_types::state::{Response, Success};
use reqwest::Client;
use serde_json::json;

use crate::{default_context, default_formfile};

/// Update an existing domain record
#[derive(Debug, Clone, Args)]
pub struct UpdateCommand {
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
    /// The domain name you want to update
    #[clap(long="domain", short='d')]
    pub domain_name: String,
    /// The build id for the instances you want this domain to point to 
    #[clap(long="build-id", short='b')]
    pub build_id: String,
    /// Whether to enable TLS (HTTPS) for this domain
    #[clap(long="tls-enabled", short='t', default_value_t=false)]
    pub ssl_cert: bool,
    /// Whether to completely replace the existing record
    #[clap(long="replace", short='r', default_value_t=false)]
    pub replace: bool,
    /// Record type (A, AAAA, CNAME, etc.)
    #[clap(long="record-type", short='R', default_value="A")]
    pub record_type: String,
}

pub fn print_update_response(domain_name: String) {
    println!(r#"
Great news! Your domain {} has been successfully updated.

The changes may take a few minutes to fully propagate through the system.
You can check the status of your domain using:

    form dns get --domain {}

"#,
    domain_name.blue(),
    domain_name.blue()
    );
}

pub fn print_update_invalid_response<T: Debug>(r: Success<T>) {
    println!(r#"
Something went {} wrong. Received {} which is not a
valid response format for a domain update.
"#,
    "terribly".bold().bright_red(),
    format!("{:?}", r).blue()
    );
}

pub fn print_update_failure(reason: Option<String>) {
    println!(r#"

Sadly, the request to update the DNS record failed.

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


impl UpdateCommand {
    pub async fn handle_update_command(&self, provider: String) -> Result<(), Box<dyn std::error::Error>> {
        let domain = self.domain_name.clone();
        
        // Construct the request to the /record/{domain}/update endpoint
        let endpoint = format!("http://{provider}:3004/record/{domain}/update");
        
        // Create the update payload based on the DomainRequest::Update structure
        let update_payload = json!({
            "replace": self.replace,
            "record_type": self.record_type,
            "ip_addr": [],  // This will be populated by the server based on the build_id
            "cname_target": null,
            "ssl_cert": self.ssl_cert,
            "build_id": self.build_id  // Including build_id to let the server find the right IPs
        });

        // Send the request
        let resp = Client::new()
            .post(&endpoint)
            .json(&update_payload)
            .send().await?
            .json::<Response<String>>().await?;

        // Handle the response
        match resp {
            Response::Success(Success::Some(_)) => {
                print_update_response(domain);
            },
            Response::Success(other) => {
                print_update_invalid_response(other);
            },
            Response::Failure { reason } => {
                print_update_failure(reason);
            }
        }

        Ok(())
    }
}
