use std::{fmt::Debug, path::PathBuf};
use clap::Args;
use colored::Colorize;
use form_types::state::Response;
use reqwest::Client;
use serde::{Serialize, Deserialize};
use serde_json::Value;
use dialoguer::{Confirm, theme::ColorfulTheme};

use crate::{default_context, default_formfile};

// Define structures to match the API's response format
#[derive(Debug, Deserialize)]
pub enum VerificationResult {
    Verified(FormDnsRecord),
    RequiresConfig(DnsConfiguration),
}

#[derive(Debug, Deserialize)]
pub struct FormDnsRecord {
    pub domain: String,
    pub verification_status: Option<String>,
    pub verification_timestamp: Option<u64>,
    // Other fields may be included but we don't need them for display
    #[serde(flatten)]
    pub other: std::collections::HashMap<String, Value>,
}

#[derive(Debug, Deserialize)]
pub struct DnsConfiguration {
    pub record_type: String,
    pub target: String,
    pub ttl: u32,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum DomainResponse {
    Success(Value),
    Failure(Option<String>),
    VerificationSuccess(VerificationResult),
    VerificationFailure(String),
}

/// Verify ownership of a domain name
#[derive(Debug, Clone, Args)]
pub struct VerifyCommand {
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
    
    /// The domain name you want to verify
    #[clap(long="domain", short='d')]
    pub domain_name: String,
    
    /// Check verification status instead of initiating verification
    #[clap(long, short)]
    pub check: bool,
    
    /// Skip confirmation prompts
    #[clap(long="yes", short='y', default_value_t=false)]
    pub skip_confirmation: bool,
}

pub fn print_verification_success(domain: String, record: &FormDnsRecord) {
    println!("\n✅ {}", format!("Domain '{}' is verified!", domain).green().bold());
    
    if let Some(timestamp) = record.verification_timestamp {
        let datetime = chrono::DateTime::<chrono::Utc>::from_timestamp(timestamp as i64, 0)
            .map(|dt| dt.format("%Y-%m-%d %H:%M:%S UTC").to_string())
            .unwrap_or_else(|| "Unknown time".to_string());
        
        println!("Verified at: {}", datetime);
    }
}

pub fn print_verification_instructions(domain: String, config: &DnsConfiguration) {
    println!("\n⚠️ {}", format!("Domain '{}' requires additional configuration.", domain).yellow().bold());
    println!("To verify domain ownership, please configure your DNS settings:");
    
    println!("Add a {} record with the following settings:", config.record_type);
    println!("  Domain: {}", domain);
    println!("  Value: {}", config.target);
    println!("  TTL: {} seconds", config.ttl);
    
    println!("\nAfter updating your DNS settings, run:");
    println!("  form dns verify --domain {} --check", domain);
}

pub fn print_verification_failure(reason: String) {
    println!(
r#"
{}

Sadly, the domain verification request failed.

Reason: {}

If you're not sure what to do from here, please consider doing one of the following:

    1. Join our discord at {} and go to the {} channel and paste this response
    2. Submitting an {} on our project github at {} 
    3. Sending us a direct message on X at {}

Someone from our core team will gladly help you out.
"#,
"VERIFICATION FAILED".red().bold(),
reason.bold().bright_red(),
"discord.gg/formation".blue(),
"chewing-glass".blue(),
"issue".bright_yellow(),
"http://github.com/formthefog/formation.git".blue(),
"@formthefog".blue(),
    )
}

impl VerifyCommand {
    pub async fn handle_verify_command(&self, provider: String) -> Result<(), Box<dyn std::error::Error>> {
        let domain = self.domain_name.clone();
        
        // If check flag is provided, check verification status
        if self.check {
            let endpoint = format!("http://{provider}:3004/record/{domain}/check_verification");
            
            // Send the request
            let resp = Client::new()
                .post(&endpoint)
                .send().await?
                .json::<DomainResponse>().await?;
                
            // Handle the response based on the specialized format
            match resp {
                DomainResponse::VerificationSuccess(VerificationResult::Verified(record)) => {
                    print_verification_success(domain, &record);
                },
                DomainResponse::VerificationSuccess(VerificationResult::RequiresConfig(config)) => {
                    print_verification_instructions(domain, &config);
                },
                DomainResponse::VerificationFailure(reason) => {
                    print_verification_failure(reason);
                },
                DomainResponse::Success(_) => {
                    println!("Unexpected success response format.");
                },
                DomainResponse::Failure(reason) => {
                    print_verification_failure(reason.unwrap_or_else(|| "Unknown error".to_string()));
                }
            }
            
            return Ok(());
        }
        
        // If not checking, initiate verification
        // Confirm verification unless --yes flag is used
        if !self.skip_confirmation {
            let confirm = Confirm::with_theme(&ColorfulTheme::default())
                .with_prompt(format!("Would you like to verify ownership of domain '{}'?", domain))
                .default(true)
                .interact()?;
            
            if !confirm {
                println!("Verification cancelled.");
                return Ok(());
            }
        }
        
        // Construct the request to the verification endpoint
        let endpoint = format!("http://{provider}:3004/record/{domain}/initiate_verification");
        
        // Send the request
        let resp = Client::new()
            .post(&endpoint)
            .send().await?
            .json::<DomainResponse>().await?;
            
        // Handle the response based on the specialized format
        match resp {
            DomainResponse::VerificationSuccess(VerificationResult::Verified(record)) => {
                print_verification_success(domain, &record);
            },
            DomainResponse::VerificationSuccess(VerificationResult::RequiresConfig(config)) => {
                print_verification_instructions(domain, &config);
            },
            DomainResponse::VerificationFailure(reason) => {
                print_verification_failure(reason);
            },
            DomainResponse::Success(_) => {
                println!("Unexpected success response format.");
            },
            DomainResponse::Failure(reason) => {
                print_verification_failure(reason.unwrap_or_else(|| "Unknown error".to_string()));
            }
        }
        
        Ok(())
    }
} 
