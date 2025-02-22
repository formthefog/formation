use std::path::PathBuf;
use clap::Args;
use colored::Colorize;
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
        let formfile = parser.parse(&content).map_err(|e| e.to_string())?;

        Ok(format!("\n{} {}\n\n{}\n{}\n\n{}\n{}\n{}\n\n{}\n{}\n",
            "✨".bright_green(),
            "Formfile validation successful!".bold().bright_green(),
            
            "📦 Build Configuration:".bold(),
            format!("   • Name: {}", formfile.name).dimmed(),

            "🚀 Next Steps:".bold(),
            "   Run this command to build:".dimmed(),
            format!("   {} {}", "form pack build".bright_blue(), ".".bright_blue()),

            "💡 Tip:".bold(),
            "   Run from the same directory as your Formfile".dimmed()
        ))
    }
}
