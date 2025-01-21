use clap::{Parser, Subcommand};
use form_cli::{
    KitCommand, ManageCommand, PackCommand, WalletCommand
};

/// The official developer CLI for building, deploying and managing 
/// verifiable confidential VPS instances in the Formation network
#[derive(Debug, Parser)]
pub struct Form {
    /// The ip or domain name of the API provider 
    /// (currently a http api, will switch to gRPC for testnet)
    /// Default is local, however, the best way to get set up 
    /// with a valid provider is to run `form kit init`.
    /// This will set you up with a full developer kit
    /// will allow you to pick a provider, or get a database
    /// of providers and randomly select/rotate providers
    /// on subsequent calls, among other features.
    #[clap(default_value="127.0.0.1")]
    provider: String, 
    /// The port where form pack build gets sent to for the provider
    /// in the future, all request (build, ship, etc.) related to 
    /// building, deploying and managing instances will be handled
    /// via a single gRPC endpoint on the provider, and therefore
    /// this will be phased out. We highly suggest you use the defaults
    /// unless you have a provider that you know is reliable that is using
    /// a different port, in the case of domain based provider, ports may 
    /// not be necessary at all.
    #[clap(default_value="3003")]
    formpack_port: u16, 
    /// The port where form pack ship gets sent to for the provider
    /// Same caveats apply here
    #[clap(default_value="3002")]
    vmm_port: u16,
    /// The subcommand that will be called 
    #[clap(subcommand)]
    pub command: FormCommand 
}

#[derive(Debug, Subcommand)]
pub enum FormCommand {
    #[clap(subcommand)]
    Kit(KitCommand),
    #[clap(subcommand)]
    Wallet(WalletCommand),
    #[clap(subcommand)]
    Pack(PackCommand),
    #[clap(subcommand)]
    Manage(ManageCommand),
    // Add Kit
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let parser = Form::parse();
    match parser.command {
        FormCommand::Pack(pack_command) => {
            match pack_command {
                PackCommand::Build(build_command) => {
                    let resp = build_command.handle(&parser.provider, parser.formpack_port).await?;
                    println!("Response: {resp:?}");
                }
                PackCommand::DryRun(dry_run_command) => {
                    let resp = dry_run_command.handle().await?;
                    println!("Response: {resp:?}");
                }
                PackCommand::Validate(validate_command) => {
                    let resp = validate_command.handle().await?;
                    for line in resp.lines() {
                        println!("{line}")
                    }
                }
                PackCommand::Ship(ship_command) => {
                    let provider = parser.provider;
                    let vmm_port = parser.vmm_port;
                    let resp = ship_command.handle(&provider, vmm_port).await?;
                    println!("Response: {resp:?}");
                }
            }
        }
        FormCommand::Kit(kit_command) => {
            match kit_command {
                KitCommand::Init(mut init) => init.handle().await?,
                KitCommand::Load => {}
            }
        }
        _ => {}
    }

    Ok(())
}
