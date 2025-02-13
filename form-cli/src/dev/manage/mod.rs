use clap::Subcommand;

pub mod start; 
pub mod stop;
pub mod delete;
pub mod add;
pub mod rm;
pub mod commit;
pub mod config;
pub mod join;

pub use start::StartCommand;
pub use stop::StopCommand;
pub use delete::DeleteCommand;
pub use add::AddCommand;
pub use rm::RemoveCommand;
pub use commit::CommitCommand;
pub use config::ConfigCommand;
pub use join::JoinCommand;

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
}
