use std::path::{Path, PathBuf};
use clap::Args;
use colored::Colorize;
use dialoguer::{theme::ColorfulTheme, Input, Select, Confirm, MultiSelect};
use form_pack::formfile::{Formfile, FormfileParser, BuildInstruction, Entrypoint, EntrypointBuilder, SystemConfigOpt, User, UserBuilder};
use std::fs;
use crate::Keystore;
use super::{BuildCommand, ShipCommand};

/// Interactive wizard to create and deploy an agent
#[derive(Debug, Clone, Args)]
pub struct WizardCommand {
    /// Output directory for the generated formfile
    #[clap(long, short)]
    pub output_dir: Option<PathBuf>,
}

/// Deployment method enum
pub enum DeploymentMethod {
    DockerCompose,
    DockerContainer,
    GitHubRepo,
}

impl WizardCommand {
    pub async fn handle(&self, provider: &str, formpack_port: u16, vmm_port: u16, keystore: Option<Keystore>) -> Result<(), Box<dyn std::error::Error>> {
        println!("\n{} {}\n", 
            "🧙".bright_green(), 
            "Formation Agent Deployment Wizard".bold().bright_green());
        
        println!("{}\n", "This wizard will help you create a formfile and deploy your agent.".dimmed());

        // Get the output directory
        let output_dir = match &self.output_dir {
            Some(dir) => dir.clone(),
            None => {
                let current_dir = std::env::current_dir()?;
                let default_dir = current_dir.to_string_lossy();
                
                let dir: String = Input::with_theme(&ColorfulTheme::default())
                    .with_prompt("Where would you like to save your formfile?")
                    .default(default_dir.to_string())
                    .interact_text()?;
                
                PathBuf::from(dir)
            }
        };

        // Create output directory if it doesn't exist
        if !output_dir.exists() {
            fs::create_dir_all(&output_dir)?;
        }

        // Gather basic information
        let agent_name: String = Input::with_theme(&ColorfulTheme::default())
            .with_prompt("Agent name")
            .validate_with(|input: &String| -> Result<(), &str> {
                if input.trim().is_empty() {
                    Err("Agent name cannot be empty")
                } else {
                    Ok(())
                }
            })
            .interact_text()?;

        let description: String = Input::with_theme(&ColorfulTheme::default())
            .with_prompt("Agent description (optional)")
            .allow_empty(true)
            .interact_text()?;

        // Get AI model requirements
        let needs_ai_model = Confirm::with_theme(&ColorfulTheme::default())
            .with_prompt("Does your agent require a specific AI model?")
            .default(false)
            .interact()?;

        let (model_required, model_id) = if needs_ai_model {
            let model_type_options = &["Required (agent will only work with this model)", "Preferred (agent works best with this model but can use others)"];
            let model_type_selection = Select::with_theme(&ColorfulTheme::default())
                .with_prompt("Model requirement type")
                .default(1)
                .items(model_type_options)
                .interact()?;
            
            let is_required = model_type_selection == 0;
            
            let model: String = Input::with_theme(&ColorfulTheme::default())
                .with_prompt("Model ID (e.g. claude-3-opus-20240229)")
                .validate_with(|input: &String| -> Result<(), &str> {
                    if input.trim().is_empty() {
                        Err("Model ID cannot be empty")
                    } else {
                        Ok(())
                    }
                })
                .interact_text()?;
            
            (is_required, Some(model))
        } else {
            (false, None)
        };

        // Get deployment method
        let deployment_options = &["Docker Compose", "Docker Container", "GitHub Repository"];
        let deployment_selection = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("How would you like to deploy your agent?")
            .default(0)
            .items(deployment_options)
            .interact()?;

        let deployment_method = match deployment_selection {
            0 => DeploymentMethod::DockerCompose,
            1 => DeploymentMethod::DockerContainer,
            2 => DeploymentMethod::GitHubRepo,
            _ => DeploymentMethod::DockerCompose,
        };

        // Get resource requirements
        let vcpus: u8 = Input::with_theme(&ColorfulTheme::default())
            .with_prompt("Number of vCPUs")
            .default(1)
            .validate_with(|input: &u8| -> Result<(), &str> {
                if *input < 1 || *input > 8 {
                    Err("vCPUs must be between 1 and 8")
                } else {
                    Ok(())
                }
            })
            .interact_text()?;

        let memory: usize = Input::with_theme(&ColorfulTheme::default())
            .with_prompt("Memory (MB)")
            .default(1024)
            .validate_with(|input: &usize| -> Result<(), &str> {
                if *input < 512 || *input > 32768 {
                    Err("Memory must be between 512MB and 32GB")
                } else {
                    Ok(())
                }
            })
            .interact_text()?;

        let disk: u16 = Input::with_theme(&ColorfulTheme::default())
            .with_prompt("Disk space (GB)")
            .default(10)
            .validate_with(|input: &u16| -> Result<(), &str> {
                if *input < 5 || *input > 100 {
                    Err("Disk space must be between 5GB and 100GB")
                } else {
                    Ok(())
                }
            })
            .interact_text()?;

        // Get user information
        let username: String = Input::with_theme(&ColorfulTheme::default())
            .with_prompt("Username for SSH access")
            .default("formation".to_string())
            .interact_text()?;

        let password: String = dialoguer::Password::with_theme(&ColorfulTheme::default())
            .with_prompt("Password for SSH access")
            .interact()?;

        let add_ssh_key = Confirm::with_theme(&ColorfulTheme::default())
            .with_prompt("Would you like to add an SSH public key?")
            .default(true)
            .interact()?;

        let ssh_keys = if add_ssh_key {
            let default_key_path = PathBuf::from(std::env::var("HOME")?).join(".ssh").join("id_rsa.pub");
            let default_key = if default_key_path.exists() {
                fs::read_to_string(&default_key_path).ok()
            } else {
                None
            };

            let ssh_key: String = match default_key {
                Some(key) => {
                    Input::with_theme(&ColorfulTheme::default())
                        .with_prompt("SSH public key")
                        .default(key)
                        .interact_text()?
                },
                None => {
                    Input::with_theme(&ColorfulTheme::default())
                        .with_prompt("SSH public key")
                        .interact_text()?
                }
            };

            vec![ssh_key]
        } else {
            vec![]
        };

        // Create deployment-specific instructions
        let (instructions, workdir) = match deployment_method {
            DeploymentMethod::DockerCompose => {
                // Get Docker Compose details
                let compose_file: String = Input::with_theme(&ColorfulTheme::default())
                    .with_prompt("Path to docker-compose.yml file (relative to current directory)")
                    .default("docker-compose.yml".to_string())
                    .interact_text()?;
                
                // Create COPY instruction for docker-compose.yml
                let mut instructions = Vec::new();
                instructions.push(BuildInstruction::Copy(
                    PathBuf::from(&compose_file),
                    PathBuf::from("/app/docker-compose.yml")
                ));
                
                // Install Docker and Docker Compose
                instructions.push(BuildInstruction::Run(
                    "apt-get update && apt-get install -y docker.io docker-compose".to_string()
                ));
                
                // Create an entrypoint to run docker-compose up
                instructions.push(BuildInstruction::Entrypoint(
                    EntrypointBuilder::new()
                        .command("docker-compose")
                        .args(vec!["-f".to_string(), "/app/docker-compose.yml".to_string(), "up".to_string()])
                        .build()
                ));
                
                (instructions, PathBuf::from("/app"))
            },
            DeploymentMethod::DockerContainer => {
                // Get Docker container details
                let image_name: String = Input::with_theme(&ColorfulTheme::default())
                    .with_prompt("Docker image name (e.g. username/repo:tag)")
                    .interact_text()?;
                
                let container_args: String = Input::with_theme(&ColorfulTheme::default())
                    .with_prompt("Additional docker run arguments (optional)")
                    .allow_empty(true)
                    .interact_text()?;
                
                // Create entrypoint to run docker
                let mut docker_args = vec!["run".to_string(), "--rm".to_string()];
                
                if !container_args.is_empty() {
                    docker_args.extend(container_args.split_whitespace().map(|s| s.to_string()));
                }
                
                docker_args.push(image_name);
                
                let mut instructions = Vec::new();
                
                // Install Docker
                instructions.push(BuildInstruction::Run(
                    "apt-get update && apt-get install -y docker.io".to_string()
                ));
                
                // Create an entrypoint to run docker
                instructions.push(BuildInstruction::Entrypoint(
                    EntrypointBuilder::new()
                        .command("docker")
                        .args(docker_args)
                        .build()
                ));
                
                (instructions, PathBuf::from("/app"))
            },
            DeploymentMethod::GitHubRepo => {
                // Get GitHub repo details
                let repo_url: String = Input::with_theme(&ColorfulTheme::default())
                    .with_prompt("GitHub repository URL (e.g. https://github.com/username/repo)")
                    .interact_text()?;
                
                let branch: String = Input::with_theme(&ColorfulTheme::default())
                    .with_prompt("Branch name")
                    .default("main".to_string())
                    .interact_text()?;
                
                let install_deps = Confirm::with_theme(&ColorfulTheme::default())
                    .with_prompt("Would you like to install dependencies?")
                    .default(true)
                    .interact()?;
                
                let mut instructions = Vec::new();
                
                // Install git and dependencies
                let mut install_cmd = "apt-get update && apt-get install -y git".to_string();
                
                if install_deps {
                    let dep_options = &["Python", "Node.js", "Rust", "Go"];
                    let selections = MultiSelect::with_theme(&ColorfulTheme::default())
                        .with_prompt("Select dependencies to install")
                        .items(dep_options)
                        .interact()?;
                    
                    for &i in selections.iter() {
                        match i {
                            0 => install_cmd.push_str(" python3 python3-pip"),
                            1 => install_cmd.push_str(" nodejs npm"),
                            2 => install_cmd.push_str(" rustc cargo"),
                            3 => install_cmd.push_str(" golang"),
                            _ => {}
                        }
                    }
                }
                
                instructions.push(BuildInstruction::Run(install_cmd));
                
                // Clone the repository
                instructions.push(BuildInstruction::Run(
                    format!("git clone -b {} {} /app", branch, repo_url)
                ));
                
                // Get run command
                let run_command: String = Input::with_theme(&ColorfulTheme::default())
                    .with_prompt("Command to run the agent (e.g. python app.py)")
                    .interact_text()?;
                
                let cmd_parts: Vec<&str> = run_command.split_whitespace().collect();
                
                if !cmd_parts.is_empty() {
                    instructions.push(BuildInstruction::Entrypoint(
                        EntrypointBuilder::new()
                            .command(cmd_parts[0])
                            .args(cmd_parts[1..].iter().map(|&s| s.to_string()).collect())
                            .build()
                    ));
                }
                
                (instructions, PathBuf::from("/app"))
            }
        };

        // Build the formfile content
        let mut formfile_content = String::new();
        
        // Add required fields
        formfile_content.push_str(&format!("NAME {}\n", agent_name));
        
        if !description.is_empty() {
            formfile_content.push_str(&format!("DESCRIPTION {}\n", description));
        }
        
        if let Some(model) = model_id {
            let prefix = if model_required { "required" } else { "preferred" };
            formfile_content.push_str(&format!("MODEL {}:{}\n", prefix, model));
        }
        
        // Add system resource configuration
        formfile_content.push_str(&format!("VCPU {}\n", vcpus));
        formfile_content.push_str(&format!("MEM {}\n", memory));
        formfile_content.push_str(&format!("DISK {}\n", disk));
        
        // Add user configuration
        let sudo = true; // Always give sudo access for now
        formfile_content.push_str(&format!("USER username:{} passwd:{} sudo:{}\n", 
            username, password, if sudo { "true" } else { "false" }));
        
        // Add workdir
        formfile_content.push_str(&format!("WORKDIR {}\n", workdir.display()));
        
        // Add build instructions
        for instruction in &instructions {
            match instruction {
                BuildInstruction::Run(cmd) => {
                    formfile_content.push_str(&format!("RUN {}\n", cmd));
                },
                BuildInstruction::Copy(from, to) => {
                    formfile_content.push_str(&format!("COPY {} {}\n", from.display(), to.display()));
                },
                BuildInstruction::Entrypoint(entrypoint) => {
                    let mut args_json = String::from("[");
                    args_json.push_str(&format!("\"{}\", ", entrypoint.command()));
                    for (i, arg) in entrypoint.args().iter().enumerate() {
                        args_json.push_str(&format!("\"{}\"", arg));
                        if i < entrypoint.args().len() - 1 {
                            args_json.push_str(", ");
                        }
                    }
                    args_json.push_str("]");
                    formfile_content.push_str(&format!("ENTRYPOINT {}\n", args_json));
                },
                _ => {}
            }
        }
        
        // Save the formfile
        let formfile_path = output_dir.join("Formfile");
        fs::write(&formfile_path, formfile_content)?;
        
        println!("\n{} {}\n", 
            "✅".bright_green(), 
            format!("Formfile created at {}", formfile_path.display()).bold().bright_green());
        
        // Ask if they want to build and deploy now
        let build_now = Confirm::with_theme(&ColorfulTheme::default())
            .with_prompt("Do you want to build and deploy your agent now?")
            .default(true)
            .interact()?;
        
        if build_now {
            println!("\n{} {}\n", 
                "🔄".bright_blue(), 
                "Building your agent...".bold());
            
            // Execute build command
            let build_cmd = BuildCommand {
                context_dir: output_dir.clone(),
                formfile: formfile_path.clone(),
                private_key: None,
                keyfile: None,
                mnemonic: None,
            };
            
            build_cmd.handle(provider, formpack_port, keystore.clone()).await?;
            
            // Check if they want to ship now
            let ship_now = Confirm::with_theme(&ColorfulTheme::default())
                .with_prompt("Do you want to deploy (ship) your agent now?")
                .default(true)
                .interact()?;
            
            if ship_now {
                println!("\n{} {}\n", 
                    "🚀".bright_blue(), 
                    "Deploying your agent...".bold());
                
                // Execute ship command
                let mut ship_cmd = ShipCommand {
                    context_dir: output_dir.clone(),
                    formfile: formfile_path.clone(),
                    private_key: None,
                    keyfile: None,
                    mnemonic: None,
                };
                
                ship_cmd.handle(provider, vmm_port, keystore).await?;
                
                println!("\n{} {}\n", 
                    "🎉".bright_green(), 
                    "Your agent has been successfully deployed!".bold().bright_green());
                
                println!("Use {} to get the IP addresses of your instances.",
                    "form manage get-ips <build-id>".bright_yellow());
            }
        }
        
        Ok(())
    }
} 