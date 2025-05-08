use tempfile::tempdir;
use std::io::Write;
use std::fs::OpenOptions;
use crate::formfile::Formfile;
use crate::types::request::PackBuildRequest;
use crate::monitor::FormPackMonitor;
use crate::manager::FormPackManager;
use crate::helpers::queue::write::{write_pack_status_completed, write_pack_status_failed, write_pack_status_started};

pub async fn handle_pack_request(manager: &mut FormPackManager, message: PackBuildRequest) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let node_id = manager.node_id.clone();
    
    // First check if we're responsible for this workload using the capability matcher
    println!("Checking if this node is responsible for handling the workload...");
    let formfile = &message.request.formfile;
    let build_id = hex::encode(message.hash);
    
    // Create the capability matcher
    let capability_matcher = crate::capability_matcher::CapabilityMatcher::new(None);
    
    // Check if we're responsible for this workload
    match capability_matcher.is_local_node_responsible(formfile, &node_id, &build_id).await {
        Ok(is_responsible) => {
            if !is_responsible {
                let reason = "Node is not responsible for this workload according to the capability matcher".to_string();
                println!("{}", reason);
                write_pack_status_failed(&message, reason).await?;
                return Ok(());
            }
            println!("Node is responsible for this workload, proceeding with build...");
        },
        Err(e) => {
            let reason = format!("Failed to determine if node is responsible: {}", e);
            println!("{}", reason);
            write_pack_status_failed(&message, reason).await?;
            return Ok(());
        }
    }
    
    // If we get here, we're responsible for the workload
    write_pack_status_started(&message, node_id).await?;
    let packdir = tempdir()?;

    println!("Created temporary directory to put artifacts into...");

    let artifacts_path = packdir.path().join("artifacts.tar.gz");
    let metadata_path = packdir.path().join("formfile.json");

    std::fs::write(&metadata_path, serde_json::to_string(&message.request.formfile)?)?;
    let mut file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(artifacts_path.clone())?;

    file.write_all(&message.request.artifacts)?;

    println!("Reading Formfile json metadata into Formfile struct...");
    let formfile: Formfile = std::fs::read_to_string(&metadata_path)
        .and_then(|s| serde_json::from_str(&s)
            .map_err(|_| {
                std::io::Error::from(
                    std::io::ErrorKind::InvalidData
                )
            })
        )?; 

    println!("Building FormPackMonitor for {} build...", formfile.name);
    let mut monitor = match FormPackMonitor::new().await {
        Ok(m) => m,
        Err(e) => {
            let err_msg = format!("Failed to create FormPackMonitor: {}", e);
            println!("{}", err_msg);
            write_pack_status_failed(&message, err_msg).await?;
            return Err(e);
        }
    };
    
    println!("Attempting to build image for {}...", formfile.name);
    match monitor.build_image(
        manager.node_id.clone(),
        message.request.name.clone(),
        formfile,
        artifacts_path,
    ).await {
        Ok(_) => {
            write_pack_status_completed(&message, manager.node_id.clone()).await?;
            Ok(())
        },
        Err(e) => {
            let err_msg = format!("Image build failed: {}", e);
            println!("{}", err_msg);
            write_pack_status_failed(&message, err_msg).await?;
            Err(e)
        }
    }
}
