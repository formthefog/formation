use clap::Subcommand;
use add::AddCommand;
use remove::RemoveCommand;
use request::RequestCommand;
use update::UpdateCommand;


pub mod add;
pub mod request;
pub mod remove;
pub mod update; 

#[derive(Debug, Clone, Subcommand)]
pub enum DnsCommand {
    Add(AddCommand),
    Request(RequestCommand),
    Remove(RemoveCommand),
    Update(UpdateCommand),
}
