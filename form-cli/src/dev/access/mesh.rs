use clap::Subcommand;

#[derive(Clone, Debug, Subcommand)]
pub enum MeshCommand {
    Join,
    Leave,
    Get,
    List,
}
