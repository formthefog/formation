use clap::Subcommand;

pub mod add;
pub mod request;
pub mod remove;
pub mod update; 

#[derive(Debug, Clone, Subcommand)]
pub enum PackCommand {
    Add(AddCommand),
    Request(RequestCommand),
    Remove(RemoveCommand),
    Update(UpdateCommand),
}
