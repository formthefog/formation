use clap::{Subcommand, Args};

pub mod start; 
pub mod stop;
pub mod delete;
pub mod add;
pub mod rm;
pub mod commit;
pub mod config;
pub mod join;
pub mod account;

pub use start::StartCommand;
pub use stop::StopCommand;
pub use delete::DeleteCommand;
pub use add::AddCommand;
pub use rm::RemoveCommand;
pub use commit::CommitCommand;
pub use config::ConfigCommand;
pub use join::{JoinCommand, FormnetUp};
pub use account::TransferOwnershipCommand;

#[derive(Debug, Subcommand)]
pub enum ManageCommand {
    Start(StartCommand),
    Stop(StopCommand),
    Delete(DeleteCommand),
    #[clap(subcommand)]
    Add(AddCommand),
    #[clap(subcommand)]
    Rm(RemoveCommand),
    Commit(CommitCommand),
    Config(ConfigCommand),
    Join(JoinCommand),
    FormnetUp(FormnetUp),
    GetIp(GetIpCommand),
    Leave(LeaveCommand),
    /// Transfer ownership of an instance from one account to another
    TransferOwnership(TransferOwnershipCommand),
}


#[derive(Clone, Debug, Args)]
pub struct GetIpCommand {
    #[clap(long, short)]
    pub build_id: String
}

#[derive(Clone, Debug, Args)]
pub struct LeaveCommand;
