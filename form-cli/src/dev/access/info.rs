use clap::{Subcommand, Args};

#[derive(Clone, Debug, Subcommand)]
pub enum InfoCommand {
    Get(GetCommand),
}

#[derive(Clone, Debug, Args)]
pub struct GetCommand {
    #[clap(long, short)]
    id: Option<String>,
    #[clap(long, short)]
    name: Option<String>,
}
