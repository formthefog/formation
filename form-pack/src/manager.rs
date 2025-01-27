#![allow(unused_assignments)]
use std::io::{Cursor, Read, Write};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::fs::{self, File, OpenOptions};
use std::sync::Arc;
use std::thread::sleep;
use std::time::Duration;
use axum::extract::State;
use flate2::read::GzDecoder;
use futures::{StreamExt, TryStreamExt};
use tokio::sync::broadcast::Receiver;
use axum::{Router, routing::post, Json, extract::Multipart};
use bollard::container::{Config, CreateContainerOptions, DownloadFromContainerOptions, UploadToContainerOptions};
use bollard::models::{HostConfig, DeviceMapping, PortBinding};
use bollard::exec::CreateExecOptions;
use bollard::Docker;
use reqwest::Client;
use serde_json::Value;
use serde::{Serialize, Deserialize};
use tempfile::tempdir;
use tokio::sync::Mutex;
use crate::image_builder::IMAGE_PATH;
use crate::formfile::Formfile;

pub const VM_IMAGE_PATH: &str = "/var/lib/formation/vm-images/";

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FormVmmService(SocketAddr);

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum PackResponse {
    Success,
    Failure
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PackRequest {
    name: String,
    formfile: Formfile,
}

pub struct FormPackManager {
    addr: SocketAddr
}

impl FormPackManager {
    pub fn new(addr: SocketAddr) -> Self {
        Self {
            addr
        }
    }

    pub async fn run(self, mut shutdown: Receiver<()>) -> Result<(), Box<dyn std::error::Error>> {
        let addr = self.addr.to_string();
        tokio::select! {
            res = serve(&addr, self) => {
                return res
            }
            _ = shutdown.recv() => {
                eprintln!("Received shutdown signal");
                return Ok(())
            }
        }
    }
}

async fn build_routes(manager: Arc<Mutex<FormPackManager>>) -> Router {
    Router::new()
        .route("/ping", post(handle_ping))
        .route("/build", post(handle_pack))
        .with_state(manager)
}

async fn serve(addr: &str, manager: FormPackManager) -> Result<(), Box<dyn std::error::Error>> {
    println!("Building routes...");
    let routes = build_routes(Arc::new(Mutex::new(manager))).await;

    println!("binding listener to addr: {addr}");
    let listener = tokio::net::TcpListener::bind(
        addr
    ).await?;


    println!("serving server on: {addr}");
    if let Err(e) = axum::serve(listener, routes).await {
        eprintln!("Error in FormPackManager API Server: {e}");
    }

    Ok(())
}

async fn handle_ping() -> Json<Value> {
    println!("Received ping request, responding");
    Json(serde_json::json!({"ping": "pong"}))
}

async fn handle_pack(
    State(_manager): State<Arc<Mutex<FormPackManager>>>,
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
            .map_err(|_| {
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

    println!("Attmpting to build image for {}...", formfile.name);
    match monitor.build_image(
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


pub struct FormPackMonitor {
    docker: Docker,
    container_id: Option<String>,
    container_name: Option<String>,
    build_server_id: Option<String>,
    build_server_uri: String,
    build_server_client: Client,
}

impl FormPackMonitor {
    pub async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        println!("Building default monitor...");
        let mut monitor = Self {
            docker: Docker::connect_with_local_defaults()?,
            container_id: None,
            container_name: None,
            build_server_id: None,
            build_server_uri: String::new(),
            build_server_client: Client::new(),
        };

        println!("Attempting to start build container...");
        let (container_id, container_name, container_ip) = monitor.start_build_container().await?;
        monitor.container_id = Some(container_id.clone());
        monitor.container_name = Some(container_name.clone());
        monitor.build_server_uri = format!("http://{container_ip}:{}", 8080);

        Ok(monitor)
    }

    pub fn container_id(&self) -> &Option<String> {
        &self.container_id
    }

    pub async fn build_image(
        &mut self,
        formfile: Formfile,
        artifacts: PathBuf,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let container_id = self.container_id.take().ok_or(
            Box::new(
                std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "Container ID should be some by the time build_image is called"
                )
            )
        )?;
        println!("Build server for {} is {container_id}", formfile.name);

        println!("Uploading artifacts to {container_id}");
        self.upload_artifacts(&container_id, artifacts).await?;
        println!("Starting build server for {}", formfile.name);
        self.start_build_server(&container_id).await?;
        println!("Requesting image build for {}", formfile.name);
        self.execute_build(&formfile).await?;
        self.extract_disk_image(&container_id, formfile.name.clone()).await?;
        println!("Image build completed for {} successfully, cleaning up {container_id}...", formfile.name);
        self.cleanup().await?;

        Ok(())
    }

    pub async fn start_build_container(&self) -> Result<(String, String, String), Box<dyn std::error::Error>> {
        let container_name = format!("form-pack-builder-{}", uuid::Uuid::new_v4());
        let options = Some(CreateContainerOptions {
            name: container_name.clone(), 
            platform: None,
        });

        let ports = Some([(
            "8080/tcp".to_string(),
            Some(vec![PortBinding {
                host_ip: Some("0.0.0.0".to_string()),
                host_port: Some("8080".to_string())
            }]),
        )].into_iter().collect());

        let host_config = HostConfig {
            port_bindings: ports,
            devices: Some(vec![DeviceMapping {
                path_on_host: Some("/dev/kvm".to_string()),
                path_in_container: Some("/dev/kvm".to_string()),
                cgroup_permissions: Some("rwm".to_string())
            }]),
            ..Default::default()
        };

        println!("Build HostConfig: {host_config:?}");
        let config = Config {
            image: Some("form-build-server:latest"),
            cmd: None, 
            tty: Some(true),
            host_config: Some(host_config),
            ..Default::default()
        };

        println!("Build Config: {config:?}");

        println!("Calling create container...");
        let container = self.docker.create_container(options, config).await?;
        println!("Calling start container...");
        self.docker.start_container::<String>(&container.id, None).await?;
        let container_ip = self.docker.inspect_container(&container_name, None)
                .await?
                .network_settings.ok_or(
                    Box::new(
                        std::io::Error::new(
                            std::io::ErrorKind::Other,
                            "Unable to acquire container network settings"
                        )
                    )
                )?.networks.ok_or(
                    Box::new(
                        std::io::Error::new(
                            std::io::ErrorKind::Other,
                            "Unable to acquire container networks"
                        )
                    )
                )?.iter().find(|(k, _)| {
                    *k == "bridge"
                }).ok_or(
                    Box::new(
                        std::io::Error::new(
                            std::io::ErrorKind::Other,
                            "Unable to find bridge network"
                        )
                    )
                )?.1.ip_address.clone().ok_or(
                    Box::new(
                        std::io::Error::new(
                            std::io::ErrorKind::Other,
                            "Unable to find IP Address"
                        )
                    )

                )?;
        Ok((container.id, container_name, container_ip))
    }

    pub async fn upload_artifacts(
        &self,
        container_id: &str,
        artifacts: PathBuf
    ) -> Result<(), Box<dyn std::error::Error>> {
        let options = UploadToContainerOptions {
            path: "/artifacts",
            ..Default::default()
        };

        let tar_contents = fs::read(artifacts)?;
        self.docker.upload_to_container(
            container_id,
            Some(options),
            tar_contents.into()
        ).await?;

        Ok(())
    }

    pub async fn start_build_server(&mut self, container_id: &str) -> Result<(), Box<dyn std::error::Error>> {
        let exec_opts = CreateExecOptions {
            cmd: Some(vec!["form-build-server", "-p", "8080"]),
            attach_stdout: Some(true),
            attach_stderr: Some(true),
            env: Some(vec!["RUST_LOG=info"]),
            tty: Some(true),
            privileged: Some(true),
            ..Default::default()
        };

        println!("Creating exec {exec_opts:?} to run on {container_id}");
        let exec = self.docker.create_exec(container_id, exec_opts).await?;
        self.build_server_id = Some(exec.id.clone());
        println!("starting exec on {container_id}");
        self.docker.start_exec(&exec.id, None).await?;

        sleep(Duration::from_secs(2));

        let max_retries = 5;
        let mut current_retry = 0;
        let mut ping_resp = None;

        while current_retry < max_retries {
            match self.build_server_client
                .post(format!("{}/ping", self.build_server_uri))
                .send()
                .await {
                    Ok(resp) if resp.status().is_success() => {
                        ping_resp = Some(resp);
                        return Ok(())
                    }
                    _ => {
                        current_retry += 1;
                        sleep(Duration::from_secs(1));
                    }
                }
        }

        match ping_resp {
            Some(r) => {
                println!("Received response from ping: {r:?}");
                return Ok(())
            },
            None => return Err(
                Box::new(
                    std::io::Error::new(
                        std::io::ErrorKind::Other,
                        "Build server never started, no response from ping request"
                    )
                )
            )
        }

    }

    pub async fn execute_build(
        &self,
        formfile: &Formfile,
    ) -> Result<(), Box<dyn std::error::Error>> {
        println!("Sending Formfile {formfile:?} for {} to build_server: {}", formfile.name, self.build_server_uri);
        let resp = self.build_server_client
            .post(format!("{}/formfile", self.build_server_uri))
            .json(formfile)
            .send()
            .await?;

        println!("Received response: {resp:?}");

        Ok(())
    }

    pub async fn extract_disk_image(
        &self,
        container_name: &str,
        vm_name: String,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let options = Some(
            DownloadFromContainerOptions {
                path: IMAGE_PATH
            }
        );
        let mut buf = Vec::new();
        let mut stream = self.docker.download_from_container(container_name, options);
        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            buf.extend_from_slice(&chunk);
        }

        let data: Box<dyn Read> = {
            if is_gzip(&buf) {
                Box::new(GzDecoder::new(Cursor::new(buf)))
            } else {
                Box::new(Cursor::new(buf))
            }
        };

        let mut archive = tar::Archive::new(data);
        let mut entries = archive.entries()?;
        let mut num_entries = 0;

        while let Some(entry) = entries.next() {
            let mut entry = entry?;
            num_entries += 1;

            if num_entries > 1 {
                return Err(
                    Box::new(
                        std::io::Error::new(
                            std::io::ErrorKind::Other,
                            "Archive should only have 1 entry for the disk image"
                        )
                    )
                )
            }

            let output_path = format!("/var/lib/formation/vm-images/{vm_name}.raw");
            let mut output_file = File::create(output_path)?;
            std::io::copy(&mut entry, &mut output_file)?;
        }

        if num_entries == 0 {
            return Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Archive is empty"
            )))
        }
        
        return Ok(())
    }

    pub async fn cleanup(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(container_id) = self.container_id.take() {
            self.docker.stop_container(&container_id, None).await?;
            self.docker.remove_container(
                &container_id,
                None,
            ).await?;
        }

        Ok(())
    }
}

fn is_gzip(data: &[u8]) -> bool {
    data.starts_with(&[0x1F, 0x8B]) // Gzip magic number
}
