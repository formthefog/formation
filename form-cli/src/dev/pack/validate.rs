use clap::Args;

#[derive(Debug, Args)]
pub struct ValidateCommand;

impl ValidateCommand {
    pub async fn handle(&self) -> Result<String, String> {
        todo!()
    }
}
