use clap::Subcommand;

pub mod mesh;
pub mod ssh;
pub mod info;
pub mod list;

pub use mesh::*;
pub use ssh::*;
pub use info::*;
pub use list::*;

#[derive(Debug, Subcommand)]
pub enum AccessCommands {
    #[clap(subcommand)]
    Mesh(MeshCommand),
    Ssh(SshCommand),
    #[clap(subcommand)]
    Info(InfoCommand),
    #[clap(subcommand)]
    List(ListCommand)
}
