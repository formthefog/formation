use std::path::PathBuf;
use clap::Subcommand;
use anyhow::Result;
use dialoguer::{theme::ColorfulTheme, Confirm, Input};
use serde::{Serialize, Deserialize};
use form_config::*;

#[derive(Clone, Debug, Subcommand, Serialize, Deserialize)]
pub enum Operator {
    Config
}

pub fn operator_config() -> Result<()> {
    let config = run_config_wizard()?;
    
    // Ask about configuration save location
    let theme = ColorfulTheme::default();
    let default_config_path = PathBuf::from("./secrets/.operator-config.json");
    
    let use_default_path = Confirm::with_theme(&theme)
        .with_prompt(format!("Save config to {}?", default_config_path.display()))
        .default(true)
        .interact()?;

    let config_path = if use_default_path {
        std::fs::create_dir_all(default_config_path.parent().unwrap())?;
        default_config_path
    } else {
        let path: String = Input::with_theme(&theme)
            .with_prompt("Enter config file path")
            .interact_text()?;
        PathBuf::from(path)
    };

    // Ask about key encryption if keys are present
    let encrypt_keys = if config.secret_key.is_some() || config.mnemonic.is_some() {
        Confirm::with_theme(&theme)
            .with_prompt("Would you like to encrypt your keys in the keystore?")
            .default(true)
            .interact()?
    } else {
        false
    };

    save_config_and_keystore(&config, &config_path, encrypt_keys)?;
    Ok(())
}
