use clap::Subcommand;
use std::path::PathBuf;
use build::BuildCommand;
use ship::ShipCommand;
use validate::ValidateCommand;
use dry_run::DryRunCommand;
use status::StatusCommand;
use clap::Args;
use wizard::WizardCommand;

pub mod build;
pub mod validate;
pub mod ship;
pub mod dry_run;
pub mod status;
pub mod wizard;

pub use build::*;
pub use validate::*;
pub use ship::*;
pub use dry_run::*;
pub use status::*;
pub use wizard::*;

pub fn default_formfile(context: PathBuf) -> PathBuf {
    context.join("Formfile")
}

pub fn default_context() -> PathBuf {
    std::env::current_dir().unwrap_or_else(|_| ".".into())
}

#[derive(Debug, Clone, Subcommand)]
pub enum PackCommand {
    /// Builds a FormPack from a directory
    Build(BuildCommand),
    /// Gets the status of a particular build
    Status(StatusCommand),
    /// Validates a Formfile
    Validate(ValidateCommand),
    /// Ships a FormPack build to become an instance
    Ship(ShipCommand),
    /// Runs a dry run of the build process
    #[clap(name = "dry-run")]
    DryRun(DryRunCommand),
    /// Interactive wizard to create and deploy an agent
    #[clap(name = "wizard")]
    Wizard(WizardCommand),
}