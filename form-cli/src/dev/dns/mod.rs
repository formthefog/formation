use clap::Subcommand;
use add::AddCommand;
use remove::RemoveCommand;
use update::UpdateCommand;

pub mod add;
pub mod remove;
pub mod update; 

#[derive(Debug, Clone, Subcommand)]
pub enum DnsCommand {
    Add(AddCommand),
    Remove(RemoveCommand),
    Update(UpdateCommand),
}
