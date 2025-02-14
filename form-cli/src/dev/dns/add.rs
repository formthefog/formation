use std::path::PathBuf;
use clap::Args;
use colored::Colorize;
use form_dns::api::{DomainResponse, Success};
use form_p2p::queue::QueueResponse;

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
    /// The build id for the instances you want this domain to point to 
    #[clap(long="build-id", short='b')]
    pub build_id: String
}

pub fn print_add_response(
    resp: QueueResponse,
    dns_resp: DomainResponse,
    domain_name: String,
    build_id: String,
    _anycast_addr: String
) {
    match resp {
        QueueResponse::OpSuccess => {
            let (arecord, cname) = {
                match dns_resp {
                    DomainResponse::Success(Success::Some(record)) => {
                        let arecord = record.public_ip.last().unwrap().clone();
                        let cname = record.cname_target.clone();
                        (arecord, cname)
                    }
                    _ => {
                        return;
                    }
                }
            };
            println!(r#"
We've got great news! Your request to point {} to your instances based on {} was successful!

In order for this to fully take effect, there are a few things you will need to do that are
outside of our control.

First, and foremost, you need to point your domain name to one of our anycast addresses as
the A Record. This will ensure that all of your instances can be reached from the same
public IP address, and as a result of that, remain resilient, granting you the full benefits
of the formation network.

    A Record: {}
    CNAME Record: {}

After that it may take up to 24 hours for your domain name to propagate throughout
public DNS servers and point to your instance, unfortunately that is out of our
control at the moment.

"#,
domain_name.blue(),
build_id.blue(),
arecord.ip().to_string().blue(),
if let Some(record) = cname { record.blue() } else { "".to_string().blue() },
);
        }
        QueueResponse::Failure { reason: _ } => {}
        _ => {}
    }
}

impl AddCommand {
    pub async fn handle_add_command(
        &self, 
        _provider: String, 
        _port: u16
    ) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }
}
