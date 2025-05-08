use std::sync::Arc;
use std::io::Write;
use std::fs::OpenOptions;
use tokio::sync::Mutex;
use axum::{Json, extract::{State, Multipart}, Extension};
use tempfile::tempdir;
use futures::{StreamExt, TryStreamExt};
use crate::manager::FormPackManager;
use crate::types::response::PackResponse;
use crate::monitor::FormPackMonitor;
use crate::formfile::Formfile;

pub(crate) async fn handle_pack(
    State(manager): State<Arc<Mutex<FormPackManager>>>,
    mut multipart: Multipart
) -> Json<PackResponse> {
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

    println!("Building FormPackMonitor for {} build...", formfile.name);
    let mut monitor = match FormPackMonitor::new().await {
        Ok(monitor) => monitor,
        Err(e) => {
            println!("Error building monitor: {e}");
            return Json(PackResponse::Failure);
        }
    }; 

    let guard = manager.lock().await;
    let node_id = guard.node_id.clone();
    drop(guard);
    println!("Attempting to build image for {}...", formfile.name);
    match monitor.build_image(
        node_id.clone(),
        formfile.name.clone(),
        formfile,
        artifacts_path,
    ).await {
        Ok(_res) => Json(PackResponse::Success),
        Err(e) => {
            println!("Error building image: {e}");
            Json(PackResponse::Failure)
        }
    }
}
