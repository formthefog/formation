use clap::{Parser, Subcommand};
use form_cli::{create::PackCommand, delete::DeleteCommand, info::GetCommand, start::StartCommand, stop::StopCommand, WalletCommand};

#[derive(Debug, Parser)]
pub struct Form {
    #[clap(long, short, default_value="127.0.0.1:3001")]
    provider: String, 
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

#[derive(Debug, Subcommand)]
pub enum ManageCommand {
    Start(StartCommand),
    Stop(StopCommand),
    Delete(DeleteCommand),
    Get(GetCommand),
    List,
}

#[tokio::main]
async fn main() -> Result<(), String> {
    let parser = Form::parse();
    println!("{parser:?}");

    match parser.command {
        FormCommand::Pack(pack_command) => {
            match pack_command {
                PackCommand::Build(build_command) => {
                   let resp = build_command.handle(&parser.provider).await?;
                   println!("Response: {resp:?}");
                }
                _ => {}
            }
        }
        FormCommand::Wallet(command) => {}
        _ => {}
    }

    Ok(())
}
