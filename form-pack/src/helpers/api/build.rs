use std::sync::Arc;
use std::io::Write;
use std::fs::OpenOptions;
use tokio::sync::Mutex;
use axum::{Json, extract::{State, Multipart}};
use tempfile::tempdir;
use futures::{StreamExt, TryStreamExt};
use tiny_keccak::{Sha3, Hasher};
use crate::{auth::RecoveredAddress, manager::FormPackManager};
use crate::types::response::PackResponse;
use crate::monitor::FormPackMonitor;
use crate::helpers::api::write::{write_pack_status_started, write_pack_status_failed, write_pack_status_completed};
use crate::formfile::Formfile;
use log::{info, error};

pub(crate) async fn handle_pack(
    State(manager): State<Arc<Mutex<FormPackManager>>>,
    recovered_address: RecoveredAddress,
    mut multipart: Multipart
) -> Json<PackResponse> {

    let address_bytes = match hex::decode(recovered_address.as_hex()) {
        Ok(addr_bytes) => addr_bytes,
        Err(e) => return Json(PackResponse::Failure),
    };


    println!("Received a multipart Form, attempting to extract data...");
    let packdir = if let Ok(td) = tempdir() {
        td
    } else {
        return Json(PackResponse::Failure);
    };
    println!("Created temporary directory to put artifacts into...");
    let artifacts_path = packdir.path().join("artifacts.tar.gz");
    let metadata_path = packdir.path().join("formfile.json");

    while let Some(field) = multipart.next_field().await.unwrap_or(None) {
        let name = field.name().unwrap_or_default();

        if name == "metadata" {
            let data = match field.text().await {
                Ok(text) => text,
                Err(_) => return Json(PackResponse::Failure)
            };

            println!("Extracted metadata field...");
            if let Err(_) = std::fs::write(&metadata_path, data) {
                return Json(PackResponse::Failure);
            }
            println!("Wrote metadata to file...");
        } else if name == "artifacts" {
            let mut file = if let Ok(f) = OpenOptions::new()
                .create(true)
                .truncate(true)
                .write(true)
                .open(packdir.path().join("artifacts.tar.gz")) {
                    f
            } else {
                return Json(PackResponse::Failure);
            };

            println!("Created file for artifacts...");
            let mut field_stream = field.into_stream();
            println!("Converted artifacts field into stream...");
            while let Some(chunk) = field_stream.next().await {
                println!("Attempting to write stream chunks into file...");
                match chunk {
                    Ok(data) => {
                        if let Err(_) = file.write_all(&data) {
                            return Json(PackResponse::Failure)
                        }
                    }
                    Err(_) => return Json(PackResponse::Failure),
                }
            }
            println!("Wrote artifacts to file...");
        }
    }

    println!("Reading metadata into Formfile struct...");
    let formfile: Formfile = match std::fs::read_to_string(&metadata_path)
        .and_then(|s| serde_json::from_str(&s)
            .map_err(|e| {
                println!("Error reading metadata: {e}");
                std::io::Error::from(
                    std::io::ErrorKind::InvalidData
                )
            })
        ) {
            Ok(ff) => ff,
            Err(_) => return Json(PackResponse::Failure)
    };

    let mut hasher = Sha3::v256();
    let mut hash = [0u8; 32];
    hasher.update(&address_bytes);
    hasher.update(formfile.clone().name.as_bytes());
    hasher.finalize(&mut hash);

    let build_id_hex = hex::encode(hash);

    info!("Building FormPackMonitor for agent name: {}, calculated build_id_hex: {}", formfile.name, build_id_hex);
    let mut monitor = match FormPackMonitor::new().await {
        Ok(monitor) => monitor,
        Err(e) => {
            error!("(handle_pack) Error building monitor: {}", e);
            return Json(PackResponse::Failure);
        }
    }; 

    let guard = manager.lock().await;
    let node_id = guard.node_id.clone();
    drop(guard);
    
    let _ = write_pack_status_started(formfile.clone(), build_id_hex.clone(), node_id.clone(), recovered_address.as_hex()).await;
    info!("Attempting to build image for agent name: {}, using build_id_hex: {} as vm_name for monitor", formfile.name, build_id_hex);
    
    match monitor.build_image(
        node_id.clone(),
        build_id_hex.clone(),
        formfile.clone(),
        artifacts_path,
    ).await {
        Ok(_res) => {
            let _ = write_pack_status_completed(formfile.clone(), build_id_hex.clone(), node_id.clone(), recovered_address.as_hex()).await;
            Json(PackResponse::Success)
        },
        Err(e) => {
            error!("(handle_pack) Error building image: {}", e);
            let _ = write_pack_status_failed(&formfile, recovered_address.as_hex(), build_id_hex.clone(), node_id.clone(), e.to_string()).await;
            Json(PackResponse::Failure)
        }
    }
}
