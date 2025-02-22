use clap::Args;
use colored::Colorize;
use form_types::state::{Response as StateResponse, Success};
use form_state::instances::Instance;
use reqwest::Client;
use tabled::{Table, Tabled, settings::Style};
use std::collections::HashMap;

/// Acquires the status of a build and its instances.
#[derive(Debug, Clone, Args)]
pub struct StatusCommand {
    /// This is the build ID that you received as part of the response
    /// from the `form pack build` command.
    /// If you lost it you can call `form pack get-build-id` from the 
    /// build context directory (the same directory as your Formfile)
    /// and `form` will derive it from your formfile and your
    /// signing key.
    #[clap(long="build-id", short='i')]
    build_id: String
}

#[derive(Tabled)]
struct InstanceStatus {
    #[tabled(rename = "Instance ID")]
    instance_id: String,
    #[tabled(rename = "Host")]
    host: String,
    #[tabled(rename = "Status")]
    status: String,
    #[tabled(rename = "vCPUs")]
    vcpus: String,
    #[tabled(rename = "Memory")]
    memory: String,
    #[tabled(rename = "Region")]
    region: String,
    #[tabled(rename = "Network")]
    network: String,
}

impl StatusCommand {
    pub async fn handle_status(&self, provider: String, port: u16) -> Result<(), Box<dyn std::error::Error>> {
        let status = Client::new()
            .get(&format!("http://{provider}:{port}/instance/{}/get_by_build_id", self.build_id))
            .send().await?
            .json::<StateResponse<Instance>>()
            .await?;

        print_pack_status(status, self.build_id.clone());

        Ok(())
    }
}

pub fn print_pack_status(status: StateResponse<Instance>, build_id: String) {
    match status {
        StateResponse::Success(Success::List(instances)) => {
            let n = instances.len();
            
            // Create status table
            let status_entries: Vec<InstanceStatus> = instances.iter().map(|inst| {
                let network_info = if let Some(ip) = inst.formnet_ip {
                    format!("{}", ip)
                } else {
                    "Not assigned".to_string()
                };

                InstanceStatus {
                    instance_id: inst.instance_id[..8].to_string(),
                    host: inst.node_id[..8].to_string(),
                    status: inst.status.to_string(),
                    vcpus: format!("{}", inst.resources.vcpus),
                    memory: format!("{} MB", inst.resources.memory_mb),
                    region: if inst.host_region.is_empty() { 
                        "default".to_string() 
                    } else { 
                        inst.host_region.clone() 
                    },
                    network: network_info,
                }
            }).collect();

            // Group instances by status
            let mut status_groups: HashMap<String, Vec<&InstanceStatus>> = HashMap::new();
            for entry in &status_entries {
                status_groups.entry(entry.status.clone())
                    .or_insert_with(Vec::new)
                    .push(entry);
            }

            println!("\n{} {}\n",
                "Build Status for".bold(),
                build_id.bright_yellow());

            println!("{} {} {}\n",
                "→".bright_blue(),
                n.to_string().bright_blue(),
                format!("instance{} found", if n == 1 { "" } else { "s" }).bold());

            // Print status table
            let mut table = Table::new(&status_entries);
            table.with(Style::modern());
            println!("{table}\n");

            // Print contextual help based on status
            if status_groups.contains_key("Building") {
                println!("{}\n{}\n",
                    "🔄 Build in Progress".bright_yellow(),
                    "   Run this command again to check for updates.".dimmed());
            }

            if status_groups.contains_key("Built") || status_groups.contains_key("Created") {
                println!("{}\n{}\n{}\n",
                    "✨ Ready to Ship".bright_green(),
                    "   To deploy your instances, run:".dimmed(),
                    "   form pack ship".bright_blue());
            }

            // Show SSH instructions if any instance has an IP
            let has_ips = instances.iter().any(|inst| inst.formnet_ip.is_some());
            if has_ips {
                println!("{}\n",
                    "🔑 SSH Access".bright_green());
                
                for inst in instances.iter() {
                    if let Some(ip) = inst.formnet_ip {
                        println!("   For instance {}:\n   {}\n",
                            inst.instance_id[..8].bright_yellow(),
                            format!("ssh {}@{}", inst.instance_owner, ip).bright_blue());
                    }
                }
            }

            if status_groups.contains_key("Started") {
                println!("{}\n{}\n{}\n",
                    "🚀 Instances Running".bright_green(),
                    "   To get updated formnet IP addresses, run:".dimmed(),
                    "   form manage get-ip".bright_blue());
            }

            if status_groups.contains_key("Failed") {
                println!("{}\n{}\n",
                    "❌ Build Failed".bright_red(),
                    "   Please check the logs for more information.".dimmed());
            }
        }
        StateResponse::Failure { reason } => {
            println!("\n{} {} {}\n",
                "❌".bright_red(),
                "Failed to get status for build".bold(),
                build_id.bright_yellow());

            if let Some(error) = reason {
                println!("{}: {}\n",
                    "Error".bright_red(),
                    error.bright_yellow());
            }

            println!("Need help? Try these resources:");
            println!("• Discord: {}", "discord.gg/formation".underline().blue());
            println!("• GitHub: {}", "github.com/formthefog/formation".underline().blue());
            println!("• Twitter: {}\n", "@formthefog".underline().blue());
        }
        _ => {
            println!("\n{} Something went wrong while fetching the status.\n",
                "❌".bright_red());
            
            println!("Need help? Try these resources:");
            println!("• Discord: {}", "discord.gg/formation".underline().blue());
            println!("• GitHub: {}", "github.com/formthefog/formation".underline().blue());
            println!("• Twitter: {}\n", "@formthefog".underline().blue());
        }
    }
}
