use std::{fmt::Debug, path::PathBuf};
use clap::Args;
use colored::Colorize;
use form_types::state::{Response, Success};
use reqwest::Client;
use url::Host;

use crate::{default_context, default_formfile};

/// Create a new instance
#[derive(Debug, Clone, Args)]
pub struct AddCommand {
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
    /// The domain name you are requesting to be added to your instances
    #[clap(long="domain", short='d')]
    pub domain_name: String,
    #[clap(long="vanity", short='V', default_value_t=true)]
    pub vanity: bool,
    #[clap(long="public", default_value_t=false)]
    pub public: bool,
    /// The build id for the instances you want this domain to point to 
    #[clap(long="build-id", short='b')]
    pub build_id: String
}

pub fn print_add_response(
    ip_addrs: Vec<String>,
    cnames: Vec<String>,
    domain_name: String,
    build_id: String,
) {
println!(r#"
We've got great news! Your request to point {} to your instances based on {} was successful!

In order for this to fully take effect, there are a few things you will need to do that are
outside of our control.

First, and foremost, you need to point your domain name to your instance addresses as
the A Record. This will ensure that all of your instances can be reached from the same
public IP address, and as a result of that, remain resilient, granting you the full benefits
of the formation network.

    A Records: {}
    {}: {}

After that it may take up to 24 hours for your domain name to propagate throughout
public DNS servers and point to your instance, unfortunately that is out of our
control at the moment.

"#,
domain_name.blue(),
build_id.blue(),
ip_addrs.join(", ").to_string().bold().blue(),
if !cnames.is_empty() { "CNAME".blue() } else { "".blue() },
if !cnames.is_empty() { cnames.join(", ").to_string().bold().blue() } else { "".to_string().bold().blue() }
);
}

pub fn print_add_invalid_response<T: Debug>(r: Success<T>, endpoint: &str) {
println!(r#"
Something went {} wrong. Received {} which is not a
valid response for endoint: {}"
"#,
"terribly".bold().bright_red(),
format!("{:?}", r).blue(),
endpoint.underline().bright_blue()
);
}

pub fn print_add_failure(reason: Option<String>) {
println!(r#"

Sadly, the request to add a DNS record failed.

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

impl AddCommand {
    pub async fn handle_add_command(
        &self, 
        provider: String, 
    ) -> Result<(), Box<dyn std::error::Error>> {
        let domain = self.domain_name.clone();
        let build_id = self.build_id.clone();
        let endpoint = if !self.public {
            format!("http://{provider}:3004/dns/{domain}/{build_id}/request_vanity")
        } else {
            format!("http://{provider}:3004/dns/{domain}/{build_id}/request_public")
        };

        let resp = Client::new()
            .post(endpoint)
            .send().await?.json::<Response<Host>>().await?;

        match resp {
            Response::Success(
                Success::List(node_hosts)
            ) => {
                let mut ips = vec![];
                let mut cname = vec![];
                for host in node_hosts {
                    match host {
                        Host::Ipv4(ipv4) => {
                            ips.push(ipv4.to_string());
                        }
                        Host::Ipv6(ipv6) => {
                            ips.push(ipv6.to_string());
                        }
                        Host::Domain(cn) => {
                            cname.push(cn);
                        }
                    }
                }
                print_add_response(ips, cname, domain, build_id);
            }
            Response::Success(r) => {
                if !self.public {
                    print_add_invalid_response(r, "/dns/:domain/:build_id/request_vanity");
                } else {
                    print_add_invalid_response(r, "/dns/:domain/:build_id/request_public");
                }
            }
            Response::Failure { reason } => {
                print_add_failure(reason);
            }
        }

        Ok(())
    }
}
