use clap::{Parser, Subcommand};
use form_cli::{create::CreateCommmand, delete::DeleteCommand, info::GetCommand, start::StartCommand, stop::StopCommand};

#[derive(Debug, Parser)]
pub struct Form {
    #[clap(long, short, default_value="127.0.0.1:3001")]
    provider: String, 
    #[clap(subcommand)]
    pub command: FormCommand 
}

#[derive(Debug, Subcommand)]
pub enum FormCommand {
    Wallet,
    Create(CreateCommmand),
    Start(StartCommand),
    Stop(StopCommand),
    Delete(DeleteCommand),
    Get(GetCommand),
    // TODO: make list specific to an owner/authorized user or org.
    List,
}

#[tokio::main]
async fn main() -> Result<(), String> {
    let parser = Form::parse();
    println!("{parser:?}");

    match parser.command {
        FormCommand::Create(create_command) => {
           let resp = create_command.handle(&parser.provider).await?;
           println!("Response: {resp:?}");
        }
        FormCommand::Wallet => {}
        _ => {}
    }

    Ok(())
}
