use clap::{Parser, Subcommand};
use form_cli::{
    PackCommand, 
    ManageCommand,
    WalletCommand,
};

#[derive(Debug, Parser)]
pub struct Form {
    #[clap(default_value="127.0.0.1")]
    provider: String, 
    #[clap(default_value="3003")]
    formpack_port: u16, 
    #[clap(default_value="3002")]
    vmm_port: u16,
    #[clap(subcommand)]
    pub command: FormCommand 
}

#[derive(Debug, Subcommand)]
pub enum FormCommand {
    #[clap(subcommand)]
    Wallet(WalletCommand),
    #[clap(subcommand)]
    Pack(PackCommand),
    #[clap(subcommand)]
    Manage(ManageCommand),
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
        _ => {}
    }

    Ok(())
}
