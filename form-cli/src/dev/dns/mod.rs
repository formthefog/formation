use clap::Subcommand;
use add::AddCommand;
use remove::RemoveCommand;
use update::UpdateCommand;
use verify::VerifyCommand;

pub mod add;
pub mod remove;
pub mod update;
pub mod verify;

#[derive(Debug, Clone, Subcommand)]
pub enum DnsCommand {
    Add(AddCommand),
    Remove(RemoveCommand),
    Update(UpdateCommand),
    Verify(VerifyCommand),
}
