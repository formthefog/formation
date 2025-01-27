use clap::Args;
use std::path::PathBuf;
use reqwest::Client;
use form_pack::image_builder::FormfileResponse;
use form_pack::formfile::{Formfile, FormfileParser};
use crate::{default_context, default_formfile};

#[derive(Debug, Clone, Args)]
pub struct DryRunCommand {
    /// Path to the context directory (e.g., . for current directory)
    /// This should be the directory containing the Formfile and other artifacts
    /// however, you can provide a path to the Formfile.
    #[clap(default_value_os_t = default_context())]
    pub context_dir: PathBuf,
    /// The endpoint to hit 
    #[clap(default_value="http://172.17.0.2:8080")]
    pub build_server: String,
    /// The directory where the form pack artifacts can be found
    #[clap(long, short, default_value_os_t = default_formfile(default_context()))]
    pub formfile: PathBuf,
}

impl DryRunCommand {
    pub async fn handle(mut self) -> Result<FormfileResponse, Box<dyn std::error::Error>> {
        let formfile = self.parse_formfile()?;
        let resp = Client::new()
            .post(&format!("{}/formfile", self.build_server))
            .json(&formfile)
            .send()
            .await?
            .json()
            .await?;

        Ok(resp)
    }

    pub fn parse_formfile(&mut self) -> Result<Formfile, String> {
        let content = std::fs::read_to_string(
            self.formfile.clone()
        ).map_err(|e| e.to_string())?;
        let mut parser = FormfileParser::new();
        Ok(parser.parse(&content).map_err(|e| e.to_string())?)

    }
}

