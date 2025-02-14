use std::path::PathBuf;

use clap::Args;
use form_pack::formfile::FormfileParser;
use crate::{default_context, default_formfile};

#[derive(Debug, Clone, Args)]
pub struct ValidateCommand {
    #[clap(default_value_os_t=default_formfile(default_context()))]
    formfile: PathBuf
}

impl ValidateCommand {
    pub async fn handle(&self) -> Result<String, String> {
        let mut parser = FormfileParser::new();
        let content = std::fs::read_to_string(&self.formfile).map_err(|e| e.to_string())?;
        let _ = parser.parse(&content).map_err(|e| e.to_string());
        Ok(r#"
Congratulations! Your Formfile is valid!

    To build your Formpack run:

        `form pack build .`

    from the root directory of your project (same location as your Formfile)
        "#.to_string())
    }
}
