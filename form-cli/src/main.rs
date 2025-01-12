use clap::{Parser, Subcommand};
use form_cli::{create::CreateCommmand, delete::DeleteCommand, get::GetCommand, start::StartCommand, stop::StopCommand};

#[derive(Debug, Parser)]
pub struct Form {
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

fn main() {
    let parser = Form::parse();
    println!("{parser:?}");
}
