use std::path::PathBuf;
use clap::Args;
use form_pack::formfile::FormfileParser;
use form_types::{CreateVmRequest, VmmResponse};
use crate::{default_context, default_formfile};


#[derive(Debug, Args)]
pub struct ShipCommand {
    #[clap(default_value_os_t=default_formfile(default_context()))]
    formfile: PathBuf
}

impl ShipCommand {
    pub async fn handle(&self, provider: &str, vmm_port: u16) -> Result<VmmResponse, Box<dyn std::error::Error>> {
        let mut parser = FormfileParser::new();
        let contents = std::fs::read_to_string(&self.formfile)?;
        let formfile = parser.parse(&contents)?;
        let formfile_string = serde_json::to_string(&formfile)?;
        let name = formfile.name.clone();
        let request = CreateVmRequest {
            name,
            formfile: formfile_string,
            signature: None,
            recovery_id: 0
        };
        Ok(reqwest::Client::new() 
            .post(&format!("http://{provider}:{vmm_port}/vm/create"))
            .json(&request)
            .send()
            .await?
            .json::<VmmResponse>()
            .await?
        )
    }
}
