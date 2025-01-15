use clap::Args;

#[derive(Clone, Debug, Args)]
pub struct GetCommand {
    #[clap(long, short)]
    id: Option<String>,
    #[clap(long, short)]
    name: Option<String>,
}
