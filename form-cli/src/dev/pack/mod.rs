use clap::Subcommand;
use std::path::PathBuf;

pub mod build;
pub mod validate;
pub mod ship;
pub mod dry_run;

pub use build::*;
pub use validate::*;
pub use ship::*;
pub use dry_run::*;

pub fn default_formfile(context: PathBuf) -> PathBuf {
    context.join("Formfile")
}

pub fn default_context() -> PathBuf {
    std::env::current_dir().unwrap_or_else(|_| ".".into())
}

#[derive(Debug, Clone, Subcommand)]
pub enum PackCommand {
    Build(BuildCommand),
    Validate(ValidateCommand),
    Ship(ShipCommand),
    DryRun(DryRunCommand),
}
